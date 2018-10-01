use std::collections::HashMap;
use std::sync::{Mutex,RwLock};
use futures::sync::mpsc;
use crate::id::{IdGen,Id};
use crate::messages::{MsgToSender,MsgToReceiver};

pub type Tx<Msg> = mpsc::Sender<Msg>;

#[derive(Clone)]
pub struct Sender {
    pub tx: Tx<MsgToSender>,
}

#[derive(Clone)]
pub struct Receiver {
    pub tx: Tx<MsgToReceiver>,
    pub sender_id: Id
}

pub struct State {
    senders: RwLock<HashMap<Id, Sender>>,
    receivers: RwLock<HashMap<Id, Receiver>>,
    // Each time a download starts, a new stream is created. This asks the sender to
    // provide the requested bytes and streams them to the receiver.
    streams: Mutex<HashMap<Id, Tx<Vec<u8>>>>,
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
    pub fn add_sender(&self, sender_tx: Tx<MsgToSender>, id: Option<Id>) -> Id {
        let this_id = id.unwrap_or_else(|| self.get_id());
        self.senders.write().unwrap().insert(this_id, Sender { tx: sender_tx });
        this_id
    }
    pub fn add_receiver(&self, sender_id: Id, receiver_tx: Tx<MsgToReceiver>, receiver_id: Option<Id>) -> Id {
        let this_id = receiver_id.unwrap_or_else(|| self.get_id());
        self.receivers.write().unwrap().insert(this_id, Receiver { tx: receiver_tx, sender_id });
        this_id
    }
    pub fn get_sender(&self, sender_id: Id) -> Option<Sender> {
        self.senders.read().unwrap().get(&sender_id).map(|s| s.clone())
    }
    pub fn get_receiver(&self, receiver_id: Id) -> Option<Receiver> {
        self.receivers.read().unwrap().get(&receiver_id).map(|s| s.clone())
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
    pub fn add_stream_sender(&self, sender: Tx<Vec<u8>>) -> Id {
        let stream_id = self.get_id();
        self.streams.lock().unwrap().insert(stream_id, sender);
        stream_id
    }
    pub fn take_stream_sender(&self, stream_id: Id) -> Option<Tx<Vec<u8>>> {
        self.streams.lock().unwrap().remove(&stream_id)
    }
}