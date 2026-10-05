# SimplexPaymaster bundler allowlist

Governance can restrict which bundler wallets may submit user operations sponsored by
`SimplexPaymaster` (`evm/src/utils/SimplexPaymaster.sol`).

## On-chain check

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

## Governance

`RequestKind.SetBundlers` (8) takes the body `0x08 ++ abi.encode(address[] bundlers, bool allowed)`.
`allowed = true` lists the wallets and `false` delists them. Listing a listed wallet or delisting an
unlisted one is a no-op, and a zero address reverts `ZeroAddress`. The request arrives through
`onAccept`, so the relayer gate applies as for every other kind.

On Hyperbridge, `pallet-intents-coprocessor` exposes `set_paymaster_bundlers(state_machine, bundlers,
allowed)` at call index 21, behind `GovernanceOrigin`. It rejects an empty list or a zero entry with
`InvalidPaymasterBundlers`, builds the body from `RequestKind::PaymasterSetBundlers`, dispatches it to
the paymaster registered for `state_machine`, and emits `PaymasterBundlersUpdateInitiated`. The SDK has
no encoder for paymaster governance requests; the pallet builds every body.

## Turning the check on for a chain

- The first `set_paymaster_bundlers` request for a chain must list, together, every rundler signer
  wallet (any signer can bundle for any enabled EntryPoint), a spare wallet, and rundler's
  simulation origin `0x0643866dA50efE0b055Cd15aF95191968c8411b5`. Without the simulation origin,
  rundler's validation and gas estimation fail for every sponsored op. Rundler only simulates from
  that origin, so it still sends bundles from an unlisted signer; they revert on-chain at the
  signer's cost. Remove a signer only after it stops bundling.
- List plain EOAs only. The paymaster serves EntryPoint v0.8 today, but v0.9 requires the
  `handleOps` caller to be the transaction sender with no code, so a wallet with an EIP-7702
  delegation stops working once the paymaster moves to v0.9.
