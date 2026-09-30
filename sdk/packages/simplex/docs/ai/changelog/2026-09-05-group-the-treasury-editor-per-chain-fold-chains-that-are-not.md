# 2026-09-05 — Group the treasury editor per chain, fold chains that are not enabled

Rebuilt `VaultRowsEditor` around per-chain groups: enabled chains first, each with a header (chain
logo, vault count, "Enabled" pill) and compact token-only rows; custom vaults live inside their
chain's group. Chains the filler does not run fold into one "Other networks" collapsible whose
groups carry a "Not enabled" pill and an "Enable chain" link (`onEnableChain`; the dashboard opens
the Chains & endpoints sheet, the wizard jumps to the Chains step through a new `goToStep` on
`StepProps`). The per-row disabled hint and the `disabledHint` prop are gone. The dashboard drawer
lost its duplicate heading; it now shows a summary line of connected vaults with Sweep now / Redeem
all beside it, and the restart caveat only when the filler booted without a vault venue. Layout
chosen from a design canvas of three options.
Files: `ui/src/components/VaultRowsEditor.tsx`, `ui/src/operator/Operations.tsx`,
`ui/src/wizard/{Wizard.tsx,steps/Treasury.tsx}`, `ui/src/styles/{treasury,responsive}.css`,
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
