# 2026-09-09 — Prefer funded, Permit2-approved fee tokens before bootstrap (#1223)

Fee-token selection now scans USDC then USDT for both a balance of at least one whole
token and a Permit2 allowance covering the existing $5 recommendation, scaled by each
token's decimals. Previously, funded but unapproved USDC could block approved, funded
USDT: a solver without native lost sponsorship, while one with native sent an unnecessary
USDC approval. Selection is read-only and stops at the first ready token. If neither is
ready, the first balance-qualified token retains the existing bootstrap path. An approved
token with no balance is not eligible.

`buildSimplexPaymasterData` and `resolvePendingPermit2Approval` share this selection rule,
so delegation setup does not request a bootstrap when another funded token is already
ready. EIP-2612 bootstrap, native-funded approval, and zero-first reset behavior remain
unchanged when no ready token exists.

Nine regression cases cover the real `buildPaymasterAndData` path with and without native,
USDC preference when both tokens are ready, balance and allowance boundaries, different
token decimals, native and permit bootstrap fallback, and delegation approval resolution.
Four cases failed before the fix; all 102 tests across seven focused files passed after it.
Lint and whitespace checks passed. Package-wide typechecking still reports unrelated
dependency/configuration errors; no on-chain testing was performed for this fix.

Files: `src/services/paymaster/provider/simplex.ts`,
`src/tests/services/SimplexPaymaster.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
