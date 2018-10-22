use warp::http::{Response,status::StatusCode};
use include_dir::{Dir, include_dir, include_dir_impl};
use std::path::PathBuf;
use tokio::fs::File;
use futures::future;
use futures::future::Future;

static CLIENT_DIR: Dir = include_dir!("../client");

/// Takes the tail of a path and returns a file from our compiled-in
/// content with the correct mime-type and such, or a 404 if not found.
pub fn return_file(real_dir: &Option<PathBuf>, path: warp::path::Tail) -> Box<dyn Future<Item = impl warp::Reply, Error = warp::Rejection> + Send + Sync> {

    // return this if we don't find what we're looking for:
    let not_found = || Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not found".as_bytes().to_owned());

    // decode and obtain paths to search for the file we want:
    let paths = match urlencoding::decode(path.as_str()) {
        Ok(s) => get_paths(&s),
        Err(_) => { return Box::new(future::ok(not_found())); }
    };

    // if a "real" folder was provided, serve things from that. We use
    // futures and such to keep it all async.
    if let Some(dir) = real_dir {

        let file_futures = paths.iter().map(|path| {
            let full_path = dir.join(path);
            let full_path2 = full_path.clone();
            File::open(full_path).map(move |file| (file,full_path2))
        });

        let file_future = future::select_ok(file_futures)
            // read the winning file contents into a vector (would rather stream this):
            .and_then(|((file,full_path),_)| {
                tokio::io::read_to_end(file, vec![]).map(move |(_file,data)| (data, full_path))
            })
            // and return the file data, or a not found status if file not found:
            .then(move |res| {
                match res {
                    Ok((data,full_path)) => {
                        let res = Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", mime_guess::guess_mime_type(&full_path).as_ref())
                            .body(data);
                        Ok(res)
                    },
                    Err(_) => {
                        Ok(not_found())
                    }
                }
            });

        Box::new(file_future)

    }
    // no "real" path was provided, so serve our built-in content:
    else {

        let mut f = None;
        for path in &paths {
            f = CLIENT_DIR.get_file(path);
            if f.is_some() { break }
        }

        let res = match f {
            Some(f) => Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime_guess::guess_mime_type(f.path()).as_ref())
                .body(f.contents().to_owned()),
            None => not_found()
        };

        Box::new(future::ok(res))
    }
}

// resolve a given path into the different possible choices for it:
fn get_paths(path: &str) -> Vec<PathBuf> {
    if path.is_empty() || path == "/" {
        vec!["index.html".into()]
    } else if path.ends_with("/") {
        vec![format!("{}index.html", path).into()]
    } else {
        vec![path.into(), format!("{}/index.html", path).into()]
    }
}