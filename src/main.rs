use bitcoin::{
    config::Config,
    error::CustomError,
    node::Node,
    peer::{get_addresses, Peer},
};
use std::{
    env,
    net::Ipv4Addr,
    path::Path,
    thread::{self, JoinHandle},
};
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

    let addresses = match get_addresses(config.seed.clone(), my_node.port) {
        Ok(addresses) => addresses,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let node_threads = handshake_all_nodes(addresses, &my_node);
    // let node_threads = match handshake_all_nodes(addresses, &my_node) {
    //     Ok(node_threads) => node_threads,
    //     Err(error) => {
    //         println!("ERROR: {}", error);
    //         return;
    //     }
    // };
}

fn handshake_all_nodes(addresses: IntoIter<SocketAddr>, my_node: &Node) -> Vec<Peer> {
    let mut handles: Vec<Peer> = vec![];

    println!("Handshaking with {} nodes", addresses.len());

    for address in addresses {
        let peer = match Peer::new(address, &my_node) {
            Ok(peer) => peer,
            Err(error) => {
                println!("ERROR: {} {}", error, address.ip());
                continue;
            }
        };
        println!("Handshake succesful with {}", peer.ip_v6);
        handles.push(peer);
    }

    handles
}
