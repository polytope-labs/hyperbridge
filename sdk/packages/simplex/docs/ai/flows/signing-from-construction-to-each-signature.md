# Signing: from construction to each signature

## Where the signer comes from

There are two entry points, and they meet at `bootFiller`.

1. **Library.** The consumer builds a `Signer` (`privateKeySigner`, `turnkeySigner`, `mpcVaultSigner`, `viemSigner`, or their own) and passes it as `Simplex.start({ signer })` (`src/simplex.ts`). Before anything else, `start` rejects an object that carries a `simplex.signer` block without a `signer` argument. `SimplexConfig` has no such field, so this is a runtime property read: what it catches is a parsed config file.
2. **Binary.** `src/bin/simplex.ts` parses the TOML as `FillerConfigFile` (the library's `FillerTomlConfig` plus the `[simplex.signer]` block), checks the block is present unless watch-only, and calls `signerFromToml` → `validateSignerConfig` → `createSigner`, which dispatches on `type` to one of the three bundled factories. The resolved instance goes into the same `Simplex.start({ signer })` call. The `paymaster-keeper` command does the same thing without a Simplex.

    The parsed object keeps its signer block on the way in — the library ignores the extra key, and `UiServer.persistConfig` regenerates the config file from that same object, so removing it would delete `[simplex.signer]` from the operator's file on the next dashboard edit.

`bootFiller` (`src/core/boot.ts`) then:

- Throws if there is no signer and not every resolved chain is watch-only.
- Passes it to `new ChainClientManager(configService, options.signer)`. When it is absent — watch-only only — `ChainClientManager` substitutes `privateKeySigner(generatePrivateKey())`, a key that exists solely so wallet-client construction has an account; nothing ever signs with it. The runtime records this as `signerless`, and the chain controller enforces it: `setWatchOnly(chainId, false)` throws and `chains.add` defaults to watch-only, so an observer started without a signer cannot be flipped into filling from the throwaway key.
- Reads it back with `chainClientManager.getSigner()` and hands that one instance to `ContractInteractionService`, `UserOpSender`, `IntentFiller`, `FXFiller`, the rebalancers and `PaymasterKeeperService`. There is exactly one signer per solver.
- Logs `EVM signing strategy: <mode>` (`mode ?? "custom"`) — the only place `mode` matters.

## Which method signs what

- **`signTypedData` — the hot path.** Two callers:

    - `ContractInteractionService` builds a bid and calls `sdkHelper.prepareSubmitBid({ solverSigner: sdkSigningAccount(this.signer), … })`; the SDK's `BidManager` signs `CryptoUtils.packedUserOpTypedData(userOp, entryPoint, chainId)`. Signing the typed data rather than the digest yields the same signature the `SolverAccount` recovers, while leaving the payload legible to a policy engine.
    - `UserOpSender.buildSignedUserOp` does the same for self-initiated UserOps — delegation-via-bundler, vault sweep and redeem.
    - `paymaster/permit2.ts` signs the per-op Permit2 `PermitTransferFrom` that lets the Simplex paymaster pull USDC/USDT for gas, and `paymaster/permit.ts` the EIP-2612 permit the first-time delegation uses instead. Both take `Pick<Signer, "signTypedData">`, not the whole signer.

    No caller passes a chain id: every payload carries `domain.chainId`, which is what the digest covers and what MPCVault reads for its request envelope.

- **`signAuthorization`.** `DelegationService.buildAuthorization` calls it for every delegation, with no branching — the signer owns the encoding. Turnkey uses its structured path; MPCVault and any digest-only backend hash `keccak256(0x05 ‖ rlp([chainId, contractAddress, nonce]))` themselves (`viem/utils`' `hashAuthorization`).
- **`signTransaction`.** Every transaction the solver sends: the type-0x04 delegation tx, rebalancing transfers, operator sends. It returns signed RLP, so the backend owns serialisation — MPCVault's vault API and Turnkey's transaction payloads both keep the transaction legible to their policy engines, and `digestSigner` serialises with viem and signs the hash.
- **`address`.** Read directly everywhere the solver's identity is needed (`fillerAddress`, delegation authority, balance lookups, vault initialisation).
- **`mode`.** Logs only: the boot line and the two delegation log lines.

## MPCVault's two-RPC ceremony

Every `mpcVaultSigner` operation is two gRPC calls in `MpcVaultService`: `createSigningRequest`
returns a signing-request uuid, then `executeSigningRequests` triggers MPCVault's policy checks,
the callback co-signer approval and the MPC ceremony. The callback server is only contacted during
execute — a failure before that step leaves no callback log at all. Execute retries
INVALID_ARGUMENT/NOT_FOUND twice with short backoff because MPCVault intermittently rejects a uuid
its own create just returned; when execute fails terminally the created request is best-effort
rejected in the vault so it does not stay pending as a signable stale payload. Any failure
propagates as an error naming the RPC, the uuid and MPCVault's x-request-id, and both RPCs also
fail on app-level errors that carry only a non-zero code with an empty message (zero is
UNSPECIFIED, not an error). Lifecycle logs default to the process-wide logger context, which an
embedded filler's `SimplexOptions.logger` never sees — `MpcVaultClientConfig.logger` injects one.

## The viem boundary

`Signer` names no viem type. `accountFor(signer)` (`src/services/wallet/account.ts`) builds the `LocalAccount` viem wants from one, and `ChainClientManager` derives it once in its constructor and hands it to every wallet client. The mapping is:

- `signTypedData` → straight through.
- `signTransaction` → viem's prepared request narrowed by `toSignerTransaction`, which rejects a request with no `chainId` rather than letting a replayable transaction be signed.
- `signMessage` → rejects. viem's `toAccount` requires it; no solver path personal-signs.
- `sign` → not implemented. Nothing calls `account.sign` now that authorizations and transactions are signer operations.

`digestSigner` is the same boundary from the other side: it turns one `sign(hash)` into the three operations, hashing typed data with viem's `hashTypedData`, authorizations with `hashAuthorization`, and transactions with `serializeTransaction` + `keccak256`.

`sdkSigningAccount(signer)` is the last piece: the sdk's `SigningAccount` takes `unknown` typed data where `Signer` takes `TypedDataPayload`, so the two call sites that hand a signer to the sdk go through it.

## Delegation, the one branching path

`DelegationService.setupDelegation(chain)` uses `resolvePendingPermit2Approval` to decide whether a bootstrap approval is needed. The resolver uses the same allowance-aware `selectToken` as sponsorship: if either funded fee token is already approved sufficiently, it returns null and no bootstrap is needed. Otherwise it checks the first balance-qualified token for a zero Permit2 allowance and EIP-2612 support (`permitCapable`). The answer picks the carrier for the approve:

- **No permit** (BNB Chain pegged stables) **and the EOA covers one set-code tx** (`nativeCoversDirectTx`): a direct type-0x04 transaction with `token.approve(Permit2, max)` batched in (650k gas floor, +60k for the approve). A native tx is the only carrier available. If it reverts, `isDelegated` is checked before moving on: EIP-7702 applies authorization tuples even when execution reverts, so the delegation usually landed and only the approve is deferred.
- **Permit capable**: the bundler instead, even on an EOA holding native — spending stablecoins beats spending native. `setupDelegationViaBundler` sets `permitBootstrap` and puts `approve(Permit2, max)` in the op's own callData as an ERC-7821 batch (`BOOTSTRAP_CALL_GAS_LIMIT`, 150k). The paymaster takes the EIP-2612 branch in step 2, so the permit pays for the very op that installs the allowance every later op authorizes against. One sponsored op, zero native, and the account never signs a 2612 permit again.
- **Nothing pending**: the bundler with a plain no-op op (`callData: "0x"`, no bootstrap flag).

**Already delegated** is its own case, and the one that matters on an upgrade. `setupDelegation` returns early once `isDelegated` holds, but not before calling `ensurePermit2Allowance(chain)`: delegation and bootstrap are separate facts, and an account delegated by a release that charged EIP-2612 permits has no Permit2 allowance at all. That method sends the same permit-funded ERC-7821 approve op minus the authorization it no longer needs. It is best-effort and never throws — a failure leaves the first sponsored op to `sendFundedApprove`, which works for a solver holding native. Without it every upgraded solver is stranded on that native path, which is the failure `docs/ai/Decisions.md`'s 2026-09-02 `skipPermit` entry records hitting on Base and Arbitrum.

The last fallback in every case is a plain direct type-0x04 self-call, which needs native balance and fails with an explicit deficit log when the EOA cannot cover it.

Either direct payload goes through `sendDelegationTransaction`. Either way `sendDelegationTransaction` calls `walletClient.sendTransaction` with the authorization list, whatever the signer is. viem prepares the transaction and hands it to the derived account's `signTransaction`, which routes to the signer's. `mpcVaultSigner` is where that matters: its structured request has no field for an authorization list, so it detects one and serialises + raw-signs instead, which is the only reason a set-code transaction from an MPC-backed solver installs a delegation at all.

`revokeDelegation` runs the same two steps against the zero address.

## What `viemSigner` derives

`viemSigner(account)` (`src/services/wallet/accounts/viem.ts`) maps a viem `LocalAccount` onto the interface: `address` → `account.address`, `mode` → `account.source` (so `privateKeyToAccount` reports `"privateKey"` and `toAccount` reports `"custom"`), `signTypedData` and `signTransaction` → the account's own.

`signAuthorization` is the one that needs work, because viem makes it optional on an account and the interface does not: the adapter uses `account.signAuthorization` when present (private keys, Turnkey) and otherwise hashes the tuple and signs it with `account.sign`. An account with neither is rejected at construction — a solver that cannot delegate cannot bid, and finding that out at the first fill is worse.

`privateKeySigner` is `viemSigner(privateKeyToAccount(key))`. `turnkeySigner` is `viemSigner(turnkeyAccount)` plus `mode: "turnkey"`. `mpcVaultSigner` uses no viem account: it implements the three operations against `MpcVaultService` directly.
