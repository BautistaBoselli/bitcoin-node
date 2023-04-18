use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::str::FromStr;

use crate::error::CustomError;

#[derive(Debug)]
pub struct Config {
    pub seed: String,
    pub protocol_version: u32,
}

impl Config {
    /// Lee un archivo de configuracion y devuelve un Config con los valores leidos.
    /// El archivo de configuracion debe tener el siguiente formato:
    /// {NOMBRE}={VALOR}
    /// Debe incluir todos los valores econtrados en la estructura Config.
    /// Devuelve CustomError si:
    /// - No se pudo encontrar el archivo.
    /// - El archivo tiene un formato invalido.
    /// - El archivo no contiene todos los valores requeridos.
    pub fn from_file(path: &str) -> Result<Self, CustomError> {
        let file = File::open(path).map_err(|_| CustomError::ConfigMissingFile)?;
        Self::from_reader(file)
    }

    /// Crea un config a partir de cualquier implementacion del trait Read
    /// con el contenido en el formato mencionado en la documentacion de from_file.
    /// Devuelve CustomError si:
    /// - El contenido tiene un formato invalido.
    /// - El contenido no contiene todos los valores requeridos.
    /// - No se pudo leer el contenido.
    fn from_reader<T: Read>(content: T) -> Result<Config, CustomError> {
        let reader = BufReader::new(content);

        let mut config = Self {
            seed: String::new(),
            protocol_version: 0,
        };

        for line in reader.lines() {
            let current_line = line.map_err(|_| CustomError::ConfigInvalid)?;

            let setting: Vec<&str> = current_line.split('=').collect();

            if setting.len() != 2 {
                return Err(CustomError::ConfigInvalid);
            }
            Self::load_setting(&mut config, setting[0], setting[1])?;
        }
        if (config.seed.is_empty()) || (config.protocol_version == 0) {
            return Err(CustomError::ConfigMissingValue);
        }
        Ok(config)
    }

    fn load_setting(&mut self, name: &str, value: &str) -> Result<(), CustomError> {
        match name {
            "SEED" => self.seed = String::from(value),
            "PROTOCOL_VERSION" => {
                self.protocol_version =
                    u32::from_str(value).map_err(|_| CustomError::ConfigErrorReadingValue)?
            }
            _ => (),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_con_formato_invalido() {
        // GIVEN: un reader con contenido invalido para el archivo de configuracion
        let content = "Hola Mundo!".as_bytes();

        // WHEN: se ejecuta la funcion from_reader con ese reader
        let cfg = Config::from_reader(content);

        // THEN: la funcion devuelve un Err porque el contenido es invalido
        assert!(cfg.is_err());
        assert!(matches!(cfg, Err(CustomError::ConfigInvalid)));
    }

    #[test]
    fn config_con_valores_faltantes() {
        // GIVEN: un reader con contenido invalido para el archivo de configuracion
        let content = "SEED=seed.testnet.bitcoin.sprovoost.nl\n".as_bytes();

        // WHEN: se ejecuta la funcion from_reader con ese reader
        let cfg = Config::from_reader(content);

        // THEN: la funcion devuelve un Err porque el contenido es invalido
        assert!(cfg.is_err());
        assert!(matches!(cfg, Err(CustomError::ConfigMissingValue)));
    }

    #[test]
    fn config_sin_valores_requeridos() -> Result<(), CustomError> {
        // GIVEN: un reader con contenido de configuracion completo
        let content = "SEED=seed.testnet.bitcoin.sprovoost.nl\n\
            PROTOCOL_VERSION=9876"
            .as_bytes();

        // WHEN: se ejecuta la funcion from_reader con ese reader
        let cfg = Config::from_reader(content)?;

        // THEN: la funcion devuelve Ok y los parametros de configuracion tienen los valores esperados
        assert_eq!(9876, cfg.protocol_version);
        assert_eq!("seed.testnet.bitcoin.sprovoost.nl", cfg.seed);
        Ok(())
    }
}
