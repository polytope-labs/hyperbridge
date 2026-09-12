# 2026-09-03 — Seed a second stablecoin quote for CNGN

Chosen: the web setup wizard seeds USDC/CNGN first and USDT/CNGN second when both CNGN and USDT are
available. The shared draft factory accepts an optional quote symbol, while its existing USDC default
keeps newly created markets unchanged. If the catalog does not expose CNGN or USDT, setup keeps the
single-market fallback rather than creating a pair that cannot resolve on the selected chains.

Alternative rejected: seeding USDT against whichever non-stable token happens to sort first would make
the default vary by network and could silently produce an unintended market.
