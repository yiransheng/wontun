use parking_lot::RwLock;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::Arc;
use tun_tap::Iface;

use std::{eprintln, io};

pub struct Device {
    use_connected_socket: bool,
    listen_port: u16,
    iface: Iface,
    peer: Peer,
}

pub struct Peer {
    endpoint: RwLock<Endpoint>,
}

#[derive(Default)]
pub struct Endpoint {
    pub addr: Option<SocketAddrV4>,
    pub conn: Option<Arc<UdpSocket>>,
}

impl Device {
    pub fn new(use_connected_socket: bool, listen_port: u16, iface: Iface, peer: Peer) -> Self {
        Self {
            use_connected_socket,
            listen_port,
            iface,
            peer,
        }
    }

    pub fn start(self, udp: Option<Arc<UdpSocket>>) -> io::Result<()> {
        let udp = if let Some(sock) = udp {
            sock
        } else {
            Arc::new(new_udp_socket(self.listen_port)?)
        };

        let dev1 = Arc::new(self);
        let dev2 = Arc::clone(&dev1);
        let jh1 = std::thread::spawn(move || {
            if let Err(err) = dev1.loop_listen_iface() {
                eprintln!("err loop 1: {:?}", err);
            }
        });
        let jh2 = std::thread::spawn(move || {
            if let Err(err) = dev2.loop_listen_udp(udp) {
                eprintln!("err loop 2: {:?}", err);
            }
        });

        jh1.join().unwrap();
        jh2.join().unwrap();

        Ok(())
    }

    fn loop_listen_iface(self: Arc<Self>) -> io::Result<()> {
        let mut buf = [0u8; 1504];
        {
            let endpoint = self.peer.endpoint.read();
            if let Some(ref conn) = endpoint.conn {
                eprintln!("initiating peer connection..");
                conn.send("hello?".as_bytes())?;
            }
        }

        loop {
            let nbytes = self.iface.recv(&mut buf[..])?;
            let endpoint = self.peer.endpoint.read();

            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("Got Ipv4 packet of size: {nbytes}, {src} -> {dst}, from tun0");
                }
                _ => {}
            }

            if let Some(ref conn) = endpoint.conn {
                conn.send(&buf[..nbytes])?;
            } else {
                eprintln!("..no peer");
            }
        }
    }

    fn loop_listen_udp(self: Arc<Self>, sock: Arc<UdpSocket>) -> io::Result<()> {
        let mut buf = [0u8; 1504];

        loop {
            let (nbytes, peer_addr) = sock.recv_from(&mut buf[..])?;
            eprintln!("Got packet of size: {nbytes}, from {peer_addr}");

            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("  {src} -> {dst}");
                }
                _ => {
                    eprintln!("not an Ipv4 packet");
                }
            }

            if let SocketAddr::V4(peer_addr) = peer_addr {
                self.peer.set_endpoint(peer_addr);

                if self.use_connected_socket && !self.peer.is_connected() {
                    if &buf[..nbytes] == b"hello?" {
                        eprintln!("Connecting peer: {peer_addr}");
                        let sock = self.peer.connect_endpoint(self.listen_port)?;

                        let dev = Arc::clone(&self);
                        let _ = std::thread::spawn(move || {
                            if let Err(err) = dev.loop_listen_udp(sock) {
                                eprintln!("err loop 2 (2): {:?}", err);
                            }
                        });
                        continue;
                    }
                }

                self.iface.send(&buf[..nbytes])?;
            }
        }
    }
}

fn new_udp_socket(port: u16) -> io::Result<UdpSocket> {
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], port));

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true)?;

    socket.bind(&socket_addr.into())?;

    Ok(socket.into())
}

impl Peer {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint: RwLock::new(endpoint),
        }
    }

    pub fn set_endpoint(&self, addr: SocketAddrV4) {
        let mut endpoint = self.endpoint.write();
        if endpoint.addr != Some(addr) {
            *endpoint = Endpoint {
                addr: Some(addr),
                conn: None,
            }
        };
    }

    pub fn is_connected(&self) -> bool {
        let endpoint = self.endpoint.read();
        endpoint.conn.is_some()
    }

    pub fn connect_endpoint(&self, port: u16) -> io::Result<Arc<UdpSocket>> {
        let mut endpoint = self.endpoint.write();

        let conn = new_udp_socket(port)?;
        conn.connect(endpoint.addr.unwrap())?;
        let conn = Arc::new(conn);

        endpoint.conn = Some(Arc::clone(&conn));

        Ok(conn)
    }
}
