# Protocol fee refunds (#1260)

New orders retain the exact protocol fee charged at placement until settlement. The
committed inputs remain net of that fee; order encoding, SDK gross-up, and `order.fees`
(solver/relayer fees) are unchanged.

A cancellation returns `floor(placementFee * refundedPrincipal / originalNetInput)`
in addition to the refunded principal, whether cancellation is voluntary or after
expiry. The protocol earns the remainder. Full completion earns the whole fee;
partial fills leave it pending. The formula follows refunded input principal,
including existing release rounding, and never uses the current fee rate. For
fee-on-transfer tokens it uses the amount received at placement; outbound token
taxes can reduce the recipient's receipt.

Settlement occurs once in `_withdraw` when `finalize` is true, including final
messages with zero principal. A fully-filled source-cancellation proof is not a
finalizing withdrawal and leaves the fee pending for the completing redemption.
An authenticated partial-fill cancellation can arrive before solver redemptions:
it refunds only proven unfilled principal and its fee share, leaving earned solver
principal available for the delayed messages.

`ProtocolFeeRefunded(bytes32 indexed commitment, address indexed token, uint256 amount)`
reports each nonzero fee refund. `EscrowRefunded.tokens` remains principal-only.
Held placement fees no longer emit `DustCollected`; earned fees emit it at settlement.
Other dust sources keep their existing event timing. Indexer revenue continues to
sum `DustCollected`, so refunded fees are never counted as revenue.

The SDK and Simplex ABIs include the non-indexed `solver` address in
`EscrowReleased(bytes32 indexed commitment, address solver, TokenInfo[] tokens)`,
matching the contract so consumers can decode settlement logs.

The indexer stores `IOrderV3ProtocolFeeRefund` records with ID
`{transactionHash}.{logIndex}`, exposed through the order's `protocolFeeRefunds`
relation. Each record contains the order, chain, token (20-byte address), amount,
timestamp, block number, transaction hash, and creation time. Recording a fee refund
is idempotent and does not write the shared order row or change its status.

Per-order fee records append to the shared storage layout; `_filled` remains at
slot 2 and `_partialFills` at slot 11. Governance sizes dust sweeps from earned
`DustCollected` amounts less `DustSwept` amounts, accounting for sweeps already
in flight. The gateway does not enforce an escrow reservation; governance must
exclude principal and pending fees from sweep amounts.

`_protocolFees(commitment, token)` returns the held `amount` and original net
`committed` input; both are zero for legacy, zero-fee, and settled orders.

Existing orders have no fee record and retain their previous, nonrefundable fee
treatment. No historical-fee migration is possible because those fees were already
recognized as dust and may have been swept. Upgrade the implementation together with
its modules. A proxy already initialized at version 3 uses empty upgrade initialization
data; calling `migrate()` again would revert. The deployment helper reads the configured
proxy version before deploying: version 2 selects `migrate()`, version 3 selects empty
data, and unsupported versions fail. A fresh deployment initializes its new proxy
separately.
