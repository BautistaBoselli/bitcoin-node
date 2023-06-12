use core::fmt;
use std::{
    io::{Error, ErrorKind},
    sync::{
        mpsc::{RecvError, SendError},
        PoisonError,
    },
    time::SystemTimeError,
};

#[derive(Debug, Clone)]

/// Custom error es un enum con los posibles errores que pueden ocurrir en el programa.
/// Cada variante representa un error distinto.
/// Cada variante debe tener un metodo description que devuelve un string con la descripcion del error.
pub enum CustomError {
    ConfigInvalid,
    ConfigMissingValue,
    ConfigMissingFile,
    ConfigErrorReadingValue,
    CannotResolveSeedAddress,
    CannotConnectToNode,
    CannotHandshakeNode,
    SerializedBufferIsInvalid,
    InvalidHeader,
    CommandNotImplemented,
    Logging,
    CannotReadMessageHeader,
    CannotOpenFile,
    CannotSendMessageToChannel,
    CloneFailed,
    CannotLockGuard,
    CannotReceiveMessageFromChannel,
    CannotRemoveFile,
    FileOperationInterrupted,
    HeaderInvalidPoW,
    InvalidMerkleRoot,
    UnknownError,
    CannotInitGUI,
    CannotGetTimestamp,
    WalletNotFound,
    Validation(String),
}

impl CustomError {
    /// Devuelve un string con la descripcion del error.
    pub fn description(&self) -> &str {
        match self {
            Self::ConfigInvalid => "invalid config file",
            Self::ConfigMissingValue => "missing config values",
            Self::ConfigMissingFile => "missing config file",
            Self::ConfigErrorReadingValue => "error reading config value",
            Self::CannotResolveSeedAddress => "cannot resolve seed address",
            Self::CannotConnectToNode => "cannot connect to node",
            Self::CannotHandshakeNode => "cannot handshake with node",
            Self::SerializedBufferIsInvalid => "serialized buffer is invalid",
            Self::InvalidHeader => "invalid header",
            Self::CommandNotImplemented => "command not implemented",
            Self::Logging => "couldn't send log",
            Self::CannotReadMessageHeader => "cannot read message header",
            Self::CannotOpenFile => "cannot open file",
            Self::CannotSendMessageToChannel => "receiving end of a channel is disconected",
            Self::CloneFailed => "couldn't clone endpoint",
            Self::CannotLockGuard => "another user of mutex panicked while holding the mutex,",
            Self::CannotReceiveMessageFromChannel => {
                "cannot receive message from channel because sender has disconnected"
            }
            Self::CannotRemoveFile => "cannot remove file",
            Self::FileOperationInterrupted => "file operation interrupted",
            Self::HeaderInvalidPoW => "header hash does not satisfy the proof of work dificulty",
            Self::InvalidMerkleRoot => "invalid merkle root",
            Self::UnknownError => "unknown error",
            Self::CannotInitGUI => "cannot init GUI",
            Self::CannotGetTimestamp => "cannot get timestamp",
            Self::WalletNotFound => "wallet not found",
            Self::Validation(_) => "validation error",
        }
    }
}

impl From<Error> for CustomError {
    fn from(error: Error) -> Self {
        match error.kind() {
            ErrorKind::NotFound => CustomError::CannotOpenFile,
            ErrorKind::PermissionDenied => CustomError::CannotOpenFile,
            ErrorKind::AlreadyExists => CustomError::CannotOpenFile,
            ErrorKind::InvalidInput => CustomError::CannotOpenFile,
            ErrorKind::Interrupted => CustomError::FileOperationInterrupted,
            _ => CustomError::UnknownError,
        }
    }
}
impl<T> From<SendError<T>> for CustomError {
    fn from(_error: SendError<T>) -> Self {
        CustomError::CannotSendMessageToChannel
    }
}
impl From<RecvError> for CustomError {
    fn from(_error: RecvError) -> Self {
        CustomError::CannotReceiveMessageFromChannel
    }
}

impl<T> From<PoisonError<T>> for CustomError {
    fn from(_error: PoisonError<T>) -> Self {
        CustomError::CannotLockGuard
    }
}

impl From<SystemTimeError> for CustomError {
    fn from(_error: SystemTimeError) -> Self {
        CustomError::CannotGetTimestamp
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description())
    }
}
