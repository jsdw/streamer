use warp::ws::WebSocket;
use std::collections::HashMap;
use crate::id::{IdGen,Id};
use std::sync::{Mutex, RwLock};

pub type Tx = futures::stream::SplitSink<WebSocket>;

pub struct Sender {
    tx: Tx,
}

pub struct Receiver {
    tx: Tx,
    sender_id: Id
}

pub struct State {
    senders: RwLock<HashMap<Id, Sender>>,
    receivers: RwLock<HashMap<Id, Receiver>>,
    // Each time a download starts, a new stream is created. This asks the sender to
    // provide the requested bytes and streams them to the receiver.
    streams: Mutex<HashMap<Id, hyper::body::Sender>>,
    id_gen: Mutex<IdGen>
}

impl State {
    pub fn new() -> State {
        State {
            senders: RwLock::new(HashMap::new()),
            receivers: RwLock::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
            id_gen: Mutex::new(IdGen::new())
        }
    }
    pub fn get_id(&self) -> Id {
        self.id_gen.lock().unwrap().make_id()
    }
    pub fn add_sender(&self, sender_tx: Tx, id: Option<Id>) -> Id {
        let this_id = id.unwrap_or_else(|| self.get_id());
        self.senders.write().unwrap().insert(this_id, Sender { tx: sender_tx });
        this_id
    }
    pub fn add_receiver(&self, sender_id: Id, receiver_tx: Tx, receiver_id: Option<Id>) -> Id {
        let this_id = receiver_id.unwrap_or_else(|| self.get_id());
        self.receivers.write().unwrap().insert(this_id, Receiver { tx: receiver_tx, sender_id });
        this_id
    }
    pub fn remove_sender(&self, sender_id: Id) -> bool {
        self.senders.write().unwrap()
            .remove(&sender_id)
            .map(|_| true)
            .unwrap_or(false)
    }
    pub fn remove_receiver(&self, receiver_id: Id) -> bool {
        self.receivers.write().unwrap()
            .remove(&receiver_id)
            .map(|_| true)
            .unwrap_or(false)
    }
    pub fn add_stream_sender(&self, sender: hyper::body::Sender) -> Id {
        let stream_id = self.get_id();
        self.streams.lock().unwrap().insert(stream_id, sender);
        stream_id
    }
    pub fn take_stream_sender(&self, stream_id: Id) -> Option<hyper::body::Sender> {
        self.streams.lock().unwrap().remove(&stream_id)
    }
}