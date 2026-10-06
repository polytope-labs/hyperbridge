# SimplexPaymaster and SolverAccount on EntryPoint v0.9, with a bundler allowlist

`SimplexPaymaster` (`evm/src/utils/SimplexPaymaster.sol`) and `SolverAccount`
(`evm/src/apps/intentsv2/SolverAccount.sol`) serve ERC-4337 EntryPoint v0.9
(`0x433709009B8330FDa32311DF1C2AFA402eD8D009`) only. Governance can also restrict which bundler
wallets may submit user operations that the paymaster sponsors.

## SimplexPaymaster on EntryPoint v0.9

`entryPoint()` returns v0.9. A `validatePaymasterUserOp` or `postOp` call from v0.8 reverts
`PaymasterUnauthorized`. The cutover is immediate: once a chain's proxy is upgraded, every v0.8
operation naming the paymaster fails.

v0.9 allows an optional `paymasterSignature` suffix on `paymasterAndData`. The paymaster does not
take one: `paymasterData` must be exactly 150 bytes in PERMIT mode and 182 bytes in PERMIT2 mode,
so a suffix reverts `InvalidPaymasterData`.

`VERSION` is 3. `migrate()` is host-only and runs only on a version-2 proxy. Governance reaches it
through `upgrade_paymaster(state_machine, new_impl, init_data)` with
`init_data = abi.encodeCall(SimplexPaymaster.migrate, ())`, the same bytes on every chain. In the
upgrade transaction it:

1. withdraws the whole v0.8 deposit to the proxy;
2. unlocks the v0.8 stake if it is staked;
3. stakes the same amount with the same unstake delay on v0.9, paid from the proxy's native balance;
4. deposits the remaining native into v0.9;
5. emits `EntryPointMigrated(withdrawn, staked, unstakeDelaySec, deposited)`.

If the native balance does not cover the stake, `migrate` skips the v0.9 stake, deposits
everything, and emits `staked` as 0. The treasury then stakes on v0.9 through `addStake`.

`withdrawStakeV08()` is permissionless. It sends the v0.8 stake to the treasury once the v0.8
unstake delay has passed (86400 seconds on the live chains). A chain with no v0.8 stake, such as
Base, stays unstaked on v0.9 until the treasury calls `addStake`.

The EntryPoint addresses are constants, so the move adds no storage. The pallet needs no change,
because `init_data` is opaque bytes.

## SolverAccount on EntryPoint v0.9

`entryPoint()` returns v0.9 only, which gates `validateUserOp`, `getNonce` and the EntryPoint's
right to execute batches. The account's code is immutable, so v0.9 is a new deployment and solvers
re-delegate to it. The `Budgets` storage slot is unchanged, so limit-order budget tallies survive
the re-delegation. Nonces are per EntryPoint, so they restart, and bids must be signed for v0.9.

## Bundler allowlist

### On-chain check

`_validatePaymasterUserOp` reverts `UnauthorizedBundler(origin)` when the bundler list is non-empty
and `tx.origin` is not on it. The check is its first step, before any permit or prefund logic.

An empty list turns the check off, so removing the last listed wallet turns it off. ERC-7562 bans
ORIGIN during validation, so `tx.origin` is read only while the list is non-empty. On a chain with
the check off, spec-enforcing bundlers keep accepting the paymaster. On a chain with it on, only our
own bundler (rundler run with `--unsafe`) can carry sponsored operations.

The list is `EnumerableSet.AddressSet _bundlers` at slots 9 and 10, taken out of `__gap`, which
shrinks to `uint256[46]`. Every other field and the end of the proxy layout keep their slots.
`getBundlers()` returns the listed wallets, and `BundlerUpdated(address indexed bundler, bool allowed)`
is emitted only when a wallet is actually added or removed.

### Governance

`RequestKind.SetBundlers` (8) takes the body `0x08 ++ abi.encode(address[] bundlers, bool allowed)`.
`allowed = true` lists the wallets and `false` delists them. Listing a listed wallet or delisting an
unlisted one is a no-op, and a zero address reverts `ZeroAddress`. The request arrives through
`onAccept`, so the relayer gate applies as for every other kind.

On Hyperbridge, `pallet-intents-coprocessor` exposes `set_paymaster_bundlers(state_machine, bundlers,
allowed)` at call index 21, behind `GovernanceOrigin`. It rejects an empty list or a zero entry with
`InvalidPaymasterBundlers`, builds the body from `RequestKind::PaymasterSetBundlers`, dispatches it to
the paymaster registered for `state_machine`, and emits `PaymasterBundlersUpdateInitiated`. The SDK has
no encoder for paymaster governance requests; the pallet builds every body.

### Turning the check on for a chain

- The first `set_paymaster_bundlers` request for a chain must list, together, every rundler signer
  wallet (any signer can bundle for any enabled EntryPoint), a spare wallet, and rundler's
  simulation origin `0x0643866dA50efE0b055Cd15aF95191968c8411b5`. Without the simulation origin,
  rundler's validation and gas estimation fail for every sponsored op. Rundler only simulates from
  that origin, so it still sends bundles from an unlisted signer; they revert on-chain at the
  signer's cost. Remove a signer only after it stops bundling.
- Bundler signer wallets must be plain EOAs. EntryPoint v0.9's `handleOps` requires its caller to
  be `tx.origin` with no code, so a wallet with an EIP-7702 delegation cannot bundle.

## Rollout constraints

- The SDK must resolve the EntryPoint for each bid and be released before any chain's paymaster is
  upgraded. An SDK that hard-codes v0.8 cannot execute v0.9 bids.
- On each chain, the SDK config must change the `SolverAccount` address and the EntryPoint
  together.
- Each chain's bundler must have v0.9 enabled.
