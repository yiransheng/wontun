use std::io;
use std::net::{SocketAddr, UdpSocket};

use nix::sys::socket::setsockopt;
use nix::sys::socket::sockopt;
use socket2::{Domain, Protocol, Socket, Type};

pub fn new_socket(port: u16) -> io::Result<UdpSocket> {
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], port));

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    setsockopt(&socket, sockopt::ReusePort, &true)?;
    socket.set_nonblocking(true)?;

    socket.bind(&socket_addr.into())?;

    Ok(socket.into())
}
