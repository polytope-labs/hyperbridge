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
- Passes it to `new ChainClientManager(configService, options.signer)`. When it is absent — watch-only only — `ChainClientManager` substitutes `privateKeySigner(generatePrivateKey())`, a key that exists solely so wallet-client construction has an account; nothing ever signs with it. The runtime records this as `signerless`, and the chain controller enforces it: `setWatchOnly(chainId, false)` throws and `chains.add` defaults to watch-only, so an observer started without a signer cannot be flipped into filling from the throwaway key.
- Reads it back with `chainClientManager.getSigner()` and hands that one instance to `ContractInteractionService`, `UserOpSender`, `IntentFiller`, `FXFiller`, the rebalancers and `PaymasterKeeperService`. There is exactly one signer per solver.
- Logs `EVM signing strategy: <mode>` (`mode ?? "custom"`) — the only place `mode` matters.

### Which method signs what

- **`signTypedData` — the hot path.** Two callers:
  - `ContractInteractionService` builds a bid and calls `sdkHelper.prepareSubmitBid({ solverSigner: sdkSigningAccount(this.signer), … })`; the SDK's `BidManager` signs `CryptoUtils.packedUserOpTypedData(userOp, entryPoint, chainId)`. Signing the typed data rather than the digest yields the same signature the `SolverAccount` recovers, while leaving the payload legible to a policy engine.
  - `UserOpSender.buildSignedUserOp` does the same for self-initiated UserOps — delegation-via-bundler, vault sweep and redeem.
  - `paymaster/permit.ts` signs the EIP-2612 permit that lets the Circle or Simplex paymaster pull USDC/USDT for gas. It takes `Pick<Signer, "signTypedData">`, not the whole signer.

  No caller passes a chain id: every payload carries `domain.chainId`, which is what the digest covers and what MPCVault reads for its request envelope.
- **`signAuthorization`.** `DelegationService.buildAuthorization` calls it for every delegation, with no branching — the signer owns the encoding. Turnkey uses its structured path; MPCVault and any digest-only backend hash `keccak256(0x05 ‖ rlp([chainId, contractAddress, nonce]))` themselves (`viem/utils`' `hashAuthorization`).
- **`signTransaction`.** Every transaction the solver sends: the type-0x04 delegation tx, rebalancing transfers, operator sends. It returns signed RLP, so the backend owns serialisation — MPCVault's vault API and Turnkey's transaction payloads both keep the transaction legible to their policy engines, and `digestSigner` serialises with viem and signs the hash.
- **`address`.** Read directly everywhere the solver's identity is needed (`fillerAddress`, delegation authority, balance lookups, vault initialisation).
- **`mode`.** Logs only: the boot line and the two delegation log lines.

### The viem boundary

`Signer` names no viem type. `accountFor(signer)` (`src/services/wallet/account.ts`) builds the `LocalAccount` viem wants from one, and `ChainClientManager` derives it once in its constructor and hands it to every wallet client. The mapping is:

- `signTypedData` → straight through.
- `signTransaction` → viem's prepared request narrowed by `toSignerTransaction`, which rejects a request with no `chainId` rather than letting a replayable transaction be signed.
- `signMessage` → rejects. viem's `toAccount` requires it; no solver path personal-signs.
- `sign` → not implemented. Nothing calls `account.sign` now that authorizations and transactions are signer operations.

`digestSigner` is the same boundary from the other side: it turns one `sign(hash)` into the three operations, hashing typed data with viem's `hashTypedData`, authorizations with `hashAuthorization`, and transactions with `serializeTransaction` + `keccak256`.

`sdkSigningAccount(signer)` is the last piece: the sdk's `SigningAccount` takes `unknown` typed data where `Signer` takes `TypedDataPayload`, so the two call sites that hand a signer to the sdk go through it.

### Delegation, the one branching path

`DelegationService.setupDelegation(chain)` prefers the bundler: a no-op UserOp with the authorization attached, gas paid by the paymaster in stablecoins (`setupDelegationViaBundler`, signed with `signTypedData` through `UserOpSender`). It falls back to a direct type-0x04 transaction when no paymaster is available or the solver holds no stablecoins, which needs native balance.

The direct path is uniform: `sendDelegationTransaction` calls `walletClient.sendTransaction` with the authorization list and a 650k gas floor, whatever the signer is. viem prepares the transaction and hands it to the derived account's `signTransaction`, which routes to the signer's. `mpcVaultSigner` is where that matters: its structured request has no field for an authorization list, so it detects one and serialises + raw-signs instead, which is the only reason a set-code transaction from an MPC-backed solver installs a delegation at all.

`revokeDelegation` runs the same two steps against the zero address.

### What `viemSigner` derives

`viemSigner(account)` (`src/services/wallet/accounts/viem.ts`) maps a viem `LocalAccount` onto the interface: `address` → `account.address`, `mode` → `account.source` (so `privateKeyToAccount` reports `"privateKey"` and `toAccount` reports `"custom"`), `signTypedData` and `signTransaction` → the account's own.

`signAuthorization` is the one that needs work, because viem makes it optional on an account and the interface does not: the adapter uses `account.signAuthorization` when present (private keys, Turnkey) and otherwise hashes the tuple and signs it with `account.sign`. An account with neither is rejected at construction — a solver that cannot delegate cannot bid, and finding that out at the first fill is worse.

`privateKeySigner` is `viemSigner(privateKeyToAccount(key))`. `turnkeySigner` is `viemSigner(turnkeyAccount)` plus `mode: "turnkey"`. `mpcVaultSigner` uses no viem account: it implements the three operations against `MpcVaultService` directly.
