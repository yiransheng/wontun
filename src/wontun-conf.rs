use clap::Parser;
use std::path::PathBuf;
use wontun::Conf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    conf: PathBuf,

    #[arg(long)]
    pretty: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let conf = std::fs::read_to_string(&args.conf)?;
    let conf = Conf::parse_from(&conf)?;

    let json_str = if args.pretty {
        serde_json::to_string_pretty(&conf)?
    } else {
        serde_json::to_string(&conf)?
    };

    println!("{json_str}");

    Ok(())
}
