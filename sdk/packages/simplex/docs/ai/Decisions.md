# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/simplex`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-19 — postOp gas limit is 40,000 exactly, and the constant is split per paymaster (#1071)

Chosen: `MAX_POST_OP_GAS_LIMIT = 40_000`, `MIN_POST_OP_GAS_LIMIT = 30_000`, and an SDK constant per paymaster.

40,000 is not a tuned figure. The EntryPoint waives its unused-gas penalty while `gasLimit <= gasUsed + PENALTY_GAS_THRESHOLD`, and that threshold is 40,000 — so any cap at or below 40,000 is penalty-free for *any* postOp cost, with no assumption about the token. A measured per-token value (postOp is ~8-12k for USDC and USDT) would be tighter but would silently start leaking margin the day a more expensive token is registered. The floor of 30,000 is the lowest value both tokens were observed to execute at, and it exists because `innerHandleOp` overhead outside every gas limit can otherwise push `actualGasCost` past the `maxCost` the prefund was sized against, underflowing the refund subtraction.

The SDK constant was shared with the Circle paymaster. Lowering it in place would have applied a bound derived from *this* contract's postOp to a different contract whose postOp was never measured, so it split into `POST_OP_GAS_LIMIT_SIMPLEX` and `POST_OP_GAS_LIMIT_CIRCLE`.

Lowering the on-chain cap (not just the client constant) is safe despite older solvers pinning 100k, because new proxy addresses ship in the same `chain.ts` release as the new SDK: an old solver keeps using the old deployment and never meets the new bound.

## 2026-08-19 — Stake gets a governance recovery path, and `addStake` is treasury-only (#1071)

Chosen: two new empty-payload request kinds (`UnlockStake` = 5, `WithdrawStake` = 6, always paying out to the treasury) and an `addStake` override gated to the treasury.

The alternative — leave stake unrecoverable and simply never stake — is not available: bundlers require a staked paymaster for the storage access this contract performs, so staking is effectively mandatory and was already done on three chains. Two kinds rather than one because the EntryPoint requires `unlockStake()` and then a wait of `unstakeDelaySec` before `withdrawStake()` will succeed; a single request could not span that delay.

Gating `addStake` is the half that cannot be deferred. The EntryPoint only ever lets `unstakeDelaySec` grow and resets any pending unlock on every `addStake`, so while the function is open an unprivileged caller can push the delay to 136 years and cancel unlocks indefinitely — which would defeat the recovery path being added here.

## 2026-08-19 — Not adopted: soft-failing the prefund, and oracle-derived validity bounds (#1071)

Rejected: returning `prefunded = false` instead of reverting in the mode-2 `_prefund`. Upstream advises it to protect bundler reputation, but reading EntryPoint v0.8 shows both outcomes are `revert FailedOp` — AA33 for a paymaster revert, AA34 for a sig-failure — so both revert `handleOps` identically. The change would trade the `Permit2Failed(token, reason)` diagnostic, which carries Permit2's own revert data, for no bundle-level benefit.

Also deferred at the maintainer's direction: bounding `validationData`'s `validUntil` by oracle freshness so bundlers drop soon-to-be-stale ops instead of building bundles that revert. Sound in principle and would have made stale-oracle failures expire cleanly, but it touches every pricing path and was out of scope for this pass.

## 2026-08-18 — Permit2 before a legacy paymaster allowance, bootstrap approves Permit2 (#1071)

Chosen: once a token is approved to Permit2, PERMIT2 mode wins over an existing allowance to the paymaster, and a solver with neither bootstraps by approving Permit2 (`maxUint256`), never the paymaster. APPROVE mode is only used while a legacy paymaster allowance is still at or above the $2 threshold, so existing BSC solvers migrate on their own: they keep APPROVE until it drains, pay the one funded approve they would have paid anyway, and never need native again. Alternative: keep approving the paymaster and only add PERMIT2 as an opt-in — rejected because it keeps native gas a recurring dependency on the one chain the paymaster was meant to free. The `max` approval goes to Permit2 itself (immutable, canonical, already used by the swap path) and is only exercisable with a solver signature; nothing is exposed to the paymaster at rest and no residual allowance is left after an op, unlike PERMIT mode.

## 2026-08-18 — Random Permit2 nonces, bounded deadline (#1071)

