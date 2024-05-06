# Hyperbridge
Hyperbridge is a hyper-scalable coprocessor for cryptographically secure, cross-chain interoperability.

## Docker

Hyperbridge is available at the official docker repository [`polytopelabs/hyperbridge`](https://hub.docker.com/r/polytopelabs/hyperbridge)

```bash
docker run polytopelabs/hyperbridge:latest --chain=messier
```

## Prebuilt Binaries

You can install a prebuilt binary for the hyperbridge node with the following bash script

```bash
wget -q --show-progress https://github.com/polytope-labs/hyperbridge/releases/download/${latest-tag}/hyperbridge-x86_64-unknown-linux-gnu.tar.gz
tar -xvzf hyperbridge-x86_64-unknown-linux-gnu.tar.gz
# copy to $PATH
cp hyperbridge-x86_64-unknown-linux-gnu/hyperbridge $HOME/.local/bin/
```

or a 1-liner shell script

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/polytope-labs/hyperbridge/releases/download/${latest-tag}/hyperbridge-installer.sh | sh
```

## Building from source

You can follow the steps below if you'd prefer to build the hyperbridge node from source:


### Install Dependencies

Building the hyperbridge node requires some dependencies

- git
- clang
- curl
- make
- build-essential
- libssl-dev
- llvm
- libudev-dev
- protobuf-compiler

Debian/Ubuntu

```bash
sudo apt update
sudo apt install --assume-yes git clang curl libssl-dev llvm libudev-dev make protobuf-compiler
```

Arch

```bash
pacman -Syu --needed --noconfirm curl git clang make protobuf
```

Fedora

```bash
sudo dnf update
sudo dnf install clang curl git openssl-devel make protobuf-compiler
```

Opensuse

```bash
sudo zypper install clang curl git openssl-devel llvm-devel libudev-devel make protobuf
```

### Install rust compiler

If you don't have an already existing rust installation, you can install it using the one-liner below. Follow the prompts displayed to proceed with a default installation.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Clone the repo

Download a local copy of the repo and checkout the latest release tag

```bash
export LATEST_TAG=v0.4.0
git clone https://github.com/polytope-labs/hyperbridge.git
cd ./hyperbridge
git checkout ${LATEST_TAG}
```

### Install WebAssembly target

Hyperbridge's blockchain runtime compiles to wasm which allows it's code to be forklessly upgraded. In order to build hyperbridge we need the wasm toolchain installed.

```bash
rustup update nightly
rustup target add wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup component add rust-src
```

### Build the node

```bash
cargo build --release -p hyperbridge
```

## Running the node

```bash
hyperbridge --chain=messier --base-path=$HOME/.hyperbridge --pruning-archive
```

> Note: `--enable-offchain-indexing` is enabled by default

## Running a local testnet with zombienet
Download the zombienet binary for your operating system [here](https://github.com/paritytech/zombienet).

```bash
zombienet spawn --provider native ./scripts/zombienet/local-testnet.toml
```

## Running a local testnet with docker
Build and run the hyperbridge docker image locally by running

```bash
docker build -t hyperbridge -f ./scripts/docker/Dockerfile .
cd scripts/parachain-launch
docker compose up
```

## Building HyperClient Javascript SDK
To build hyperclient
```bash
cargo install wasm-pack
cd client
wasm-pack build --no-default-features --features wasm
```
