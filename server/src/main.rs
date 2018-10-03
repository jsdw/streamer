#![recursion_limit="1024"]

mod client;
mod id;
mod state;
mod messages;

use serde_derive::{Serialize,Deserialize};
use futures::{future, Future, Sink, Stream, sync::{oneshot,mpsc}};
use warp::{path, Filter, ws::{Message,WebSocket}};
use warp::http::{Response,status::StatusCode};
use std::sync::{Arc, RwLock};
use derive_more::{FromStr,Display};
use hyper::Body;

use crate::messages::{MsgToReceiver, MsgFromReceiver, MsgToSender, MsgFromSender};
use crate::id::Id;

#[derive(FromStr)]
struct FileId(Id);

#[derive(FromStr)]
struct SenderId(Id);

#[derive(FromStr)]
struct StreamId(Id);

type State = Arc<state::State>;

fn main() {

    // Make some shared state available in every route that needs it:
    let state: State = Arc::new(state::State::new());
    let with_state = move || {
        let s = state.clone();
        warp::any().map(move || s.clone())
    };

    // WS /api/sender/ws
    let api_sender_ws = path!("api" / "sender" / "ws")
        .and(warp::ws2())
        .and(with_state())
        .map(|ws: warp::ws::Ws2, state: State| {
            ws.on_upgrade(|websocket| {
                handle_sender_ws(websocket, state)
            })
        });

    // WS /api/receiver/ws
    let api_receiver_ws = path!("api" / "receiver" / "ws")
        .and(warp::ws2())
        .and(with_state())
        .map(|ws: warp::ws::Ws2, state: State| {
            ws.on_upgrade(|websocket| {
                handle_receiver_ws(websocket, state)
            })
        });

    // upload files to sender
    let api_upload = path!("api" / "upload" / StreamId)
        .and(warp::post2())
        .and(warp::filters::body::stream())
        .and(with_state())
        .and_then(handle_upload);

    // Download files from sender
    let api_download = path!("api" / "download" / SenderId / FileId)
        .and(warp::get2())
        .and(with_state())
        .and_then(handle_download);

    // GET client files
    let other = warp::get2()
        .and(warp::path::tail())
        .map(client::return_file);

    // put our routes together and serve them:
    let routes = api_sender_ws
        .or(api_receiver_ws)
        .or(api_upload)
        .or(api_download)
        .or(other);

    println!("Starting server!");
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));

}

fn handle_upload<S, B>(stream_id: StreamId, body: S, state: State) -> Result<impl warp::Reply, warp::Rejection>
    where
        S: Stream<Item = B, Error = warp::Error> + Send + 'static,
        B: bytes::Buf
{

    // find the stream we want to pipe to. If it does not exist, bail out with a 404.
    let stream_id = stream_id.0;
    let stream = match state.take_stream(stream_id) {
        Some(s) => s,
        None => return Err(warp::reject::not_found())
    };

    // Turn our stream of bytes into the format we want to send:
    let bytes = body
        .map(|chunk| chunk.bytes().to_owned())
        .map_err(|e| Err::new(format!["Stream error: {}", e]));

    // Stream the bytes to the receiving end, only finishing when it's complete:
    let s = stream.data
        .sink_map_err(|e| Err::new(format!["Send error: {}", e]))
        .send_all(bytes)
        .and_then(|_| Ok("Transfer successful"))
        .into_stream();

    // Return the stream, which hopefully will resolve into a body message:
    let res = Response::builder()
        .status(200)
        .body(Body::wrap_stream(s))
        .unwrap();

    Ok(res)

}

fn handle_download(sender_id: SenderId, file_id: FileId, state: State) -> impl Future<Item = impl warp::Reply, Error = warp::Rejection> {

    let sender_id = sender_id.0;
    let file_id = file_id.0;

    let (stream_data, data_receiver) = mpsc::channel(0);
    let (stream_info, info_receiver) = oneshot::channel();

    let stream_id = state.add_stream(state::Stream {
        data: stream_data,
        info: stream_info
    });

    let sender = match state.get_sender(sender_id) {
        Some(s) => s,
        None => return future::Either::A(future::err(warp::reject::not_found()))
    };

    let msg = MsgToSender::PleaseUpload {
        file_id: file_id,
        stream_id: stream_id
    };

    let res = sender.tx
        .send(msg)
        .map_err(|e| warp::reject::server_error().with(e))
        .and_then(|_| info_receiver.map_err(|e| warp::reject::server_error().with(e)))
        .and_then(|stream_info| {

            let body_stream = data_receiver.map_err(|()| Err::boxed_never());
            let name = stream_info.name;
            let size = stream_info.size;

            // stream the response back to the receiver:
            let res = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime_guess::guess_mime_type(&name).as_ref())
                .header("content-length", size)
                .body(Body::wrap_stream(body_stream));

            Ok(res)

        });

    future::Either::B(res)

}

