use std::{net::{ToSocketAddrs, SocketAddr}};
use std::vec::IntoIter;

use crate::{config::Config, error::CustomError};

/// Conecta con la semilla DNS y devuelve un iterador de direcciones IP.
/// Devuelve CustomError si:
/// - No se pudo resolver la semilla DNS.
pub fn connect(config: Config) -> Result<IntoIter<SocketAddr>, CustomError>{
    (config.seed, config.port).to_socket_addrs().map_err(|_| CustomError::CannotResolveSeedAddress)
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn connect_to_seed_invalida() {
        let config = Config{
            seed: String::from("seed.test"),
            protocol_version: 7000,
            port: 4321,
        };
        let addresses = connect(config);
        assert!(addresses.is_err());
    }

    #[test]
    fn connect_to_seed_valida() -> Result<(), CustomError> {
        let config = Config{
            seed: String::from("google.com"),
            protocol_version: 7000,
            port: 80,
        };
        let addresses = connect(config)?;
        assert!(addresses.len() > 0);
        Ok(())
    }
 }