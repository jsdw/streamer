import * as Mode from "./mode";
import { Channel } from "./channel";
import { string } from "prop-types";

if(!document.location) {
    throw Error("document.location is null");
}

const BASE_URL = `ws://${document.location.host}`;

function MakeWebSocket(): WebSocket {
    const type = Mode.isSender ? "sender" : "receiver";
    return new WebSocket(`${BASE_URL}/api/${type}/ws`);
}

export function Socket<From, To>() {

    let ws = open();
    let hasClosed = false;
    const minRetryTime = 1000;
    const maxRetryTime = 10000;
    let retryTime = minRetryTime;

    const onclose = Channel<null>();
    const onopen = Channel<null>();
    const onerror = Channel<any>();
    const onmessage = Channel<To>();

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

export enum Status {
    CONNECTING = WebSocket.CONNECTING,
    OPEN = WebSocket.OPEN,
    CLOSING = WebSocket.CLOSING,
    CLOSED = WebSocket.CLOSED
}
