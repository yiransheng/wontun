use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfError {
    #[error("invalid ini: {0}")]
    Ini(#[from] serde_ini::de::Error),

    #[error("invalid ip address: {0}")]
    IpFormat(String),

    #[error("multiple interface definition")]
    ExtraInterface,

    #[error("missing interface definition")]
    MissingInterface,
}

#[derive(Debug, Serialize)]
pub struct Conf {
    pub interface: InterfaceConf,
    pub peers: Vec<PeerConf>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InterfaceConf {
    pub name: String,
    pub address: (Ipv4Addr, u8),
    pub listen_port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PeerConf {
    pub name: String,
    pub endpoint: Option<SocketAddrV4>,
    pub allowed_ips: Vec<(Ipv4Addr, u8)>,
}

impl Conf {
    pub const DEFAULT_LISTEN_PORT: u16 = 19988;

    pub fn parse_from(source: &str) -> Result<Self, ConfError> {
        let sections: Vec<Section> = serde_ini::from_str(source)?;

        let mut interface = None;
        let mut peers = vec![];

        for section in sections.into_iter() {
            match section {
                Section::Peer {
                    Name,
                    Endpoint,
                    AllowedIPs,
                } => {
                    let allowed_ips: Vec<_> = AllowedIPs
                        .as_deref()
                        .unwrap_or("")
                        .split(',')
                        .map(|allowed_ip| allowed_ip.trim())
                        .filter_map(|allowed_ip| {
                            let ipn =
                                ip_network::Ipv4Network::from_str_truncate(allowed_ip).ok()?;
                            Some((ipn.network_address(), ipn.netmask()))
                        })
                        .collect();
                    let endpoint = Endpoint.and_then(|ep| SocketAddrV4::from_str(&ep).ok());
                    let peer = PeerConf {
                        name: Name,
                        allowed_ips,
                        endpoint,
                    };
                    peers.push(peer);
                }
                Section::Interface {
                    Name,
                    Address,
                    ListenPort,
                } => {
                    if interface.is_none() {
                        let address = parse_cidr(Address.trim())?;
                        interface = Some(InterfaceConf {
                            name: Name,
                            address,
                            listen_port: ListenPort.unwrap_or(Self::DEFAULT_LISTEN_PORT),
                        });
                    } else {
                        return Err(ConfError::ExtraInterface);
                    }
                }
            }
        }
        if let Some(interface) = interface {
            Ok(Conf { interface, peers })
        } else {
            Err(ConfError::MissingInterface)
        }
    }
}

fn parse_cidr(cidr: &str) -> Result<(Ipv4Addr, u8), ConfError> {
    let (ip_str, subnet_str) = cidr
        .split_once('/')
        .ok_or_else(|| ConfError::IpFormat("Invalid CIDR format".to_string()))?;

    let ip = ip_str
        .parse::<Ipv4Addr>()
        .map_err(|_| ConfError::IpFormat("Invalid IP address".to_string()))?;

    let subnet = subnet_str
        .parse::<u8>()
        .map_err(|_| ConfError::IpFormat("Invalid subnet mask".to_string()))?;

    if subnet > 32 {
        return Err(ConfError::IpFormat(
            "Subnet mask must be in the range 0-32".to_string(),
        ));
    }

    Ok((ip, subnet))
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[allow(non_snake_case)]
pub enum Section {
    Interface {
        Name: String,
        Address: String,
        ListenPort: Option<u16>,
    },
    Peer {
        Name: String,
        Endpoint: Option<String>,
        AllowedIPs: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let input = r#"
[Interface]
Name=client
Address=192.0.2.2/24
ListenPort=19988

[Peer]
Name=node1
PublicKey=ignore

[Peer]
Name=node2
AllowedIPs=192.0.2.1/24
"#;
        let conf_items: Vec<Section> = serde_ini::from_str(input).unwrap();
        assert_eq!(
            vec![
                Section::Interface {
                    Name: "client".into(),
                    Address: "192.0.2.2/32".into(),
                    ListenPort: Some(19988)
                },
                Section::Peer {
                    Name: "node1".into(),
                    Endpoint: None,
                    AllowedIPs: None,
                },
                Section::Peer {
                    Name: "node2".into(),
                    Endpoint: None,
                    AllowedIPs: Some("192.0.2.1/24".into())
                }
            ],
            conf_items
        );
    }
}
