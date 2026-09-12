# 2026-08-18 — Mode 2 gated on a `PERMIT2()` probe of the paymaster (#1071)

Chosen: `paymasterSupportsPermit2` reads the `PERMIT2()` constant that only the Permit2-capable implementation exposes; positive results are cached for the process lifetime, negative ones for five minutes. Alternative: rely on release ordering (client after all five redeploys) — fragile, since redeploys land chain by chain and a governance upgrade keeps the address, and a mode-2 op against an old deployment fails validation with `InvalidMode`, which for a bid means a lost fill.
