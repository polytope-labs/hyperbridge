# Hyperclient

Hyperclient is a simple CLI tool for sending cross-chain requests through Hyperbridge. You'll need testnet tokens and a hex-encoded private key to use this CLI. We don't reccomend sharing your mainnet & testnet account for obvious reasons.

### Installation

We provide a prebuilt binariy for both linux & macos, you can download them like so:

```
wget "https://github.com/polytope-labs/hyperbridge/releases/download/v0/hyperclient-{linux/mac}" -o=hyperclient;
./hyperclient --help
```

If your platform isn't on this list supported, you'll have to build from scratch. This requires [a rust installation](https://doc.rust-lang.org/cargo/getting-started/installation.html)


```
cargo install --git=https://github.com/polytope-labs/hyperbridge hyperclient
hyperclient --help
```
