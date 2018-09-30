use warp::ws::WebSocket;
use std::collections::HashMap;
use crate::id::{IdGen,Id};

pub type Tx = futures::stream::SplitSink<WebSocket>;

pub struct Sender {
    tx: Tx,
}

pub struct Receiver {
    tx: Tx,
    sender_id: Id
}

pub struct State {
    senders: HashMap<Id, Sender>,
    receivers: HashMap<Id, Receiver>,
    id_gen: IdGen
}

impl State {
    pub fn new() -> State {
        State {
            senders: HashMap::new(),
            receivers: HashMap::new(),
            id_gen: IdGen::new()
        }
    }
    pub fn add_sender(&mut self, sender_tx: Tx, id: Option<Id>) -> Id {
        let this_id = id.unwrap_or_else(|| self.id_gen.make_id());
        self.senders.insert(this_id, Sender { tx: sender_tx });
        this_id
    }
    pub fn add_receiver(&mut self, sender_id: Id, receiver_tx: Tx, receiver_id: Option<Id>) -> Id {
        let this_id = receiver_id.unwrap_or_else(|| self.id_gen.make_id());
        self.receivers.insert(this_id, Receiver { tx: receiver_tx, sender_id });
        this_id
    }
    pub fn remove_sender(&mut self, sender_id: Id) -> bool {
        self.senders
            .remove(&sender_id)
            .map(|_| true)
            .unwrap_or(false)
    }
    pub fn remove_receiver(&mut self, receiver_id: Id) -> bool {
        self.receivers
            .remove(&receiver_id)
            .map(|_| true)
            .unwrap_or(false)
    }
}