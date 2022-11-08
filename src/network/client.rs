use std::net::{IpAddr, SocketAddr, UdpSocket};

use super::*;
use crate::player;
use crate::states;
use bevy::prelude::*;

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
        let (_size, sender_addr) =
            self.socket
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

    /// Push a body that will be sent to the server
    fn enqueue_body(&mut self, body: ClientBodyElem) {
        self.bodies.push(body);
    }

    /// Client logic for handling bodies received from the server
    /// TODO: add actual logic
    fn handle_body(&mut self, body: ServerBodyElem) {
        match body {
            ServerBodyElem::Pong(pong) => info!("got pong for seqnum: {}", pong),
            ServerBodyElem::Terrain(t) => info!("got terrain, ignoring for now"),
        }
    }
}

pub struct ClientPlugin {
    pub server_address: IpAddr,
    pub server_port: u16,
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(states::client::GameState::InGame).with_system(create_client),
        )
        .add_system_set(
            SystemSet::on_update(states::client::GameState::InGame)
                .with_system(o_pause_client)
                .with_system(increase_tick.after(o_pause_client))
                .with_system(p_queues_ping.after(increase_tick))
                .with_system(queue_inputs.after(increase_tick))
                .with_system(client_handle_messages.after(p_queues_ping))
                .with_system(send_bodies.after(client_handle_messages)),
        )
        .add_system_set(
            SystemSet::on_exit(states::client::GameState::InGame).with_system(destroy_client),
        );
    }
}

fn create_client(mut commands: Commands) {
    let client = match Client::new(SocketAddr::from((DEFAULT_SERVER_IP, DEFAULT_SERVER_PORT))) {
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
        client.enqueue_body(ClientBodyElem::Ping);
    }
}

/// Scrape client inputs and queue up sending them to server
fn queue_inputs(mut client: ResMut<Client>, bevy_input: Res<Input<KeyCode>>) {
    // TODO: remove
    // only send out once every x frames
    if client.current_sequence % NETWORK_TICK_DELAY != 0 {
        return;
    }

    if client.debug_paused {
        return;
    }

    let input = player::PlayerInput {
        left: bevy_input.pressed(KeyCode::A),
        right: bevy_input.pressed(KeyCode::D),
        jump: bevy_input.pressed(KeyCode::Space),
    };

    // TODO: add block mining attempts

    client.enqueue_body(ClientBodyElem::Input(input));
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
                info!("client received message with {} bodies", message.bodies.len());
                // only process newer messages, ignore old ones that arrive out of orders
                if message.header.sequence > client.last_received_sequence {
                    // handle all bodies sent from the server
                    for body in message.bodies {
                        client.handle_body(body);
                    }

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
        bodies: client.bodies.clone(),
    };
    let success_str = format!("client sent message to server: {:?}", message);
    match client.send_message(message) {
        Ok(_) => info!("{}", success_str),
        Err(e) => error!("failed to send message to server: {:?}", e),
    }

    // client doesn't care if message arrives -- it never retransmits bodies
    client.bodies.clear();
}

// TODO: client-side timeout!
