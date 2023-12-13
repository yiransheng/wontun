use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfError {
    #[error("invalid ini: {0}")]
    Ini(#[from] serde_ini::de::Error),

    #[error("invalid ip address: {0}")]
    IpFormat(#[from] ip_network::IpNetworkParseError),

    #[error("multiple interface definition")]
    ExtraInterface,
}

pub struct Conf {
    pub interface: Option<InterfaceConf>,
    pub peers: Vec<PeerConf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceConf {
    pub name: String,
    pub address: Option<(Ipv4Addr, u8)>,
    pub listen_port: u16,
}

pub struct PeerConf {
    pub name: String,
    pub endpoint: Option<IpAddr>,
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
                        .split(",")
                        .map(|allowed_ip| allowed_ip.trim())
                        .filter_map(|allowed_ip| {
                            let ipn =
                                ip_network::Ipv4Network::from_str_truncate(allowed_ip).ok()?;
                            Some((ipn.network_address(), ipn.netmask()))
                        })
                        .collect();
                    let endpoint = Endpoint.and_then(|ep| IpAddr::from_str(&ep).ok());
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
                        let address = if let Some(ref address) = Address {
                            let ipn = ip_network::Ipv4Network::from_str(address)?;
                            Some((ipn.network_address(), ipn.netmask()))
                        } else {
                            None
                        };
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

        Ok(Conf { interface, peers })
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[allow(non_snake_case)]
pub enum Section {
    Interface {
        Name: String,
        Address: Option<String>,
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
Address=192.0.2.2/32
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
                    Address: Some("192.0.2.2/32".into()),
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
