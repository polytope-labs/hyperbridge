# 2026-09-05 — A bid is not a fill: settle orders from the on-chain OrderFilled log

Under solver selection the filler emits `orderFilled` when Hyperbridge accepts its bid, so the
history called every bid "Filled" — including one a rival then filled. `orderFilled` now carries
`commitment`; the recorder ignores it for bids and records the accepted bid from `orderExecuted`
as a new `bid` activity type (txHash = extrinsic hash). `EventMonitor.handleFill` emits a new
`orderFillObserved` `{ commitment, filler, chainId, txHash, ours }` for every OrderFilled log on a
configured chain (`ScannedFill.transactionHash` added, from the log); the recorder records `filled`
when `ours`, and `lost` (reason = the winner's address) when not, but only for orders it has rows
for (`ActivityStore.knowsOrder`). Public `Simplex` events gain `order:fill-observed`. The UI status
now reads Filled > Lost ("filled by 0x…") > Bid placed / Bid retracted > Executed/Failed > Skipped >
Detected; the Bids cell shows the latest bid's standing (Accepted / Retracted / Failed, no counts)
with its extrinsic hash or error; the fill link uses the observed fill's tx hash (or a direct
attempt's UserOp hash, which the explorer's /tx page resolves) and never a bid's extrinsic hash.
Rows recorded before this change keep their bid-time "filled" rows.
Files: `src/scanner/{types,chain-scanner}.ts`, `src/core/{event-monitor,filler}.ts`,
`src/simplex.ts`, `src/data/{types,recorder,memory}.ts`, `src/data/sqlite/activity.ts`,
`src/services/server/dto.ts`, `src/tests/activity-recorder.test.ts`,
`ui/src/operator/Orders.tsx`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
