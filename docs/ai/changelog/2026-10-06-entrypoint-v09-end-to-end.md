# EntryPoint v0.9 end to end: solver selection by userOpHash

An order's session key now selects one bid UserOperation instead of a solver address. With
`solverSelection` on, `fillOrder` succeeds only while ERC-4337 EntryPoint v0.9
(`0x433709009B8330FDa32311DF1C2AFA402eD8D009`) is executing that operation. The SDK, simplex and
the indexer handle both selection formats. The paymaster and `SolverAccount` move to v0.9 is
described in `docs/ai/changelog/2026-10-05-simplex-paymaster-entrypoint-v09-and-bundler-allowlist.md`.

## Contracts

- `SELECT_SOLVER_TYPEHASH` is `keccak256("SelectSolver(bytes32 commitment,bytes32 userOpHash)")`.
  `SelectOptions` (`sdk/packages/core/contracts/apps/IntentGatewayV2.sol`) is
  `{ bytes32 commitment; bytes32 userOpHash; bytes signature; }`, so `select` has selector
  `0x81b06069`. On a gateway that selects by address, `select((bytes32,address,bytes))` is
  `0x6d20e7dc`.
- `select` is permissionless. It recovers the signer of `SelectSolver(commitment, userOpHash)` and
  sets transient slot `keccak256(abi.encode(commitment, userOpHash, signer))` to 1. Keying on the
  signer means a selection signed by any other key cannot overwrite the session key's. It still
  reverts `Filled` on a finalized order.
- `fillOrder` with `solverSelection` on reads `getCurrentUserOpHash()` from the v0.9 constant
  `ENTRYPOINT_V09`, never from the caller. It reverts `Unauthorized` when the hash is zero (no
  operation executing, as for a direct call) or the slot for
  `(commitment, userOpHash, order.session)` is unset. `msg.sender` is not checked, so any contract
  the selected operation calls can fill.
- `SolverAccount.validateUserOp` passes its `userOpHash` into `select`. The 162-byte signature
  layout and the nonce key `uint192(keccak256(commitment ‖ sessionKey ‖ keccak256(callData)))` are
  unchanged.
- The gateway keeps `VERSION` 3 and its storage layout. The implementation upgrade carries empty
  init data.

## SDK (`@hyperbridge/sdk` 2.9.0)

Breaking:

- `ChainConfigData.addresses.EntryPointV08` is now `addresses.EntryPoint`, the EntryPoint the
  chain's `SolverAccount` validates against. It changes together with `SolverAccount`.
- `ChainConfigService.getEntryPointV08Address` is now `getEntryPointAddress(chain)`, which returns
  `HexString | undefined`.
- `SelectOptions.solver` is now `SelectOptions.userOpHash`.
- `CryptoUtils.signSolverSelection` is removed. In its place:
  - `signUserOpHashSelection(commitment, userOpHash, domainSeparator, privateKey)` signs the
    userOpHash format.
  - `signLegacySolverSelection(commitment, solverAddress, domainSeparator, privateKey)` signs the
    address format.
  - Both return the signature and never `null`.

Added exports:

- `ENTRY_POINT_V08` and `ENTRY_POINT_V09`.
- `KNOWN_ENTRY_POINTS`, which lists v0.9 first.
- `LEGACY_SELECT_SOLVER_TYPEHASH`.
- `readSelectionFormat` and `SelectionFormat` (`"userOpHash" | "address"`).
- `fetchRundlerPriorityFee` and `applyRundlerPriorityFee`.

### Bid routing

`BidImpl` finds the bid's EntryPoint by recovering the solver signature (`userOp.signature[32:97]`)
over the userOpHash for each entry of `KNOWN_ENTRY_POINTS`.

A v0.9 bid pairs only with a gateway whose format is `userOpHash`, and a v0.8 bid only with an
`address` gateway, because each `SolverAccount` calls its own `select`. The format comes from the
gateway's `SELECT_SOLVER_TYPEHASH()`:

- `readSelectionFormat` caches it per chain id and gateway address.
- A failed read is not cached.
- An unknown typehash throws.
- On a mismatch, `refreshSelectionFormat` reads it once more before the bid is refused, so a gateway
  upgraded in place is picked up.

