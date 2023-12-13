use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[allow(non_snake_case)]
pub enum Conf {
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
        let conf_items: Vec<Conf> = serde_ini::from_str(input).unwrap();
        assert_eq!(
            vec![
                Conf::Interface {
                    Name: "client".into(),
                    Address: Some("192.0.2.2/32".into()),
                    ListenPort: Some(19988)
                },
                Conf::Peer {
                    Name: "node1".into(),
                    Endpoint: None,
                    AllowedIPs: None,
                },
                Conf::Peer {
                    Name: "node2".into(),
                    Endpoint: None,
                    AllowedIPs: Some("192.0.2.1/24".into())
                }
            ],
            conf_items
        );
    }
}
