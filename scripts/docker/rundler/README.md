# Rundler testnet bundlers

Runs the rundler fork as the ERC-4337 bundler for BSC Chapel and Polygon Amoy. Both serve
EntryPoint v0.9 (`0x433709009B8330FDa32311DF1C2AFA402eD8D009`) only and run in unsafe mode,
because the paymaster reads `tx.origin` during validation.

| Chain | Chain ID | Network spec | RPC | Metrics |
|-------|----------|--------------|-----|---------|
| BSC Chapel | 97 | `bsc_testnet` (base `bsc`) | `http://127.0.0.1:4337` | `9337` |
| Polygon Amoy | 80002 | `polygon_amoy` (base `polygon`), EIP-7702 turned on | `http://127.0.0.1:4338` | `9338` |

The signer (`RUNDLER_SIGNER_PRIVATE_KEY`) must be a plain EOA funded on both chains.

## Start

The image builds from the fork checkout at `RUNDLER_DIR`, which needs its submodules
(`git submodule update --init --recursive` in the fork).

```sh
cd scripts/docker/rundler
cp .env.example .env # then fill it in
docker compose up -d --build
docker compose logs -f
```

## Native binary

With the variables from `.env` exported, run these from the fork checkout:

```sh
NODE_HTTP="$CHAPEL_RPC_URL" SIGNER_PRIVATE_KEYS="$RUNDLER_SIGNER_PRIVATE_KEY" RUST_LOG=info \
  cargo run --bin rundler -- node --network bsc_testnet --enabled_entry_points v0.9 --unsafe \
  --rpc.port 4337 --metrics.port 9337

CHAIN_EIP7702_ENABLED=true NODE_HTTP="$AMOY_RPC_URL" SIGNER_PRIVATE_KEYS="$RUNDLER_SIGNER_PRIVATE_KEY" RUST_LOG=info \
  cargo run --bin rundler -- node --network polygon_amoy --enabled_entry_points v0.9 --unsafe \
  --rpc.port 4338 --metrics.port 9338
```

## Check

Each bundler should return only the v0.9 EntryPoint:

```sh
for port in 4337 4338; do
  curl -s -X POST -H 'content-type: application/json' \
    --data '{"jsonrpc":"2.0","id":1,"method":"eth_supportedEntryPoints","params":[]}' \
    "http://127.0.0.1:$port"
  echo
done
```

## Paymaster bundler list

When `SimplexPaymaster.getBundlers()` is non-empty on a chain, both the signer address and
rundler's simulation origin `0x0643866dA50efE0b055Cd15aF95191968c8411b5` must be on it.
Otherwise validation reverts with `UnauthorizedBundler`.
