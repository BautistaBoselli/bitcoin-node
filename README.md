# 23C1-Rust-eze

Repo for Rust Taller De Programacion 1 FIUBA

## Compile and run

To compile and run the program, the config file must be created with the following format:

```
SEED=seed.testnet.bitcoin.sprovoost.nl
PROTOCOL_VERSION=70012
PORT=18333
LOG=log.txt
NPEERS=10
STORE_PATH=store
CLIENT_ONLY=false
```

A working example of this is shown in the _example-config_ file.

Then we run the following command line:

```
cargo run --release configpath
```

## Run two nodes in the same machine

To connect a second node to the first one, we must create a second config file with the following format:

```
SEED=127.0.0.1
PROTOCOL_VERSION=70012
PORT=18333
LOG=second-log.txt
NPEERS=1
STORE_PATH=second-store
CLIENT_ONLY=true
```

The _client_only_ flag must be set to true to avoid the second node to act as a server and coliding with the first one on the p2p port.
The _store_path_ must be different from the first one to avoid colisions on the database.
