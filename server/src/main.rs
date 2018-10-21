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

use crate::messages::{MsgToReceiver, MsgToSender};
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
    let stream_data = match state.streams.take_data(stream_id) {
        Some(s) => s,
        None => return Err(warp::reject::not_found())
    };

    // Turn our stream of bytes into the format we want to send:
    let bytes = body
        .map(|chunk| chunk.bytes().to_owned())
        .map_err(|e| Err::new(format!["Stream error: {}", e]));

    // Stream the bytes to the receiving end, only finishing when it's complete:
    let s = stream_data
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

    let stream_id = state.streams.add(stream_data, stream_info);

    let sender = match state.senders.get(sender_id) {
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

    // Get hold of a transmitter and receiver of messages:
    let (tx, messages_from_sender) = ws.split();

    // Make an unbounded channel that takes MsgToSender:
    let (messages_to_sender, rx) = mpsc::unbounded();

    // convert rx to websocket messages and pipe to tx:
    let pipe = with_serialized_sink(tx).sink_map_err(|_| ()).send_all(rx);

    // keep track of sender ID, once it's known, here:
    let shared_sender_id = Arc::new(RwLock::new(None as Option<id::Id>));

    // clones to move into "then" closure:
    let shared_sender_id2 = shared_sender_id.clone();
    let state2 = state.clone();

    // handle each message we receive from the sender:
    let from_sender = messages_from_sender
        // Catch and report any errors:
        .map_err(|e| {
            eprintln!("Websocket error from sender: {}", e);
        })
        // Each time a message comes in, handle it:
        .for_each(move |msg| {

            let maybe_sender_id = shared_sender_id.read().unwrap().clone();

            println!("From sender {}: {:?}", maybe_sender_id.unwrap_or_else(Id::none), msg);

            let msg_str = msg.to_str().unwrap_or("");
            let msg = match serde_json::from_str(msg_str) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error decoding message {}: {}", msg_str, e);
                    return Ok(())
                }
            };

            let send_message = |msg: MsgToReceiver, receiver_id: Option<Id>| {
                if let Some(receiver_id) = receiver_id {
                    state.receivers.write().send_one(receiver_id, msg);
                } else if let Some(sender_id) = maybe_sender_id {
                    state.receivers.write().send_if(msg, |r| r.sender_id == sender_id);
                }
            };

            use crate::messages::MsgFromSender::*;
            match msg {
                Handshake { id: maybe_id } => {
                    match maybe_sender_id {
                        Some(current_id) => {
                            // If we have done a handshake, don't allow another one and return the same ID.
                            // there is no reason we should want to re-handshake unless we lose our connection..
                            let _ = messages_to_sender.unbounded_send(MsgToSender::HandshakeAck{ id: current_id });
                        },
                        None => {
                            let sender_id = state.senders.add(messages_to_sender.clone(), maybe_id);
                            *shared_sender_id.write().unwrap() = Some(sender_id);
                            let _ = messages_to_sender.unbounded_send(MsgToSender::HandshakeAck{ id: sender_id });
                        }
                    }

                },
                PleaseUploadAck { stream_id, info } => {
                    if let Some(chan) = state.streams.take_info(stream_id) {
                        let _ = chan.send(info);
                    }
                },
                FilesAdded { receiver_id, files } => {
                    send_message(MsgToReceiver::FilesAdded { files }, receiver_id);
                },
                FilesRemoved { receiver_id, files } => {
                    send_message(MsgToReceiver::FilesRemoved { files }, receiver_id);
                },
                FileList { receiver_id, files } => {
                    send_message(MsgToReceiver::FileList { files }, receiver_id);
                }
            }

            Ok(())

        })
        // When the connection is closed, for_each ends and we clean up:
        .then(move |res| {
            if let Some(sender_id) = *shared_sender_id2.read().unwrap() {
                state2.senders.remove(sender_id);
            }
            res
        });

    // Run our stream and our channel forwarding futures until completion:
    from_sender.join(pipe).map(|_| ())
}

fn handle_receiver_ws(ws: WebSocket, state: State) -> impl Future<Item = (), Error = ()> {

    // Get hold of a transmitter and receiver of messages:
    let (tx, messages_from_receiver) = ws.split();

    // Make an unbounded channel that takes MsgToSender:
    let (messages_to_receiver, rx) = mpsc::unbounded();

    // convert rx to websocket messages and pipe to tx:
    let pipe = with_serialized_sink(tx).sink_map_err(|_| ()).send_all(rx);

    // keep track of sender ID and receiver ID, once it's known, here:
    let shared_ids = Arc::new(RwLock::new(None));

    // clones to move into "then" closure:
    let shared_ids2 = shared_ids.clone();
    let state2 = state.clone();

    // handle each message we receive from the sender:
    let from_sender = messages_from_receiver
        // Catch and report any errors:
        .map_err(|e| {
            eprintln!("Websocket error from sender: {}", e);
        })
        // Each time a message comes in, handle it:
        .for_each(move |raw_msg| {

            let maybe_receiver_id = shared_ids.read().unwrap().clone().map(|(_, r)| r);

            println!("From receiver {}: {:?}", maybe_receiver_id.unwrap_or_else(Id::none), raw_msg);

            let msg_str = raw_msg.to_str().unwrap_or("");
            let msg = match serde_json::from_str(msg_str) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error decoding message {}: {}", msg_str, e);
                    return Ok(())
                }
            };

            use crate::messages::MsgFromReceiver::*;
            match msg {
                Handshake { sender_id, id: maybe_id } => {
                    match maybe_receiver_id {
                        Some(current_receiver_id) => {
                            // If we have done a handshake, don't allow another one and return the same ID.
                            // there is no reason we should want to re-handshake unless we lose our connection..
                            let _ = messages_to_receiver.unbounded_send(MsgToReceiver::HandshakeAck{ id: current_receiver_id });
                        },
                        None => {
                            let receiver_id = state.receivers.add(sender_id, messages_to_receiver.clone(), maybe_id);
                            *shared_ids.write().unwrap() = Some((sender_id, receiver_id));
                            let _ = messages_to_receiver.unbounded_send(MsgToReceiver::HandshakeAck{ id: receiver_id });
                        }
                    }

                },
                PleaseUpload { file_id, stream_id } => {
                    if let Some((sender_id, _receiver_id)) = *shared_ids.read().unwrap() {
                        state.senders.send(sender_id, MsgToSender::PleaseUpload{ file_id, stream_id });
                    }
                },
                PleaseFileList => {
                    if let Some((sender_id, receiver_id)) = *shared_ids.read().unwrap() {
                        state.senders.send(sender_id, MsgToSender::PleaseFileList{ receiver_id });
                    }
                }
            }

            Ok(())

        })
        // When the connection is closed, for_each ends and we clean up:
        .then(move |res| {
            if let Some((_sender_id, receiver_id)) = *shared_ids2.read().unwrap() {
                state2.receivers.write().remove(receiver_id);
            }
            res
        });

    // Run our stream and our channel forwarding futures until completion:
    from_sender.join(pipe).map(|_| ())
}

fn with_serialized_sink<InSink, I, E>(tx: InSink) -> impl Sink<SinkItem = I, SinkError = E>
    where
        InSink: Sink<SinkItem = Message, SinkError = E>,
        I: serde::Serialize,
 {
    tx.with(|input: I| {
        let bytes = serde_json::to_string(&input).expect("should encode");
        Ok(Message::text(bytes))
    })
}

#[derive(Display, Debug, Clone)]
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