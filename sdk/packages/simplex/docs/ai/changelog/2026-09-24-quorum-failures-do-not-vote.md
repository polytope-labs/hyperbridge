# 2026-09-24 — An endpoint that fails does not vote

A quorum read's bar is now the BFT threshold over the endpoints that answered it, not over every
endpoint asked. An endpoint that errors, or is still out at the deadline, does not vote.

## Why

A failed request says nothing about the chain. Counting it against the bar only failed reads that
the answering endpoints agreed on. BNB Chain runs six endpoints, which put the bar at five, so any
two failing at once failed the read and threw away four matching answers. On mainnet that happened
19 times in three hours: the scanner retried, and orders were seen a few seconds late.

The threshold exists to catch endpoints that answer wrongly, and every answer still counts. Six
endpoints with two failing now decide over the other four, three of which must agree.

## The rule

`settleUntilQuorum` computes the bar as `quorumThreshold(asked − failed)` and hands it to each
read. An endpoint still out counts as a voter, so an early decision never uses a lower bar than the
full response set would get. At the deadline the endpoints still out become failures, and the call
is decided over the endpoints that answered. A read fails only when those do not agree, or when
none answered.

`getBlockNumber` takes the bar-th highest head, so it now fails only when no endpoint answers.

The bench is unchanged: rate-limited, malformed and lagging endpoints are still not asked for a
while. `threshold` still reports the full-set bar.

## The trade

A read can be decided by fewer endpoints than the operator configured. At the limit, one endpoint
answering while the rest fail decides alone, as it already did when the rest were benched. The
voters are still only the operator's own endpoints.
