use bitcoin::config::Config;

use std::{env, path::Path};

const CANT_ARGS: usize = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != CANT_ARGS {
        println!("ERROR: Debes ingresar la ruta del archivo");
        return;
    }
    let path = Path::new(&args[1]);
    if !path.exists() {
        println!("ERROR: El archivo no existe");
        return;
    }

    let config = match Config::from_file(args[1].as_str()) {
        Ok(config) => config,
        Err(error) => {
            println!("ERROR: {}", error);
            return;
        }
    };

    println!("Config: {:?}", config);
}
