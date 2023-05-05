use bitcoin::peer::get_addresses;
use bitcoin::peer_worker::PeerAction;
use bitcoin::{config::Config, logger::Logger, node::Node};
use std::{env, path::Path};

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

    let logger = Logger::new(&config.log_file);
    let logger_sender = logger.get_sender();

    let addresses = match get_addresses(config.seed.clone(), config.port) {
        Ok(addresses) => addresses,
        Err(error) => {
            logger_sender.send(format!("ERROR: {}", error)).unwrap();
            return;
        }
    };

    let mut my_node = Node::new(&config, &logger);

    my_node.connect(addresses);
    my_node.execute(PeerAction::GetHeaders);

    // for _i in 0..20 {
    //     my_node.execute(PeerAction::Echo(
    //         "Soy un peer hablando en concurrencia".to_string(),
    //     ))
    // }
}
