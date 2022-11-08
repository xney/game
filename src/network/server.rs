use super::*;
use crate::{states, world::Terrain};
use bevy::prelude::*;
use iyes_loopless::prelude::*;
use std::{
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
};

const NETWORK_TICK_DELAY: u64 = 60;
const SERVER_TIMESTEP_LABEL: &'static str = "SERVER_TICK";

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
        let addr = SocketAddr::from((DEFAULT_SERVER_IP, port));
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
        let (_size, sender_addr) =
            self.socket
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
    pub save_file: PathBuf,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_fixed_timestep(
            std::time::Duration::from_secs_f64(1. / 60.),
            SERVER_TIMESTEP_LABEL,
        )
        .add_enter_system(states::server::GameState::Running, create_server)
        .add_fixed_timestep_system(
            SERVER_TIMESTEP_LABEL,
            0,
            increase_tick
                .run_in_state(states::server::GameState::Running)
                .label("increase_tick"),
        )
        .add_fixed_timestep_system(
            SERVER_TIMESTEP_LABEL,
            0,
            server_handle_messages
                .run_in_state(states::server::GameState::Running)
                .after("increase_tick")
                .label("handle_messages"),
        )
        .add_fixed_timestep_system(
            SERVER_TIMESTEP_LABEL,
            0,
            enqueue_terrain
                .run_in_state(states::server::GameState::Running)
                .after("increase_tick")
                .label("enqueue_terrain"),
        )
        .add_fixed_timestep_system(
            SERVER_TIMESTEP_LABEL,
            0,
            send_all_messages
                .run_in_state(states::server::GameState::Running)
                .after("handle_messages")
                .label("send_messages"),
        )
        .add_fixed_timestep_system(
            SERVER_TIMESTEP_LABEL,
            0,
            drop_disconnected_clients
                .run_in_state(states::server::GameState::Running)
                .after("send_messages")
                .label("drop_disconnected"),
        )
        .add_exit_system(states::server::GameState::Running, destroy_server);
    }
}

fn create_server(mut commands: Commands) {
    // TODO: use command line arguments for port and handle failure better
    let server = match Server::new(DEFAULT_SERVER_PORT) {
        Ok(s) => s,
        Err(e) => panic!("Unable to create server: {}", e),
    };

    commands.insert_resource(server);

    info!("server created");
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
    info!("server got message from {:?} with {} bodies", client, message.bodies.len());

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
        .bodies
        .iter()
        // match client bodies to server bodies
        .map(|elem| match elem {
            ClientBodyElem::Ping => Some(ServerBodyElem::Pong(message.header.current_sequence)),
            ClientBodyElem::Input(input) => {
                // TODO: handle player input
                info!("ignoring player input for now");
                None
            }
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
        ServerBodyElem::Terrain(_) => true, // always keep terrains
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
            bodies: client_info.bodies.clone(),
        };

        // form message via borrow before consuming it
        let success_msg = format!(
            "server sent message to {:?}",
            client_info.addr
        );
        match server.send_message(message) {
            Ok(_) => info!("{}", success_msg),
            Err(e) => error!("server unable to send message: {:?}", e),
        }
    }
}

/// Add the terrain to the next packet sent
/// TODO: convert to delta and baseline
/// TODO: add resource instead of sending static terrain
fn enqueue_terrain(mut server: ResMut<Server>) {
    // TODO: remove
    // only send out once every x frames
    if server.sequence % NETWORK_TICK_DELAY != 0 {
        return;
    }

    if let Some(client) = &mut server.client {
        let terrain = Terrain::empty();
        client.bodies.push(ServerBodyElem::Terrain(terrain));
        info!("enqueued terrain");
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
