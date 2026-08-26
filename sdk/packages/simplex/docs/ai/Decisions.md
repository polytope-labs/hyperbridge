# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/simplex`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-24 — Review fixes: robust bootstrap, chain-keyed probe, no dead config (#1147 review)

Seun's high-effort review surfaced several correctness gaps, resolved here:

- **Batched delegation fails open, not closed.** Folding `approve(Permit2, max)` into the native delegation coupled the delegation's success to the approve's. Now the batched tx is attempted, and on any revert the delegation retries as a plain self-call (approval defers to the first sponsored op). Otherwise a token that rejects the approve — USDT's non-zero→non-zero rule, a blacklist, insufficient batched gas — would have blocked delegation permanently, since the resolver recomputes the same approval each attempt.
- **Zero-first approve.** `sendFundedApprove` now resets a stale non-zero allowance to zero before approving max, so Ethereum USDT (which rejects a non-zero→non-zero change) can be bootstrapped even when a leftover Permit2 allowance from another integration exists.
- **PERMIT2() probe: narrowed catch + chain-keyed cache.** Only a genuine contract revert marks a deployment as lacking PERMIT2 mode; a transport error (429/timeout) now propagates rather than being cached as "unsupported" (which would have dropped a migrated solver back to a native-funded standing paymaster allowance). The support cache is keyed by `${chainId}:${address}` so a CREATE2 redeploy sharing an address across chains can't have the first-upgraded chain mark it supported everywhere.
- **Dropped `permit2DeadlineSeconds`.** It was threaded through the builder but no production caller could set it, and the "overridable per call" doc claim had no runtime path. Removed the field; the fixed 1-hour deadline applies everywhere.
- **`_prefund` uses `prefunder_`.** The mode-0x02 branch now reads the base's `prefunder_` parameter instead of re-reading `userOp.sender`, keeping it aligned with the mode-0/1 branch that forwards it to `super._prefund`.

## 2026-08-20 — Permit2 signature stored as v, r, s; delegation-batched approve is native-fallback only (#1071)

Chosen (review request): mode 0x02 stores the signature as explicit `v, r, s` fields, matching the EIP-2612 mode 0x00 layout, rather than a `bytes` blob. The `bytes` form could in principle carry a non-65-byte ERC-1271 signature, but the fixed 182-byte `PERMIT2_DATA_LENGTH` already ruled that out, and SolverAccount verifies via plain ECDSA recovery, so nothing is lost and the two modes now read consistently. The test helper keeps two distinct packers: the account signature stays canonical `r ‖ s ‖ v` (what `ECDSA.recover` expects), while the paymasterData blob is `v ‖ r ‖ s` — conflating them silently breaks account validation with AA23.

Chosen (review request): the `approve(Permit2, max)` bootstrap is folded into the native EIP-7702 delegation **only on the native-fallback path**, not made the default bootstrap strategy. The sponsored delegation stays primary; when it is unavailable and Simplex sends a native set-code tx anyway, the approve rides along. Scoped this way (over "always bootstrap with one native delegate+approve tx") because the native fallback is already the rare path, the change stays additive and guarded — a resolver failure or a chain that does not need it falls back to the plain self-call delegation — and it avoids reworking the delegation strategy or the which-token-to-preapprove question at startup (the resolver only acts when the solver already holds a fee token).

## 2026-08-19 — postOp gas: SDK sends 40,000, contract ceiling stays 100,000 (#1071)

Chosen: the SDK sends `POST_OP_GAS_LIMIT_SIMPLEX = 40_000` (a per-paymaster constant), while the contract keeps `MAX_POST_OP_GAS_LIMIT = 100_000` and adds `MIN_POST_OP_GAS_LIMIT = 30_000`.

