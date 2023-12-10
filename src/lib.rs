use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::sync::Arc;
use std::{eprintln, io};

use parking_lot::{RwLock, RwLockReadGuard};
use socket2::{Domain, Protocol, Socket, Type};
use tun_tap::Iface;

mod poll;

use poll::{Poll, Token};

pub struct DeviceConfig<'a> {
    use_connected_peer: bool,
    listen_port: u16,
    tun_name: &'a str,
    peer_addr: Option<SocketAddrV4>,
}

pub struct Device {
    udp: Arc<UdpSocket>,
    iface: Iface,
    peer: Peer,
    poll: Poll,

    use_connected_peer: bool,
    listen_port: u16,
}

pub struct Peer {
    endpoint: RwLock<Endpoint>,
}

#[derive(Default)]
pub struct Endpoint {
    pub addr: Option<SocketAddrV4>,
    pub conn: Option<Arc<UdpSocket>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SockID {
    Disconnected,
    ConnectedPeer,
}

impl From<i32> for SockID {
    fn from(value: i32) -> Self {
        if value == -1 {
            SockID::Disconnected
        } else {
            SockID::ConnectedPeer
        }
    }
}

impl From<SockID> for i32 {
    fn from(value: SockID) -> Self {
        match value {
            SockID::Disconnected => -1,
            SockID::ConnectedPeer => 0,
        }
    }
}

const BUF_SIZE: usize = 1504;

struct ThreadData {
    src_buf: [u8; BUF_SIZE],
}

impl<'a> DeviceConfig<'a> {
    pub fn new(
        use_connected_peer: bool,
        listen_port: u16,
        tun_name: &'a str,
        peer_addr: Option<SocketAddrV4>,
    ) -> Self {
        Self {
            use_connected_peer,
            listen_port,
            tun_name,
            peer_addr,
        }
    }
}

impl Device {
    pub fn new(config: DeviceConfig) -> io::Result<Self> {
        let iface = tun_tap::Iface::without_packet_info(config.tun_name, tun_tap::Mode::Tun)?;
        iface.set_non_blocking()?;

        let poll = Poll::new()?;
        let use_connected_peer = config.use_connected_peer;
        let listen_port = config.listen_port;

        let peer = Peer::new(Endpoint::default());
        let udp = if let Some(addr) = config.peer_addr {
            let _ = peer.set_endpoint(addr);
            peer.connect_endpoint(listen_port)?
        } else {
            Arc::new(new_udp_socket(config.listen_port)?)
        };

        Ok(Self {
            iface,
            udp,
            poll,
            peer,
            use_connected_peer,
            listen_port,
        })
    }

    pub fn wait(&self) {
        let mut t = ThreadData {
            src_buf: [0; BUF_SIZE],
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
                Token::Sock(SockID::ConnectedPeer) => {
                    if let Some(conn) = self.peer.endpoint().conn.as_deref() {
                        if let Err(err) = self.handle_connected_peer(conn, &mut t) {
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

        self.initiate_handshake()
    }

    fn initiate_handshake(&self) -> io::Result<()> {
        let msg = b"hello?";

        let endpoint = self.peer.endpoint();
        if let Some(ref conn) = endpoint.conn {
            eprintln!("initiating handshake..");

            conn.send(msg)?;
        } else if let Some(ref addr) = endpoint.addr {
            eprintln!("initiating handshake..");

            self.udp.send_to(msg, addr)?;
        };

        Ok(())
    }

    fn handle_tun(&self, thread_data: &mut ThreadData) -> io::Result<()> {
        let buf = &mut thread_data.src_buf[..];
        while let Ok(nbytes) = self.iface.recv(buf) {
            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("Got Ipv4 packet of size: {nbytes}, {src} -> {dst}, from tun0");
                }
                _ => continue,
            }

            let endpoint = self.peer.endpoint();
            let _send_result = if let Some(ref conn) = endpoint.conn {
                conn.send(&buf[..nbytes])
            } else if let Some(ref addr) = endpoint.addr {
                self.udp.send_to(&buf[..nbytes], addr)
            } else {
                Ok(0)
            };
        }

        Ok(())
    }

    fn handle_udp(&self, sock: &UdpSocket, thread_data: &mut ThreadData) -> io::Result<()> {
        let buf = &mut thread_data.src_buf[..];
        while let Ok((nbytes, peer_addr)) = sock.recv_from(&mut buf[..]) {
            eprintln!("Got packet of size: {nbytes}, from {peer_addr}");

            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("  {src} -> {dst}");
                }
                _ => {
                    eprintln!("not an Ipv4 packet: {:?}", &buf[..nbytes]);
                }
            }

            if let SocketAddr::V4(peer_addr_v4) = peer_addr {
                if &buf[..nbytes] == b"hello?" {
                    eprintln!("recieved handshake..");

                    let (endpoint_changed, conn) = self.peer.set_endpoint(peer_addr_v4);
                    if let Some(conn) = conn {
                        self.poll.delete(conn.as_ref()).expect("epoll delete");
                        drop(conn);
                    }

                    if endpoint_changed && self.use_connected_peer {
                        match self.peer.connect_endpoint(self.listen_port) {
                            Ok(conn) => {
                                self.poll
                                    .register_read(Token::Sock(SockID::ConnectedPeer), &*conn)
                                    .expect("epoll add");
                            }
                            Err(err) => {
                                eprintln!("error connecting to peer: {:?}", err);
                            }
                        }
                    }
                    continue;
                }
                let _ = self.iface.send(&buf[..nbytes]);
            }
        }

        Ok(())
    }

    fn handle_connected_peer(
        &self,
        sock: &UdpSocket,
        thread_data: &mut ThreadData,
    ) -> io::Result<()> {
        let buf = &mut thread_data.src_buf[..];
        while let Ok(nbytes) = sock.recv(&mut buf[..]) {
            eprintln!("Got packet of size: {nbytes}, from a connected peer");

            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("  {src} -> {dst}");
                }
                _ => {
                    eprintln!("not an Ipv4 packet: {:?}", &buf[..nbytes]);
                }
            }

            let _ = self.iface.send(&buf[..nbytes]);
        }

        Ok(())
    }
}

fn new_udp_socket(port: u16) -> io::Result<UdpSocket> {
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], port));

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;

    socket.bind(&socket_addr.into())?;

    Ok(socket.into())
}

impl Peer {
    fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint: RwLock::new(endpoint),
        }
    }

    fn endpoint(&self) -> RwLockReadGuard<Endpoint> {
        self.endpoint.read()
    }

    fn set_endpoint(&self, addr: SocketAddrV4) -> (bool, Option<Arc<UdpSocket>>) {
        let endpoint = self.endpoint.read();
        if endpoint.addr == Some(addr) {
            return (false, None);
        }
        drop(endpoint);

        let mut endpoint = self.endpoint.write();
        endpoint.addr = Some(addr);

        (true, endpoint.conn.take())
    }

    fn connect_endpoint(&self, port: u16) -> io::Result<Arc<UdpSocket>> {
        let mut endpoint = self.endpoint.write();
        let addr = endpoint.addr.unwrap();

        assert!(endpoint.conn.is_none());

        let conn = new_udp_socket(port)?;
        conn.connect(addr)?;
        let conn = Arc::new(conn);

        endpoint.conn = Some(Arc::clone(&conn));

        Ok(conn)
    }
}
