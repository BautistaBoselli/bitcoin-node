use bitcoin::{
    config::Config,
    node::{get_addresses, Node},
};
use std::{env, net::Ipv4Addr, path::Path};

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

    let my_node = Node {
        ip_v6: Ipv4Addr::new(0, 0, 0, 0).to_ipv6_mapped(),
        services: 0x00,
        port: config.port,
        version: config.protocol_version,
        stream: None,
        handshake: false,
    };

    let mut addresses = match get_addresses(config.seed.clone(), config.port) {
        Ok(addresses) => addresses,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let first_address = match addresses.next() {
        Some(address) => address,
        None => {
            println!("ERROR: no addresses found");
            return;
        }
    };

    let mut first_node = match Node::new(first_address) {
        Ok(node) => node,
        Err(_) => {
            println!("ERROR: no addresses found");
            return;
        }
    };

    match first_node.handshake(&my_node) {
        Ok(_) => println!("Handshake successful"),
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    println!("First Node: {:?}", first_node);

    //SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xf,0xf,0xf,0xf,0, 0, 0, 0)), 0)
}
