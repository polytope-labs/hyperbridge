# 2026-09-20 — `IIntentGatewayV2`: `calculateCommitmentSlotHash` and `Cancelled` removed

`calculateCommitmentSlotHash(bytes32)` returned the storage key of `_filled[commitment]`. That key
was the cross-chain cancel proof until #980 and #1279 moved the proof to
`_partialFills[commitment][index]`, one key per order leg. The gateway now builds those keys itself
in `cancelFromSource`, so the public helper had no on-chain consumer left and is gone. There is no
public replacement. A caller that wants an order's fill status should read the `_filled(bytes32)`
getter, which returns the filler's address.

`error Cancelled()` was declared but never thrown, on any release. It leaves the interface and the
gateway's ABI.

Three callers in `@hyperbridge/sdk` still read the removed function and need a follow-up:
`OrderStatusChecker.isOrderFilled`, and `OrderCanceller`'s `quoteCancelFromSource` and
`fetchDestinationProof`. The last two were already building their proof over the `_filled` slot
while the dispatched GET asks for the `_partialFills` slots, so they need the per-leg keys rather
than a like-for-like swap.

`IIntentPriceOracle` keeps its declaration but no longer has an implementation in this repo: the
`VWAPOracle` that implemented it is deleted. It was never deployed, and the gateway has never read
`Params.priceOracle`. That field stays, because it occupies a storage slot the live proxies depend
on.

Files: `contracts/apps/IntentGatewayV2.sol`,
`docs/ai/changelog/2026-09-20-iintentgatewayv2-calculatecommitmentslothash-and-cancelled-removed.md`.
