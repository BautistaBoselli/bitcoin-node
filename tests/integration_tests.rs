#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::{BufRead, BufReader, Write},
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6, TcpStream},
        sync::{mpsc, Arc, Mutex},
        thread,
        time::Duration,
    };

    use bitcoin::{
        config::Config,
        logger::Logger,
        loops::{
            peer_action_loop::PeerAction, pending_blocks_loop::pending_blocks_loop,
            tcp_listener_loop::TcpListenerLoop,
        },
        node::Node,
        node_state::NodeState,
        peer::Peer,
        utils::get_addresses,
    };
    use gtk::glib::{self, Priority};

    #[test]
    fn node_and_state_creation() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("tests/test_log.txt"), gui_sender.clone()).unwrap();
        let logger_sender = logger.get_sender();

        let store_path = String::from("tests/store");
        let node_state_ref =
            NodeState::new(logger_sender.clone(), gui_sender, &store_path).unwrap();

        let config = Config::from_file("example-config.txt").unwrap();
        let node = Node::new(&config, &logger, node_state_ref.clone());
        assert!(node.is_ok());
        fs::remove_file("tests/test_log.txt").unwrap();
        fs::remove_dir_all("tests/store").unwrap();
    }

    #[test]
    fn handshake_peers() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());
        let mut addresses =
            get_addresses("seed.testnet.bitcoin.sprovoost.nl".to_string(), 18333).unwrap();
        let (_peer_action_sender, receiver) = mpsc::channel();
        let peer_action_receiver = Arc::new(Mutex::new(receiver));
        let (node_action_sender, _node_action_receiver) = mpsc::channel();
        let logger = Logger::new(&String::from("tests/test_log2.txt"), gui_sender.clone()).unwrap();
        let logger_sender = logger.get_sender();

        let peer = Peer::call(
            addresses.next().unwrap(),
            SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 18333, 0, 0),
            0,
            70012,
            peer_action_receiver.clone(),
            logger_sender.clone(),
            node_action_sender.clone(),
        );

        assert!(peer.is_ok());
        peer.unwrap();

        let peer2 = Peer::call(
            addresses.next().unwrap(),
            SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 18333, 0, 0),
            0,
            70012,
            peer_action_receiver,
            logger_sender,
            node_action_sender,
        );

        assert!(peer2.is_ok());
        peer2.unwrap();

        thread::sleep(Duration::from_secs(1));

        let reader = BufReader::new(File::open("tests/test_log2.txt").unwrap());
        let mut handshakes = 0;
        for line in reader.lines() {
            let line = line.unwrap();
            if line.contains("Successful handshake with") {
                handshakes += 1;
            }
        }
        assert_eq!(handshakes, 2);
        fs::remove_file("tests/test_log2.txt").unwrap();
    }

    #[test]
    fn node_state_pending_blocks_get_updated() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("tests/test_log3.txt"), gui_sender.clone()).unwrap();
        let logger_sender = logger.get_sender();

        let (peer_action_sender, receiver) = mpsc::channel();
        let peer_action_receiver = Arc::new(Mutex::new(receiver));

        let store_path = String::from("tests");
        let node_state_ref =
            NodeState::new(logger_sender.clone(), gui_sender, &store_path).unwrap();
        let node_state = node_state_ref.clone();
        let mut node_state = node_state.lock().unwrap();
        node_state.append_pending_block(vec![1, 2, 3]).unwrap();
        assert_eq!(node_state.is_pending_blocks_empty().unwrap(), false);
        drop(node_state);

        pending_blocks_loop(node_state_ref, peer_action_sender, logger_sender);

        thread::sleep(Duration::from_secs(5));

        let message = peer_action_receiver.lock().unwrap().recv().unwrap();
        if let PeerAction::GetData(_) = message {
            assert!(true);
        } else {
            assert!(false);
        }
        fs::remove_file("tests/test_log3.txt").unwrap();
    }

    #[test]
    fn node_anwsers_handshakes() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("tests/test_log4.txt"), gui_sender.clone()).unwrap();
        let logger_sender = logger.get_sender();

        let (_peer_action_sender, receiver) = mpsc::channel();
        let peer_action_receiver = Arc::new(Mutex::new(receiver));

        let (node_action_sender, _node_action_receiver) = mpsc::channel();

        let store_path = String::from("tests");
        let node_state_ref =
            NodeState::new(logger_sender.clone(), gui_sender, &store_path).unwrap();

        let _tcp_listener = TcpListenerLoop::spawn(
            logger_sender.clone(),
            node_state_ref.clone(),
            SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 18334, 0, 0),
            123,
            70015,
            peer_action_receiver.clone(),
            node_action_sender.clone(),
        );
        let mut addresses = get_addresses("127.0.0.1".to_string(), 18334).unwrap();
        thread::sleep(Duration::from_secs(5));

        let peer = Peer::call(
            addresses.next().unwrap(),
            SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 2), 18334, 0, 0),
            1,
            13,
            peer_action_receiver.clone(),
            logger_sender.clone(),
            node_action_sender.clone(),
        )
        .unwrap();

        assert_eq!(peer.version, 70015);
        assert_eq!(peer.services, 123);

        thread::sleep(Duration::from_secs(1));

        let mut node_state = node_state_ref.lock().unwrap();
        let peers = node_state.get_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].version, 13);
        assert_eq!(peers[0].services, 1);
        drop(node_state);

        fs::remove_file("tests/test_log4.txt").unwrap();
    }
}
