import { MakeSocket, Socket } from "./socket";
import { Mode } from  "./mode";

type Id = string;

type MsgToReceiver
    = { type: "HandshakeAck", id: Id }
    | { type: "FilesAdded", files: File[] }
    | { type: "FilesRemoved", files: File[] }
    | { type: "FileList", files: File[] };

type MsgFromReceiver
    = { type: "Handshake", id: Id|null }
    | { type: "PleaseUpload", file_id: Id, stream_id: Id }
    | { type: "PleaseFileList" };

type MsgToSender
    = { type: "HandshakeAck", id: Id }
    | { type: "PleaseUpload", file_id: Id, stream_id: Id }
    | { type: "PleaseFileList", receiver_id: Id };

type MsgFromSender
    = { type: "Handshake", id: Id|null }
    | { type: "FilesAdded", receiver_id: Id|null, files: File[] }
    | { type: "FilesRemoved", receiver_id: Id|null, files: File[] }
    | { type: "FileList", receiver_id: Id|null, files: File[] }
    | { type: "PleaseUploadAck", stream_id: Id, info: FileInfoForStream };

type FileInfoForStream = {
    name: string,
    size: number
}

type File = {
    id: Id,
    name: string,
    size: number
};

// Get a socket or use the cached one:
let socket: Socket<any,any>|null = null;
function getSocket(): Socket<any,any> {
    return socket || (socket = MakeSocket());
}

// Type the socket based on the mode we provide to get hold of it:
// need to type guard on mode to use the correct API, or specify mode by hand.
export function Api(mode: Mode.Receiver): Socket<MsgFromReceiver,MsgToReceiver>;
export function Api(mode: Mode.Sender): Socket<MsgFromSender,MsgToSender>;
export function Api(_: Mode): Socket<any,any> {
    return getSocket();
}