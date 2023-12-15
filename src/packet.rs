use crate::peer::PeerName;

#[derive(Debug)]
pub enum Packet<'a> {
    HandshakeInit(HandshakeInit<'a>),
    HandshakeResponse(HandshakeResponse),
    Data(PacketData<'a>),
    Empty,
}

#[derive(Debug)]
pub struct HandshakeInit<'a> {
    pub sender_name: PeerName<&'a [u8]>,
    pub assigned_idx: u32,
}

#[derive(Debug)]
pub struct HandshakeResponse {
    pub assigned_idx: u32,
    pub sender_idx: u32,
}

#[derive(Debug)]
pub struct PacketData<'a> {
    pub sender_idx: u32,
    pub data: &'a [u8],
}

const HANDSHAKE_INIT: u8 = 1;
const HANDSHAKE_RESPONSE: u8 = 2;
const PACKET_DATA: u8 = 3;

const HANDSHAKE_INIT_SIZE: usize = 105;
const HANDSHAKE_RESPONSE_SIZE: usize = 9;
const DATA_MIN_SIZE: usize = 5;

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
                let remote_idx = u32::from_le_bytes(src[1..5].try_into().unwrap());
                let sender_name = PeerName::from(&src[5..105]);
                Ok(Packet::HandshakeInit(HandshakeInit {
                    sender_name,
                    assigned_idx: remote_idx,
                }))
            }
            (HANDSHAKE_RESPONSE, HANDSHAKE_RESPONSE_SIZE) => {
                let assigned_idx = u32::from_le_bytes(src[1..5].try_into().unwrap());
                let sender_idx = u32::from_le_bytes(src[5..9].try_into().unwrap());

                Ok(Packet::HandshakeResponse(HandshakeResponse {
                    assigned_idx,
                    sender_idx,
                }))
            }
            (PACKET_DATA, n) if n >= DATA_MIN_SIZE => {
                let sender_idx = u32::from_le_bytes(src[1..5].try_into().unwrap());

                Ok(Packet::Data(PacketData {
                    sender_idx,
                    data: &src[5..],
                }))
            }
            _ => Err(PackeParseError::ProtocolErr),
        }
    }
}

impl<'a> HandshakeInit<'a> {
    pub fn format(&self, dst: &mut [u8]) -> usize {
        assert!(dst.len() >= HANDSHAKE_INIT_SIZE);

        dst[0] = HANDSHAKE_INIT;
        dst[1..5].copy_from_slice(&self.assigned_idx.to_le_bytes());
        dst[5..105].copy_from_slice(self.sender_name.as_slice());

        HANDSHAKE_INIT_SIZE
    }
}

impl HandshakeResponse {
    pub fn format(&self, dst: &mut [u8]) -> usize {
        assert!(dst.len() >= HANDSHAKE_RESPONSE_SIZE);

        dst[0] = HANDSHAKE_RESPONSE;
        dst[1..5].copy_from_slice(&self.assigned_idx.to_le_bytes());
        dst[5..9].copy_from_slice(&self.sender_idx.to_le_bytes());

        HANDSHAKE_RESPONSE_SIZE
    }
}

impl<'a> PacketData<'a> {
    pub fn format(&self, dst: &mut [u8]) -> usize {
        let n = self.data.len();
        let len = n + 5;
        assert!(dst.len() >= len);

        dst[0] = PACKET_DATA;
        dst[1..5].copy_from_slice(&self.sender_idx.to_le_bytes());
        dst[5..(5 + n)].copy_from_slice(self.data);

        len
    }
}
