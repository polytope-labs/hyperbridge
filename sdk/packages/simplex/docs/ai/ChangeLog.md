# ChangeLog

AI-maintained log of code changes in `sdk/packages/simplex`. Every AI-assisted change appends an
entry: date, what changed and why, and the files touched. This is not the release changelog —
published packages use changesets `CHANGELOG.md` files.

Newest entries first.

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
