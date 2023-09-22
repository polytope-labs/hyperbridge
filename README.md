# Hyperbridge
Hyperbridge is a scalable, multi-chain bridhge network. Powered by zkSNARKs, Secured by Polkadot.

## Running a local tesnet with zombienet
1. Download the zombienet binary for your os from https://github.com/paritytech/zombienet
2. Run `./zombienet spawn --provider native ./scripts/zombienet/local-testnet.toml`

## Running a local testnet with docker
1. Build the hyperbridge docker image by running  `docker build . -t polytopelabs/hyperbridge`
2. Navigate to `scripts/parachain-launch`
3. Run `docker compose up`