# ChangeLog

AI-maintained log of code changes in `sdk/packages/simplex`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/simplex/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-20 — pairs.test.ts catches up with the probe and paymaster changes, and CI now runs it

`src/tests/pairs.test.ts` failed 12 of its 55 tests on main, unnoticed because no CI script ran
the file. Both failure groups were stale test expectations, not product regressions — each was
verified against the intent recorded in the 2026-08-19 entries before editing:

- Three phantom-probe tests still expected `quotePhantomFill` to cap its quote at the pair's
  `maxOrderSize`. The cap no longer rations probes (see "A phantom probe is no longer rationed by
  the pair's exposure cap", and the Decisions entry "The exposure cap governs fills, never
  probes") — that change updated `fx.one-sided-lp.test.ts` only and left this file behind. The
  assertions now expect the full unrationed quotes (e.g. 200,000 ZARP × 100 = 20,000,000 CNGN
  where the old cap produced 10,000,000), and the test names/comments no longer claim probes are
  capped.
- Nine profit-gates tests scored 0 where they expected a positive result (or the partial-fill
  flag). The leg loop now calls `paymasterReserveForToken` (see "Fill sizing reserves the
  paymaster's gas pull"), whose `hasPaymaster` check calls
  `configService.getCirclePaymasterAddress` / `getSimplexPaymasterAddress` — absent on the
  suite's `cfg` mock, so every evaluation threw `TypeError` into `calculateProfitability`'s catch
  and returned 0. Root cause pinned with a throwaway probe test against the exact mock shape. The
  mock now defines both getters as `() => undefined` (no paymaster configured → zero reserve),
  which restores the suite's exact spread/fee arithmetic.

`test:filler` now includes `src/tests/pairs.test.ts`, so the file runs in CI (the "Run simplex
test" step of `.github/workflows/test-sdk.yml`). It is pure-unit — no network, no env. Verified:
55/55 pass via `pnpm vitest run --maxConcurrency=1 src/tests/pairs.test.ts`, biome lint clean on
the file, and `vitest list` collects all three `test:filler` files after codegen.

Files: `src/tests/pairs.test.ts`, `package.json`, `docs/ai/{ChangeLog,Decisions}.md`.

## 2026-08-19 — A phantom probe is no longer rationed by the pair's exposure cap

`quotePhantomFill` fed the pair's per-order exposure budget into `computeLegPolicyOutput`, which
clamps the priced quantity to `min(legMaxToken0, remainingToken0)`. A probe is a price quote, not
an allocation — it commits no capital — so the clamp had no exposure to protect, and when it bound
it corrupted the published price: the output covered less than the standard amount while every
consumer still divides by the FULL standard amount. A pair capped below the probe therefore
published a proportionally worse rate with nothing to signal it. At `maxOrderSize` 10 against a
100-token probe that is a rate 10x too low.

`computeLegPolicyOutput`'s `remainingToken0` is now `Decimal | null`; `null` means "price the whole
input, unbudgeted" and only `quotePhantomFill` passes it. `fill()` passes the real budget exactly
as before, which is where exposure is actually taken.

Two related changes in the same path:

- The rate is now sampled at the leg's OWN notional (`sized.legNotionals[i]`) rather than the
  pair's exposure-capped budget. On a sloped curve, sampling at a smaller notional advertises a
  tighter rate than this filler would give at the probe's size, and optimistic is the one
  direction a published rate must never be — a quote built from it has to stay fillable.
- A probe whose notional exceeds the pair's `maxOrderSize` now logs a warning. The price is
  honest, but no order that size can clear it, so the operator needs to see the config gap.

This mattered because the coprocessor pallet's standard amount is moving from 1 to 1000 tokens to
buy quote precision. At one token the clamp effectively never bound (a pair's whole budget is
~2 tokens against a `maxOrderSize` of thousands); at 1000 it plausibly does.

Files: `src/strategies/fx.ts`, `src/tests/strategies/fx.one-sided-lp.test.ts`.

Not verified: `node_modules` was not installed in this worktree, so `tsc` and `vitest` were not
run. Needs a normal CI run.

## 2026-08-19 — Two more ways a fill could be sized past what the wallet can pay

Both found while tracing the paymaster shortfall below, both with the same failure signature — a credit or a balance that the sizing believed in and the chain did not.

**Uniswap V4 credited liquidity it was not going to remove.** `removeCallParameters` re-derives the decrease as `liquidityPercentage.multiply(position.liquidity).quotient`, which truncates, while the planner priced the fill from the untruncated liquidity it asked for. The gap is always in the reverting direction and scales with the withdrawal: at the old 1e6 denominator, a slice of ~0.0159% of a position shed 0.63% of it, roughly $93 of phantom credit on a $14.8k draw. A new `liquidityRemoval` helper resolves the percentage and the liquidity it actually encodes together, at a 1e18 denominator, and the planner now credits and consumes that figure and hands the same `Percent` to the SDK. It also returns null for a slice too thin to register rather than letting the SDK's ZERO_LIQUIDITY invariant throw.

**The escrow-release dispatch fee was unaccounted for.** `HyperApp.dispatchWithFeeToken` pulls `dispatchFee` from the solver's wallet in the destination host's fee token — USDC on Base, the same token most fills pay out. `buildApprovalAndFillCalldata` already added it to the approval; nothing subtracted it from the balance. It is priced by `estimateGasFillPost`, which depends on the funding calls the leg loop produces and so cannot run before it, so `evaluateOrder` now checks affordability straight after the estimate: for a cross-chain order it requires the fee token's post-fill residue to cover the dispatch fee plus the paymaster reserve, and skips otherwise. Shrinking the fill is not an option — cross-chain orders cannot be partially filled.

Files: `src/funding/uniswapV4/UniswapV4FundingPlanner.ts`, `src/strategies/fx.ts`, `src/tests/funding/uniswapV4Removal.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-19 — Fill sizing reserves the paymaster's gas pull

A partial fill on Base reverted with `ERC20: transfer amount exceeds balance` after being short by 28,993 units — $0.029 on a $14,808.699383 fill. The fill was sized as wallet balance plus the Uniswap V4 credit and bid to the last unit, but the paymaster charges gas in the same USDC and pulls it via `transferFrom` during `validatePaymasterUserOp`, before the batch runs. The bid amount was bit-exact `balance + credited`, and the revert delta was bit-exact the prefund.

The only reserve the sizing loop knew about came from `FundingVenue.walletReserveForToken`, which is the vault's `minBalance`. `UniswapV4FundingPlanner` returns `0n` there by design (LP positions have no wallet float), and a filler configured with no venue at all never enters that loop, so both setups sized fills with zero headroom. A new `paymasterReserveForToken` in the paymaster module now contributes a reserve for the chain's USDC and USDT whenever a paymaster is configured, scaled by each token's own decimals, and the leg loop seeds `reserve` with it.

Files: `src/services/paymaster/index.ts`, `src/strategies/fx.ts`, `src/tests/services/paymaster-reserve.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-19 — Cross-lane orders skip cleanly instead of erroring "Shared cache is not initialized"

An operator running only Base saw `ERROR: [intent-filler]: Shared cache is not initialized` whenever an order destined to a chain their filler does not run scrolled past. The solver-selection cache is only ever populated for configured, non-watch-only chains (boot's `solverSelectionChains`), so any other destination fell through `handleNewOrder`'s cache check and was dropped with an error that reads like the filler is broken. The behavior (drop) was correct — there is no client, bundler or float for an unconfigured destination — the diagnosis was not, and it dates to #287.

`handleNewOrder` now checks the destination first: not a configured chain, or configured but watch-only → debug-level skip, cache untouched. The "Shared cache is not initialized" error survives for what it actually means now — a configured, filling destination with a genuinely absent entry, which is an initialization bug. Pinned by `order-destination.test.ts`: unconfigured / non-EVM / watch-only destinations never touch the cache, a filling destination proceeds to the allowlist, and the configured-but-uncached case still errors.

Files: `src/core/filler.ts`, `src/tests/core/order-destination.test.ts` (new).
## 2026-08-19 — The binary silences @polkadot/* startup noise; the library still never touches the console

Every start of the bundled CLI printed a wall of `@polkadot/util has multiple versions` warnings, `REGISTRY: Unknown signed extensions` / `API/INIT: RPC methods not decorated` logger chatter, and Node's punycode deprecation. `src/bin/quiet.ts` — the entry's first import, since most of this fires during `@polkadot/*` module init — sets polkadot's official `POLKADOTJS_DISABLE_ESM_CJS_WARNING=1` (silencing the same-version dual-instantiation the single-file bundle necessarily produces), sets `process.noDeprecation`, and wraps `console.warn` with a narrow filter for the remaining known patterns. Only `console.warn` is touched (both noise sources write there); `console.error` is untouched, and real polkadot output — connection failures included — passes through, pinned by test.

Strictly bin-scoped: `dist/index.js`/`dist/sqlite.js` contain none of it (verified by grep), because a library consumer seeing duplicate-package warnings has a real dedupe to do in their own tree.

Two gotchas worth recording: `package.json`'s `"sideEffects": false` silently tree-shook the bare `import "./quiet"` out of the bundle — it is now an array listing `src/bin/quiet.ts` as the one side-effectful module — and vitest's own console interception makes the patched `console.warn` unobservable through stderr, which is why the test asserts through the exported `filteringWarn` factory instead.

The genuine version skew is fixed at the source in the same change, at the maintainer's call: the sdk's `@polkadot/api: "latest"` (and its `types`/`util`/`util-crypto`/`keyring` "latest" pins) became concrete `^16.5.6`/`^14.0.3` ranges, simplex's direct `@polkadot/util{,-crypto} ^13.5.6` moved to `^14.0.3` to match what api 16.x requires, and both packages' `resolutions` blocks — which pnpm warned were ineffective — are deleted rather than moved (they were doing nothing; nothing changed by removing them). Verified structurally, not just by silence: the rebuilt binary's bundle contains zero `13.5.9` occurrences where it previously carried both versions, `pnpm why @polkadot/util` resolves a single 14.0.3 in both package trees, and substrate key derivation (`balance-provider.test.ts`) passes on util-crypto 14.

Files: `src/bin/quiet.ts` (new), `src/bin/simplex.ts`, `package.json`, `sdk/packages/sdk/package.json`, `src/tests/cli/quiet.test.ts` (new).
## 2026-08-19 — Quorum client suspends rate-limited endpoints for 5 minutes

`QuorumPublicClient` previously re-queried a 429ing endpoint on every call — `isRateLimited` existed but only labelled diagnostics — which both wastes the call and deepens the provider's throttle. An endpoint whose failure is unambiguously a request-rate limit (`isSuspendableRateLimit` — stricter than the diagnostic `isRateLimited` label: `-32005` alone never benches, since Infura returns it for deterministic getLogs result caps, and the free-text match excludes URL-bearing metaMessages) is now suspended for `RATE_LIMIT_SUSPENSION_MS` (5 minutes) and dropped by `participants()` — from the query set and from the quorum bar both: each call's threshold is `quorumThreshold(endpoints actually queried)`, so the remaining endpoints keep serving reads while a provider throttles (first shipped with a fixed full-set threshold; reversed by the maintainer — a throttled endpoint answers nothing either way, and counting it only makes the scanner miss orders). With every endpoint benched, all are queried again. Suspension is recorded even from stragglers that settle after a call already decided, `suspended()` exposes the benched URLs, and QuorumError messages carry `responders: N/M queried (K/S suspended for rate limiting)` — the skipped count snapshotted at endpoint selection, not re-sampled at throw time (a long call can outlive a suspension window). `settleUntilQuorum` now takes `{ idx, task }` pairs so failures map to real endpoint indices (`getTransactionConfirmations` previously reported `unknown` URLs in failure detail).

Tested on the real-HTTP harness in `rate-limit-detection.test.ts` (genuine 429/500/-32005 responses through viem). The traffic-stops assertion runs through uncached `getLogs`, not `getBlockNumber` — viem caches eth_blockNumber for 4s, and an adversarial reviewer proved the naive version passed with suspension disabled entirely; the hardened suite fails 2 tests under that regression (verified by probe). Also covered: re-query after expiry via a `Date.now` spy, the small-set availability rule, threshold non-shrinkage at n=5, -32005 result caps not benching, and 500s staying stateless.

Files: `src/services/QuorumPublicClient.ts`, `src/tests/rate-limit-detection.test.ts`.

## 2026-08-18 — Signer return contracts spelled out

The docs showed `Promise<HexString>` twice and `Promise<Signature>` once without saying that the three mean different things: `signTypedData` returns a bare 65-byte `r ‖ s ‖ v` signature (`v` 27/28, the `eth_signTypedData_v4` form), `signAuthorization` returns split components with `yParity` strictly 0/1, and `signTransaction` returns the whole signed transaction as typed-envelope RLP — not a signature at all. An implementer had to reverse-engineer that from the adapters. The contracts now live in the `Signer` and `Signature` docblocks (what an IDE shows), a "Return exactly" table on both doc pages, and the sdk's `SigningAccount.signTypedData` docblock.

Files: `src/services/wallet/types.ts`, `sdk/packages/sdk/src/types/index.ts`, `docs/content/developers/sdk/{simplex,api/simplex}.mdx`.

## 2026-08-18 — Audit fixes: yParity guards, the signerless one-way door, and honest tests

A six-dimension adversarial audit of the signer branch confirmed 21 distinct issues; this change fixes all of them except the two deliberate semver calls (patch releases carrying breaking surface changes — restated in the PR and left to the maintainer).

Two code defects. `digestSigner` now validates the backend's signature once for every operation — most HSM docs return the legacy v (27/28), viem encodes any truthy `yParity` as parity 1, and EIP-7702 skips an invalid tuple without reverting, so an unguarded 27 meant "Delegation successful" in the logs and an undelegated solver on chain; `DelegationService.buildAuthorization` carries the same guard so hand-written signers are covered. And a signerless boot is now recorded (`FillerRuntime.signerless`) and enforced: boot's new per-chain watch-only exemption had made it possible to start an observer and later flip it into filling with the generated throwaway key via `chains.add`/`setWatchOnly(false)` — both now refuse, and `add` defaults new chains to watch-only. The setup API's gate also stops crashing on a signerless watch-only config (`TypeError` on an absent block) and instead mirrors run's rule.

Test integrity. `assertSignsForItsAddress`'s transaction leg asserted shape only — a signer signing the digest of a *different* transaction passed — and now parses the signed bytes and recovers the signer, matching the gated integration tests. The refactor had migrated the code but not several fixtures: `UserOpSender`, `ContractInteractionService.rpc`, `pairs`, `fx.price-guard` and `fx.one-sided-lp` tests still stubbed `{ account: { address } }`, so the paths under test ran with the solver address `undefined` (hidden by `as unknown as Signer` casts); all swept to `{ address }`, and the UserOp tests now assert `op.sender`. The deleted `validateConfig` signer-requirement tests are replaced at the layer that owns the rule now: `boot-signer.test.ts` pins the boot rejection through a mock RPC, unit-tests `allChainsWatchOnly` (exported for the purpose), and pins the signerless guards. The shared `TYPED_DATA` fixture lists `EIP712Domain`, honouring the branch's own contract; one ungated assertion now recovers an authorization against the hand-built `keccak256(0x05 ‖ rlp(...))` preimage, so sign and verify no longer share viem's hasher; and a persist-roundtrip test asserts the `[simplex.signer]` block survives a dashboard rewrite — the regression the `FillerConfigFile` split exists to prevent.

Docs drift from the design's three iterations corrected across both packages' `docs/ai` (final interface shape, `signRawHash`'s removal, `validateConfig` does not read the signer block, `UserOpSender.buildSignedUserOp`), the README's quick-start now compiles, and the `digestSigner` docs state the split-signature contract and the new yParity rejection. The API reference documents the signerless one-way door.

Files: `src/services/wallet/account.ts`, `src/services/DelegationService.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/services/server/setup-api.ts`. Tests: `src/tests/core/boot-signer.test.ts` (new), `src/tests/wallet/signer.test.ts`, `src/tests/services/{UserOpSender,ContractInteractionService.rpc,SimplexPaymaster}.test.ts`, `src/tests/{pairs,ui-server}.test.ts`, `src/tests/strategies/{fx.price-guard,fx.one-sided-lp}.test.ts`. Docs: both packages' `docs/ai/*`, `README.md`, `docs/content/developers/sdk/{simplex,api/simplex}.mdx`.

## 2026-08-18 — `Simplex.start` takes a Signer interface instead of a signer config

Signing was reachable only through `config.simplex.signer`, a tagged union over the three backends simplex happens to ship, so a consumer embedding the library could not sign with anything else. `Simplex.start` now takes `signer: Signer` — the interface the solver already used internally — and the config block is the binary's TOML format only.

The interface carries no viem types at all, and names operations rather than digests: `address`, `mode`, `signTypedData`, `signAuthorization`, `signTransaction` — all required. `signRawHash` is gone, from both packages: once a signer must be able to sign an authorization and a transaction, nothing in either package calls it. `digestSigner({ address, mode, sign })` is the escape hatch for backends that only see 32 bytes; it does the EIP-712 hashing, the EIP-7702 authorization hashing and the transaction serialising, so that work stays in one tested place instead of in every integration. The `account: LocalAccount` field is gone — `accountFor(signer)` builds the viem account wallet clients need on our side of the boundary, so a consumer no longer has to match this package's viem version to satisfy the type (the `pnpm.overrides` warning that used to head `types.ts`). `TypedDataPayload`, `Signature`, `SignerTransaction` and `Eip7702Authorization` are ours, defined from the specs rather than from viem.

Two members went as unused or misplaced. `signMessage`, along with its declaration in the SDK's `SigningAccount` — bids are signed as EIP-712 UserOperations, and no path in either package ever invoked it. And the `chainId` argument on `signTypedData`, in both packages: EIP-712 carries it in `domain.chainId`, every payload simplex signs sets it, and MPCVault (the only backend that reads one, for its request envelope) now takes it from the payload instead of an argument that defaulted to mainnet.

`sendEip7702DelegationTransaction` was replaced by the general `signTransaction`: `DelegationService` sends every set-code transaction through the wallet client, and a backend that takes transactions rather than digests implements `signTransaction`. MPCVault does, including the set-code case its structured `evmSendCustom` request cannot express (no authorization-list field), which it serialises and raw-signs itself. It no longer builds a viem account at all.

Added `viemSigner(account)`, which derives the whole interface from any viem local account, and rebuilt `privateKeySigner` and `turnkeySigner` on top of it (Turnkey keeps its structured `signAuthorization`). Factories renamed to their public names: `createSimplexSigner` → `createSigner`, `create*SigningAccount` → `privateKeySigner` / `mpcVaultSigner` / `turnkeySigner`, `initializeSignerFromToml` → `signerFromToml`, and the `SigningAccount` type → `Signer`.

`FillerTomlConfig` (exported as `SimplexConfig`) no longer declares `simplex.signer` at all. The block moved to a new `FillerConfigFile extends FillerTomlConfig`, the on-disk shape the binary parses, which the CLI, the setup API, the TOML writer, the wizard and the dashboard's config type now use. The binary passes the parsed file straight to `Simplex.start`, extra key and all — `UiServer.persistConfig` regenerates the TOML from the running config object, so stripping the block there would erase the operator's signer from their file on the next curve edit.

Boot now rejects a missing signer unless every chain is watch-only, replacing the `[simplex.signer]` presence check inside `validateConfig` (a config-shape validator has no business requiring — or reading — an argument that is no longer part of the config; a present block is validated by its consumers: `signerFromToml`, the wizard's write step, the setup API's gate). `Simplex.start` throws when an object carries `simplex.signer` and no `signer` was passed — a runtime check, since the type no longer has the field — rather than silently ignoring it or resolving it. The CLI resolves the TOML block into a `Signer` itself and keeps its file-oriented error message.

Verified against a live MPCVault vault (10/10, chain 1). Every signature is checked by recovery, transactions included: the signed bytes are parsed back and asserted to carry the chain id, nonce, recipient and value we asked for, and to recover to the solver's address — the vault builds the transaction itself on the structured path, so "it returned hex" proves nothing. The set-code branch additionally asserts the authorization list survives into the signed transaction. `createMpcVaultAccount` is deleted — with the interface no longer carrying a viem account and `accountFor` building one from any `Signer`, nothing called it; an MPC-backed viem account is `accountFor(mpcVaultSigner(config))`.

Turnkey verified live too: `signTypedData`, `signAuthorization` and `signTransaction` all recover to the wallet address, with the transaction parsed back and checked field by field. The delegate-then-revoke case on Sepolia still needs a funded wallet. Adding those cases is what turned up the EIP-712 finding below — it is not an MPCVault quirk, it is how both remote-hashing backends behave.

Rig facts, since they cost a round of guessing: the vault uuid in the dev machine's `~/.zshrc` is rejected by the API, and the live one is the `vault-uuid` in `~/mpcvault/config.yml` (the value the running client-signer container serves); the vault accepts chain 1 only, so `MPCVAULT_TEST_CHAIN_ID=11155111` fails every chain-scoped call with `Invalid chain id`.

Files: `sdk/packages/sdk/src/types/index.ts` (SigningAccount narrowed), `src/services/wallet/types.ts`, `src/services/wallet/mpcvault.ts`, `src/services/wallet/signer.ts`, `src/services/wallet/index.ts`, `src/services/wallet/accounts/viem.ts` (new), `src/services/wallet/account.ts` (new: `accountFor`, `sdkSigningAccount`), `src/services/paymaster/{types,permit}.ts`, `src/services/paymaster/provider/{circle,simplex}.ts`, `src/services/rebalancers/{binance,usdt0}.ts`, `src/services/RebalancingService.ts`, `src/services/wallet/accounts/privatekey.ts`, `src/services/wallet/accounts/mpc.ts`, `src/services/wallet/accounts/turnkey.ts`, `src/services/DelegationService.ts`, `src/services/ChainClientManager.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/index.ts`, `src/bin/simplex.ts`, `src/config/filler-toml.ts`, `src/services/server/{UiServer,setup-api}.ts`, `src/cli/init/{index,state,emit-toml}.ts`, `src/cli/init/steps/write.ts`, `ui/src/types.ts`, plus type-only renames across `src/core/filler.ts`, `src/services/{ContractInteractionService,PaymasterKeeperService,UserOpSender}.ts`, `src/strategies/fx.ts`. Tests: `src/tests/wallet/signer.test.ts` (new), `src/tests/wallet/turnkey.test.ts`, `src/tests/wallet/mpcvault.integration.test.ts`, `src/tests/cli/filler-toml-validate.test.ts`. Docs: `README.md`, `docs/content/developers/sdk/simplex.mdx`, `docs/content/developers/sdk/api/simplex.mdx`, `docs/content/developers/evm/intent-gateway/simplex.mdx`.
