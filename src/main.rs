use bitcoin::{
    config::Config,
    gui::init::{gui_init, MyStruct},
    logger::Logger,
    network::get_addresses,
    node::Node,
};
use gtk::glib::{self, Priority};
use std::{
    env,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

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

    let (gui_sender, gui_receiver) = glib::MainContext::channel(Priority::default());

    let logger = match Logger::new(&config.log_file, gui_sender.clone()) {
        Ok(logger) => logger,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    let logger_sender = logger.get_sender();
    let logger_sender_clone = logger_sender.clone();

    thread::spawn(move || {
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

        let _my_node = match Node::new(&config, &logger, addresses, gui_sender.clone()) {
            Ok(node) => node,
            Err(error) => {
                println!("ERROR: {}", error);
                return;
            }
        };
    });

    let number = Arc::new(Mutex::new(MyStruct { vec: vec![] }));
    let number_clone = number.clone();

    thread::spawn(move || loop {
        let mut number = number_clone.lock().unwrap();
        (*number).vec.push(1);
        drop(number);
        thread::sleep(std::time::Duration::from_secs(1));
    });

    match gui_init(gui_receiver, number) {
        Err(error) => {
            logger_sender_clone
                .send(format!("ERROR: {}", error))
                .unwrap_or_else(|_| {
                    println!("ERROR: {}", error);
                    println!("ERROR: cannot send error to logger thread");
                });
            return;
        }
        _ => (),
    };
}