A bid that recovers under no known EntryPoint, or does not pair with the gateway, fails `simulate()`
and `execute()`, so `selectAndExecuteBest` skips it. `execute()` submits to the recovered EntryPoint
and reads its fill logs from that EntryPoint's events. The session signature is made in the
gateway's format and cached.

### Simulation and estimation

`Bid.simulate()` is a single `eth_call` of `userOp.callData` from the solver account. The value is
the native outputs plus the dispatch fee. It makes no `select` call and no signature.

The call carries a state override from `selectionOffStateDiff`, which reads gateway slot 5 live and
clears only the `solverSelection` byte (bits 160 to 167). `GasEstimator.buildStateOverride` uses the
same diff, so it no longer needs the call dispatcher address.

`GasEstimator.estimateFillOrder` estimates against `getEntryPointAddress`. With no EntryPoint
configured, it takes the bundler-failure path. It signs the selection in the gateway's format.

`GasEstimator.estimateBidPreVerificationGas` overrides the code of the bid's paymaster with
`BID_PVG_ESTIMATION_PAYMASTER_CODE`, alongside the solver account's. The stub accepts every
operation with an empty context, so the estimate does not run the paymaster's validation. Pimlico
simulates an estimate with `paymasterPostOpGasLimit` at 2,000,000, outside the 30,000 to 100,000
that `SimplexPaymaster` accepts. Without the override the estimate fails there, the bid is signed
with the fill estimate's smaller `preVerificationGas`, and Pimlico rejects it as
`preVerificationGas is not enough`.

### Rundler fee floor

For a bundler URL containing neither `pimlico.io` nor `alchemy.com`, `fetchRundlerPriorityFee` asks
for `rundler_maxPriorityFeePerGas`:

- A bundler that answers `-32600`, `-32601` or `-32602` is not asked again.
- Other failures are not remembered.

When the bundler gives a fee, `applyRundlerPriorityFee` raises:

- the priority fee to at least that fee plus the priority bump;
- the max fee to at least the base fee plus the max-fee bump, plus that priority floor.

`GasEstimator` and simplex's `UserOpSender.getGasPrice` both apply it.

### Used-op keys

`OrderExecutor` always hashes its used-UserOp keys with `ENTRY_POINT_V08`. This keeps keys already
persisted per order valid when a chain's EntryPoint changes.

### Chain config

- Chapel (97) and Amoy (80002) set `EntryPoint` v0.9, `SolverAccount`
  `0x7484C6Ff790b898bf7e6960Ffc82f63330445Dc2`, and `SimplexPaymaster`:
  - Chapel: `0x9f4a6F1254f27373C2aca5bDd0e48FbA45EA3aeC`, plus `Permit2`.
  - Amoy: `0x6085a14078B7d26259145a894140f7C8361e02ae`.
- Every other chain keeps v0.8 until it is rolled out.

## simplex (0.17.0)

### Bundler preflight

`assertBundlersServeEntryPoint` (`src/services/bundler-preflight.ts`) asks each chain's bundler for
`eth_supportedEntryPoints`, with a 10 second timeout.

- **Refused:** a list of addresses without the chain's `getEntryPointAddress`. It throws
  `Bundler <host> for <chain> does not support EntryPoint <address>; it lists ...`, with every
  failing chain joined into one error. Messages show only the bundler's host, because URLs carry API
  keys.
- **Warning only:** no answer, an HTTP or JSON-RPC error, a timeout, or a result that is not a list
  of strings.
- **Skipped:** watch-only chains, and chains with no bundler URL or no known EntryPoint.

It runs:

- in `bootFiller`, before chain clients are built;
- in `ChainController.add`, before the chain is registered;
- in `setBundlerUrl`;
- in `setWatchOnly(chainId, false)`.

### Setup API

`validate-bundler` takes an optional `chainId`. `ok` is `false` only for an empty URL. Otherwise it
is `true`, and the response carries a `warning` when:

- the bundler does not list the chain's EntryPoint (Simplex will refuse it);
- the answer is not a list of addresses (Simplex will use it and warn);
- the bundler does not answer.

