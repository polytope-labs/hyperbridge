# 2026-09-08 — Polygon's confirmation ceiling drops from 32 blocks to 5

Seun reports Polygon finalizes in ~5s on average, so the built-in default's 32-block ceiling
(~64s at ~2s blocks) held $100k orders far longer than the chain needs. The max point is now 5
blocks, ~10s, which leaves headroom over the observed finality without the old minute-long wait.
The min point (2 blocks at $1k) is unchanged, and every value between the two still interpolates.
Only the built-in default moved; a user `[confirmationPolicies."137"]` entry still overrides it.
Files: src/config/interpolated-curve.ts, docs/content/developers/evm/simplex/confirmations.mdx.
