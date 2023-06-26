use std::fs;
use std::io::Write;
use std::path::Path;
use std::{
    fs::OpenOptions,
    sync::mpsc::{self, Sender},
    thread,
};

use chrono::Local;
use gtk::glib;

use crate::error::CustomError;
use crate::gui::init::GUIEvents;

#[derive(Debug, Clone)]
/// Log es el tipo de dato que se envia al logger.
/// Puede ser un Message o un Error.
/// Los Message reciben un String y esos seran los logs que se guarden en el archivo, impriman en consola y se muestren en la interfaz.
/// Los Error reciben un CustomError y haran los mismo que los Message, y ademas muestran una ventana popup con el error.
pub enum Log {
    Message(String),
    Error(CustomError),
}

/// Logger es una estructura que contiene los elementos necesarios para manejar los logs.
/// Los elementos son:
/// - tx: Sender para enviar logs al logger.
pub struct Logger {
    tx: Sender<Log>,
}

impl Logger {
    /// Inicializa el logger.
    /// Si el archivo donde se guardan los logs existe, lo borra.
    /// Crea el archivo de logs en la ubicacion recibida
    /// Inicializa el thread que escucha los Logs recibidos por un channel que crea.
    /// Los logs se manejan como se comenta en el enum Log.
    pub fn new(
        filename: &String,
        gui_sender: glib::Sender<GUIEvents>,
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
                        let current_time = Local::now();
                        let formatted_time = current_time.format("%Y-%m-%d %H:%M:%S");
                        println!("[{}] {}", formatted_time, string);
                        writeln!(file, "[{}] {}", formatted_time, string)?;
                        if let Err(error) = gui_sender.send(GUIEvents::Log(message)) {
                            println!("Error sending log message to gui: {}", error);
                        }
                    }
                    Log::Error(ref error) => {
                        let current_time = Local::now();
                        let formatted_time = current_time.format("%Y-%m-%d %H:%M:%S");
                        println!("[{}] [ERROR] {}", formatted_time, error);
                        writeln!(file, "[{}] [ERROR] {}", formatted_time, error)?;
                        if let Err(error) = gui_sender.send(GUIEvents::Log(message)) {
                            println!("Error sending log error to gui: {}", error);
                        }
                    }
                }
            }
            Ok(())
        });

        Ok(Self { tx })
    }

    /// Devuelve el sender para enviar logs al logger.
    pub fn get_sender(&self) -> Sender<Log> {
        self.tx.clone()
    }
}

/// Funcion auxiliar para enviar logs al logger.
/// Si no se puede enviar el log, se imprime el error y se imprime el mensaje original.
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

        let logger = Logger::new(&String::from("test1.txt"), tx).unwrap();
        let sender = logger.get_sender();
        let timestamp_string_1 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender
            .send(Log::Message(String::from("Sender test 1")))
            .unwrap();
        let timestamp_string_2 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender
            .send(Log::Message(String::from("Sender test 2")))
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        let result = format!(
            "[{}] Sender test 1\n[{}] Sender test 2\n",
            timestamp_string_1, timestamp_string_2
        );
        assert_eq!(fs::read_to_string("test1.txt").unwrap(), result);
        fs::remove_file("test1.txt").unwrap();
    }

    #[test]
    fn log_error_gets_written() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("test2.txt"), tx).unwrap();
        let sender = logger.get_sender();
        let timestamp_string_1 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender
            .send(Log::Error(CustomError::CannotRemoveFile))
            .unwrap();
        let timestamp_string_2 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender
            .send(Log::Error(CustomError::CannotRemoveFile))
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        let result = format!(
            "[{}] [ERROR] Error: cannot remove file\n[{}] [ERROR] Error: cannot remove file\n",
            timestamp_string_1, timestamp_string_2
        );
        assert_eq!(fs::read_to_string("test2.txt").unwrap(), result);
        fs::remove_file("test2.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_senders() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("test3.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        let timestamp_string_1 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender1
            .send(Log::Message(String::from("Sender test 1")))
            .unwrap();
        let timestamp_string_2 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sender2
            .send(Log::Message(String::from("Sender test 2")))
            .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        let result = format!(
            "[{}] Sender test 1\n[{}] Sender test 2\n",
            timestamp_string_1, timestamp_string_2
        );
        assert_eq!(fs::read_to_string("test3.txt").unwrap(), result);
        fs::remove_file("test3.txt").unwrap();
    }

    #[test]
    fn log_file_gets_written_by_two_threads() {
        let (tx, _rx) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("test4.txt"), tx).unwrap();
        let sender1 = logger.get_sender();
        let sender2 = logger.get_sender();
        let timestamp_string_1 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        thread::spawn(move || {
            sender1
                .send(Log::Message(String::from("Sender test 1")))
                .unwrap()
        })
        .join()
        .unwrap();
        let timestamp_string_2 = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        thread::spawn(move || {
            sender2
                .send(Log::Message(String::from("Sender test 2")))
                .unwrap()
        })
        .join()
        .unwrap();
        thread::sleep(time::Duration::from_millis(100));

        let result = format!(
            "[{}] Sender test 1\n[{}] Sender test 2\n",
            timestamp_string_1, timestamp_string_2
        );
        assert_eq!(fs::read_to_string("test4.txt").unwrap(), result);
        fs::remove_file("test4.txt").unwrap();
    }
}
