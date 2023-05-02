use bitcoin::{
    config::Config,
    error::CustomError,
    logger::Logger,
    node::Node,
    peer::{get_addresses, Peer},
};
use std::{env, path::Path};
use std::{net::SocketAddr, vec::IntoIter};

const CANT_ARGS: usize = 2;

/// Obtiene la configuracion del archivo de configuracion. En esta se encuentra la semilla DNS y la version del protocolo.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != CANT_ARGS {
        println!("ERROR: config file path missing");
        return;
    }
    let path = Path::new(&args[1]);
    if !path.exists() {
        println!("ERROR: config file not found at {}", path.display());
        return;
    }

    let config = match Config::from_file(args[1].as_str()) {
        Ok(config) => config,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let my_node = Node::new(&config);

    let addresses = match get_addresses(config.seed, my_node.port) {
        Ok(addresses) => addresses,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let logger = Logger::new(&config.log_file);
    let _node_threads = match handshake_all_nodes(addresses, &my_node, &logger) {
        Ok(node_threads) => node_threads,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };
}

fn handshake_all_nodes(
    addresses: IntoIter<SocketAddr>,
    my_node: &Node,
    logger: &Logger,
) -> Result<Vec<Peer>, CustomError> {
    let mut handles: Vec<Peer> = vec![];
    let logger_sender = logger.get_sender();

    logger_sender
        .send(format!("Handshaking with {} nodes", addresses.len()))
        .map_err(|_| CustomError::Logging)?;

    for address in addresses {
        let peer = match Peer::new(address, my_node) {
            Ok(peer) => peer,
            Err(error) => {
                logger_sender
                    .send(format!("ERROR: {} {}", error, address.ip()))
                    .map_err(|_| CustomError::Logging)?;
                continue;
            }
        };
        logger_sender
            .send(format!("Handshake succesful with {}", peer.ip_v6))
            .map_err(|_| CustomError::Logging)?;

        handles.push(peer);
    }

    Ok(handles)
}
