use clap::Parser;
use std::{eprintln, io, net::SocketAddr, sync::Arc};
use wontun::Device;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    peer: Option<String>,
}

fn run(peer_addr: Option<&str>) -> io::Result<()> {
    let iface = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;

    let peer = peer_addr
        .and_then(|addr| addr.parse::<SocketAddr>().ok())
        .and_then(|addr| {
            if let SocketAddr::V4(addr) = addr {
                Some(addr)
            } else {
                None
            }
        });

    let dev = Device::new(iface, peer);
    let dev1 = Arc::new(dev);
    let dev2 = Arc::clone(&dev1);

    let jh1 = std::thread::spawn(move || {
        if let Err(err) = dev1.loop_listen_iface() {
            eprintln!("err loop 1: {:?}", err);
        }
    });
    let jh2 = std::thread::spawn(move || {
        if let Err(err) = dev2.loop_listen_udp() {
            eprintln!("err loop 2: {:?}", err);
        }
    });

    jh1.join().unwrap();
    jh2.join().unwrap();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Start!");

    let args = Cli::parse();

    run(args.peer.as_deref())?;

    Ok(())
}
