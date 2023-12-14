use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::sync::Arc;
use std::{eprintln, io};

use tun_tap::Iface;

use crate::allowed_ip::AllowedIps;
use crate::packet::Packet;
use crate::peer::{Action, Peer, PeerName};
use crate::poll::{Poll, Token};
use crate::udp;

pub struct DeviceConfig<'a> {
    pub name: PeerName,
    pub use_connected_peer: bool,
    pub listen_port: u16,
    pub tun_name: &'a str,
}

pub struct Device {
    name: PeerName,
    udp: Arc<UdpSocket>,
    iface: Iface,
    poll: Poll,
    peers_by_name: HashMap<PeerName, Arc<Peer>>,
    peers_by_index: Vec<Arc<Peer>>,
    peers_by_ip: AllowedIps<Arc<Peer>>,

    use_connected_peer: bool,
    listen_port: u16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SockID {
    Disconnected,
    ConnectedPeer(u32),
}

impl From<i32> for SockID {
    fn from(value: i32) -> Self {
        if value == -1 {
            SockID::Disconnected
        } else {
            SockID::ConnectedPeer(value as u32)
        }
    }
}

impl From<SockID> for i32 {
    fn from(value: SockID) -> Self {
        match value {
            SockID::Disconnected => -1,
            SockID::ConnectedPeer(i) => i as i32,
        }
    }
}

const BUF_SIZE: usize = 1504;

struct ThreadData {
    src_buf: [u8; BUF_SIZE],
    dst_buf: [u8; BUF_SIZE],
}

impl Device {
    pub fn new(config: DeviceConfig) -> io::Result<Self> {
        let iface = tun_tap::Iface::without_packet_info(config.tun_name, tun_tap::Mode::Tun)?;
        iface.set_non_blocking()?;

        let poll = Poll::new()?;
        let use_connected_peer = config.use_connected_peer;
        let listen_port = config.listen_port;

        let udp = Arc::new(udp::new_socket(config.listen_port)?);

        Ok(Self {
            name: config.name,
            iface,
            udp,
            poll,
            peers_by_name: HashMap::new(),
            peers_by_index: Vec::new(),
            peers_by_ip: AllowedIps::new(),
            use_connected_peer,
            listen_port,
        })
    }

    pub fn add_peer(&mut self, name: PeerName, mut peer: Peer) {
        let local_idx = self.peers_by_index.len();
        peer.set_local_idx(local_idx as u32);

        let peer = Arc::new(peer);

        self.peers_by_name.insert(name, Arc::clone(&peer));
        self.peers_by_ip.extend(
            peer.allowed_ips()
                .iter()
                .map(|(_, ip, cidr)| (ip, cidr, Arc::clone(&peer))),
        );
        self.peers_by_index.push(peer);
    }

    pub fn wait(&self) {
        let mut t = ThreadData {
            src_buf: [0; BUF_SIZE],
            dst_buf: [0; BUF_SIZE],
        };

        while let Ok(token) = self.poll.wait() {
            match token {
                Token::Tun => {
                    if let Err(err) = self.handle_tun(&mut t) {
                        eprintln!("tun error: {:?}", err);
                    }
                }
                Token::Sock(SockID::Disconnected) => {
                    if let Err(err) = self.handle_udp(&self.udp, &mut t) {
                        eprintln!("udp error: {:?}", err);
                    }
                }
                Token::Sock(SockID::ConnectedPeer(i)) => {
                    let Some(peer) = self.peers_by_index.get(i as usize) else {
                        continue;
                    };
                    if let Some(conn) = peer.endpoint().conn.as_deref() {
                        if let Err(err) = self.handle_udp(conn, &mut t) {
                            eprintln!("udp error: {:?}", err);
                        }
                    }
                }
            }
        }
    }

    pub fn start(&self) -> io::Result<()> {
        eprintln!("Start!");

        self.poll
            .register_read(Token::Sock(SockID::Disconnected), self.udp.as_ref())?;

        let tun_fd = unsafe { BorrowedFd::borrow_raw(self.iface.as_raw_fd()) };
        self.poll.register_read::<_, SockID>(Token::Tun, &tun_fd)?;

        let mut buf = [0u8; BUF_SIZE];
        for (_, peer) in self.peers_by_name.iter() {
            match peer.send_handshake(self.name.as_ref(), &mut buf) {
                Action::WriteToTunn(data, src_addr) => {
                    if peer.is_allowed_ip(src_addr) {
                        let _ = self.iface.send(data);
                    }
                }
                Action::WriteToNetwork(data) => {
                    let _ = self.send_over_udp(peer, data);
                }
                Action::None => (),
            }
        }

        Ok(())
    }

