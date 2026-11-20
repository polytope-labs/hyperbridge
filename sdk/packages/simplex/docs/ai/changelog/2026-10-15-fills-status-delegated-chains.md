# 2026-10-15 — Funding shortfall warning only before fills are enabled

On the dashboard Chains page, a running chain's funding warning ("Needs at least 5 USDC", the
yellow styling, and the "Check again" link) now shows only while the chain is not yet delegated.
A delegated chain reads "Fills enabled" in the normal muted style whatever its balance.

`ChainFunding.funded` and `ChainFunding.delegated` are independent reads: the paymaster draws the
fee token down as it fills, so a delegated chain routinely sits under `MIN_FEE_TOKEN_BALANCE`.
That is expected, not a shortfall the operator can act on — re-checking the balance there changed
nothing, since enabling fills is what the balance was for.
