# 2026-09-03 — Prevent the Uniswap pricing view from crashing

Restored the missing Uniswap icon import used by the selected-position summary. Added UI typechecking
to the standard check command so unresolved runtime identifiers fail before the bundle reaches a
browser, and repaired two stale identifiers in the live chain editor uncovered by that check.

Files: `ui/src/wizard/steps/Strategies.tsx`, `ui/src/operator/{Chains.tsx,chains/useChainSettings.ts}`,
`package.json`, `docs/ai/ChangeLog.md`.
