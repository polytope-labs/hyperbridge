# Phantom price snapshot to pool rates (PhantomBidWindowExhausted)

Verified 2026-08-19 against live mainnet data.

1. `PhantomBidWindowExhausted` on Hyperbridge triggers `handlePhantomOrderPrices` (`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`). It loads the `PhantomOrderV2` and its registered `PhantomOrderLeg` rows, then calls `aggregatePhantomBids` from the SDK, which fetches every bid for the commitment, verifies each one (solver signature over the userOp hash plus an EIP-7702 delegation check), and reduces them per leg.

   A bid's `paymasterAndData` arrives in one of two shapes, and the SDK's `decodePhantomBidPaymasterAndData` reads both before the declaration is used: the bare declaration blob (every bid until simplex moved to Permit2), or the 234-byte EntryPoint v0.8 payload for the Simplex paymaster's PERMIT2 mode with the declaration appended after the permit (a bid built on simplex's real-bid path since #1223). A sponsored bid with nothing appended counts as having declared nothing — null accepted sources, no positions — the same as an empty field. The solver signature covers the whole field in both shapes, so `recoverBidSignerVm2` is unchanged; `phantom-decode.test.ts` checks the ethers digest over the long payload matches viem's.

   The chain ids in a declaration are decoded in the SDK without `TextDecoder`. That matters here specifically: the handler runs inside SubQuery's vm2 sandbox, where `TextDecoder` is not defined and the `util` fallback rejects a sandbox-created `Uint8Array`, so a decoder reaching for it threw inside the per-bid try/catch of `aggregatePhantomBids` — logged as "Failed to process bid for price snapshot", bid dropped, run continues. That was the whole failure behind bids with a source-chain declaration vanishing from the snapshots (verified 2026-09-09 against the live bids and the deployed indexer's data); `phantom-decode.sandbox.test.ts` runs the shipped bundle inside vm2 to keep it from coming back. A chain with a phantom order but no `solverAccount` in `config-mainnet.json` is skipped with "No SolverAccount configured for chain" — Gnosis (EVM-100) was, until its entry was added.

2. Per leg, a solver's quote is weighted by **its balance of that leg's OUTPUT token on the destination chain** — the inventory that actually backs the leg. Zero-weight quotes are dropped entirely, not down-weighted: they never reach the median, `bidCount`, or the bidder list. A leg where no bidder holds the output token is absent from the result, exactly as if nobody quoted it.

3. The leg's price is `weightedMedian` of the backed quotes — a **selection**, not a blend. It returns one bidder's exact integer, so a solver holding over half the leg's weight sets the published price verbatim, and the result can never be a value nobody quoted. `lowestPrice` and `highestPrice` are deliberately overwritten with the median so consumers cannot read an outlier bid as a tradeable bound.

4. `updateLiquidityPools` (`src/services/liquidityPool.service.ts`) turns those per-leg medians into pool rows. `resolvePoolLeg` maps a leg's tokens to a pool id and direction via the token registry, and the sample's rate is

   ```
   medianPrice * 10 ** (18 - outDecimals) * 10 ** inDecimals / standardAmount
   ```

   i.e. the quote renormalized from the probe size back to one whole input token. This holds for any standard amount the pallet configures; it collapses to `medianPrice * scale` when the probe is exactly one unit. Multiplications happen before the division, so only the last step truncates, by under one unit of 1e18 and downward.

5. Chain rows (`PoolChainLiquidity`, one per pool/chain/direction) are merged into the pool's single `sellRate`/`buyRate` by `weightedRate` — a depth-weighted **mean**, which unlike the median in step 3 does produce values no filler quoted. Samples older than `MAX_SAMPLE_AGE_BLOCKS` are excluded unless every sample is stale.

Precision note: a leg's quoted output integer *is* the price, to whatever resolution the output token's decimals allow. cNGN into 6-decimal USDC quotes ~715 base units, so the grid is 1/715 = 0.14% and the filler's floor rounding costs up to one full step. Chains whose output token has 18 decimals carry full precision on the same leg — which is why EVM-56 publishes `716845878136200` where Base publishes a bare `715`. The fix is a larger `standardAmount`, which step 4 now supports; see Decisions.md for why the filler's flooring must stay.
