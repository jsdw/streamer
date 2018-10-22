use structopt::StructOpt;
use std::path::PathBuf;

#[derive(StructOpt, Debug)]
#[structopt(
    about = "A File Streamer"
)]
pub struct Options {

    #[structopt(
        short = "a",
        long = "address",
        default_value = "0.0.0.0:8080",
        help = "network address and port to run this server on"
    )]
    pub address: std::net::SocketAddr,

    #[structopt(
        long = "client-files",
        help = "serve these files instead of the embedded client files",
        parse(from_os_str)
    )]
    pub client_files: Option<PathBuf>

}