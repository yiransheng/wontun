pub enum Packet<'a> {
    HandshakeInit(HandshakeInit),
    HandshakeResponse(HandshakeResponse),
    Data(PacketData<'a>),
}

pub struct HandshakeInit {
    sender_id: u32,
}

pub struct HandshakeResponse {
    //
}

pub struct PacketData<'a> {
    pub recieve_id: u32,
    pub data: &'a [u8],
}

const HANDSHAKE_INIT: u8 = 1;
const HANDSHAKE_RESPONSE: u8 = 2;
const DATA: u8 = 3;
