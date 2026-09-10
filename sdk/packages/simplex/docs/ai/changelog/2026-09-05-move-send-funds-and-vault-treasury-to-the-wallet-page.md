# 2026-09-05 — Move Send funds and Vault treasury to the Wallet page

Split `Operations.tsx`: the Send sheet, the vault treasury sheet, their save/sweep logic and
helpers now live in `ui/src/operator/WalletTools.tsx`, rendered by `Wallet.tsx` above the
transaction history under a "Funds" heading; `OperationLink` moved to
`ui/src/components/OperationLink.tsx` so both pages share it. Operations keeps the allowlist and
chain sheets and accepts `initialPanel`/`onInitialPanelShown`, which `Operator` uses to open the
Chains sheet when the vault editor's Enable chain link is clicked from the Wallet page. Nav and
page copy updated (Wallet: "Funds and history"; Operations: "Live configuration").
Files: `ui/src/operator/{Operator,Operations,Wallet,WalletTools}.tsx`,
`ui/src/components/OperationLink.tsx`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
