# 2026-09-03 — Explain vault balance controls and seed curated defaults

Added focusable info icons beside the sweep-threshold and minimum-wallet-balance labels using the
shared `@hyperbridge/ui` tooltip primitives. Newly selected Aave stataUSDC vaults now start at
`20`/`10`, and Yield Bearing cNGN starts at `1000`/`1`, matching the supplied reference; custom and
unknown vaults retain the generic fallback values.

Files: `ui/src/components/VaultRowsEditor.tsx`, `ui/src/styles/treasury.css`, `package.json`,
`../../pnpm-lock.yaml`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
