# Testnet scripts

## EntryPoint v0.9 rollout

Moves BSC Chapel (`bsc-testnet`, 97) and Polygon Amoy (`polygon-amoy`, 80002) to EntryPoint v0.9,
governed from Gargantua. The EVM steps run with `entrypoint-v09.sh` from `evm/`. The sudo steps run
with `e2e/entrypoint-v09-sudo.mjs` from `sdk/packages/simplex`.

Both scripts read `PRIVATE_KEY`, `SECRET_PHRASE`, `BSC_CHAPEL`, `POLYGON_AMOY` and
`HYPERBRIDGE_GARGANTUA` from `sdk/.env.local`, and skip a step that already holds on a chain.
With `--dry-run`:

- the EVM steps run on local anvil forks against a temporary copy of the config;
- the sudo steps print each decoded call and its hex without signing.

Every EVM broadcast goes through `script/deploy.sh`, which asks for confirmation and verifies the
contracts. Live runs write the new addresses into `config.testnet.toml`. Keep `VERSION` (default
`entrypoint-v09`) the same for every EVM step, because it seeds every CREATE2 salt.

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
