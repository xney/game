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

/// Default size of allocated bodies vec, larger numbers may help reduce reallocation
const DEFAULT_BODIES_VEC_CAPACITY: usize = 10;

/// How many frames does a client have to not respond for before the server assumes it's dead
const FRAME_DIFFERENCE_BEFORE_DISCONNECT: u64 = 60 * 5;

/// Marker trait for network structs
trait NetworkMessage: Encode + Decode {}

/// Message from the server to a client
#[derive(Encode, Decode, Debug)]
pub struct ServerToClient {
    header: ServerHeader,
    body: Vec<ServerBodyElem>,
}

/// Header for ServerToClient message
#[derive(Encode, Decode, Debug)]
pub struct ServerHeader {
    /// Sequence/tick number
    sequence: u64,
}

/// One element (message) for the body of a ServerToClient message
#[derive(Encode, Decode, Debug, Clone)]
pub enum ServerBodyElem {
    Pong(u64), // contains sequence number of ping
}

impl NetworkMessage for ServerToClient {}

/// Message from a client to the server
#[derive(Encode, Decode, Debug)]
pub struct ClientToServer {
    header: ClientHeader,
    body: Vec<ClientBodyElem>,
}

/// Header for ClientToServer message
#[derive(Encode, Decode, Debug)]
pub struct ClientHeader {
    /// Client's current sequence/tick number
    /// TODO: is this ever useful?
    current_sequence: u64,
    /// Last received sequence/tick number
    last_received_sequence: u64,
}

/// One element (message) for the body of a ClientToServer message
#[derive(Encode, Decode, Debug, Clone)]
pub enum ClientBodyElem {
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

    const NETWORK_TICK_DELAY: u64 = 60;

    /// Should be used as a global resource on the server
    struct Server {
        /// UDP socket that should be used for everything
        socket: UdpSocket,
        /// Currently only 1 client supported
        /// TODO: use a vec or map to support multiple
        client: Option<ClientInfo>,
        /// The current sequence/tick number
        sequence: u64,
    }

    /// Information about a client
    #[derive(Debug)]
    struct ClientInfo {
        /// The socket address
        addr: SocketAddr,
        /// The last confirmed sequence number
        last_ack: u64,
        /// Body elements that we build up
        bodies: Vec<ServerBodyElem>,
        /// How many frames until we drop it
        until_drop: u64,
    }

    impl ClientInfo {
        fn new(addr: SocketAddr) -> Self {
            ClientInfo {
                addr,
                last_ack: 0,
                bodies: Vec::with_capacity(DEFAULT_BODIES_VEC_CAPACITY),
                until_drop: FRAME_DIFFERENCE_BEFORE_DISCONNECT,
            }
        }
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
                sequence: 1u64,
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
        fn get_one_message(&mut self) -> Result<(&mut ClientInfo, ClientToServer), ReceiveError> {
            // TODO: move buffer into struct
            let mut buffer = [0u8; 2048];

            // read from socket
            let (_size, sender_addr) = self
                .socket
                .recv_from(&mut buffer)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::WouldBlock => ReceiveError::NoMessage,
                    _ => ReceiveError::IoError(e),
                })?;

            // decode
            let (message, _size) = bincode::decode_from_slice(&buffer, BINCODE_CONFIG)
                .map_err(|e| ReceiveError::DecodeError(e))?;

            // TODO: change whenever we support more than one client
            // if the client doesn't match the one we have
            if match &self.client {
                Some(client) => client.addr != sender_addr,
                None => true,
            } {
                // (re)set the client to the most recent
                self.client = Some(ClientInfo::new(sender_addr));
            }

