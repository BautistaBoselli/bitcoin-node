use std::net::Ipv6Addr;

use crate::{config::Config, peer::Peer};

pub struct Node {
    pub ip_v6: Ipv6Addr,
    pub services: u64,
    pub port: u16,
    pub version: i32,
    pub peers: Vec<Peer>,
}

impl Node {
    pub fn new(config: &Config) -> Self {
        Node {
            ip_v6: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            services: 0x00,
            port: config.port,
            version: config.protocol_version,
            peers: vec![],
        }
    }
}
