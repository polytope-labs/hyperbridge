# 2026-09-03 — Remove obsolete BSC gas warnings from the Simplex UI

Removed the chain-card warning renderer from setup and operator network screens, deleted the obsolete
chain-note metadata, and removed the BNB-specific native-gas text from the setup review. BSC paymaster
support is now represented consistently throughout the Simplex UI; runtime paymaster behavior is
unchanged.

Files: `src/cli/init/chains.ts`, `ui/src/{operator/Chains.tsx,wizard/steps/{Chains,Review}.tsx}`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
