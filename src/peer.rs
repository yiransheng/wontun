use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use std::io;
use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard};

use crate::allowed_ip::AllowedIps;
use crate::new_udp_socket;
use crate::packet::{HandshakeInit, HandshakeResponse, Packet, PacketData};

#[derive(Hash, Eq, PartialEq)]
pub struct PeerName<T = [u8; 100]>(T);

pub struct Peer {
    local_idx: u32,
    handshake_state: RwLock<HandshakeState>,
    endpoint: RwLock<Endpoint>,
    allowed_ips: AllowedIps<()>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct AllowedIp {
    pub ip: Ipv4Addr,
    pub cidr: u8,
}

#[derive(Default)]
pub struct Endpoint {
    pub addr: Option<SocketAddrV4>,
    pub conn: Option<Arc<UdpSocket>>,
}

pub enum Action<'a> {
    WriteToTunn(&'a [u8], Ipv4Addr),
    WriteToNetwork(&'a [u8]),
    None,
}

#[derive(Debug, Copy, Clone)]
enum HandshakeState {
    None,
    HandshakeSent,
    HandshakeReceived { remote_idx: u32 },
    Connected { remote_idx: u32 },
}

impl Peer {
    pub fn new() -> Self {
        Self {
            local_idx: 0,
            handshake_state: RwLock::new(HandshakeState::None),
            endpoint: RwLock::new(Endpoint::default()),
            allowed_ips: AllowedIps::new(),
        }
    }

    pub fn allowed_ips(&self) -> &AllowedIps<()> {
        &self.allowed_ips
    }

    pub fn add_allowed_ip(&mut self, addr: Ipv4Addr, cidr: u8) {
        self.allowed_ips.insert(addr.into(), cidr, ());
    }

    pub fn is_allowed_ip(&self, addr: Ipv4Addr) -> bool {
        self.allowed_ips.get(addr.into()).is_some()
    }

    pub fn local_idx(&self) -> u32 {
        self.local_idx
    }

    pub fn set_local_idx(&mut self, idx: u32) {
        self.local_idx = idx;
    }

    pub fn endpoint(&self) -> RwLockReadGuard<Endpoint> {
        self.endpoint.read()
    }

    pub fn set_endpoint(&self, addr: SocketAddrV4) -> (bool, Option<Arc<UdpSocket>>) {
        let endpoint = self.endpoint.read();
        if endpoint.addr == Some(addr) {
            return (false, None);
        }
        drop(endpoint);

        let mut endpoint = self.endpoint.write();
        endpoint.addr = Some(addr);

        (true, endpoint.conn.take())
    }

    pub fn connect_endpoint(&self, port: u16) -> io::Result<Arc<UdpSocket>> {
        let mut endpoint = self.endpoint.write();
        let addr = endpoint.addr.unwrap();

        assert!(endpoint.conn.is_none());

        let conn = new_udp_socket(port)?;
        conn.connect(addr)?;
        let conn = Arc::new(conn);

        endpoint.conn = Some(Arc::clone(&conn));

        Ok(conn)
    }

    pub fn handle_packet<'a>(&self, packet: Packet<'a>, dst: &'a mut [u8]) -> Action<'a> {
        match packet {
            Packet::Empty => Action::None,
            Packet::HandshakeInit(msg) => self.handle_handshake_init(msg, dst),
            Packet::HandshakeResponse(msg) => self.handle_handshake_response(msg, dst),
            Packet::Data(msg) => self.handle_packet_data(msg, dst),
        }
    }

    fn handle_handshake_init<'a>(&self, msg: HandshakeInit<'a>, dst: &'a mut [u8]) -> Action<'a> {
        let mut state = self.handshake_state.write();
        if let HandshakeState::None = &*state {
            *state = HandshakeState::HandshakeReceived {
                remote_idx: msg.assigned_idx,
            };
            drop(state);

            let local_idx = self.local_idx;
            let response = HandshakeResponse {
                assigned_idx: local_idx,
                sender_idx: msg.assigned_idx,
            };
            let n = Packet::from(response).format(dst);
            Action::WriteToNetwork(&dst[..n])
        } else {
            Action::None
        }
    }

    fn handle_handshake_response<'a>(
        &self,
        msg: HandshakeResponse,
        _dst: &'a mut [u8],
    ) -> Action<'a> {
        let mut state = self.handshake_state.write();
        if let HandshakeState::HandshakeSent = &*state {
            *state = HandshakeState::Connected {
                remote_idx: msg.assigned_idx,
            };
        }
        Action::None
    }

    fn handle_packet_data<'a>(&self, msg: PacketData<'a>, _dst: &'a mut [u8]) -> Action<'a> {
        let state = self.handshake_state.read();
        match &*state {
            HandshakeState::Connected { .. } => (),
            HandshakeState::HandshakeReceived { remote_idx } => {
                let remote_idx = *remote_idx;
                drop(state);

                let mut state = self.handshake_state.write();
                *state = HandshakeState::Connected { remote_idx };
            }
            _ => return Action::None,
        };
        match etherparse::Ipv4HeaderSlice::from_slice(msg.data) {
            Ok(iph) => {
                let src = iph.source_addr();
                Action::WriteToTunn(msg.data, src)
            }
            _ => Action::None,
        }
    }
}

impl std::borrow::Borrow<[u8]> for PeerName<[u8; 100]> {
    fn borrow(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<'a> From<&'a [u8]> for PeerName<&'a [u8]> {
    fn from(slice: &'a [u8]) -> Self {
        PeerName(slice)
    }
}

impl<'a> PeerName<&'a [u8]> {
    pub fn as_slice(&self) -> &'a [u8] {
        self.0
    }
}

impl PeerName<[u8; 100]> {
    pub fn new(name: &str) -> Result<Self, &str> {
        let mut bytes = [0u8; 100];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len();

        if len > 100 {
            Err(name)
        } else {
            (&mut bytes[..len]).copy_from_slice(name_bytes);
            Ok(PeerName(bytes))
        }
    }
}
