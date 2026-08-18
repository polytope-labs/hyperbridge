# Flow

AI-maintained map of how code paths in `sdk/packages/simplex` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## Signing: from construction to each signature

### Where the signer comes from

There are two entry points, and they meet at `bootFiller`.

1. **Library.** The consumer builds a `Signer` (`privateKeySigner`, `turnkeySigner`, `mpcVaultSigner`, `viemSigner`, or their own) and passes it as `Simplex.start({ signer })` (`src/simplex.ts`). Before anything else, `start` rejects a config that carries a `simplex.signer` block without a `signer` argument — the TOML block is never resolved here.
2. **Binary.** `src/bin/simplex.ts` parses the TOML, checks `[simplex.signer]` is present (unless watch-only), and calls `signerFromToml` → `validateSignerConfig` → `createSigner`, which dispatches on `type` to one of the three bundled factories. The resolved instance goes into the same `Simplex.start({ signer })` call. The `paymaster-keeper` command does the same thing without a Simplex.

`bootFiller` (`src/core/boot.ts`) then:

- Throws if there is no signer and not every resolved chain is watch-only.
- Passes it to `new ChainClientManager(configService, options.signer)`. When it is absent — watch-only only — `ChainClientManager` substitutes `privateKeySigner(generatePrivateKey())`, a key that exists solely so wallet-client construction has an account; nothing ever signs with it.
- Reads it back with `chainClientManager.getSigner()` and hands that one instance to `ContractInteractionService`, `UserOpSender`, `IntentFiller`, `FXFiller`, the rebalancers and `PaymasterKeeperService`. There is exactly one signer per solver.
- Logs `EVM signing strategy: <mode>` (`mode ?? "custom"`) — the only place `mode` matters.

### Which method signs what

- **`signTypedData` — the hot path.** Two callers:
  - `ContractInteractionService` builds a bid and calls `sdkHelper.prepareSubmitBid({ solverSigner: this.signer, … })`; the SDK's `BidManager` signs `CryptoUtils.packedUserOpTypedData(userOp, entryPoint, chainId)`. Signing the typed data rather than the digest yields the same signature the `SolverAccount` recovers, while leaving the payload legible to a policy engine.
  - `UserOpSender.sign` (`src/services/UserOpSender.ts`) does the same for self-initiated UserOps — delegation-via-bundler, vault sweep and redeem.
  - `src/services/paymaster/permit.ts` signs the EIP-2612 permit that lets the Circle or Simplex paymaster pull USDC/USDT for gas. It takes a structural `{ signTypedData }`, not the whole `Signer`.
- **`signRawHash`.** `DelegationService.buildAuthorization` signs the EIP-7702 authorization digest (`keccak256(0x05 ‖ rlp([chainId, delegate, nonce]))`) with it — but only when the signer has no `signAuthorization`. The MPC adapter also uses it internally to sign the serialised delegation transaction.
- **`signAuthorization` (optional).** Preferred over the digest when present, so the backend sees `(chainId, delegate, nonce)`. Only `turnkeySigner` implements it.
- **`account`.** `ChainClientManager.getWalletClient` builds every viem wallet client on it, so it signs every transaction the solver sends directly: the EIP-7702 delegation transaction, rebalancing transfers, and operator sends from the dashboard. Fills and bids do not go through it — they are UserOperations.

### Delegation, the one branching path

`DelegationService.setupDelegation(chain)` prefers the bundler: a no-op UserOp with the authorization attached, gas paid by the paymaster in stablecoins (`setupDelegationViaBundler`, signed with `signTypedData` through `UserOpSender`). It falls back to a direct type-0x04 transaction when no paymaster is available or the solver holds no stablecoins, which needs native balance.

The direct path is uniform: `sendDelegationTransaction` calls `walletClient.sendTransaction` with the authorization list and a 650k gas floor, whatever the signer is. viem prepares the transaction and hands it to `account.signTransaction`, so a backend that needs special handling for set-code transactions does it there. `createMpcVaultAccount` is the one that does: MPCVault's `evmSendCustom` request has no field for an authorization list, so its `signTransaction` branches on `authorizationList` being present, serialises the transaction, raw-signs the digest through `signRawHashComponents`, and returns the signed bytes; everything else keeps the structured path, where the vault's policy engine still sees to/value/data.

`revokeDelegation` runs the same two steps against the zero address.

### What `viemSigner` derives

`viemSigner(account)` (`src/services/wallet/accounts/viem.ts`) maps a viem `LocalAccount` onto the interface: `signTypedData` → `account.signTypedData`, `signRawHash` → `account.sign({ hash })` parsed into `{ r, s, yParity }`, `mode` → `account.source` (so `privateKeyToAccount` reports `"privateKey"` and `toAccount` reports `"custom"`). It throws at construction if the account has no `sign`, because the delegation path above would otherwise fail at the first fill. `privateKeySigner` is `viemSigner(privateKeyToAccount(key))`; `turnkeySigner` is `viemSigner(turnkeyAccount)` plus `mode: "turnkey"` and `signAuthorization`.
