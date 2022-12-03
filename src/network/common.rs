use std::net::{SocketAddr, UdpSocket};

use bincode::{Decode, Encode};

use crate::{
    player::{PlayerInput, PlayerPosition},
    world::Terrain,
};

use super::server::ClientAddress;

/// This is the bincode config that we should use everywhere
pub const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard()
    .with_little_endian()
    .with_variable_int_encoding()
    .write_fixed_array_length();

/// placeholders
/// TODO: remove whenever command line arguments can be parsed
pub const DEFAULT_SERVER_PORT: u16 = 8888u16;
pub const DEFAULT_SERVER_IP: [u8; 4] = [127, 0, 0, 1];

/// incoming buffer size for networking
/// TODO: reduce whenever delta compression is implemented
pub const BUFFER_SIZE: usize = 65536;

/// Default size of allocated bodies vec, larger numbers may help reduce reallocation
pub const DEFAULT_BODIES_VEC_CAPACITY: usize = 10;

/// How many frames does a client have to not respond for before the server assumes it's dead
pub const FRAME_DIFFERENCE_BEFORE_DISCONNECT: u64 = 5;

/// how many times per second will the network tick occur
pub const NETWORK_TICK_HZ: u64 = 1;

/// timestep for sending out network messages
pub const NETWORK_TICK_LABEL: &str = "NETWORK_TICK";

/// how many times per second will the game tick occur
pub const GAME_TICK_HZ: u64 = 60;

/// timestep for doing world calculations
pub const GAME_TICK_LABEL: &str = "GAME_TICK";

/// Marker trait for network structs
pub trait NetworkMessage: Encode + Decode {}

/// Message from the server to a client
#[derive(Encode, Decode, Debug)]
pub struct ServerToClient {
    pub header: ServerHeader,
    pub bodies: Vec<ServerBodyElem>,
}

/// Header for ServerToClient message
#[derive(Encode, Decode, Debug)]
pub struct ServerHeader {
    /// Sequence/tick number
    pub sequence: u64,
}

/// One element (message) for the body of a ServerToClient message
#[derive(Encode, Decode, Debug, Clone)]
pub enum ServerBodyElem {
    /// contains sequence number of ping
    /// TODO: remove
    Pong(u64),
    /// simple terrain update
    /// TODO: separate into baseline and delta
    /// TODO: use ref instead
    Terrain(Terrain),
    PlayerInfo(Vec<SingleNetPlayerInfo>),
}

/// Contains information about a single player
#[derive(Encode, Decode, Debug, Clone)]
pub struct SingleNetPlayerInfo {
    pub addr: ClientAddress,
    pub position: PlayerPosition, // TODO: put inputs here if we want client-side prediction
}

impl NetworkMessage for ServerToClient {}

/// Message from a client to the server
#[derive(Encode, Decode, Debug)]
pub struct ClientToServer {
    pub header: ClientHeader,
    pub bodies: Vec<ClientBodyElem>,
}

/// Header for ClientToServer message
#[derive(Encode, Decode, Debug)]
pub struct ClientHeader {
    /// Client's current sequence/tick number
    /// TODO: is this ever useful?
    pub current_sequence: u64,
    /// Last received sequence/tick number
    pub last_received_sequence: u64,
}

/// One element (message) for the body of a ClientToServer message
#[derive(Encode, Decode, Debug, Clone)]
pub enum ClientBodyElem {
    /// asks server to send a pong as a response
    /// pong should contain the sequence number of this packet
    Ping,
    /// sends entire input
    Input(PlayerInput),
}

impl NetworkMessage for ClientToServer {}

#[derive(Debug)]
pub enum SendError {
    IoError(std::io::Error),
    EncodeError(bincode::error::EncodeError),
    //NoSuchPeer,
}

#[derive(Debug)]
pub enum ReceiveError {
    IoError(std::io::Error),
    DecodeError(bincode::error::DecodeError),
    UnknownSender,
    NoMessage,
}

/// Helper method for sending a message
pub fn send_message<M: NetworkMessage>(
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
