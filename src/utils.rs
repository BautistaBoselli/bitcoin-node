use std::{
    fs::OpenOptions,
    net::{SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs},
    time::{Duration, SystemTime},
    vec::IntoIter,
};

use crate::error::CustomError;

pub fn get_addresses(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError> {
    (seed, port)
        .to_socket_addrs()
        .map_err(|_| CustomError::CannotResolveSeedAddress)
}

pub fn open_stream(address: SocketAddr) -> Result<TcpStream, CustomError> {
    TcpStream::connect_timeout(&address, Duration::from_millis(500))
        .map_err(|_| CustomError::CannotConnectToNode)
}

pub fn get_address_v6(address: SocketAddr) -> SocketAddrV6 {
    let ip_v6 = match address {
        SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
        SocketAddr::V6(addr) => addr.ip().to_owned(),
    };
    SocketAddrV6::new(ip_v6, address.port(), 0, 0)
}

pub fn open_new_file(path_to_file: String, append: bool) -> Result<std::fs::File, CustomError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(append)
        .open(path_to_file)?;
    Ok(file)
}

pub fn get_current_timestamp() -> Result<u64, CustomError> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}

#[cfg(test)]

mod tests {
    use std::{
        fs::{self, remove_file},
        io::Write,
        net::Ipv6Addr,
    };

    use super::*;

    #[test]
    fn get_addresses_returns_an_iterator_of_addresses_if_given_a_seed() {
        assert!(get_addresses("google.com".to_string(), 80).is_ok());
    }

    #[test]
    fn get_addresses_returns_an_error_if_given_an_invalid_seed() {
        assert!(get_addresses("invalid.seed".to_string(), 4321).is_err());
    }

    #[test]
    fn open_stream_returns_a_tcp_stream_if_given_a_valid_address() {
        let address = "google.com:80".to_socket_addrs().unwrap().next().unwrap();
        let stream = open_stream(address);
        assert!(stream.is_ok());
    }

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

    #[test]
    fn test_get_current_timestamp() {
        assert!(get_current_timestamp().is_ok());
        assert!(get_current_timestamp().unwrap() > 1687668678);
    }

    #[test]
    fn test_open_new_file_creates_new_if_doesnt_exist() {
        let mut file = open_new_file("tests/does_not_exist.txt".to_string(), false).unwrap();

        assert!(file.write_all(b"test").is_ok());

        remove_file("tests/does_not_exist.txt").unwrap();
    }

    #[test]
    fn test_open_new_file_existing_file() {
        fs::copy("tests/does_exist.txt", "tests/does_exist_copy.txt").unwrap();
        let mut file = open_new_file("tests/does_exist_copy.txt".to_string(), true).unwrap();

        assert!(file.write_all(b"test").is_ok());

        remove_file("tests/does_exist_copy.txt").unwrap();
    }
}
