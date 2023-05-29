use std::fs;
use std::io::Write;
use std::path::Path;
use std::{
    fs::OpenOptions,
    sync::mpsc::{self, Sender},
    thread,
};

use gtk::glib;

use crate::error::CustomError;
use crate::gui::init::GUIActions;

pub struct Logger {
    tx: Sender<String>,
}

impl Logger {
    pub fn new(
        filename: &String,
        gui_sender: glib::Sender<GUIActions>,
    ) -> Result<Self, CustomError> {
        let (tx, rx) = mpsc::channel();

        if Path::new(filename).exists() {
            fs::remove_file(filename).map_err(|_| CustomError::CannotRemoveFile)?;
        }

        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(filename)?;

        thread::spawn(move || -> Result<(), CustomError> {
            while let Ok(message) = rx.recv() {
                println!("logger: {}", message);
                writeln!(file, "{}", message)?;
                gui_sender.send(GUIActions::Log(message)).unwrap();
            }
            Ok(())
        });

        Ok(Self { tx })
    }
    pub fn get_sender(&self) -> Sender<String> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::time;

    use super::*;

    #[test]
    fn log_file_gets_written() {
        let (tx, _rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        println!("Testing");

        let logger = Logger::new(&String::from("test1.txt"), tx).unwrap();
        let sender = logger.get_sender();
        println!("Sender: {:?}", sender);
        sender.send(String::from("Sender test 1")).unwrap();
        sender.send(String::from("Sender test 2")).unwrap();
        thread::sleep(time::Duration::from_millis(100));

        assert_eq!(
            fs::read_to_string("test1.txt").unwrap(),
            String::from("Sender test 1\nSender test 2\n")
        );
        fs::remove_file("test1.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_senders() {
        let (tx, _rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let logger = Logger::new(&String::from("test2.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        sender1.send(String::from("Sender test 1")).unwrap();
        sender2.send(String::from("Sender test 2")).unwrap();
        thread::sleep(time::Duration::from_millis(100));

        assert_eq!(
            fs::read_to_string("test2.txt").unwrap(),
            String::from("Sender test 1\nSender test 2\n")
        );
        fs::remove_file("test2.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_threads() {
        let (tx, _rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let logger = Logger::new(&String::from("test3.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        thread::spawn(move || sender1.send(String::from("Sender test 1")).unwrap())
            .join()
            .unwrap();
        thread::spawn(move || sender2.send(String::from("Sender test 2")).unwrap())
            .join()
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        assert_eq!(
            fs::read_to_string("test3.txt").unwrap(),
            String::from("Sender test 1\nSender test 2\n")
        );
        fs::remove_file("test3.txt").unwrap();
    }
}
