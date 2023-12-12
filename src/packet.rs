use crate::peer::PeerName;

pub enum Packet<'a> {
    HandshakeInit(HandshakeInit<'a>),
    HandshakeResponse(HandshakeResponse),
    Data(PacketData<'a>),
    Empty,
}

pub struct HandshakeInit<'a> {
    pub sender_name: PeerName<&'a [u8]>,
    pub assigned_idx: u32,
}

pub struct HandshakeResponse {
    pub assigned_idx: u32,
    pub sender_idx: u32,
}

pub struct PacketData<'a> {
    pub sender_idx: u32,
    pub data: &'a [u8],
}

const HANDSHAKE_INIT: u8 = 1;
const HANDSHAKE_RESPONSE: u8 = 2;
const PACKET_DATA: u8 = 3;

const HANDSHAKE_INIT_SIZE: usize = 105;
const HANDSHAKE_RESPONSE_SIZE: usize = 2;

#[derive(Debug, Copy, Clone)]
pub enum PackeParseError {
    InvalidPeerName,
    ProtocolErr,
}

impl<'a> Packet<'a> {
    pub fn parse_from(src: &'a [u8]) -> Result<Self, PackeParseError> {
        if src.is_empty() {
            return Ok(Packet::Empty);
        }
        match (src[0], src.len()) {
            (HANDSHAKE_INIT, HANDSHAKE_INIT_SIZE) => {
                let remote_idx = u32::from_le_bytes(src[1..][..4].try_into().unwrap());
                let sender_name = PeerName::from(&src[5..][..100]);
                Ok(Packet::HandshakeInit(HandshakeInit {
                    sender_name,
                    assigned_idx: remote_idx,
                }))
            }
            (HANDSHAKE_RESPONSE, HANDSHAKE_RESPONSE_SIZE) => unimplemented!(),
            (PACKET_DATA, _) => unimplemented!(),
            _ => Err(PackeParseError::ProtocolErr),
        }
    }

    pub fn format(&self, dst: &mut [u8]) -> usize {
        0
    }
}

impl<'a> From<HandshakeResponse> for Packet<'a> {
    fn from(value: HandshakeResponse) -> Self {
        Packet::HandshakeResponse(value)
    }
}
