# 2026-09-03 — Clarify testnet terminology

Replaced the inaccurate “Sepolia-family” wording in the CLI initializer and web setup wizard with
“EVM test networks”. The supported testnet catalog also includes Polygon Amoy and BSC Chapel, which
are EVM-compatible but are not Sepolia-family chains.

Files: `src/cli/init/steps/chains.ts`, `ui/src/wizard/steps/Signer.tsx`,
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
