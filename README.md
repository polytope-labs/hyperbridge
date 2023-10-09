# Hyperbridge
Hyperbridge is a hyper-scalable, interoperability coprocessor.

## Running a local tesnet with zombienet
1. Download the zombienet binary for your os from https://github.com/paritytech/zombienet
2. Run `./zombienet spawn --provider native ./scripts/zombienet/local-testnet.toml`

## Running a local testnet with docker
1. Build the hyperbridge docker image by running  `docker build -t hyperbridge -f ./scripts/docker/Dockerfile .`
2. Navigate to `scripts/parachain-launch`
3. Run `docker compose up`
