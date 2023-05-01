use core::fmt;

#[derive(Debug)]

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
}

impl CustomError {
    /// Devuelve un string con la descripcion del error.
    fn description(&self) -> &str {
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
        }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description())
    }
}
