# 2026-09-03 — Installable offline UI and complete dashboard liquidity accounting

Made the Simplex UI installable as a desktop PWA with a manifest, branded FX icons, an offline app
shell, and a permanent install entry in both setup and the operator dashboard. The desktop-only
guide now walks through one generic three-step flow with visual examples; cancelled or unavailable
native prompts report through toasts rather than altering dialog layout.

Replaced the dashboard's wallet-only stablecoin total with an explicit liquidity model. The vault
planner now exposes a mutex-consistent read-only snapshot, and the balance API reports wallet,
vault position, vault availability, wallet reserve, total holdings, and actual available liquidity
per asset. Initial balances are loaded during startup instead of after a five-second timer, failed
reads are surfaced as partial/unavailable state rather than silently rendered as zero or a dash,
and the dashboard presents the full breakdown per network.

Files: `src/{core/boot,funding/types,funding/vault/{VaultFundingPlanner,VaultLiquidityState},index,services/BalanceProvider,services/server/{dto,static}}.ts`,
`src/tests/{balance-provider,funding/vault,ui-server}.test.ts`, `ui/{index.html,public,src}`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
