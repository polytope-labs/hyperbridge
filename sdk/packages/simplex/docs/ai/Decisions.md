# Decisions

AI-maintained record of non-obvious choices in `sdk/packages/simplex`: what was decided, what the
alternatives were, and why. Read this before changing related code so a later change does not
silently undo a deliberate trade-off. Newest first.

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
