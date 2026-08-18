# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/simplex`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

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

