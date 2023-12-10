use clap::Parser;
use std::{
    io,
    net::{SocketAddr, SocketAddrV4},
};
use wontun::{Device, DeviceConfig};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    peer: Option<String>,
}

impl Cli {
    fn peer_addr(&self) -> Option<SocketAddrV4> {
        self.peer
            .as_ref()
            .and_then(|addr| addr.parse::<SocketAddr>().ok())
            .and_then(|addr| {
                if let SocketAddr::V4(addr) = addr {
                    Some(addr)
                } else {
                    None
                }
            })
    }
}

fn run(peer: Option<SocketAddrV4>) -> io::Result<()> {
    let conf = DeviceConfig::new(peer.is_none(), 19988, "tun0", peer);

    let dev = Device::new(conf)?;
    dev.start()?;
    dev.wait();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    run(args.peer_addr())?;

    Ok(())
}
