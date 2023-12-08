use parking_lot::{Mutex, MutexGuard};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use tun_tap::Iface;

use std::io;

pub struct Peer {
    endpoint: Mutex<Option<SocketAddrV4>>,
}

pub struct Device {
    udp: UdpSocket,
    iface: Iface,
    peer: Peer,
}

impl Device {
    pub fn new(iface: Iface, peer: Option<SocketAddrV4>) -> Self {
        let udp = new_udp_socket(19988).unwrap();
        Self {
            udp,
            iface,
            peer: Peer {
                endpoint: Mutex::new(peer),
            },
        }
    }

    pub fn loop_listen_iface(&self) -> io::Result<()> {
        let mut buf = [0u8; 1504];
        {
            let peer = self.peer.endpoint();
            if let Some(peer_addr) = peer.as_ref() {
                eprintln!("initiating \"handshake\" to peer: {peer_addr}");
                self.udp.send_to("hello?".as_bytes(), peer_addr)?;
            }
        }

        loop {
            let nbytes = self.iface.recv(&mut buf[..])?;
            match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
                Ok(iph) => {
                    let src = iph.source_addr();
                    let dst = iph.destination_addr();
                    eprintln!("Got Ipv4 packet of size: {nbytes}, {src} -> {dst}, from tun0");
                }
                _ => {}
            }
            let peer = self.peer.endpoint();
            if let Some(peer_addr) = peer.as_ref() {
                self.udp.send_to(&buf[..nbytes], peer_addr)?;
            } else {
                eprintln!("..no peer");
            }
        }
    }

    pub fn loop_listen_udp(&self) -> io::Result<()> {
        let mut buf = [0u8; 1504];

        loop {
            let (nbytes, peer_addr) = self.udp.recv_from(&mut buf[..])?;
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

            if let SocketAddr::V4(peer_addr_v4) = peer_addr {
                if &buf[..nbytes] == b"hello?" {
                    self.peer.set_endpoint(peer_addr_v4);
                    continue;
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
    fn endpoint(&self) -> MutexGuard<Option<SocketAddrV4>> {
        self.endpoint.lock()
    }

    fn set_endpoint(&self, addr: SocketAddrV4) {
        let mut endpoint = self.endpoint.lock();
        if endpoint.is_none() {
            *endpoint = Some(addr);
        }
    }
}