    fn handle_tun(&self, thread_data: &mut ThreadData) -> io::Result<()> {
        let src_buf = &mut thread_data.src_buf[..];
        while let Ok(nbytes) = self.iface.recv(src_buf) {
            let (src, dst) = match etherparse::Ipv4HeaderSlice::from_slice(&src_buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    (src, dst)
                }
                _ => continue,
            };
            eprintln!("Got Ipv4 packet of size: {nbytes}, {src} -> {dst}, from tun0");
            let Some(peer) = self.peers_by_ip.get(dst.into()) else {
                eprintln!("  no peer.");
                continue;
            };
            match peer.encapsulate(&src_buf[..nbytes], &mut thread_data.dst_buf) {
                Action::WriteToTunn(data, src_addr) => {
                    eprintln!("To run. {src_addr}, {}", peer.is_allowed_ip(src_addr));
                    if peer.is_allowed_ip(src_addr) {
                        let _ = self.iface.send(data);
                    }
                }
                Action::WriteToNetwork(data) => {
                    let _ = self.send_over_udp(peer, data);
                }
                Action::None => (),
            }
        }

        Ok(())
    }

    fn handle_udp(&self, sock: &UdpSocket, thread_data: &mut ThreadData) -> io::Result<()> {
        let src = &mut thread_data.src_buf[..];
        while let Ok((nbytes, peer_addr)) = sock.recv_from(&mut src[..]) {
            let SocketAddr::V4(peer_addr) = peer_addr else {
                continue;
            };
            let Ok(packet) = Packet::parse_from(&src[..nbytes]) else {
                continue;
            };
            let peer = match packet {
                Packet::Empty => continue,
                Packet::HandshakeInit(ref msg) => {
                    eprintln!("Handshake init received {:?}", msg);
                    self.peers_by_name.get(msg.sender_name.as_slice())
                }
                Packet::HandshakeResponse(ref msg) => {
                    eprintln!("Handshake response received {:?}", msg);
                    self.peers_by_index.get(msg.sender_idx as usize)
                }
                Packet::Data(ref msg) => {
                    eprintln!("data recieved");
                    self.peers_by_index.get(msg.sender_idx as usize)
                }
            };
            if peer.is_none() {
                eprintln!("no peer");
            }
            let Some(peer) = peer else {
                continue;
            };

            let (endpoint_changed, conn) = peer.set_endpoint(peer_addr);
            if let Some(conn) = conn {
                self.poll.delete(conn.as_ref()).expect("epoll delete");
                drop(conn);
            }
            if endpoint_changed && self.use_connected_peer {
                match peer.connect_endpoint(self.listen_port) {
                    Ok(conn) => {
                        self.poll
                            .register_read(
                                Token::Sock(SockID::ConnectedPeer(peer.local_idx())),
                                &*conn,
                            )
                            .expect("epoll add");
                    }
                    Err(err) => {
                        eprintln!("error connecting to peer: {:?}", err);
                    }
                }
            }

            match peer.handle_incoming_packet(packet, &mut thread_data.dst_buf) {
                Action::WriteToTunn(data, src_addr) => {
                    if peer.is_allowed_ip(src_addr) {
                        eprintln!("to tun..");
                        let _ = self.iface.send(data);
                    }
                }
                Action::WriteToNetwork(data) => {
                    eprintln!("to network.. {:?}", data);
                    let _ = self.send_over_udp(peer, data);
                }
                Action::None => (),
            }
        }

        Ok(())
    }

    fn send_over_udp(&self, peer: &Peer, data: &[u8]) -> io::Result<usize> {
        let endpoint = peer.endpoint();
        if let Some(ref conn) = endpoint.conn {
            conn.send(data)
        } else if let Some(ref addr) = endpoint.addr {
            self.udp.send_to(data, addr)
        } else {
            Ok(0)
        }
    }
}