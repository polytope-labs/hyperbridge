# 2026-09-06 — The Simplex builder returns the balances it read instead of `null`

Chosen: when no configured fee token reaches one whole unit, `buildSimplexPaymasterData` returns
`{ insufficient: [{ symbol, balance, required }] }` — the reads `selectToken` already made, in
selection order — and `buildPaymasterAndData` formats them into the skip reason
(`simplex: solver USDC balance 0 < 1000000, USDT balance 0 < 1000000`), the form the Circle branch
already logs for its own balance gate.

Alternatives rejected:
- Re-reading the balances in `buildPaymasterAndData` when the builder returns `null` — two extra
  RPC reads on the path that has already lost sponsorship, a second read that can disagree with the
  one selection acted on, and a second place that can throw, turning an expected skip into the
  "builder failed" warn.
- Logging from inside the builder — it has no logger, and the caller assembles the skip reason so
  that one `type: "none"` result carries every candidate's reason; a log line in the provider would
  sit apart from that.
- Throwing a typed error for the no-balance case — the caller's catch exists to demote real
  failures (RPC errors, a reverted bootstrap approve) with a warn; a solver holding no stablecoin
  on a chain is the ordinary state of every chain it has no capital on, not a failure.
