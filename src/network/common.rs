use std::net::{SocketAddr, UdpSocket};

use bincode::{Decode, Encode};

use crate::{world::Terrain, player::PlayerInput};

/// This is the bincode config that we should use everywhere
pub const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard()
    .with_little_endian()
    .with_variable_int_encoding()
    .write_fixed_array_length();

/// placeholders
/// TODO: remove whenever command line arguments can be parsed
pub const DEFAULT_SERVER_PORT: u16 = 8888u16;
pub const DEFAULT_SERVER_IP: [u8; 4] = [127, 0, 0, 1];

/// Default size of allocated bodies vec, larger numbers may help reduce reallocation
pub(super) const DEFAULT_BODIES_VEC_CAPACITY: usize = 10;

/// How many frames does a client have to not respond for before the server assumes it's dead
pub(super) const FRAME_DIFFERENCE_BEFORE_DISCONNECT: u64 = 60 * 5;

/// Marker trait for network structs
pub(super) trait NetworkMessage: Encode + Decode {}

/// Message from the server to a client
#[derive(Encode, Decode, Debug)]
pub(super) struct ServerToClient {
    pub header: ServerHeader,
    pub bodies: Vec<ServerBodyElem>,
}

/// Header for ServerToClient message
#[derive(Encode, Decode, Debug)]
pub(super) struct ServerHeader {
    /// Sequence/tick number
    pub sequence: u64,
}

/// One element (message) for the body of a ServerToClient message
#[derive(Encode, Decode, Debug, Clone)]
pub(super) enum ServerBodyElem {
    /// contains sequence number of ping
    /// TODO: remove
    Pong(u64),
    /// simple terrain update
    /// TODO: separate into baseline and delta
    /// TODO: use ref instead
    Terrain(Terrain)
}

impl NetworkMessage for ServerToClient {}

/// Message from a client to the server
#[derive(Encode, Decode, Debug)]
pub(super) struct ClientToServer {
    pub header: ClientHeader,
    pub bodies: Vec<ClientBodyElem>,
}

/// Header for ClientToServer message
#[derive(Encode, Decode, Debug)]
pub(super) struct ClientHeader {
    /// Client's current sequence/tick number
    /// TODO: is this ever useful?
    pub current_sequence: u64,
    /// Last received sequence/tick number
    pub last_received_sequence: u64,
}

/// One element (message) for the body of a ClientToServer message
#[derive(Encode, Decode, Debug, Clone)]
pub(super) enum ClientBodyElem {
    /// asks server to send a pong as a response
    /// pong should contain the sequence number of this packet
    Ping,
    /// sends entire input
    Input(PlayerInput)
}

impl NetworkMessage for ClientToServer {}

#[derive(Debug)]
pub(super) enum SendError {
    IoError(std::io::Error),
    EncodeError(bincode::error::EncodeError),
    NoSuchPeer,
}

#[derive(Debug)]
pub(super) enum ReceiveError {
    IoError(std::io::Error),
    DecodeError(bincode::error::DecodeError),
    UnknownSender,
    NoMessage,
}

/// Helper method for sending a message
pub(super) fn send_message<M: NetworkMessage>(
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
