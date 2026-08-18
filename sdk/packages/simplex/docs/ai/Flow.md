# Flow

AI-maintained map of how code paths in `sdk/packages/simplex` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## Signing: from construction to each signature

### Where the signer comes from

There are two entry points, and they meet at `bootFiller`.

1. **Library.** The consumer builds a `Signer` (`privateKeySigner`, `turnkeySigner`, `mpcVaultSigner`, `viemSigner`, or their own) and passes it as `Simplex.start({ signer })` (`src/simplex.ts`). Before anything else, `start` rejects an object that carries a `simplex.signer` block without a `signer` argument. `SimplexConfig` has no such field, so this is a runtime property read: what it catches is a parsed config file.
2. **Binary.** `src/bin/simplex.ts` parses the TOML as `FillerConfigFile` (the library's `FillerTomlConfig` plus the `[simplex.signer]` block), checks the block is present unless watch-only, and calls `signerFromToml` → `validateSignerConfig` → `createSigner`, which dispatches on `type` to one of the three bundled factories. The resolved instance goes into the same `Simplex.start({ signer })` call. The `paymaster-keeper` command does the same thing without a Simplex.

   The parsed object keeps its signer block on the way in — the library ignores the extra key, and `UiServer.persistConfig` regenerates the config file from that same object, so removing it would delete `[simplex.signer]` from the operator's file on the next dashboard edit.

`bootFiller` (`src/core/boot.ts`) then:

- Throws if there is no signer and not every resolved chain is watch-only.
- Passes it to `new ChainClientManager(configService, options.signer)`. When it is absent — watch-only only — `ChainClientManager` substitutes `privateKeySigner(generatePrivateKey())`, a key that exists solely so wallet-client construction has an account; nothing ever signs with it.
- Reads it back with `chainClientManager.getSigner()` and hands that one instance to `ContractInteractionService`, `UserOpSender`, `IntentFiller`, `FXFiller`, the rebalancers and `PaymasterKeeperService`. There is exactly one signer per solver.
- Logs `EVM signing strategy: <mode>` (`mode ?? "custom"`) — the only place `mode` matters.

### Which method signs what

- **`signTypedData` — the hot path.** Two callers:
  - `ContractInteractionService` builds a bid and calls `sdkHelper.prepareSubmitBid({ solverSigner: sdkSigningAccount(this.signer), … })`; the SDK's `BidManager` signs `CryptoUtils.packedUserOpTypedData(userOp, entryPoint, chainId)`. Signing the typed data rather than the digest yields the same signature the `SolverAccount` recovers, while leaving the payload legible to a policy engine.
  - `UserOpSender.sign` does the same for self-initiated UserOps — delegation-via-bundler, vault sweep and redeem.
  - `paymaster/permit.ts` signs the EIP-2612 permit that lets the Circle or Simplex paymaster pull USDC/USDT for gas. It takes `Pick<Signer, "signTypedData">`, not the whole signer.

  No caller passes a chain id: every payload carries `domain.chainId`, which is what the digest covers and what MPCVault reads for its request envelope.
- **`signRawHash`.** `DelegationService.buildAuthorization` signs the EIP-7702 authorization digest (`keccak256(0x05 ‖ rlp([chainId, delegate, nonce]))`) with it, when the signer has no `signAuthorization`. It is also the fallback for transactions: `accountFor`'s `signTransaction` serialises the transaction and signs the hash when the signer implements no `signTransaction`.
- **`signAuthorization` (optional).** Preferred over the digest when present, so the backend sees `(chainId, delegate, nonce)`. `viemSigner` carries it through from any viem account that has it, which covers `privateKeyToAccount` and Turnkey.
- **`signTransaction` (optional).** Where a backend takes transactions rather than digests. MPCVault implements it against its structured vault request; `viemSigner` delegates to the account's own `signTransaction`, which is how Turnkey keeps its transaction policies meaningful.
- **`address`.** Read directly everywhere the solver's identity is needed (`fillerAddress`, delegation authority, balance lookups, vault initialisation).

### The viem boundary

`Signer` names no viem type. `accountFor(signer)` (`src/services/wallet/account.ts`) builds the `LocalAccount` viem wants from one, and `ChainClientManager` derives it once in its constructor and hands it to every wallet client. The mapping is:

- `sign({ hash })` → `signRawHash`, re-serialised with viem's `serializeSignature`.
- `signTypedData` → straight through.
- `signTransaction` → the signer's own if it has one (viem's prepared request narrowed by `toSignerTransaction`), otherwise serialise with viem's serializer and sign the digest.
- `signMessage` → throws. viem's `toAccount` requires it; no solver path personal-signs.

`sdkSigningAccount(signer)` is the other half of the boundary: the sdk's `SigningAccount` takes `unknown` typed data where `Signer` takes `TypedDataPayload`, so the two call sites that hand a signer to the sdk go through it.

### Delegation, the one branching path

`DelegationService.setupDelegation(chain)` prefers the bundler: a no-op UserOp with the authorization attached, gas paid by the paymaster in stablecoins (`setupDelegationViaBundler`, signed with `signTypedData` through `UserOpSender`). It falls back to a direct type-0x04 transaction when no paymaster is available or the solver holds no stablecoins, which needs native balance.

The direct path is uniform: `sendDelegationTransaction` calls `walletClient.sendTransaction` with the authorization list and a 650k gas floor, whatever the signer is. viem prepares the transaction and hands it to the derived account's `signTransaction`, which routes to the signer's own or to the digest fallback. `mpcVaultSigner` is where that matters: its structured request has no field for an authorization list, so it detects one and serialises + raw-signs instead, which is the only reason a set-code transaction from an MPC-backed solver installs a delegation at all.

`revokeDelegation` runs the same two steps against the zero address.

### What `viemSigner` derives

`viemSigner(account)` (`src/services/wallet/accounts/viem.ts`) maps a viem `LocalAccount` onto the interface: `address` → `account.address`, `signTypedData` → `account.signTypedData`, `signRawHash` → `account.sign({ hash })` parsed into `{ r, s, yParity }`, `signTransaction` → `account.signTransaction`, `signAuthorization` → `account.signAuthorization` when the account has one, and `mode` → `account.source` (so `privateKeyToAccount` reports `"privateKey"` and `toAccount` reports `"custom"`). It throws at construction if the account has no `sign`, because the delegation path would otherwise fail at the first fill.

`privateKeySigner` is `viemSigner(privateKeyToAccount(key))`. `turnkeySigner` is `viemSigner(turnkeyAccount)` plus `mode: "turnkey"` — Turnkey's account implements `signTransaction` and `signAuthorization` itself, so nothing else is needed. `mpcVaultSigner` uses no viem account: it implements the interface against `MpcVaultService` directly.
