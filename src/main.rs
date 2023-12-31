use bitcoin::{
    config::Config,
    gui::init::GUI,
    logger::{send_log, Log, Logger},
    loops::node_action_loop::NodeAction,
    node::Node,
    node_state::NodeState,
    utils::get_addresses,
};
use gtk::glib::{self, Priority};
use std::{env, path::Path};

const CANT_ARGS: usize = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < CANT_ARGS {
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
            println!("ERROR: {error}");
            return;
        }
    };

    let (gui_sender, gui_receiver) = glib::MainContext::channel(Priority::default());

    let logger = match Logger::new(&config.log_file, gui_sender.clone()) {
        Ok(logger) => logger,
        Err(error) => {
            println!("ERROR: {error}");
            return;
        }
    };

    let logger_sender = logger.get_sender();

    let node_state_ref = match NodeState::new(
        logger_sender.clone(),
        gui_sender.clone(),
        &config.store_path,
    ) {
        Ok(node_state) => node_state,
        Err(error) => {
            send_log(&logger_sender, Log::Error(error));
            return;
        }
    };

    let node = match Node::new(&config, &logger, node_state_ref.clone()) {
        Ok(node) => node,
        Err(error) => {
            send_log(&logger_sender, Log::Error(error));
            return;
        }
    };

    let node_action_sender = node.node_action_sender.clone();

    let addresses = get_addresses(config.seed.clone(), config.port);
    let addresses = match addresses {
        Ok(addresses) => addresses,
        Err(error) => {
            send_log(&logger_sender, Log::Error(error));
            return;
        }
    };

    let node_thread = node.spawn(addresses, gui_sender);

    let gui = GUI::start(
        gui_receiver,
        node_state_ref,
        logger_sender.clone(),
        node_action_sender.clone(),
    );

    if let Err(error) = gui {
        send_log(
            &logger_sender,
            Log::Message(format!("Error starting GUI: {}", error)),
        );
    };

    if node_action_sender.send(NodeAction::Terminate).is_ok() {
        if let Err(error) = node_thread.join() {
            send_log(
                &logger_sender,
                Log::Message(format!("Error closing node thread: {:?}", error)),
            );
        };
    }

    if logger.tx.send(Log::Terminate).is_ok() {
        if let Err(error) = logger.thread.join() {
            send_log(
                &logger_sender,
                Log::Message(format!("Error closing logger thread: {:?}", error)),
            );
        };
    }
}
