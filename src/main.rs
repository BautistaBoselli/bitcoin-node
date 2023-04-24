use bitcoin::{config::Config, connect::{connect, Node}};
use std::{env, path::Path, clone};

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

    let mut addresses = match connect(config.seed.clone(), config.port){
        Ok(addresses) => addresses,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let first_adress = match addresses.next(){
        Some(address) => {
            let node = Node{
                ipv6: address.ip(),
                services: 0x00,
                port: address.port(),
                version: config.protocol_version,
            };
        },
        None => {
            println!("ERROR: no addresses found");
            return;
        }
    };

    //SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xf,0xf,0xf,0xf,0, 0, 0, 0)), 0)
    

    }

