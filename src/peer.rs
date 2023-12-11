use std::net::{SocketAddrV4, UdpSocket};

use std::io;
use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard};

use crate::new_udp_socket;

pub struct Peer {
    endpoint: RwLock<Endpoint>,
}

#[derive(Default)]
pub struct Endpoint {
    pub addr: Option<SocketAddrV4>,
    pub conn: Option<Arc<UdpSocket>>,
}

impl Peer {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint: RwLock::new(endpoint),
        }
    }

    pub fn endpoint(&self) -> RwLockReadGuard<Endpoint> {
        self.endpoint.read()
    }

    pub fn set_endpoint(&self, addr: SocketAddrV4) -> (bool, Option<Arc<UdpSocket>>) {
        let endpoint = self.endpoint.read();
        if endpoint.addr == Some(addr) {
            return (false, None);
        }
        drop(endpoint);

        let mut endpoint = self.endpoint.write();
        endpoint.addr = Some(addr);

        (true, endpoint.conn.take())
    }

    pub fn connect_endpoint(&self, port: u16) -> io::Result<Arc<UdpSocket>> {
        let mut endpoint = self.endpoint.write();
        let addr = endpoint.addr.unwrap();

        assert!(endpoint.conn.is_none());

        let conn = new_udp_socket(port)?;
        conn.connect(addr)?;
        let conn = Arc::new(conn);

        endpoint.conn = Some(Arc::clone(&conn));

        Ok(conn)
    }
}