fn handle_sender_ws(ws: WebSocket, state: State) -> impl Future<Item = (), Error = ()> {

    // Get hold of a transmitter and receiver of messages. Forward from
    // a channel so that we can clone the messages_to_sender side and pass it around:
    let (tx, messages_from_sender) = ws.split();
    let (messages_to_sender, rx) = mpsc::channel(0);
    let shared_sender_id = Arc::new(RwLock::new(None as Option<id::Id>));

    let shared_sender_id2 = shared_sender_id.clone();
    let state2 = state.clone();

    let f1 = tx.sink_map_err(|_| ()).send_all(rx);
    let f2 = messages_from_sender
        // Each time a message comes in, handle it:
        .for_each(move |msg| {

            let msg = match msg.to_str() {
                Ok(s) => s,
                Err(_) => return Ok(())
            };

            match serde_json::from_str(msg) {
                Ok(msg) => handle_sender_message(shared_sender_id.clone(), state.clone(), msg),
                Err(e) => eprintln!("Could not decode message: {}",e)
            }

            Ok(())

        })
        // When the connection is closed, for_each ends and we clean up:
        .then(move |res| {
            shared_sender_id2.read().unwrap().map(|sender_id| {
                state2.remove_sender(sender_id);
            });
            res
        })
        // Catch and report any errors:
        .map_err(|e| {
            eprintln!("Websocket error from sender: {}", e);
        });

    // Run our stream and our channel forwarding futures until completion:
    f1.join(f2).map(|_| ())
}

fn handle_sender_message(_id: Arc<RwLock<Option<id::Id>>>, state: State, msg: MsgFromSender) {
    use self::messages::MsgFromSender::*;
    match msg {
        Handshake { id: _maybeId } => {

            // if ID provided and no sender using it already, use that, else
            // make a new one. reply to the sender with this ID

        },
        FileInfoForStream { stream_id: _, info: _ } => {

            // find relevant stream ID and push the info onto the oneshot provided.

        },
        FileList { files: _ } => {

            // forward on to receivers

        },
        FilesAdded { files: _ } => {

            // forward on

        },
        FilesRemoved { files: _ } => {

            // forward on

        }
    }
}

fn handle_receiver_ws(ws: WebSocket, _state: State) -> impl Future<Item = (), Error = ()> {

    let (_tx, messages_from_receiver) = ws.split();
    // let this_id = state.write().unwrap().add_receiver(tx);

    messages_from_receiver
        // Each time a message comes in, handle it:
        .for_each(|_msg| {

            Ok(())
        })
        // When the connection is closed, for_each ends
        // and this will run:
        .then(move |res| {
            // state.write().unwrap().remove_receiver(&this_id);
            res
        })
        // Catch and report any errors:
        .map_err(|e| {
            eprintln!("Websocket error from receiver: {}", e);
        })
}

fn send_message<Msg: serde::Serialize>(mut tx: impl Sink<SinkItem = Message>, msg: &Msg) {
    let json = serde_json::to_string(msg).expect("should encode");
    let _ = tx.start_send(Message::text(json));
}

#[derive(Display, Debug)]
struct Err {
    msg: String
}

impl Err {
    pub fn new(s: impl Into<String>) -> Err {
        Err { msg: s.into() }
    }
    pub fn boxed(s: impl Into<String>) -> Box<Err> {
        Box::new(Err::new(s))
    }
    pub fn boxed_never() -> Box<Err> {
        Box::new(Err { msg: String::new() })
    }
}

impl std::error::Error for Err {
    fn description(&self) -> &str {
        &self.msg
    }
}