import { Socket } from "./socket";
import { mode, Mode } from  "./mode";
import { any } from "prop-types";

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

// This is a hack to be able to talk about specific types of
// socket without having to expose the type of Socket from its
// package. These functions arent used anywhere, just the types
// we infer from them:
const anySocket = () => Socket<any,any>();
type SocketAny = ReturnType<typeof anySocket>;
const recvSocket = () => Socket<MsgToReceiver,MsgFromReceiver>();
type SocketRecv = ReturnType<typeof recvSocket>;
const sendSocket = () => Socket<MsgToSender,MsgFromSender>();
type SocketSend = ReturnType<typeof sendSocket>;

// Get a socket or use the cached one:
let socket: SocketAny|null = null;
function getSocket(): SocketAny {
    return socket || (socket = Socket());
}

// Type the socket based on the mode we provide to get hold of it:
export function Api(mode: Mode.Receiver): SocketRecv;
export function Api(mode: Mode.Sender): SocketSend;
export function Api(_: Mode): SocketAny {
    return getSocket();
}