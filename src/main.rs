use bitcoin::{config::Config, logger::Logger, network::get_addresses, node::Node};
use std::{env, path::Path};

const CANT_ARGS: usize = 2;

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

    let logger = match Logger::new(&config.log_file) {
        Ok(logger) => logger,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };
    let logger_sender = logger.get_sender();

    let addresses = match get_addresses(config.seed.clone(), config.port) {
        Ok(addresses) => addresses,
        Err(error) => {
            match logger_sender.send(format!("ERROR: {}", error)) {
                Ok(_) => (),
                Err(error) => println!("ERROR: {}", error),
            }
            return;
        }
    };

    let _my_node = match Node::new(&config, &logger, addresses) {
        Ok(node) => node,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    println!("Finished main tasks");
}
