use std::net::{SocketAddr, UdpSocket};

use bincode::{Decode, Encode};

// TODO: split this into several files

/// This is the bincode config that we should use everywhere
pub const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard()
    .with_little_endian()
    .with_variable_int_encoding()
    .write_fixed_array_length();

/// placeholders
/// TODO: remove whenever command line arguments can be parsed
const SERVER_PORT: u16 = 8888u16;
const SERVER_IP: [u8; 4] = [127, 0, 0, 1];

/// Marker trait for network structs
trait NetworkMessage: Encode + Decode {}

/// Message from the server to a client
#[derive(Encode, Decode, Debug)]
pub enum ServerToClient {
    Pong,
}

impl NetworkMessage for ServerToClient {}

/// Message from a client to the server
#[derive(Encode, Decode, Debug)]
pub enum ClientToServer {
    Ping,
}

impl NetworkMessage for ClientToServer {}

#[derive(Debug)]
enum SendError {
    IoError(std::io::Error),
    EncodeError(bincode::error::EncodeError),
    NoSuchPeer,
}

#[derive(Debug)]
enum ReceiveError {
    IoError(std::io::Error),
    DecodeError(bincode::error::DecodeError),
    UnknownSender,
    NoMessage,
}

/// Helper method for sending a message
fn send_message<M: NetworkMessage>(
    socket: &UdpSocket,
    target: SocketAddr,
    message: M,
) -> Result<(), SendError> {
    // TODO: use a buffer instead of allocating into vector
    let encoded =
        bincode::encode_to_vec(message, BINCODE_CONFIG).map_err(|e| SendError::EncodeError(e))?;
    socket
        .send_to(&encoded, target)
        .map_err(|e| SendError::IoError(e))?;
    Ok(())
}

/// Code to be used by the server
pub mod server {
    use super::*;
    use crate::states;
    use bevy::prelude::*;
    use std::net::{SocketAddr, UdpSocket};

    /// Should be used as a global resource on the server
    struct Server {
        /// UDP socket that should be used for everything
        socket: UdpSocket,
        /// Currently only 1 client supported
        /// TODO: use a vec or map to support multiple
        client: Option<ClientInfo>,
    }

    /// Information about a client
    #[derive(Debug, Copy, Clone)]
    struct ClientInfo {
        addr: SocketAddr,
    }

    impl Server {
        /// Binds the socket
        fn new(port: u16) -> Result<Self, std::io::Error> {
            let addr = SocketAddr::from((SERVER_IP, port));
            let sock = UdpSocket::bind(addr)?;

            // we want nonblocking sockets!
            sock.set_nonblocking(true)?;

            Ok(Server {
                socket: sock,
                client: None,
            })
        }

        /// For now, simply sends to the only client if it's connected
        /// TODO: take in a parameter to distinguish clients
        fn send_message(&self, message: ServerToClient) -> Result<(), SendError> {
            match &self.client {
                Some(client) => {
                    send_message(&self.socket, client.addr, message)?;
                    Ok(())
                }
                None => Err(SendError::NoSuchPeer),
            }
        }

        /// Non-blocking way to get one message from the socket
        /// TODO: loop over all clients whenever more than one is supported
        fn get_one_message(&mut self) -> Result<(ClientInfo, ClientToServer), ReceiveError> {
            // TODO: move buffer into struct
            let mut buffer = [0u8; 2048];

            // read from socket
            let (_size, addr) = self
                .socket
                .recv_from(&mut buffer)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::WouldBlock => ReceiveError::NoMessage,
                    _ => ReceiveError::IoError(e),
                })?;

            // decode
            let (message, _size) = bincode::decode_from_slice(&buffer, BINCODE_CONFIG)
                .map_err(|e| ReceiveError::DecodeError(e))?;

            // set our client to the sender of this message
            // TODO: change whenever we support more than one client
            match &mut self.client {
                Some(client) => {
                    if client.addr != addr {
                        warn!("new client: {}", addr);
                        client.addr = addr;
                    }
                }
                None => {
                    warn!("new client: {}", addr);
                    let new_client = ClientInfo { addr };
                    self.client = Some(new_client);
                }
            }

