// Work out the current mode info given the document location.
// Are we a sender or a receiver? If we're a receiver, what sender
// do we want to connect to?
function getModeDetails(): ModeDetails {

    if(!document.location) {
        throw Error("no document.location, can't work out whether I'm a sender or a receiver");
    }

    const search = document.location.search
        .slice(1)
        .split("&")
        .map(piece => piece.trim().split("="))
        .reduce((o, [key,val]) => { o[key] = val; return o }, {} as {[key:string]:string});

    const senderId = search.id;

    return senderId
        ? { mode: Mode.Receiver, senderId: senderId }
        : { mode: Mode.Sender };
}

type ModeDetails
    = { readonly mode: Mode.Sender }
    | { readonly mode: Mode.Receiver, readonly senderId: string };

export enum Mode {
    Sender,
    Receiver
}

export const details = getModeDetails();
export const mode = details.mode;
export const isSender = details.mode === Mode.Sender;
export const isReceiver = details.mode === Mode.Receiver;
