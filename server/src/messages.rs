use serde_derive::{Serialize,Deserialize};
use crate::id::Id;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum MsgToReceiver {
    /// Acknowledge a handshake message, giving back the ID:
    HandshakeAck { id: Id }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum MsgFromReceiver {
    /// Expected when first connected. If client already has ID they provide it:
    Hankshake { sender_id: Id, id: Option<Id> },
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum MsgToSender {
    /// Acknowledge a handshake message, giving back the ID:
    HandshakeAck { id: Id },
    /// Ask sender to upload a given file to a url defined by stream_id:
    PleaseUpload { file_id: Id, stream_id: Id },
    /// Send back error if something went wrong:
    Error { reason: String }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum MsgFromSender {
    /// Expected when first connected. If client already has ID they provide it:
    Handshake { id: Option<Id> },
    /// Notification when files have been added:
    FilesAdded { files: Vec<File> },
    /// Notification when files have been removed:
    FilesRemoved { files: Vec<File> },
    /// A list of files that the sender has:
    FileList { files: Vec<File> },
    /// Info for a file for some active stream. needed for download to begin:
    PleaseUploadAck { stream_id: Id, info: FileInfoForStream }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct FileInfoForStream {
    /// Name of the file:
    pub name: String,
    /// Size in bytes of the file:
    pub size: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct File {
    id: String,
    name: String,
    size: u64
}