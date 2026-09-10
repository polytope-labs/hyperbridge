# 2026-09-10 — Balances narrows to one network, behind a switcher

"Balances by network" stacked every configured network's token grid down the Overview. That is fine
for one chain; a nine-chain mainnet config produced about fourteen cards down a long scroll, so
"do I have USDC on Arbitrum" was a hunt. The section is now "Balances" plus a network switcher in
its heading, and it renders one network's cards at a time.

Narrowing to one network must not lose the aggregate, so a totals strip sits under the heading: one
cell per tracked token with its available-to-fill total across every network, a share bar, and a
line saying how much of that sits on the selected network. Tokens keep config order rather than
being ranked by magnitude, and a fifth token is named in a "+1 more DAI" note rather than dropped
silently. Any token with an unread contributor reads `Unavailable` — the same refusal to estimate
`availableStablecoinLiquidity` already made for the headline metric, which now shares its helper.

The switcher's rows carry each network's chain logo, whether it is filling or observing, and its
available USDC+USDT; the trigger names the selected network over a plain "9 networks". No
per-network health signal is derived from the snapshot — a network whose read failed shows
`Unavailable` in the stables column, which is a fact the data already carries, and the
section-level "Some balances are unavailable" notice reports the failures.

The section moved to its own `OperatorBalances.tsx` with the card components; `AssetBalanceCard`
and its null rendering are unchanged. `AppSelect` grew optional `description`/`trailing` on an
option and `caption`/`header`/`contentClassName` on the select, so the switcher is the real Radix
component rather than a hand-rolled popover.

Files: `ui/src/operator/OperatorBalances.tsx`, `ui/src/operator/OperatorOverview.tsx`,
`ui/src/components/AppSelect.tsx`, `ui/src/lib/format.ts`,
`ui/src/styles/{operator,controls,responsive}.css`, and `package.json`.
