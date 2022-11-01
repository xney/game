use std::{net::IpAddr, str::FromStr};

use std::env;

pub enum GameArgs {
    /// Server mode
    Server(ServerArgs),

    /// Client mode
    Client(ClientArgs),

    None,
}

// TODO: figure out clap or at least use a Result
pub fn get_args() -> GameArgs {
    let tokens: Vec<String> = env::args().collect();

    if tokens.len() == 1 {
        return GameArgs::None;
    }
    let first = tokens.get(1).expect("missing client/server");
    if first == "client" {
        return GameArgs::Client(ClientArgs {
            port: u16::from_str(tokens.get(2).expect("missing port"))
                .expect("unable to parse port"),
            server_ip: IpAddr::from_str(tokens.get(3).expect("missing server ip"))
                .expect("unable to parse server ip"),
        });
    }
    if first == "server" {
        return GameArgs::Server(ServerArgs {
            port: u16::from_str(tokens.get(2).expect("missing port"))
                .expect("unable to parse port"),
            filename: tokens.get(3).expect("missing save filename").to_string(),
        });
    }
    panic!("first argument must be client or server");
}

pub struct ServerArgs {
    /// Port to open server on
    pub port: u16,

    /// File to load and save to
    pub filename: String,
}

pub struct ClientArgs {
    /// Address of server
    pub server_ip: IpAddr,

    /// Port of server
    pub port: u16,
}