Chosen: a random 256-bit nonce per op (`crypto.getRandomValues`) and `deadline = now + PERMIT2_DEADLINE_SECONDS` (1 hour, overridable per call). Alternatives: lowest unused bit read from `nonceBitmap` — mimics EIP-2612's self-invalidation but makes concurrent bids on one chain collide (bids use distinct account nonce keys precisely to run concurrently); `maxUint256` deadline as PERMIT mode uses — wrong here because unordered Permit2 nonces never self-invalidate, so every losing bid would leave a live $5 permit forever. One hour comfortably exceeds bid-to-execution latency (order deadlines are ~10 to 40 minutes of blocks) and clock skew; a random nonce usually touches a fresh bitmap word (about 17k extra gas), negligible in stablecoin terms. The EIP-2612 path's `maxUint256` deadline is correct for USDC, whose v2.2 permit short-circuits `deadline == max` before reading the timestamp.

## 2026-08-18 — Mode 2 gated on a `PERMIT2()` probe of the paymaster (#1071)

Chosen: `paymasterSupportsPermit2` reads the `PERMIT2()` constant that only the Permit2-capable implementation exposes; positive results are cached for the process lifetime, negative ones for five minutes. Alternative: rely on release ordering (client after all five redeploys) — fragile, since redeploys land chain by chain and a governance upgrade keeps the address, and a mode-2 op against an old deployment fails validation with `InvalidMode`, which for a bid means a lost fill.

## 2026-08-18 — `forceApproveMode` renamed to `skipPermit` (#1071)

Chosen: the delegation flow's flag now only skips EIP-2612 permit detection; PERMIT2 and APPROVE stay available. The flag exists because delegation ops pass fixed, measured account-side gas limits; the Simplex builder sets its own paymaster verification limit per mode (`VERIFICATION_GAS_LIMIT_PERMIT2` = 200k, measured at ~135k on Ethereum and BSC forks), so PERMIT2 does not disturb them. Keeping the old name would have forced BSC delegations to keep a paymaster allowance, i.e. two funded approvals per token instead of one.

## 2026-08-18 — Known: PERMIT2 mode is not ERC-7562-clean (#1071)

Permit2's `nonceBitmap[owner][word]` and the token's `allowance[owner][Permit2]` are not sender-associated storage under ERC-7562, so a spec-enforcing bundler could reject mode 2 during validation. Accepted because the paymaster already reads `block.timestamp` (also banned) in every mode and is live through the bundlers in use, and because PERMIT/APPROVE remain as fallbacks. Verified empirically on Base Sepolia: Alchemy's bundler accepts mode 2 for both a fresh EOA (Permit2 ecrecover path) and a delegated account (ERC-1271 path). Pimlico was not probed (no key). If a bundler rejects mode 2, the fallback design is to keep APPROVE and refill the allowance inside sponsored ops (ERC-7821 batch) rather than with native txs.

## 2026-08-18 — Bootstrap approve waits for two confirmations (#1071)

