use clap::Parser;
use std::{eprintln, io, net::SocketAddr};
use wontun::{Device, Endpoint, Peer};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    peer: Option<String>,
}

fn run(peer_addr: Option<&str>) -> io::Result<()> {
    let iface = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;

    let use_connected_socket = peer_addr.is_none();
    let peer = Peer::new(Endpoint::default());

    let sock = peer_addr
        .and_then(|addr| addr.parse::<SocketAddr>().ok())
        .and_then(|addr| {
            if let SocketAddr::V4(addr) = addr {
                eprintln!("Peer: {addr}");
                peer.set_endpoint(addr);
                peer.connect_endpoint(19988).ok()
            } else {
                None
            }
        });

    Device::new(use_connected_socket, 19988, iface, peer).start(sock)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Start!");

    let args = Cli::parse();

    run(args.peer.as_deref())?;

    Ok(())
}
