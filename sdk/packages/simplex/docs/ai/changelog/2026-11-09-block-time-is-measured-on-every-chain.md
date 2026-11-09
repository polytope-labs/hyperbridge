# 2026-11-09 — Block time is measured on every chain, and the measurement is used

Base makes a block every 0.2s, but viem declares 2s for it. `ContractInteractionService.blockTimeSeconds`
discarded any measurement below half the declared figure, so Base bids were converted at 2s: the
330s window (`bidValiditySeconds` 300 plus the 30s discovery pad) became `validUntil = head + 165`,
which is 33s on Base. Bids expired before the placer's bundle landed. Commitment
`0x6d0e9d86…0dbf` (Ethereum → Base) was signed at block 55273890 with `validUntil` 55274055. It
landed at 55274060, one second late, and reverted `FillExpired()`. The solver paid the gas.

## Behavior

- `blockTimeSeconds` measures every configured chain at startup (`initCache`) over the last 1,000
  blocks, as before, and now uses that figure however far it is from the declared one. Base converts
  at 0.2s, so 330s is 1,650 blocks.
- The faster of the measured and declared figures is still used, so a stall in the sample never
  shortens a bid.
- The only plausibility check is a flat 0.1s floor on every chain. Below it the measurement is
  discarded for the declared figure, then 2s. The half-of-declared bound is gone.
- Measured figures are logged at info (`Measured the block time`, with `measured`, `declared`,
  `seconds`).
- The fx strategy's order-deadline timestamp estimate uses `blockTimeSeconds` instead of viem's
  `Chain.blockTime`.
- `BridgeTopUp` takes an optional `blockTimeSeconds` dependency (wired to the same service in
  `boot.ts`) and sizes its evidence margin with it. Without it, or when it fails, the margin uses the
  declared figure as before. On Base the margin is 1,500 blocks instead of 150.
- Other consumers of `blockTimeSeconds`, such as the order cache's confirmation wait, get the
  measured figure too.

Live block times on 2026-11-09 (1,000-block sample): Base 0.200s against 2s declared. Every other
mainnet was already within the old bound, so only Base changes: Ethereum 12.05s (12), BSC 0.45s
(0.75), Arbitrum 0.27s (0.25), Polygon 1.5s (2), Optimism 2.0s (2), Gnosis 5.1s (5), Soneium 2.0s
(2), Unichain 1.0s (1), Polkadot Hub 1.99s (none), Pharos 0.95s (none).

## Trade-off

The half-of-declared bound capped how far one endpoint serving bad startup timestamps could stretch
a bid, at twice its window. Now only the 0.1s floor caps it. A bid carrying the Simplex paymaster
is still bounded in wall-clock time by its Permit2 deadline (head timestamp + window + 120s); a bid
without one is not.

See `decisions/2026-08-27-bid-tenor-is-configured-in-seconds-and-written-in-blocks.md`.