            // unwrap OK because we just set self.client or it was already a Some
            Ok((self.client.as_mut().unwrap(), message))
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
                SystemSet::on_update(states::GameState::InGame)
                    .with_system(increase_tick)
                    .with_system(server_handle_messages.after(increase_tick))
                    .with_system(send_all_messages.after(server_handle_messages))
                    .with_system(drop_disconnected_clients.after(send_all_messages)),
            )
            .add_system_set(
                SystemSet::on_exit(states::GameState::InGame).with_system(destroy_server),
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

    fn destroy_server(mut commands: Commands) {
        commands.remove_resource::<Server>();
    }

    /// Server increase tick count
    fn increase_tick(mut server: ResMut<Server>) {
        server.sequence += 1;
    }

    /// Server system
    fn server_handle_messages(mut server: ResMut<Server>) {
        loop {
            // handle all messages on our socket
            match server.get_one_message() {
                Ok((client, message)) => {
                    compute_new_bodies(client, message);
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

    /// Process a client's message and push new bodies to the next packet sent to the client
    /// TODO: will probably need direct World access in the future
    fn compute_new_bodies(client: &mut ClientInfo, message: ClientToServer) {
        info!("server got message from {:?}: {:?}", client, message);

        // this message is in-order
        // TODO: whenever the clients send inputs, ignore any that are out of order
        // i.e. only use the most recent input
        if message.header.last_received_sequence > client.last_ack {
            client.last_ack = message.header.last_received_sequence;
            client.bodies.clear();

            // reset its drop timer
            client.until_drop = FRAME_DIFFERENCE_BEFORE_DISCONNECT;
        }

        // compute our responses
        let mut body_elems: Vec<ServerBodyElem> = message
            .body
            .iter()
            // match client bodies to server bodies
            .map(|elem| match elem {
                ClientBodyElem::Ping => Some(ServerBodyElem::Pong(message.header.current_sequence)),
            })
            // ignore any Nones
            .filter(|response| response.is_some())
            // we are left with all Somes, so we can unwrap them safely
            .map(|some| some.unwrap())
            .collect();

        // info!(
        //     "server responses += {}",
        //     // debug format of all new elems
        //     body_elems.iter().fold(String::new(), |mut acc, s| {
        //         acc.push_str(&format!("({:?}) ", s));
        //         acc
        //     })
        // );

        // queue up our responses to be sent our in the next packet
        client.bodies.append(&mut body_elems);

        // only keep pongs that are in response to a ping newer than or equals to the client's last_ack
        client.bodies.retain(|elem| match elem {
            ServerBodyElem::Pong(seq) => *seq >= client.last_ack,
        });
    }

    fn send_all_messages(server: ResMut<Server>) {
        // TODO: remove
        // only send out once every x frames
        if server.sequence % NETWORK_TICK_DELAY != 0 {
            return;
        }

        // TODO: loop over clients whenever more than one are supported
        if let Some(client_info) = &server.client {
            let message = ServerToClient {
                header: ServerHeader {
                    sequence: server.sequence,
                },
                body: client_info.bodies.clone(),
            };

            // form message via borrow before consuming it
            let success_msg = format!(
                "server sent message to {:?}: {:?}",
                client_info.addr, message
            );
            match server.send_message(message) {
                Ok(_) => info!("{}", success_msg),
                Err(e) => error!("server unable to send message: {:?}", e),
            }
        }
    }

    fn drop_disconnected_clients(mut server: ResMut<Server>) {
        // TODO: remove
        if server.sequence % NETWORK_TICK_DELAY != 0 {
            return;
        }
        // TODO: loop over all clients once supported
        if let Some(client) = &mut server.client {
            if client.until_drop < NETWORK_TICK_DELAY {
                // drop the client
                warn!("dropping client!");
                server.client = None;
            } else {
                client.until_drop -= NETWORK_TICK_DELAY;
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

    const NETWORK_TICK_DELAY: u64 = 60;

    /// Should be used as a global resource on the client
    #[derive(Debug)]
    struct Client {
        /// UDP socket that should be used for everything
        socket: UdpSocket,
        /// There is only ever one server we care about
        server: SocketAddr,
        /// Our current sequence number
        current_sequence: u64,
        /// Last sequence we received from the server
        last_received_sequence: u64,
        /// Which bodies should be sent in the next outgoing packet
        bodies: Vec<ClientBodyElem>,
        /// Debugging pause: drop all packets in and out, stop any processing
        debug_paused: bool,
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
                last_received_sequence: 0,
                current_sequence: 0,
                bodies: Vec::with_capacity(DEFAULT_BODIES_VEC_CAPACITY),
                debug_paused: true, // TODO: remove
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
            let (_size, sender_addr) = self
                .socket
                .recv_from(&mut buffer)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::WouldBlock => ReceiveError::NoMessage,
                    _ => ReceiveError::IoError(e),
                })?;

            // check if it's actually from the server
            if sender_addr != self.server {
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
                    .with_system(o_pause_client)
                    .with_system(increase_tick.after(o_pause_client))
                    .with_system(p_queues_ping.after(increase_tick))
                    .with_system(client_handle_messages.after(p_queues_ping))
                    .with_system(send_bodies.after(client_handle_messages)),
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

    fn increase_tick(mut client: ResMut<Client>) {
        // don't increment when paused
        if !client.debug_paused {
            client.current_sequence += 1;
        }
    }

    fn o_pause_client(mut client: ResMut<Client>, input: Res<Input<KeyCode>>) {
        if !input.just_pressed(KeyCode::O) {
            return;
        }

        client.debug_paused = !client.debug_paused;

        warn!(
            "client now {}paused",
            if client.debug_paused { "" } else { "un" }
        );
    }

    /// simple system to make P queue up a ping to the server
    fn p_queues_ping(mut client: ResMut<Client>, input: Res<Input<KeyCode>>) {
        // return early if P was not pressed
        if !input.just_pressed(KeyCode::P) {
            return;
        }

        if client.debug_paused {
            return;
        }

        // TODO: remove if more than one type of message can be sent
        if client.bodies.is_empty() {
            info!("client queueing a ping");
            client.bodies.push(ClientBodyElem::Ping);
        }
    }

    /// Get and handle all messages from server
    fn client_handle_messages(mut client: ResMut<Client>) {
        if client.debug_paused {
            // eat all the messages
            let mut void = [0u8; 0];
            while match client.socket.recv_from(&mut void) {
                Ok(_) => true,
                Err(_) => false,
            } {}
            return;
        }

        loop {
            match client.get_one_message() {
                Ok(message) => {
                    info!("client received message: {:?}", message);
                    // only process newer messages, ignore old ones that arrive out of orders
                    if message.header.sequence > client.last_received_sequence {
                        // TODO: force update world

                        // if we are desync'd
                        if client.current_sequence != message.header.sequence {
                            let ticks_ahead =
                                client.current_sequence as i64 - message.header.sequence as i64;
                            let ahead = ticks_ahead > 0;
                            warn!(
                                "client out of sync, {} ticks {}!",
                                if ahead { ticks_ahead } else { -ticks_ahead },
                                if ahead { "ahead" } else { "behind" }
                            );

                            // jump to server's sequence
                            client.current_sequence = message.header.sequence;
                        }

                        // remember the last sequence that we received
                        client.last_received_sequence = message.header.sequence;
                    }
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

    fn send_bodies(mut client: ResMut<Client>) {
        if client.debug_paused {
            return;
        }

        // TODO: remove
        // only send out once every x frames
        if client.current_sequence % NETWORK_TICK_DELAY != 0 {
            return;
        }

        let message = ClientToServer {
            header: ClientHeader {
                current_sequence: client.current_sequence,
                last_received_sequence: client.last_received_sequence,
            },
            body: client.bodies.clone(),
        };
        let success_str = format!("client sent message to server: {:?}", message);
        match client.send_message(message) {
            Ok(_) => info!("{}", success_str),
            Err(e) => error!("failed to send message to server: {:?}", e),
        }

        // client doesn't care if message arrives -- it never retransmits bodies
        client.bodies.clear();
    }
}
