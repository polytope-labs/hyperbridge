# 2026-08-19 — The dispatch fee is a gate after the estimate, not a reserve in the leg loop

Chosen: `evaluateOrder` checks the fee token's post-fill residue against `dispatchFee + paymasterReserve` immediately after `estimateGasFillPost`, and skips the order when it falls short.

Alternatives considered: (a) reserving the dispatch fee in the leg loop alongside the paymaster reserve; (b) moving `estimateGasFillPost` above the leg loop so the figure is available there; (c) reserving a flat configured amount of the fee token.

Why not (a): the figure does not exist yet. `estimateGasFillPost` bumps `callGasLimit` by the funding prepends the leg loop produces and caches the result, so it cannot be priced before the loop that depends on it. (b) is the same problem stated as an ordering change — calling it early caches an estimate with no funding gas bump, and the later call returns that wrong cached value. (c) guesses at a number the estimator already knows exactly.

A gate rather than a resize because a cross-chain order cannot be partially filled (`partialEligibleCheap` requires `sourceChain === destChain`): the fill is all-or-nothing, so an unaffordable one is not ours to take. Same-chain orders dispatch nothing and are unaffected — `dispatchFee` is 0 and `buildApprovalAndFillCalldata` likewise only adds the fee token requirement when the chains differ.

The residue is read through `getAndCacheBalance`, which post-loop returns each output token's balance net of what the fill draws from it, and reads fresh for a fee token no leg paid out. The paymaster reserve is added to the requirement so the dispatch cannot be paid out of the gas headroom.
