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

#[derive(Debug, Clone)]
pub enum Log {
    Message(String),
    Error(CustomError),
}

pub struct Logger {
    tx: Sender<Log>,
}

impl Logger {
    pub fn new(
        filename: &String,
        gui_sender: glib::Sender<GUIActions>,
    ) -> Result<Self, CustomError> {
        let (tx, rx) = mpsc::channel::<Log>();

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
                match message {
                    Log::Message(ref string) => {
                        println!("logger: {}", string);
                        writeln!(file, "{}", string)?;
                        gui_sender.send(GUIActions::Log(message)).unwrap();
                    }
                    Log::Error(ref error) => {
                        println!("logger: [ERROR] {}", error);
                        writeln!(file, "[ERROR] {}", error)?;
                        gui_sender.send(GUIActions::Log(message)).unwrap();
                    }
                }
            }
            Ok(())
        });

        Ok(Self { tx })
    }
    pub fn get_sender(&self) -> Sender<Log> {
        self.tx.clone()
    }
}

pub fn send_log(logger_sender: &Sender<Log>, message: Log) {
    if let Err(error) = logger_sender.send(message.clone()) {
        println!("Error sending log message: {}", error);
        println!("Original message: {:?}", message);
    }
}

#[cfg(test)]
mod tests {
    use std::time;

    use gtk::glib::Priority;

    use super::*;

    #[test]
    fn log_file_gets_written() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());
        println!("Testing");

        let logger = Logger::new(&String::from("test1.txt"), tx).unwrap();
        let sender = logger.get_sender();
        println!("Sender: {:?}", sender);
        sender
            .send(Log::Message(String::from("Sender test 1")))
            .unwrap();
        sender
            .send(Log::Message(String::from("Sender test 2")))
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        assert_eq!(
            fs::read_to_string("test1.txt").unwrap(),
            String::from("Sender test 1\nSender test 2\n")
        );
        fs::remove_file("test1.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_senders() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("test2.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        sender1
            .send(Log::Message(String::from("Sender test 1")))
            .unwrap();
        sender2
            .send(Log::Message(String::from("Sender test 2")))
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        assert_eq!(
            fs::read_to_string("test2.txt").unwrap(),
            String::from("Sender test 1\nSender test 2\n")
        );
        fs::remove_file("test2.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_threads() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("test3.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        thread::spawn(move || {
            sender1
                .send(Log::Message(String::from("Sender test 1")))
                .unwrap()
        })
        .join()
        .unwrap();
        thread::spawn(move || {
            sender2
                .send(Log::Message(String::from("Sender test 2")))
                .unwrap()
        })
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
