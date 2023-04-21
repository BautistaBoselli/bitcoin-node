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
}

impl CustomError {
    fn description(&self) -> &str {
        match self {
            Self::ConfigInvalid => "invalid config file",
            Self::ConfigMissingValue => "missing config values",
            Self::ConfigMissingFile => "missing config file",
            Self::ConfigErrorReadingValue => "error reading config value",
        }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description())
    }
}
