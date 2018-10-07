use std::collections::HashMap;
use std::sync::{RwLockWriteGuard,Mutex,RwLock};
use futures::sync::{oneshot,mpsc};
use crate::id::{IdGen,Id};
use crate::messages::{MsgToSender,MsgToReceiver,FileInfoForStream};

pub type Tx<Msg> = mpsc::Sender<Msg>;
pub type UnboundedTx<Msg> = mpsc::UnboundedSender<Msg>;

/// Senders provide the files
pub struct Senders {
    senders: RwLock<HashMap<Id, Sender>>,
    id_gen: Mutex<IdGen>
}

impl Senders {
    pub fn new() -> Senders {
        Senders {
            senders: RwLock::new(HashMap::new()),
            id_gen: Mutex::new(IdGen::new())
        }
    }
    fn get_id(&self) -> Id {
        self.id_gen.lock().unwrap().make_id()
    }
    pub fn add(&self, sender_tx: UnboundedTx<MsgToSender>, id: Option<Id>) -> Id {
        let this_id = id.unwrap_or_else(|| self.get_id());
        self.senders.write().unwrap().insert(this_id, Sender { tx: sender_tx });
        this_id
    }
    pub fn remove(&self, sender_id: Id) -> bool {
        self.senders.write().unwrap()
            .remove(&sender_id)
            .map(|_| true)
            .unwrap_or(false)
    }
    pub fn get(&self, sender_id: Id) -> Option<Sender> {
        self.senders.read().unwrap().get(&sender_id).map(|s| s.clone())
    }
}

#[derive(Clone)]
pub struct Sender {
    pub tx: UnboundedTx<MsgToSender>,
}

/// Receivers connect to senders and ask for files
pub struct Receivers {
    receivers: RwLock<HashMap<Id, Receiver>>,
    id_gen: Mutex<IdGen>
}

impl Receivers {
    pub fn new() -> Receivers {
        Receivers {
            receivers: RwLock::new(HashMap::new()),
            id_gen: Mutex::new(IdGen::new())
        }
    }
    fn get_id(&self) -> Id {
        self.id_gen.lock().unwrap().make_id()
    }
    pub fn add(&self, sender_id: Id, receiver_tx: UnboundedTx<MsgToReceiver>, receiver_id: Option<Id>) -> Id {
        let this_id = receiver_id.unwrap_or_else(|| self.get_id());
        self.receivers.write().unwrap().insert(this_id, Receiver { tx: receiver_tx, sender_id });
        this_id
    }
    pub fn get(&self, receiver_id: Id) -> Option<Receiver> {
        self.receivers.read().unwrap().get(&receiver_id).map(|s| s.clone())
    }
    pub fn write(&self) -> ReceiversWriteLock {
        ReceiversWriteLock{ lock: self.receivers.write().unwrap() }
    }
    // pub fn get_receivers_mut(&mut self) -> impl Iterator<Item = &mut Receiver> {
    //     self.receivers.write().unwrap().values_mut()
    // }
}

#[derive(Clone)]
pub struct Receiver {
    pub tx: UnboundedTx<MsgToReceiver>,
    pub sender_id: Id
}

pub struct ReceiversWriteLock<'a> {
    lock: RwLockWriteGuard<'a, HashMap<Id, Receiver>>
}

impl <'a> ReceiversWriteLock<'a> {
    pub fn send_all(&mut self, msg: MsgToReceiver) {
        for r in self.lock.values_mut() {
            r.tx.unbounded_send(msg.clone());
        }
    }
    pub fn send_one(&mut self, receiver_id: Id, msg: MsgToReceiver) -> bool {
        match self.lock.get_mut(&receiver_id) {
            Some(r) => { r.tx.unbounded_send(msg); true },
            None => false
        }
    }
    pub fn send(&mut self, msg: MsgToReceiver, receiver_id: Option<Id>) {
        match receiver_id {
            Some(id) => { self.send_one(id, msg); },
            None => { self.send_all(msg); }
        };
    }
    pub fn remove(&mut self, receiver_id: Id) -> bool {
        self.lock.remove(&receiver_id).map(|_| true).unwrap_or(false)
    }
}

/// Streams represent a single sender-receiver-file transaction
/// Each time a download starts, a new stream is created. This asks the sender to
/// provide the requested bytes and streams them to the receiver.
pub struct Streams {
    streams: Mutex<HashMap<Id, Stream>>,
    id_gen: Mutex<IdGen>
}

impl Streams {
    pub fn new() -> Streams {
        Streams {
            streams: Mutex::new(HashMap::new()),
            id_gen: Mutex::new(IdGen::new())
        }
    }
    fn get_id(&self) -> Id {
        self.id_gen.lock().unwrap().make_id()
    }
    pub fn add(&self, stream_data: Tx<Vec<u8>>, stream_info: oneshot::Sender<FileInfoForStream>) -> Id {
        let stream_id = self.get_id();
        self.streams.lock().unwrap().insert(stream_id, Stream {
            info: Some(stream_info),
            data: Some(stream_data)
        });
        stream_id
    }
    pub fn take_info(&self, stream_id: Id) -> Option<StreamInfo> {
        match self.streams.lock().unwrap().get_mut(&stream_id) {
            Some(s) => std::mem::replace(&mut s.info, None),
            None => None
        }
    }
    pub fn take_data(&self, stream_id: Id) -> Option<StreamData> {
        match self.streams.lock().unwrap().get_mut(&stream_id) {
            Some(s) => std::mem::replace(&mut s.data, None),
            None => None
        }
    }
}

pub type StreamInfo = oneshot::Sender<FileInfoForStream>;
pub type StreamData = Tx<Vec<u8>>;

pub struct Stream {
    // These props are optional because they will be removed
    // separately from the stream and set to none.
    data: Option<StreamData>,
    info: Option<StreamInfo>
}

/// State holds everything the application needs to share
pub struct State {
    pub senders: Senders,
    pub receivers: Receivers,
    pub streams: Streams
}

impl State {
    pub fn new() -> State {
        State {
            senders: Senders::new(),
            receivers: Receivers::new(),
            streams: Streams::new()
        }
    }
}