Chosen: `sendFundedApprove` waits for `confirmations: 2`. On Base Sepolia the first sponsored op right after a one-confirmation approve was rejected `AA33` (Permit2 `TRANSFER_FROM_FAILED`: the bundler's simulation node had not seen the approve yet) twice in a row, and the identical op seconds later was accepted; with two confirmations a fresh EOA went through first try. Alternative: retry the op on `AA33` — more code for a once-per-token-lifetime event that costs one extra block.
## 2026-08-18 — The signer block lives on a separate `FillerConfigFile` type, not on `SimplexConfig`

Chosen: `FillerTomlConfig` drops `simplex.signer`; a new `FillerConfigFile extends FillerTomlConfig` adds it back for the binary's file format. The CLI, setup API, TOML writer, wizard state and `UiServer`'s operator context are typed with the file shape; the library never is.

Alternatives considered: keeping the optional field on the shared type and documenting that the library ignores it; a structural `Omit<>` on `SimplexConfig` so a file object is a type error at `Simplex.start`.

Why: leaving the field on the library's type advertises an input the library refuses to read — the one thing a config field must never do. The `Omit` variant goes too far the other way: the binary hands the parsed file straight to `Simplex.start`, and it must, because `UiServer.persistConfig` regenerates the TOML from that same running object. Strip the block on the way in and the operator's signer disappears from their config file the first time someone edits a curve in the dashboard. So the extra key rides along at runtime and is simply absent from the type the library publishes.

The consequence to keep in mind: `Simplex.start`'s guard against a signer-carrying config is now a runtime property read, because the type says the field cannot be there. That is deliberate — the objects it protects against are parsed TOML, which the type system never saw.

## 2026-08-18 — A config with `simplex.signer` and no `signer` argument is a hard error

Chosen: `Simplex.start` throws when `config.simplex.signer` is set and `options.signer` is not.

Alternatives considered: (a) resolve the block automatically, keeping the old behaviour as a fallback; (b) ignore it silently, since the config block now belongs to the binary.

Why the throw won: (a) leaves two ways to choose the key that owns the solver's funds, one of them invisible in the `Simplex.start` call, which is exactly the coupling this change removes — and it would drag `@turnkey/sdk-server` and the MPCVault gRPC client into the boot path of every consumer whether or not they use them. (b) starts a solver on an address the operator did not intend — a config that names a key is evidence of intent, and quietly filling with a throwaway watch-only key instead is worse than not starting. The error names the fix (`signer: await createSigner(config.simplex.signer)`), so the TOML path stays one line away.

## 2026-08-18 — The signer requirement moved from `validateConfig` to boot

Chosen: `validateConfig` neither requires the `[simplex.signer]` block nor looks at it — the block is not part of the config type it validates. A present block is validated where it is consumed: `signerFromToml` on the binary path, the wizard's write step, and the setup API's gate (which also mirrors run's watch-only exemption for an absent block). `bootFiller` rejects a missing `options.signer` unless every resolved chain is watch-only, and the CLI keeps its own `[simplex.signer]`-worded check so a file-driven run still fails at parse time with a file-oriented message.

Alternative considered: keeping the check in `validateConfig` and passing a "a signer was supplied" flag through it.

Why: `validateConfig` is exported for consumers to gate a config before starting, and boot calls the same function — leaving the rule there would have made every library consumer's valid config throw, since the signer is no longer in it. Threading a flag through would keep a config validator asking about an argument that is not config. The duplicated CLI check is deliberate: it costs three lines and preserves the error the binary's users already know.

## 2026-08-18 — EIP-712 payloads must list `EIP712Domain` in `types`

Chosen: `TypedDataPayload`'s doc comment requires it, and the MPCVault integration test pins it.

Why: viem ignores `types.EIP712Domain` when hashing locally, so a payload without it verifies fine against every local signer and looks correct in tests. Backends that hash server-side from the JSON derive the domain type from that entry — omit it and they sign a different digest than the one recovery checks, which is what #1134 was. This is not an MPCVault quirk: it reproduced on **both** backends that hash remotely. The MPCVault case surfaced first (the service-level test, which lists the entry, passed while a new signer-level one that did not failed against the same vault in the same run), and the identical failure then appeared on Turnkey when its typed-data case was added — recovery returned a stranger's address while the signature itself was well-formed. Only the transaction and authorization paths were unaffected, because neither is hashed from a JSON payload. `CryptoUtils.packedUserOpTypedData` in the sdk already sets it deliberately for this reason; the constraint now lives on the type a consumer reads rather than only in that one builder.

## 2026-08-18 — Every operation is required, and digest-only backends get a factory instead of optionality

Chosen: `signTypedData`, `signAuthorization` and `signTransaction` are all required, `mode` with them. `signRawHash` is deleted. `digestSigner({ address, mode, sign })` builds a `Signer` from a single `sign(hash)`.

Alternatives considered: optional `signAuthorization`/`signTransaction` with `signRawHash` as the always-present fallback (the previous cut); requiring them with no factory.

Why: with the structural methods optional, the interface said two contradictory things — "tell me what you can do" and "here is a digest, never mind". Requiring them makes the contract one thing: these are the three operations a solver needs authorised, encode each however your backend can. The cost is that a digest-only backend now has to hash an EIP-7702 authorization and serialise an EIP-1559 transaction, which is real work and exactly where a subtle bug produces a valid signature over the wrong bytes — so that work is not pushed outward, it is packaged as `digestSigner`. The one-liner an HSM integration needs is unchanged; it just goes through a factory instead of leaving holes in the interface.

Note what required-ness deleted: with both structural methods guaranteed, `signRawHash` had no caller left — not in `DelegationService`, not in `accountFor`, not in the sdk (which never called it). Keeping it "optional" would have re-created the `signMessage` situation: a member on the published interface that nothing invokes. It is removed from `SigningAccount` in the sdk too, leaving that interface with the one method the sdk actually calls.

## 2026-08-18 — `Signer` carries no viem types; `accountFor` bridges to viem inside the package

Chosen: no viem type anywhere on `Signer`. (This pass first shipped `address` + `signTypedData` + `signRawHash` with the rest optional; the entry above tightened that to the final all-required shape and deleted `signRawHash` — the viem-free property is what this entry decided, and it survived unchanged.) `accountFor(signer)` (`services/wallet/account.ts`) builds the viem `LocalAccount` wallet clients run on, and `ChainClientManager` derives it once.

Alternatives considered: keeping `account: LocalAccount` on the interface (what the first cut did); going the other way and making `Signer` *be* a viem `LocalAccount`, deleting the abstraction entirely.

Why: the `account` field was the expensive viem type on the published surface — `types.ts` used to carry a warning that consumers must keep viem on this workspace's version or the field would not typecheck, because a `LocalAccount` from a different viem resolves to a different type. Removing it means implementing a signer needs no viem at all. The package's `dist/index.d.ts` still imports viem, for `viemSigner`'s parameter and for the scanner and client-manager surfaces (`PublicClient`, `WalletClient`, `QuorumPublicClient`), which were viem-typed before this change and are unaffected by it; the point is that the signing contract is not among them. Going the other way (Signer = LocalAccount) would have deleted more code, but it pins consumers to viem's account shape permanently and drags `publicKey` — required by `LocalAccount`, never set by viem's own `toAccount` — into every hand-written implementation.

The cost is that we own `TypedDataPayload`, `Signature`, `SignerTransaction` and `Eip7702Authorization`. Three of those are spec shapes that do not move. `SignerTransaction` does move, and is the one to watch: it models the EIP-1559 and EIP-7702 fields simplex actually sends, and `toSignerTransaction` maps viem's prepared request onto it. Every backend sees only those fields: `accountFor` narrows viem's prepared request through `toSignerTransaction` before any signer — `digestSigner` included — touches it, so a field we do not model is invisible to all of them, and widening `SignerTransaction` is the deliberate act that admits it.

## 2026-08-18 — `signMessage` and the `chainId` argument dropped, in both packages

Chosen: neither `Signer` nor the sdk's `SigningAccount` declares `signMessage`, and `signTypedData` takes the payload alone.

Alternatives considered: keeping both, since `Signer` extended `SigningAccount` and the sdk declared them.

Why: nothing called `signMessage`. Bids are signed as EIP-712 UserOperations through `signTypedData` (`BidManager.prepareSubmitBid`), and the sdk's own interface declared it without ever invoking it, so the requirement propagated out to every implementer for nothing. The `chainId` argument was the same shape of problem: EIP-712 puts the chain id in `domain.chainId`, which is what the digest covers, and every viem-backed adapter took the argument and discarded it. Its one consumer was MPCVault's request envelope — which its own account wrapper already derived from the payload — and the adapter defaulted a missing value to `1`, so a call site that forgot would have had the vault authorise a signature under mainnet. Reading the domain and throwing when it is absent replaced that.

Removing both from the sdk is safe in the direction that matters: type narrowing breaks callers of the removed member, and there are none. `Signer` no longer extends `SigningAccount` (the payload types differ in variance); `sdkSigningAccount(signer)` adapts at the two call sites that hand a signer to the sdk.

## 2026-08-18 — `viemSigner` derives `signAuthorization`, and rejects only an account that can neither authorize nor sign digests

Chosen: viem makes `signAuthorization` optional on an account; the interface does not. The adapter uses the account's own when present (private keys, Turnkey), falls back to hashing the tuple and signing it with `sign`, and throws at construction when the account has neither.

Alternative considered: failing lazily at the first delegation attempt.

Why: solver selection is the only fill path simplex uses, and it requires a signed EIP-7702 authorization. An account that cannot produce one yields a solver that boots, scans, and fails at its first delegation — a failure separated from its cause by everything in between. Watch-only solvers do not build a signer at all (boot stands a throwaway key in), so nothing legitimate is blocked by failing early.

## 2026-08-18 — `mode` is a required free-form string, not a union of the shipped backends

Chosen: `Signer.mode: string`. Free-form — the union it replaced made every custom signer misreport itself as one of three backends it is not — and required, since the final interface pass made every member required: a label costs an implementer one string and buys every log line a real backend name instead of `"custom"`.

Alternative considered: keeping `mode: "privateKey" | "mpcVault" | "turnkey"`, or leaving it optional with a `"custom"` default.

Why: the field is only ever read for logs (`DelegationService`, and the boot-time "EVM signing strategy" line). A label with no behaviour attached should not be a closed set, and a fleet running several backends wants each one named.

