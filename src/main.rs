use bitcoin::{config::Config, node::{Node, get_addresses}};
use std::{env, path::Path, net::{Ipv6Addr}};

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

    println!("Config: {:?}", config);

    let my_node = Node {
        ipv6: Ipv6Addr::new(0xf,0xf,0xf,0xf,0, 0, 0, 0),
        services: 0x00,
        port: config.port,
        version: config.protocol_version,
    };


    let mut addresses = match get_addresses(config.seed.clone(), config.port){
        Ok(addresses) => addresses,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let first_address = match addresses.next(){
        Some(address) => match address {
            std::net::SocketAddr::V6(address) => Node::from_address(my_node, address),
            std::net::SocketAddr::V4(address) => {
                let address_v6 = std::net::SocketAddrV6::new(address.ip().to_ipv6_mapped(), address.port(), 0, 0);
                    Node::from_address(my_node, address_v6)
            }

            
        },
        None => {
            println!("ERROR: no addresses found");
            return;
        }
    };

    println!("First address: {:?}", first_address);



    //SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xf,0xf,0xf,0xf,0, 0, 0, 0)), 0)
    

    }

