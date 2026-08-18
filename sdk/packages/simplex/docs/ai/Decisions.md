# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/simplex`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

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
