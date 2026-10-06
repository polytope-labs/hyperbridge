# Testnet scripts

## EntryPoint v0.9 rollout

Moves BSC Chapel (`bsc-testnet`, 97) and Polygon Amoy (`polygon-amoy`, 80002) to EntryPoint v0.9,
governed from Gargantua. The EVM steps run with `entrypoint-v09.sh` from `evm/`. The sudo steps run
with `e2e/entrypoint-v09-sudo.mjs` from `sdk/packages/simplex`.

Both scripts read `PRIVATE_KEY`, `GARGANTUA_SUDO_SEED`, `BSC_CHAPEL`, `POLYGON_AMOY` and
`HYPERBRIDGE_GARGANTUA` from `sdk/.env.local`, and skip a step that already holds on a chain.
`GARGANTUA_SUDO_SEED` is the seed of Gargantua's `sudo.key()`, checked before anything is signed. It
is not `SECRET_PHRASE`, which is the e2e BRIDGE funder. With `--dry-run`:

- the EVM steps run on local anvil forks against a temporary copy of the config;
- the sudo steps print each decoded call and its hex without signing.

Every EVM broadcast goes through `script/deploy.sh`, which asks for confirmation on each chain and
verifies the contracts. Live runs write the new addresses into `config.testnet.toml`. Keep `VERSION`
(default `entrypoint-v09`) the same for every EVM step, because it seeds every CREATE2 salt.
On Amoy, forge pays `ETH_PRIORITY_GAS_PRICE=30gwei` and `ETH_GAS_PRICE=40gwei` unless you set them,
because the RPC suggests a priority fee about 10x what lands.

| # | Command | Verified by the script |
|---|---------|------------------------|
| 1 | `script/testnet/entrypoint-v09.sh solver-account` | `entryPoint()` is v0.9, and both chains have the same address |
| 2 | `script/testnet/entrypoint-v09.sh gateway-impl` | the implementation has the new `SELECT_SOLVER_TYPEHASH`, and the `execute_on_gateway` calldata is printed |
| 3 | `script/testnet/entrypoint-v09.sh paymaster` | feeds deployed, and the proxy is at version 3 with relayer `0xc8809D…2757` and the deployer as treasury |
| 4 | `node e2e/entrypoint-v09-sudo.mjs register-paymaster` | `Paymasters(Evm(id))` holds the proxy |
| 5 | `node e2e/entrypoint-v09-sudo.mjs upgrade-gateway` | the gateway implementation slot is the new one, and `SELECT_SOLVER_TYPEHASH()` is `0xe706…e09e` |
| 6 | `script/testnet/entrypoint-v09.sh fund` | the EntryPoint deposit and stake are at target, with an unstake delay of 86400 |
| 7 | Start the bundlers with [`scripts/docker/rundler`](../../../scripts/docker/rundler/README.md) | `eth_supportedEntryPoints` returns only v0.9 |
| 8 | `node e2e/entrypoint-v09-sudo.mjs allow-bundlers` | `getBundlers()` lists the rundler signer and its simulation origin |
| 9 | `node e2e/entrypoint-v09-sudo.mjs clear-bundlers`, once acceptance passes | `getBundlers()` is empty |

Step details:

- **Step 5** waits for the relayer, which can take minutes. Steps 8 and 9 wait for it too. Use
  `--timeout-min` to change how long they wait.
- **Step 6** tops up to 0.3 tBNB deposit and 0.05 stake on Chapel, and 1 POL deposit and 0.1 stake
  on Amoy. Use `--deposit` and `--stake` (in ether) to change the targets.
- **Steps 8 and 9.** The runtime has no `set_paymaster_bundlers`, so the allowlist rides on
  `upgrade_paymaster`:
  - The proxy is pointed at its current implementation.
  - The init data is an `onAccept` call carrying a nested `SetBundlers` request: source
    `KUSAMA-4009`, from `pallet-intents`, relayer `0xc8809D…2757`.
  - The proxy delegatecalls this init data with the host still the caller.
  - `node e2e/entrypoint-v09-sudo.mjs init-data` prints the bytes, and
    `tests/foundry/SimplexPaymasterBundlerRolloutTest.t.sol` pins them.

To recheck state:

- `script/testnet/entrypoint-v09.sh status` re-reads every EVM check.
- `node e2e/entrypoint-v09-sudo.mjs metadata` prints the live call shapes, the sudo key and what the
  pallet stores.

## Local end-to-end run

`e2e/entrypoint-v09-local.mjs` in `sdk/packages/simplex` runs the v0.9 stack on an anvil fork of
Chapel. It needs no Gargantua access, so it stands in for a live run. It starts anvil and the
rundler fork's native binary, checks every step, and stops both processes at the end.

1. Governance is delivered with the host impersonated. Step 5's `Execute` body upgrades the gateway,
   and step 8's nested `UpgradeContract` route sets the allowlist, using the bytes `init-data` prints.
2. A fresh solver EOA delegates to the v0.9 SolverAccount through EIP-7702.
3. The SDK places a same-chain USD.h for cNGN order. The solver signs a bid sponsored in PERMIT2
   mode, using the SDK's gas estimate, simplex's paymaster builder and `prepareSubmitBid`.
4. The SDK's `buildBids`, `simulate` and `execute` send the bid to the local rundler, which bundles
   it. The script checks:
   - the fill and the balances
   - the bundle sender and the `UserOperationEvent`
   - the call trace: `select` during validation, and `getCurrentUserOpHash` inside `fillOrder`
   - the indexer's `findUserOpHash`
5. Step 9 runs for the rundler signer alone. A second sponsored bid must then fail with
   `UnauthorizedBundler(signer)`.

Run it from `sdk/`:

```sh
pnpm -C packages/sdk build
FORK_URL=<Chapel RPC> pnpm -C packages/simplex exec tsx e2e/entrypoint-v09-local.mjs
```

It needs `anvil` and a build of the rundler fork (`cargo build --bin rundler`). The script looks for
the binary at `../rundler/target/debug/rundler` beside this repository, and `RUNDLER_BIN` overrides
it. It reads `RUNDLER_SIGNER_PRIVATE_KEY` from `sdk/.env.local`, and `FORK_URL` falls back to
`BSC_CHAPEL` there. Anvil takes the fork URL only as an argument, so any API key in it shows in
`ps` on this machine; the script's output and its anvil and rundler logs redact it.

Rundler in unsafe mode simulates validation from `0x0643…11b5`. With only the signer removed, rundler
still accepts the second op. The bundle then reverts on chain with
`FailedOpWithRevert(AA33, UnauthorizedBundler)`, and the signer pays for the reverted transaction.
