use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use wontun::{Conf, Device, DeviceConfig, Peer, PeerName};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    conf: PathBuf,

    #[arg(long)]
    log_level: Option<Level>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level.unwrap_or(Level::INFO))
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "setting default tracing subscriber failed")?;

    // tun interface name is derived from the file name of config file
    let Some(tun_name) = args.conf.file_stem().and_then(|s| s.to_str()) else {
        bail!("invalid conf file name")
    };
    let conf = std::fs::read_to_string(&args.conf).with_context(|| "failed to read conf file")?;
    let conf = Conf::parse_from(&conf).with_context(|| "conf file parse error")?;

    let mut dev = Device::new(DeviceConfig {
        name: PeerName::new(&conf.interface.name)?,
        tun_name,
        use_connected_peer: true,
        listen_port: conf.interface.listen_port,
    })
    .with_context(|| "cannot create a Device")?;

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

    dev.start()?;
    dev.wait();

    Ok(())
}
