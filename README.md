# 23C1-Rust-eze
Repo for Rust Taller De Programacion 1 FIUBA

## Ejecutar

Para compilar y ejecutar el programa, debe estar creado el archivo config con el siguiente formato:
```
SEED=dns
PROTOCOL_VERSION=1
PORT=1
LOG=file
NPEERS=1
```
Una vez hecho esto, corremos la siguiente línea de comandos:

```
cargo run -- configpath
```