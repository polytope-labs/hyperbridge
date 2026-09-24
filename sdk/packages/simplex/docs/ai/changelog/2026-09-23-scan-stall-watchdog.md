# 2026-09-23 — A stalled scan pass says so

A scan pass that outlives the quorum budgets now logs at `warn`, naming the phase
it is waiting in, and logs again when it finishes.

## Why a stall was invisible

`ChainScanner.start` ticks on an interval and skips the tick when the scan mutex
is held:

```ts
if (this.mutex.isLocked()) return
```

That is the right behaviour — passes must not overlap — but it is silent. A pass
that hangs produces no log line of its own, so the scanner just stops emitting
`Scanning blocks` and nothing says why. From outside, a chain stuck for an hour
and a chain with no blocks to scan look identical.

On a 24h mainnet run this happened on Base six times between 06:24 and 07:44,
for 4776s in total, the worst a single 2295s pass. The other four chains scanned
normally throughout and the process was healthy. Base logged nothing from the
scanner for the whole episode, and not one Base endpoint was benched while it was
stuck, so the pass was not visibly attempting anything.

## What the watchdog reports

A tick that finds the mutex held checks how long the pass has been running. Past
`STALL_THRESHOLD_MS` (45s) it logs, then repeats every `STALL_REPORT_INTERVAL_MS`
(60s) rather than on every tick. The threshold is set above the budgets: a pass
is capped at roughly 36s — one head read (1 attempt, 12s) plus one log read (2
attempts, 24s) — so anything past 45s is stuck somewhere the budgets do not
reach.

The line carries `phase`, `heldMs`, `cursor` and `rpcUrls`. The phase is the part
that matters, and it is the only reason the scanner tracks one:

| phase | the await it is on |
|---|---|
| `head` | `getBlockNumber` |
| `logs` | `getLogs` for the current range |
| `publish-orders` | delivering `OrderPlaced` to consumers |
| `publish-fills` | delivering `OrderFilled` / `PartialFill` to consumers |

A pass that was reported also logs when it ends, with its total duration. Without
that the log shows a chain going quiet and never shows it coming back.

## It watches, it does not intervene

The stuck pass is left alone. Aborting one would need a cancel path through
`QuorumPublicClient` that does not exist, and the cursor is safe regardless: a
pass that never completes never advances it, so the range is rescanned rather
than skipped. The 2295s Base stall resumed on a contiguous range.

Endpoint swaps also hold the mutex, and are not scan passes. `setRpcUrls` leaves
no in-flight pass registered, so the watchdog stays quiet for them.
