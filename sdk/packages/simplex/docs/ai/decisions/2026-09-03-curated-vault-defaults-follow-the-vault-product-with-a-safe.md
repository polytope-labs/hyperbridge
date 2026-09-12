# 2026-09-03 — Curated vault defaults follow the vault product, with a safe generic fallback

Chosen: seed Aave stataUSDC with a `20` USD sweep threshold and `10` USD wallet floor, and Yield
Bearing cNGN with `1000` and `1`. The mapping is applied whenever a curated row is selected,
including Select all; custom and unrecognised catalog entries keep the existing `5000`/`3000`
defaults because the UI has no product-specific basis for choosing their liquidity profile.

Alternative rejected: replacing the single generic defaults globally would give one of the two
curated products the wrong values and would silently impose reference-specific settings on custom
vaults.
