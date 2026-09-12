# 2026-09-05 — Order history: amounts, tokens, chains, user, referrer, and links per order

The Activity page's order feed showed only a time, a truncated id and an event badge. Each activity
event now carries an `order` summary (`OrderSummary` in `src/data/types.ts`): user, source and
destination chain, placement tx hash, referrer (the order's graffiti tag, 20 bytes, null when
absent or equal to the placer, mirroring the indexer's rule), input and output legs (token address,
raw amount, symbol, decimals) and deadline. The scanner passes `graffiti` from the `OrderPlaced`
log through `ScannedOrder` and the monitor's `newOrder` event; `ActivityRecorder` builds the
summary on detection (token symbol from the asset registry, decimals via
`ContractInteractionService.getTokenDecimals`, both injected from `boot.ts` as `describeToken`),
caches it per order id, and attaches it to the order's later filled/executed/skipped rows. SQLite
gains an `order_json` column by migration; the memory store mirrors it. The UI groups events per
order and renders a HyperFX-style history table (referrer, status with detail, amount in/out with
token icon + chain badge, user, placed time and date, links to the HyperFX order page and to the
placement and fill transactions on the block explorers). Rebalance events list below the table.
Files: `src/scanner/{reconstruct,types}.ts`, `src/core/{event-monitor,boot}.ts`,
`src/data/{types,recorder,memory}.ts`, `src/data/sqlite/activity.ts`, `src/services/server/dto.ts`,
`ui/src/operator/{Activity,Operator}.tsx`, `ui/src/lib/format.ts`, `ui/src/types.ts`,
`ui/src/styles/operator.css`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
