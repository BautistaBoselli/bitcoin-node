use std::{
    net::{SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs},
    vec::IntoIter,
};

use crate::error::CustomError;

pub fn get_addresses(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError> {
    (seed, port)
        .to_socket_addrs()
        .map_err(|_| CustomError::CannotResolveSeedAddress)
}

pub fn open_stream(address: SocketAddrV6) -> Result<TcpStream, CustomError> {
    TcpStream::connect(address).map_err(|_| CustomError::CannotConnectToNode)
}

pub fn get_address_v6(address: SocketAddr) -> SocketAddrV6 {
    let ip_v6 = match address {
        SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
        SocketAddr::V6(addr) => addr.ip().to_owned(),
    };
    SocketAddrV6::new(ip_v6, address.port(), 0, 0)
}