**Why the contract ceiling stays 100k (review, F1):** the margin win comes entirely from the *client* sending 40k — the contract only needs to *accept* it. Lowering the contract ceiling to 40k would break backward compatibility: recovering the stake stuck on the live proxies needs an in-place `UpgradeContract`, and a 40k ceiling on an existing proxy would reject every client still sending the old 100k limit (`InvalidPostOpGasLimit` → AA33 → the filler silently drops to native gas). Keeping the ceiling at 100k makes in-place upgrades safe and still realises the full margin. The only thing given up is a marginal anti-grief tightening — a griefer inflating postOp to 100k forfeits ~8.85k gas of penalty out of the cushion the op already pays — which can be revisited once every integrator is on a 40k client. The `MIN_POST_OP_GAS_LIMIT = 30_000` floor (closing the refund-underflow window) is safe on an in-place upgrade because every existing client sends 100k ≥ 30k.

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

Chosen: a random 256-bit nonce per op (`crypto.getRandomValues`) and `deadline = now + PERMIT2_DEADLINE_SECONDS` (a fixed 1 hour). Alternatives: lowest unused bit read from `nonceBitmap` — mimics EIP-2612's self-invalidation but makes concurrent bids on one chain collide (bids use distinct account nonce keys precisely to run concurrently); `maxUint256` deadline as PERMIT mode uses — wrong here because unordered Permit2 nonces never self-invalidate, so every losing bid would leave a live $5 permit forever. One hour comfortably exceeds bid-to-execution latency (order deadlines are ~10 to 40 minutes of blocks) and clock skew; a random nonce usually touches a fresh bitmap word (about 17k extra gas), negligible in stablecoin terms. The EIP-2612 path's `maxUint256` deadline is correct for USDC, whose v2.2 permit short-circuits `deadline == max` before reading the timestamp.

## 2026-08-18 — Mode 2 gated on a `PERMIT2()` probe of the paymaster (#1071)

Chosen: `paymasterSupportsPermit2` reads the `PERMIT2()` constant that only the Permit2-capable implementation exposes; positive results are cached for the process lifetime, negative ones for five minutes. Alternative: rely on release ordering (client after all five redeploys) — fragile, since redeploys land chain by chain and a governance upgrade keeps the address, and a mode-2 op against an old deployment fails validation with `InvalidMode`, which for a bid means a lost fill.

## 2026-08-18 — `forceApproveMode` renamed to `skipPermit` (#1071)

Chosen: the delegation flow's flag now only skips EIP-2612 permit detection; PERMIT2 and APPROVE stay available. The flag exists because delegation ops pass fixed, measured account-side gas limits; the Simplex builder sets its own paymaster verification limit per mode (`VERIFICATION_GAS_LIMIT_PERMIT2` = 200k, measured at ~135k on Ethereum and BSC forks), so PERMIT2 does not disturb them. Keeping the old name would have forced BSC delegations to keep a paymaster allowance, i.e. two funded approvals per token instead of one.

## 2026-08-18 — Known: PERMIT2 mode is not ERC-7562-clean (#1071)

Permit2's `nonceBitmap[owner][word]` and the token's `allowance[owner][Permit2]` are not sender-associated storage under ERC-7562, so a spec-enforcing bundler could reject mode 2 during validation. Accepted because the paymaster already reads `block.timestamp` (also banned) in every mode and is live through the bundlers in use, and because PERMIT/APPROVE remain as fallbacks. Verified empirically on Base Sepolia: Alchemy's bundler accepts mode 2 for both a fresh EOA (Permit2 ecrecover path) and a delegated account (ERC-1271 path). Pimlico was not probed (no key). If a bundler rejects mode 2, the fallback design is to keep APPROVE and refill the allowance inside sponsored ops (ERC-7821 batch) rather than with native txs.

## 2026-08-18 — Bootstrap approve waits for two confirmations (#1071)