### Limit orders

- `LimitOrder.entryPoint` and `LimitOrderPosting.entryPoint` record the EntryPoint the posted op was
  signed for.
- SQLite stores it in column `entry_point`, which is added in place to older databases.
- A null value on a live posting means v0.8.
- It is written only with a posting that landed.

`reconcile` checks every open posting the orderbook still lists. When its `entryPoint` differs
from the fill chain's, it
claims the order (`open` to `resizing`), withdraws the posting, and posts it again signed for
the current EntryPoint. These count in `ReconcileReport.reposted`. A fill's `resize` waits for an
in-flight re-sign rather than posting alongside it.

## Indexer

- `ENTRY_POINTS` (`src/utils/userOp.helpers.ts`) includes v0.9, so fills inside v0.9 bundles are
  attributed to the operation's sender.
- `IntentGatewayV3.abi.json` carries the new `SelectOptions`.
- The testnet `solverAccount` lists for Chapel and Amoy add
  `0x7484C6Ff790b898bf7e6960Ffc82f63330445Dc2` next to the earlier accounts.

## Testnet deployment

Deployed on Chapel (97) and Amoy (80002):

- `SolverAccount` `0x7484C6Ff790b898bf7e6960Ffc82f63330445Dc2`
- `IntentGatewayV2` implementation `0x38a82f8283a0fAdB763888707aD18fCd34AF719D`
- `SimplexPaymaster` proxies Chapel `0x9f4a6F1254f27373C2aca5bDd0e48FbA45EA3aeC` and Amoy
  `0x6085a14078B7d26259145a894140f7C8361e02ae`, each funded on EntryPoint v0.9 with a deposit and
  a stake (86400 second unstake delay)

Remaining steps, each a Gargantua sudo call: `intentsCoprocessor.addPaymasterDeployment` for each
paymaster proxy, and `intentsCoprocessor.executeOnGateway` carrying
`upgradeToAndCall(0x38a82f8283a0fAdB763888707aD18fCd34AF719D, "")` for each chain's gateway.

The SDK and indexer testnet configs already point at the new contracts. Until the gateway proxy
upgrade lands, fills on Chapel and Amoy fail with SDK 2.9.0 and simplex 0.17.0. A v0.9
`SolverAccount` calls the new `select`, which the current gateway implementation rejects, and the
SDK refuses a v0.9 bid on a gateway whose format is `address`.

The testnet swaps run (`.github/workflows/test-simplex-e2e.yml`) sends its UserOperations through
Pimlico, which serves v0.9 on both chains. Its bundler URLs, `E2E_BSC_TESTNET_BUNDLER_URL` and
`E2E_POLYGON_AMOY_BUNDLER_URL`, are required and no longer default to the RPC URLs: Alchemy, the
run's RPC provider, answers a v0.9 operation with `EntryPoint version 0.9 is not currently enabled`.

## Mainnet rollout

Upgrading a chain's gateway is a hard cutover for that chain:

- Solvers still delegated to the old `SolverAccount` cannot fill there until they re-delegate.
- Users on an SDK older than 2.9.0 cannot select bids there until they upgrade.

Order:

1. Release `@hyperbridge/sdk` 2.9.0 and get integrators onto it before any mainnet gateway upgrade.
   It routes v0.8 bids on gateways not yet upgraded and v0.9 bids on upgraded ones.
2. List the chain's new `SolverAccount` in the indexer config before any solver re-delegates to it.
   Otherwise HyperFX drops those solvers' orders.
3. Make sure the chain's bundlers serve v0.9. Simplex refuses a bundler that lists EntryPoints
   without the chain's.
4. Upgrade the chain's gateway implementation, with empty init data, and its paymaster, with
   `migrate` as described in the paymaster note.
5. With each gateway upgrade, ship the SDK chain config that sets `EntryPoint` v0.9 and the new
   `SolverAccount` together, so solvers re-delegate.

Polkadot Hub (420420419) has `solverSelection` on but no EntryPoint v0.9. Before upgrading its
gateway, turn selection off there, or leave the chain out of the upgrade. Otherwise `fillOrder`
reverts there for every order.
