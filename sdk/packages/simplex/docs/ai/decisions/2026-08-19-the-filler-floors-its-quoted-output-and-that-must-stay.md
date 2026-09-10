# 2026-08-19 — The filler floors its quoted output, and that must stay

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
