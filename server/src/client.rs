use warp::http::{Response,status::StatusCode};
use include_dir::{Dir, include_dir, include_dir_impl};

static CLIENT_DIR: Dir = include_dir!("../client");

/// Takes the tail of a path and returns a file from our compiled-in
/// content with the correct mime-type and such, or a 404 if not found.
pub fn return_file(path: warp::path::Tail) -> impl warp::Reply {
    let file = try_get_client_file(path.as_str());
    match file {
        Some(f) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", mime_guess::guess_mime_type(f.path()).as_ref())
            .body(f.contents()),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".as_bytes())
    }
}

fn try_get_client_file(path: &str) -> Option<::include_dir::File<'static>> {
    if path.is_empty() || path == "/" {
        CLIENT_DIR.get_file("index.html")
    } else if path.ends_with("/") {
        CLIENT_DIR.get_file(format!("{}index.html", path))
    } else {
        CLIENT_DIR.get_file(path).or_else(|| CLIENT_DIR.get_file(format!("{}/index.html", path)))
    }
}