Chosen: `sendFundedApprove` waits for `confirmations: 2`. On Base Sepolia the first sponsored op right after a one-confirmation approve was rejected `AA33` (Permit2 `TRANSFER_FROM_FAILED`: the bundler's simulation node had not seen the approve yet) twice in a row, and the identical op seconds later was accepted; with two confirmations a fresh EOA went through first try. Alternative: retry the op on `AA33` — more code for a once-per-token-lifetime event that costs one extra block.
## 2026-08-20 — Multi-leg orders are rejected at the monitor, not deeper in the fill path

Chosen: `EventMonitor` filters on leg count, so an order with more than one input or output asset
never reaches `IntentFiller`.

The monitor is the one place every real order passes through exactly once, and it already owns the
other two intake filters (filler-address matching on fills, de-duplication). Rejecting there means
the constraint holds for every strategy without each of them re-checking, and the cost of a
rejected order stays at one debug log.

Alternatives rejected:

- *Reject in `FXFiller.evaluateOrder`.* It already refuses an order whose input and output counts
  disagree, but a balanced multi-leg order would still be priced, quoted and possibly bid. The
  work is wasted, and a second strategy would need its own copy of the rule.
- *Filter in the scanner.* The scanner is shared between fillers; a filler that wants multi-leg
  orders could not opt back in. Per-filler intake policy belongs to the per-filler event bus.
- *Strip extra legs and fill the first one.* Silently changes what the user asked for. An order is
  filled whole or not at all.

## 2026-08-20 — Removing a cap is its own endpoint, not a null on the update

Chosen: `DELETE /api/strategies/:index/max-order-size`, with `AdminStrategy.clearMaxOrderSize` and
`Simplex.clearMaxOrderSize(index)` behind it. `PUT /api/strategies/:index` still requires a positive
value.

The obvious alternative — accept `maxOrderSize: null` (or `""`) on the PUT — is one fewer endpoint
and was rejected on blast radius. `DELETE /api/strategies/:index` already removes the *market*. A
scheme where a cap removal and a market removal differ by a path suffix is bad enough; one where a
cap removal is a field value that a serialization bug, a form default, or a stray `undefined` can
produce is worse, because the failure is silent and the fix is a re-add. An explicit verb on an
explicit resource cannot be reached by accident.

The handler is idempotent: clearing an already-uncapped market returns 200 with the same state
rather than 404 or 409. The UI decides what to send from the field's contents alone and does not
track which state the market is in, so a strict version would make the client carry knowledge it has
no reason to have.

Alternatives rejected:

- *`PATCH` with `maxOrderSize: null`.* See above.
- *A `capped: boolean` toggle beside the value.* Two controls for one setting, and it forces a
  decision about what the value field holds while the toggle is off — remembered, cleared, or
  ignored. A blank field is the same information with nothing to keep in sync.
- *Reuse `PUT /api/strategies/:index` with an empty body.* Ambiguous with a no-op update, and the
  handler already rejects unknown/absent fields to catch exactly that class of mistake.

In the UI the field itself is the control: blank means uncapped, the button relabels to "Remove
cap", and it enables on any divergence from the persisted value rather than on a non-empty value —
the old guard made blanking unsubmittable, which is precisely the state that now needs submitting.

## 2026-08-20 — `maxOrderSize` is optional

Chosen: `TradingPair.maxOrderSize` is `Decimal | undefined`. Absent means uncapped.

The cap was mandatory in `validatePairConfigs` but already optional in the TOML type, so
`tradingPairFrom` bridged the gap with `new Decimal(pair.maxOrderSize ?? "0")` — a placeholder that
only worked because reference-only pairs never reach the sizing path. A zero cap and no cap are
opposites, and encoding "no cap" as the most restrictive possible value is the kind of thing that
holds until someone routes a new pair type through the same code.

`sizeOrder` is where absence is resolved: an uncapped pair sets `cappedByPair` to the order's own
per-pair total and `capFraction` to 1. That keeps the per-pair ration in `computeLegPolicyOutput`
doing its other job — stopping two legs of the same pair from spending the same token0 twice —
without it ever binding below the order.

Alternatives rejected:

- *Keep it required and let operators write a very large number.* Works, but "uncapped" then has no
  representation, only an approximation, and the log line reads as a cap that happens not to bind.
- *Default an absent cap to `Infinity`.* Same behaviour, but `Decimal(Infinity)` propagates into
  `capFraction` division and into every log that stringifies the cap. `undefined` makes each
  consumer state what it does when there is no cap.

Kept deliberately: `assertPairValid` still exempts reference-only pairs from the positive-value
check. `FXFiller` takes `TradingPair[]` as a public constructor argument, and callers written
against the old required field still pass `new Decimal(0)` there. A reference pair never fills, so
its cap is never read either way — rejecting it would break those callers for nothing.

Since resolved: removal is now reachable at runtime — see "Removing a cap is its own endpoint".

## 2026-08-20 — The curve amount is the fill, and the exposure cap does not shorten it

Chosen: `targetOutput = policyMaxOutput`, unconditionally.

Overfilling is a feature of the protocol, not an accident the filler should suppress.
`IntrinsicIntents.sol` has an explicit `solverAmount > totalRequired` branch that splits the excess
between the beneficiary and the protocol (`surplusShareBps`); `quotePhantomFill` publishes
`policyMaxOutput` as our quoted rate, so paying `output.amount` advertises a price we do not
honour; and `calculateProfitability`'s own doc says we overfill "if the pair pricing makes that
attractive. This is how we stay competitive."

`desiredOutput` is redundant with the ration `computeLegPolicyOutput` already applies to
`token0ForLeg`, and it is no longer a ceiling on payout at all — it survives only as the price
gate's comparand and in the short-fill logs. `maxOrderSize` binds in the token0 dimension, before
the rate is applied, which is where the exposure actually is: a capped leg never pays out more than
the capped slice's worth at the curve.

The trade-off, taken knowingly: escrow releases as `fillAmount / totalRequired`, so on a capped leg
paying above the user's pro-rata ask draws down more input than the cap fraction nominally allots.
That is more of the user's token for the same outlay — the cap bounds what the filler spends, not
what it receives — and treating the receive side as the thing to ration was clamping the price the
operator configured.

`capLimited` had to follow, and is now `capFraction.lt(1) && policyMaxOutput < output.amount`
rather than `desiredOutput < output.amount`. It gates partial-fill eligibility, and with the payout
unclamped a curve far enough above the order's rate can cover the whole ask out of a capped slice.
That is a full fill; gating it as a partial would reject cross-chain and calldata orders the filler
can serve.

Alternatives rejected:

- *Clamp to `desiredOutput` on capped legs only.* What this replaced. It keeps the escrow draw
  exactly proportional to the cap, but at the cost of quoting one price and filling another on
  every capped order — the same defect as the unconditional clamp, just rarer and harder to see.
- *Keep the clamp and stop publishing `policyMaxOutput` from `quotePhantomFill`.* Makes the quote
  match the fill, but by degrading the quote to the order's own rate — which is the counterparty's
  number, not ours. The price feed would stop carrying any information about the operator's curve.
- *Re-enable `maxOverfillBps` as part of this change.* Deliberately left alone. The clamp at the
  overfill-ceiling block is still a no-op assignment (`const policyMaxOutput = rawPolicyMaxOutput`)
  and `recordOrderOutcome` is still always called with `false`, so `maxOverfillBps` and the halt
  subsystem remain dormant config. Restoring the payout makes that ceiling meaningful again and it
  should be either re-armed or deleted outright — a separate decision from fixing the payout, and
  one that changes the filler's loss bound rather than its price.

Left standing, and known stale: `curveSurplusUsd` (the P&L fallback for legs with no opposite
curve) measures `policyMaxOutput - output.amount` and calls the difference "ours". With the surplus
now paid out that term is structurally zero on uncapped legs. It is report-only telemetry — it
never rejects an order or feeds the execute score — so it under-reports rather than mis-fills, and
re-basing it on the opposite curve was left out of this change.

## 2026-08-19 — The exposure cap governs fills, never probes

Chosen: `computeLegPolicyOutput` takes `remainingToken0: Decimal | null`, and `quotePhantomFill`
passes `null`. The pair's `maxOrderSize` budget rations real fills only.

The two paths are not the same kind of number. On a fill the output is an amount to pay out, and
`maxOrderSize` is a real exposure limit — clamping is the point. On a probe the output is a
*price*: the leg is quoted against a fixed standard amount and every consumer recovers the rate as
`medianPrice / standardAmount`. Clamping the quantity there does not reduce exposure (there is
none), it just makes the numerator smaller than the denominator assumes, and the published rate is
wrong by exactly the clamp ratio — silently, with no error and no warning.

Alternatives rejected:

- *Skip the leg when the probe exceeds the pair's cap.* Honest, but it removes the pair from the
  price feed entirely and zeroes its depth downstream. A correct price for a size the filler
  would cap is more useful than no price, and the warning covers the operator's need to know.
- *Clamp, then scale the output back up.* Identical to not clamping for a linear curve, and
  actively misleading on a sloped one, since the scaled figure would not be a price the curve ever
  produced.
- *Leave it and raise `maxOrderSize` in operator config.* This is a code bug that produces a wrong
  published number; requiring every operator to know that would guarantee someone does not.

## 2026-08-19 — The filler floors its quoted output, and that must stay

Not a change — a decision to leave `computeLegPolicyOutput`'s `.floor()` alone, recorded because
the alternative is tempting and was actually attempted and reverted during this work.

At a one-token standard amount, flooring looks like a pricing error: a cNGN/USDC curve of 1398
publishes as 1398.6014, off by 0.043%, because the output integer is only ~715 base units and the
price grid is `1/715` = 0.14% coarse. Rounding to nearest halves that error and removes its bias,
which is why it looks like a fix.

It is not. The floor is a fillability guarantee. The SDK's `PhantomSnapshotQuoter` derives
`amountOut` from `netAmountIn * medianPrice / standardAmount`, and the gateway's fully-filled check
(`if (totalRequired > amountFilled) isFullyFilled = false`) has zero tolerance. Flooring keeps the
published rate at or below the filler's true curve rate, so a quote built from the snapshot is
always honourable. Rounding to nearest puts the published rate ABOVE the curve about half the time,
and every order quoted at such a rate under-fills — reverting outright for calldata and
cross-chain orders. Simulated across curve values 1392–1402 at 0.5 steps and three order sizes:
floor 96/96 fillable, nearest 49/96.

Precision belongs to the probe size, not the rounding mode. Raising the pallet's standard amount to
1000 tokens shrinks the same conservative buffer ~1000x (worst error 0.140% -> 0.00014%) while
keeping every quote fillable.

## 2026-08-19 — Published phantom prices are gross of the gateway protocol fee

Noted, not changed. `placeOrder` deducts `protocolFeeBps` from each input, mutates `order.inputs`
to the reduced amounts, and takes the commitment over those — so on a real order the strategy
already receives a post-fee input and must not net it again. Phantom orders never pass through
`placeOrder`, so their standard amount is un-netted and the published price is the gross rate.

This does not break quoting: `PhantomSnapshotQuoter` calls `deductProtocolFee` before applying the
rate, mirroring the gateway's floored arithmetic exactly. Any *other* consumer reading
`medianPrice / standardAmount` as an executable rate is ~0.3% optimistic at the deployed 30 bps.
Netting it inside `quotePhantomFill` would need the strategy to read a gateway parameter it does
not currently know about; left open pending a decision on whether the published surface should be
gross or net.

## 2026-08-19 — The V4 planner credits the liquidity the calldata encodes, rather than rounding the percentage up

Chosen: `liquidityRemoval(totalLiquidity, desiredLiquidity)` returns the `Percent` and the liquidity the SDK will derive from it, applying the SDK's own truncation, and the planner prices the withdrawal from that liquidity. The denominator went from 1e6 to 1e18 at the same time.

Alternatives considered: (a) rounding the percentage up so the removal covers what was asked; (b) raising the denominator alone and leaving the credit computed from the requested liquidity.

Why (a) loses: rounding up removes more liquidity than the planner reserved against `remainingLiquidity`, so concurrent plans over one position would over-commit it, and at 100% it cannot round up at all — the case that has to stay exact. Truncating and telling the truth about it is strictly safer than removing more than intended. (b) shrinks the error without removing it, and the error is unbounded in the wrong direction: the credit stays an estimate the chain is not obliged to honour, which is exactly what makes it revert. Fixing the arithmetic and fixing the honesty are separate; only the second one closes the failure mode.

What this does NOT close: `credited` is still the amount the position yields at the *plan-time* `sqrtPriceX96`. Normally the planner's slippage buffer absorbs drift — it targets `remaining × (1 + slippageBps)` and `finalOutputAmount` is capped at `targetOutput`, so the overshoot stays in the wallet as headroom. A drained position (`cappedLiq === availLiq`) has no overshoot and therefore no headroom, which is the exact shape of the fill that failed. Anyone able to move the pool price between bid and execution, by less than the min-amounts tolerance so the withdrawal still succeeds, can make it yield less than was credited and revert the fill. It is griefing rather than theft — `amount0Min`/`amount1Min` prevent a genuinely bad payout, so the cost is a wasted revert that still bills the paymaster. The fix would be to credit `burnAmountsWithSlippage` (the minimum the calldata will accept) instead of the expected amounts, at the price of a systematic under-fill of one slippage tolerance on every drained withdrawal; that trade-off was left to the operator rather than taken here.

The zero guard belongs on the product, not the percentage. Both floors collapse independently: for a 3,441,646,880,004-unit position a single unit of liquidity gives a percentage of 290,558/1e18, which is non-zero, and a decrease of zero — which is what the SDK's ZERO_LIQUIDITY invariant throws on. Guarding the percentage looks equivalent and is not.

Both are applied because they solve different halves. The 1e18 denominator keeps the withdrawal from silently under-delivering against the target (the caller would otherwise plan a second position it did not need); the truthful credit keeps the fill from being sized against tokens the pool will not pay.

## 2026-08-19 — The dispatch fee is a gate after the estimate, not a reserve in the leg loop

Chosen: `evaluateOrder` checks the fee token's post-fill residue against `dispatchFee + paymasterReserve` immediately after `estimateGasFillPost`, and skips the order when it falls short.

Alternatives considered: (a) reserving the dispatch fee in the leg loop alongside the paymaster reserve; (b) moving `estimateGasFillPost` above the leg loop so the figure is available there; (c) reserving a flat configured amount of the fee token.

Why not (a): the figure does not exist yet. `estimateGasFillPost` bumps `callGasLimit` by the funding prepends the leg loop produces and caches the result, so it cannot be priced before the loop that depends on it. (b) is the same problem stated as an ordering change — calling it early caches an estimate with no funding gas bump, and the later call returns that wrong cached value. (c) guesses at a number the estimator already knows exactly.

A gate rather than a resize because a cross-chain order cannot be partially filled (`partialEligibleCheap` requires `sourceChain === destChain`): the fill is all-or-nothing, so an unaffordable one is not ours to take. Same-chain orders dispatch nothing and are unaffected — `dispatchFee` is 0 and `buildApprovalAndFillCalldata` likewise only adds the fee token requirement when the chains differ.

The residue is read through `getAndCacheBalance`, which post-loop returns each output token's balance net of what the fill draws from it, and reads fresh for a fee token no leg paid out. The paymaster reserve is added to the requirement so the dispatch cannot be paid out of the gas headroom.

## 2026-08-19 — Both paymaster tokens carry the wallet reserve, rather than predicting which one is charged

Chosen: `paymasterReserveForToken` (exported from `src/services/paymaster`, called by `FXFiller`'s leg loop) reserves on the leg's output token whenever it is the chain's USDC or USDT and a paymaster is configured — without working out which of the two this UserOp will actually be charged in.

Alternatives considered: (a) replicating the selection ladder (Circle USDC, then Simplex USDC, then Simplex USDT, each gated on a >= 1 token balance) so only the winning token is reserved; (b) resolving the paymaster token once per order before the leg loop and threading it down; (c) returning a non-zero reserve from `UniswapV4FundingPlanner.walletReserveForToken`.

Why (a) loses: it is circular. `buildPaymasterAndData` picks the token at submit time from live balances, and the sizing being computed here is one of the inputs to that pick — a fill that draws USDC under one token makes `selectToken` fall through to USDT, so the prediction invalidates itself. It also duplicates the ladder outside the paymaster module, where a third token would silently not be covered. (b) is sound and would avoid the double-reserve, but it buys little: most chains configure one stablecoin, so the over-reservation is usually nothing and at worst a few dollars of held-back float, against a failure mode that reverts a five-figure fill. (c) was the original instinct and is the wrong layer twice over — `walletReserveForToken` is called for every output token, including exotics the paymaster never charges in, and the venue cannot resolve decimals for a token outside its own pools. Decisively, the UniswapV4 venue is only constructed when `vault.uniswapV4.positions` is non-empty, and the reserve loop only iterates `fundingVenues`: a filler with neither a vault nor V4 positions never runs that loop at all and has the identical bug, so a fix living in the venue would cover some configurations and silently miss others.

Where it lives is a separate question from where it is called. The call has to be unconditional in the leg loop, but the knowledge — which tokens are eligible, at what decimals — belongs beside `selectToken` in the paymaster module that already owns token selection. So `FXFiller` holds one call and no USDC/USDT/decimals detail, and adding a third paymaster token is a change in one file next to the selection ladder it has to stay consistent with.

Sizing: `PAYMASTER_RESERVE_TOKENS = 2n`, scaled per token by `getUsdcDecimals` / `getUsdtDecimals`. The observed prefund was under three cents and most of it came back in postOp, so this is deliberately headroom — for gas spikes, and for other UserOps in flight against the same balance between evaluation and execution. Note the reserve is a no-op for any fill the wallet comfortably covers; it only binds when the fill is balance-limited, which is exactly the partial-fill case that broke.

Not covered: `dispatchWithFeeToken` pulls `relayerFee` in the fee token from this same wallet. It was `0` on the order that failed (same-chain, no dispatch), but on a cross-chain fill it is dollars, not cents, and the reserve does not currently account for it.

## 2026-08-19 — Review round: shared bench state, Retry-After, and a warn on bench

Chosen, from PR review findings (Wizdave97): (a) bench state is a static map keyed by endpoint URL, not per-instance — the scanner and the confirmation poller construct separate QuorumPublicClients over the same URLs, so instance-local benches halved the relief and evaporated on `setRpcUrls` rebuilds; `clearAllSuspensions()` exists for tests. (b) The bench duration honours the provider's `Retry-After` (clamped to [1s, 5min]) — a per-second limiter should not cost five minutes of degraded quorum. (c) Benching warns through the caller's LoggerContext, naming the endpoint, the duration, and the new effective bar — the bar silently dropping from 2-of-2 to 1-of-1 was otherwise invisible unless a QuorumError happened to be thrown. (d) The all-benched fallback names itself in QuorumError messages instead of reporting `skipped: 0` as if nothing were wrong.

Also corrected from the same review: the rationale for the drop-the-bar reversal understated the scanner's failure mode — on a getLogs quorum failure the cursor does NOT advance, so the pre-drop behavior was stall-then-catch-up (lost timeliness), while the degraded path can advance the cursor on a 1-of-1 answer whose gaps are then permanent for every consumer. The trade stands as the maintainer made it, restated as latency-versus-silent-gap rather than liveness-versus-nothing. The confirmation-gate half of that trade was put to the maintainer explicitly (options: floor-of-2 voters for confirmations; exempting the gate at the full bar; or uniform degradation) and decided: degradation applies everywhere, the fill gate included. Accepted consequence, recorded so nobody rediscovers it as a bug: at n=2 with one endpoint benched, a single endpoint's receipt can vouch inclusion depth for the suspension window — fill liveness under provider throttling was judged worth more than the marginal reorg/fabricated-receipt protection, and the voters are still exclusively the operator's own endpoints.

## 2026-08-19 — REVERSED: a benched endpoint is dropped from the bar, not just the traffic

Chosen (maintainer decision, reversing the entry below): a rate-limited endpoint is excluded from the query set AND the quorum bar — each call's threshold is `quorumThreshold(endpoints actually queried)`. With every endpoint benched, all are queried again.

What it replaces: the first design kept the threshold over the full set and queried benched endpoints whenever the quorum was impossible without them, preserving the trust model exactly at the cost of availability — at n ≤ 3 a sustained 429 meant every call failed for the whole window.

Why the reversal is right: a throttled endpoint answers nothing either way, so keeping it in the denominator can only fail calls the remaining endpoints agree on — and a scanner that cannot form a quorum misses orders, which is a concrete revenue loss. The threat the fixed bar defended against (an attacker inducing 429s on public endpoints to lower the agreement bar) degrades 4-of-5 to 3-of-4 among endpoints the operator still chose — a marginal weakening against a speculative adversary, paid for with certain blindness under ordinary provider throttling. The voters are always exclusively the operator's own endpoints; the bar simply matches who was asked.

`threshold` (the public field) still reports the full-set bar — the scanner logs it — and the per-call bar appears in every QuorumError message.

## 2026-08-19 — Suspension has its own classifier, stricter than the diagnostic label

Chosen: `noteFailure` benches on `isSuspendableRateLimit` (HTTP 429, throttle-specific codes -32016/-32097, or throttle text in message/details/shortMessage), while the loose `isRateLimited` keeps labelling diagnostics. `-32005` alone never benches, and the suspension-path text match reads no metaMessages and has no bare-`429` pattern.

Alternative considered: one classifier for both, which is what the first cut shipped.

Why: `isRateLimited`'s breadth was designed for a role where a false positive cost a misleading log tag — its own removed comment said it "does not change control flow". Promoting it unchanged into an availability gate weaponised that breadth: EIP-1474 defines `-32005` as generic "limit exceeded", Infura returns it for eth_getLogs queries over its 10k result cap — a deterministic property of the query — and the scanner's 1000-block catch-up ranges hit that cap on busy chains, so the first cut would have benched a healthy endpoint for 5 minutes and re-benched it on every retry, leaving a 4-endpoint quorum at zero fault tolerance for the duration. Same logic for the free-text breadth: the request URL must not be read, or a key containing "ratelimit" benches its endpoint on any failure. Getting that right took two passes — the first skipped `metaMessages` but still read `message`, and viem FOLDS metaMessages (URL included) into `message`; caught in PR review with an end-to-end probe. `message` is now consulted only when the error carries no `metaMessages`; real viem errors put provider text in `details`/`shortMessage`, which are always read, and the probe is a permanent test. The label stays loose because mislabelling costs nothing; the bench is strict because benching costs quorum slack.

## 2026-08-19 — SUPERSEDED (see the reversal above): rate-limit suspension never shrinks the quorum, and yields when the quorum needs the benched endpoint

Chosen: a rate-limited endpoint is suspended for 5 minutes, but (a) the threshold stays `quorumThreshold(full set)` — suspension changes who is asked, never what is required — and (b) when the unsuspended endpoints alone cannot reach that threshold, suspended endpoints are queried anyway.

Alternatives considered: recomputing the threshold over the active set (a 5-endpoint operator would drop from 4-of-5 to 3-of-4 agreement — an attacker who can induce 429s on public endpoints, by hammering them independently, could lower the agreement bar without controlling any endpoint); hard suspension (honouring the bench even when it makes quorum impossible — for the common 2–3 endpoint sets, where the threshold is all of them, one 429 would turn into a guaranteed 5-minute total outage where today's behavior at least retries and fails per-call).

Why this shape: the class's trust model is that the operator provisioned n-way BFT; no availability optimisation may weaken it. The two rules keep both properties exactly: agreement requirements identical to the pre-suspension client in every case, and traffic to a throttled provider reduced precisely when the quorum can afford it (n ≥ 4). The 5-minute window is a constant, not config — no operator knob until someone actually needs one.

Also chosen: suspension is recorded in `settleUntilQuorum`'s rejection handler unconditionally, including stragglers settling after the call already decided early — a rate limit learned late still spares the endpoint on the next call.


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
