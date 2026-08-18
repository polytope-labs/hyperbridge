# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/simplex`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-18 — A config with `simplex.signer` and no `signer` argument is a hard error

Chosen: `Simplex.start` throws when `config.simplex.signer` is set and `options.signer` is not.

Alternatives considered: (a) resolve the block automatically, keeping the old behaviour as a fallback; (b) ignore it silently, since the config block now belongs to the binary.

Why the throw won: (a) leaves two ways to choose the key that owns the solver's funds, one of them invisible in the `Simplex.start` call, which is exactly the coupling this change removes — and it would drag `@turnkey/sdk-server` and the MPCVault gRPC client into the boot path of every consumer whether or not they use them. (b) starts a solver on an address the operator did not intend — a config that names a key is evidence of intent, and quietly filling with a throwaway watch-only key instead is worse than not starting. The error names the fix (`signer: await createSigner(config.simplex.signer)`), so the TOML path stays one line away.

## 2026-08-18 — The signer requirement moved from `validateConfig` to boot

Chosen: `validateConfig` no longer requires `[simplex.signer]`; it validates the block only when present. `bootFiller` rejects a missing `options.signer` unless every resolved chain is watch-only, and the CLI keeps its own `[simplex.signer]`-worded check so a file-driven run still fails at parse time with a file-oriented message.

Alternative considered: keeping the check in `validateConfig` and passing a "a signer was supplied" flag through it.

Why: `validateConfig` is exported for consumers to gate a config before starting, and boot calls the same function — leaving the rule there would have made every library consumer's valid config throw, since the signer is no longer in it. Threading a flag through would keep a config validator asking about an argument that is not config. The duplicated CLI check is deliberate: it costs three lines and preserves the error the binary's users already know.

## 2026-08-18 — No delegation-send hook on the interface: MPCVault's EIP-7702 gap is handled in its viem account

Chosen: `Signer` has no `sendEip7702DelegationTransaction`. `DelegationService` always calls `walletClient.sendTransaction`, and `createMpcVaultAccount`'s `signTransaction` branches on a present `authorizationList` to serialise and raw-sign the set-code transaction instead of using MPCVault's structured `evmSendCustom` request.

Alternatives considered: an optional interface member defaulting to the wallet-client send (the shape this change first took); a `mode === "mpcVault"` branch inside `DelegationService`.

Why: only one backend ever needed the hook, and the reason is narrow — MPCVault's `evmSendCustom` has no field for an authorization list, so a set-code tx sent through it goes out as a plain 0-value transfer with the delegation silently dropped. That is a property of MPCVault's transaction API, so it belongs in the viem account that wraps that API, where viem already hands `signTransaction` the full transaction. Exposing it on the interface made every implementer read about a problem that is not theirs, and put a transaction-sending concern in an interface that is otherwise about signatures. The `mode` branch was worse: it reintroduces exactly the closed-world knowledge this change removed.

Structured signing is deliberately preserved for everything that is not a set-code transaction: the vault's policy engine still sees to/value/data for rebalances and transfers, and only loses that view for the delegation tx, which it could not represent at all.

## 2026-08-18 — `signMessage` dropped from `Signer`, and from the SDK's `SigningAccount`

Chosen: neither interface declares `signMessage`. Simplex's `Signer` is `account` + `signTypedData` + `signRawHash` (plus optional `mode` and `signAuthorization`).

Alternative considered: keeping it, since `Signer` extends `@hyperbridge/sdk`'s `SigningAccount`, which required it.

Why: nothing called it. Bids are signed as EIP-712 UserOperations through `signTypedData` (`BidManager.prepareSubmitBid`), not as personal-signed hashes — the SDK's own `SigningAccount` declared `signMessage` and never invoked it either, so the requirement propagated into simplex and out to every implementer for nothing. Removing it from the SDK type is safe for its consumers: a type narrowing only breaks callers of the removed member, and there are none. The bundled adapters' viem accounts still sign messages (viem's `toAccount` requires it and wallet clients use it); the MPCVault account keeps its explanatory throw, since its personal-message API needs a chain id that viem's signature does not carry.

## 2026-08-18 — `viemSigner` rejects an account with no `sign` at construction

Chosen: throw when building a signer from a viem account that cannot sign raw digests, instead of failing when the delegation path first calls it.

Alternative considered: leave `signRawHash` throwing lazily, so a watch-only or non-delegating deployment could still use such an account.

Why: solver selection is the only fill path simplex uses, and it requires an EIP-7702 delegation signed with a raw digest. An account that cannot produce one yields a solver that boots, scans, and fails at its first delegation attempt — a failure separated from its cause by everything in between. Watch-only solvers do not build a signer at all (boot stands a throwaway key in), so nothing legitimate is blocked by failing early.

## 2026-08-18 — `mode` is a free-form optional string, not a union of the shipped backends

Chosen: `Signer.mode?: string`, defaulting to `"custom"` where it is logged.

Alternative considered: keeping `mode: "privateKey" | "mpcVault" | "turnkey"`.

Why: the union made every custom signer misreport itself as one of three backends it is not, and the field is only ever read for logs (`DelegationService`, and the boot-time "EVM signing strategy" line). A label with no behaviour attached should not be a closed set.
