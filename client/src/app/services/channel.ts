export function Channel<T>() {

    let listeners: ((msg: T) => void)[] = [];

    return {
        /** Emit a message to anything subscribed to `onemit` */
        emit: function(msg: T) {
            for(const l of listeners) {
                l(msg);
            }
        },
        /** Subscribe to messages given to `emit` */
        onemit: function(fn: (msg: T) => void): () => void {
            listeners.push(fn);
            return () => {
                listeners = listeners.filter(f => f !== fn);
            }
        }
    }

}