import * as React from "react";
import { Api } from "./services/api";

export interface Props {

}

export interface State {

}

export class App extends React.Component<Props,State> {

    constructor(props: Props) {
        super(props);
    }

    componentDidMount = () => {

        // just to get api etc imported and checked:
        let a = Api(1);
        a.send({ type: "Handshake", id: null });

    }

    render = () => {
        return (
            <div>Hello World, File Streamer</div>
        );
    }

}