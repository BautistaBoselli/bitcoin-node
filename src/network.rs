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

#[cfg(test)]

mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn get_addresses_returns_an_iterator_of_addresses_if_given_a_seed() {
        assert!(get_addresses("testnet-seed.bitcoin.jonasschnelli.ch".to_string(), 8333).is_ok());
    }

    #[test]
    fn get_addresses_returns_an_error_if_given_an_invalid_seed() {
        assert!(get_addresses("invalid.seed".to_string(), 8333).is_err());
    }

    //#[test]
    // fn open_stream_returns_a_tcp_stream_if_given_a_valid_address() {
    //     let stream = TcpStream::connect("invalid.address:4321");
    //     assert!(stream.is_err(), "Failed to connect to the server");
    // }

    #[test]
    fn get_address_v6_with_ipv4_address_maps_to_ipv6() {
        let address = SocketAddr::from(([127, 0, 0, 1], 8333));
        let address_v6 = get_address_v6(address);
        assert_eq!(
            address_v6.ip().to_owned(),
            Ipv6Addr::new(0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0xffff, 0x7f00, 0x0001)
        );
        assert_eq!(address_v6.port(), 8333);
    }

    #[test]
    fn get_address_v6_with_ipv6_address_returns_the_same_address() {
        let address = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 8333));
        let address_v6 = get_address_v6(address);
        assert_eq!(
            address_v6.ip().to_owned(),
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)
        );
        assert_eq!(address_v6.port(), 8333);
    }
}
