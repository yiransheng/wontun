use clap::Parser;
use std::{
    io,
    net::{SocketAddr, SocketAddrV4},
    path::PathBuf,
};
use wontun::{Conf, Device, DeviceConfig, Peer, PeerName};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    conf: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let tun_name = args.conf.file_stem().and_then(|s| s.to_str()).unwrap();
    let conf = std::fs::read_to_string(&args.conf)?;
    let conf = Conf::parse_from(&conf)?;

    let mut dev = Device::new(DeviceConfig {
        name: PeerName::new(&conf.interface.name)?,
        tun_name,
        use_connected_peer: true,
        listen_port: conf.interface.listen_port,
    })?;

    for peer_conf in &conf.peers {
        let peer_name = PeerName::new(&peer_conf.name)?;
        let mut peer = Peer::new();
        if let Some(endpoint) = peer_conf.endpoint {
            peer.set_endpoint(endpoint);
        }
        for (ip, cidr) in &peer_conf.allowed_ips {
            peer.add_allowed_ip(*ip, *cidr);
        }
        dev.add_peer(peer_name, peer);
    }

    Ok(())
}
