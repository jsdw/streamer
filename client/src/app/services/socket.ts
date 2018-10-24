import * as Mode from "./mode";
import { MakeChannel } from "./channel";

if(!document.location) {
    throw Error("document.location is null");
}

const BASE_URL = `ws://${document.location.host}`;

function MakeWebSocket(): WebSocket {
    const type = Mode.isSender ? "sender" : "receiver";
    return new WebSocket(`${BASE_URL}/api/${type}/ws`);
}

export function MakeSocket<From, To>() {

    let ws = open();
    let hasClosed = false;
    const minRetryTime = 1000;
    const maxRetryTime = 10000;
    let retryTime = minRetryTime;

    const onclose = MakeChannel<null>();
    const onopen = MakeChannel<null>();
    const onerror = MakeChannel<any>();
    const onmessage = MakeChannel<To>();

    function open(): WebSocket {
        ws = MakeWebSocket();
        ws.onclose = () => {
            onclose.emit(null);
            retry();
            retryTime = Math.min(maxRetryTime, retryTime * 2);
        }
        ws.onopen = () => {
            onopen.emit(null);
            retryTime = minRetryTime
        }
        ws.onerror = (e) => {
            onerror.emit(e);
        }
        ws.onmessage = (e) => {
            const msg: To = JSON.parse(e.data);
            onmessage.emit(msg);
        }
        return ws;
    }

    function retry() {
        if(hasClosed) return;
        hasClosed = true;
        let thisRetryTime = retryTime;
        setTimeout(() => { hasClosed = false; open() }, thisRetryTime);
    }

    return {
        /** try to send a message through the socket (will fail if it's not open) */
        send: (msg: From) => ws.send(JSON.stringify(msg)),
        /** notify when the socket becomes ready to send messages */
        onopen: onopen.onemit,
        /** notify when an error happens on the socket */
        onerror: onerror.onemit,
        /** notify when the socket is closed (it will auto re-open eventually when possible) */
        onclose: onclose.onemit,
        /** receive incoming messages */
        onmessage: onmessage.onemit,
        /** the current status of the socket. Should equal one of the constants below */
        status: () => ws.readyState as Status,
    }

}

// A bit of hackery to return a generic type describing what we
// get back from the above:
class _s_<From,To> { sock = MakeSocket<From,To>(); }
export type Socket<From,To> = _s_<From,To>['sock'];

export enum Status {
    CONNECTING = WebSocket.CONNECTING,
    OPEN = WebSocket.OPEN,
    CLOSING = WebSocket.CLOSING,
    CLOSED = WebSocket.CLOSED
}
