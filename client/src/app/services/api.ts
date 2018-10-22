import { MODE } from "./mode";

if(!document.location) {
    throw Error("document.location is null");
}

const BASE_URL = `${document.location.protocol}//${document.location.host}/`;

export function Api() {

    const ws = new WebSocket(`${BASE_URL}api/ws`);




}