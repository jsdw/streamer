import * as React from "react";

export interface Props {

}

export interface State {

}

export class App extends React.Component<Props,State> {

    constructor(props: Props) {
        super(props);
    }

    render = () => {
        return (
            <div>Hello World, File Streamer</div>
        );
    }

}