            // unwrap OK because we just set self.client
            Ok((self.client.unwrap(), message))
        }
    }

    /// Bevy plugin that implements server logic
    pub struct ServerPlugin {
        pub port: u16,
        pub filename: String
    }

    impl Plugin for ServerPlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(
                SystemSet::on_enter(states::GameState::InGame).with_system(create_server),
            )
            .add_system_set(
                SystemSet::on_update(states::GameState::InGame).with_system(server_handle_messages),
            )
            .add_system_set(
                SystemSet::on_exit(states::GameState::InGame).with_system(destory_server),
            );
        }
    }

    fn create_server(mut commands: Commands) {
        // TODO: use command line arguments for port and handle failure better
        let server = match Server::new(SERVER_PORT) {
            Ok(s) => s,
            Err(e) => panic!("Unable to create server: {}", e),
        };

        commands.insert_resource(server);
    }

    fn destory_server(mut commands: Commands) {
        commands.remove_resource::<Server>();
    }

    /// Server system
    fn server_handle_messages(mut server: ResMut<Server>) {
        loop {
            match server.get_one_message() {
                Ok((from, message)) => {
                    handle_message(&mut server, from, message);
                }
                Err(ReceiveError::NoMessage) => {
                    // break whenever we run out of messages
                    break;
                }
                Err(e) => {
                    // anything else is a "real" error that we should complain about
                    error!("server receive error: {:?}", e);
                }
            }
        }
    }

    /// Helper method to handle server game logic
    /// TODO: will probably need direct World access in the future
    fn handle_message(mut server: &mut Server, from: ClientInfo, message: ClientToServer) {
        info!("server got message from {:?}: {:?}", from, message);

        let response = match message {
            ClientToServer::Ping => ServerToClient::Pong,
        };

        // create our success string (via borrow) before moving response
        let success_str = format!("server sent to ({:?}) response: {:?}", from, response);

        match server.send_message(response) {
            Ok(()) => {
                info!("{}", success_str);
            }
            Err(e) => {
                error!("unable to send message to {:?}: {:?}", from, e);
            }
        }
    }
}

/// Code to be used by the client
pub mod client {
    use super::*;
    use crate::states;
    use bevy::prelude::*;
    use std::net::{SocketAddr, UdpSocket, IpAddr};

    /// Should be used as a global resource on the client
    struct Client {
        /// UDP socket that should be used for everything
        socket: UdpSocket,
        /// There is only ever one server we care about
        server: SocketAddr,
    }

    impl Client {
        fn new(server_address: SocketAddr) -> Result<Self, std::io::Error> {
            // port 0 means we let the OS decide
            let addr = SocketAddr::from(([0, 0, 0, 0], 0));
            let sock = UdpSocket::bind(addr)?;

            // we want nonblocking sockets!
            sock.set_nonblocking(true)?;

            Ok(Self {
                socket: sock,
                server: server_address,
            })
        }

        /// Send a message to the server
        fn send_message(&self, message: ClientToServer) -> Result<(), SendError> {
            send_message(&self.socket, self.server, message)?;
            Ok(())
        }

        /// Non-blocking way to get one message from the socket
        fn get_one_message(&mut self) -> Result<ServerToClient, ReceiveError> {
            // TODO: move buffer into struct
            let mut buffer = [0u8; 2048];

            // read from socket
            let (_size, from) = self
                .socket
                .recv_from(&mut buffer)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::WouldBlock => ReceiveError::NoMessage,
                    _ => ReceiveError::IoError(e),
                })?;

            // check if it's actually from the server
            if from != self.server {
                return Err(ReceiveError::UnknownSender);
            }

            // decode message
            let (message, _size) = bincode::decode_from_slice(&buffer, BINCODE_CONFIG)
                .map_err(|e| ReceiveError::DecodeError(e))?;

            Ok(message)
        }
    }

    pub struct ClientPlugin {
        pub server_address: IpAddr,
        pub server_port: u16
    }

    impl Plugin for ClientPlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(
                SystemSet::on_enter(states::GameState::InGame).with_system(create_client),
            )
            .add_system_set(
                SystemSet::on_update(states::GameState::InGame)
                    .with_system(client_handle_messages)
                    .with_system(p_sends_ping),
            )
            .add_system_set(
                SystemSet::on_exit(states::GameState::InGame).with_system(destroy_client),
            );
        }
    }

    fn create_client(mut commands: Commands) {
        let client = match Client::new(SocketAddr::from((SERVER_IP, SERVER_PORT))) {
            Ok(s) => s,
            Err(e) => panic!("Unable to create client: {}", e),
        };
        commands.insert_resource(client);
    }

    fn destroy_client(mut commands: Commands) {
        commands.remove_resource::<Client>();
    }

    /// simple system to make P send a ping to the server
    fn p_sends_ping(mut client: ResMut<Client>, input: Res<Input<KeyCode>>) {
        // return early if P was not pressed
        if !input.just_pressed(KeyCode::P) {
            return;
        }

        match client.send_message(ClientToServer::Ping) {
            Ok(_) => info!("client sent ping to server"),
            Err(e) => error!("failed to send ping to server: {:?}", e),
        }
    }

    fn client_handle_messages(mut client: ResMut<Client>) {
        loop {
            match client.get_one_message() {
                Ok(message) => {
                    info!("client received message: {:?}", message);
                }
                Err(ReceiveError::UnknownSender) => {
                    warn!("client got message, but not from server!");
                }
                Err(ReceiveError::NoMessage) => {
                    // no more messages at the moment
                    break;
                }
                Err(e) => {
                    error!("client receive error: {:?}", e);
                }
            }
        }
    }
}
