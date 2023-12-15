# QSTN Substrate Docker Image

First, install [Docker](https://docs.docker.com/get-docker/).

Then to generate the latest substrate image. Please run:
```sh
./build.sh
```

> If you wish to create a debug build rather than a production build, then you may modify the [.Dockerfile](./substrate_builder.Dockerfile) replacing `cargo build --locked --release` with just `cargo build --locked` and replacing `target/release` with `target/debug`. 

> If you get an error that a tcp port address is already in use then find an available port to use for the host port in the [.Dockerfile](./substrate_builder.Dockerfile).

The image can be used by passing the selected binary followed by the appropriate tags for this binary.

Your best guess to get started is to pass the `--help flag`. Here are a few examples:

- `./run.sh substrate --version`
- `./run.sh subkey --help`
- `./run.sh node-template --version`
- `./run.sh chain-spec-builder --help`

Then try running the following command to start a single node development chain using the Substrate Node Template binary `node-template`:

```sh
./run.sh node-template --dev --ws-external
```

Note: It is recommended to provide a custom `--base-path` to store the chain database. For example:

```sh
# Run Substrate Node Template without re-compiling
./run.sh node-template --dev --ws-external --base-path=/data
```

> To print logs follow the [Substrate debugging instructions](https://docs.substrate.io/test/debug/).

```sh
# Purge the local dev chain
./run.sh node-template purge-chain --dev --base-path=/data -y
```