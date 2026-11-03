CREATE EXTENSION IF NOT EXISTS btree_gist;

CREATE SCHEMA app;

CREATE TYPE app."13ba7d8a6a" AS ENUM (
    'DEPOSIT',
    'WITHDRAW',
    'TRANSFER_IN',
    'TRANSFER_OUT'
);

COMMENT ON TYPE app."13ba7d8a6a" IS '@enum
@enumName VaultLedgerEventType
 Capital entering or leaving an LP through ERC-4626 deposits/withdrawals or ordinary share transfers.
Mint/burn Transfer events are excluded because Deposit/Withdraw already account for them.';

CREATE TYPE app."167d3578e1" AS ENUM (
    'SOURCE',
    'HYPERBRIDGE_DELIVERED',
    'DESTINATION',
    'HYPERBRIDGE_TIMED_OUT',
    'TIMED_OUT'
);

COMMENT ON TYPE app."167d3578e1" IS '@enum
@enumName Status
 The status of a request';

CREATE TYPE app."1b9d21697d" AS ENUM (
    'CONSENSUS_REWARD',
    'MESSAGING_REWARD'
);

COMMENT ON TYPE app."1b9d21697d" IS '@enum
@enumName RelayerRewardType
 Types of relayer rewards';

CREATE TYPE app."1c84cbf86c" AS ENUM (
    'RELAYER',
    'FILLER',
    'USER',
    'REFERRER'
);

COMMENT ON TYPE app."1c84cbf86c" IS '@enum
@enumName ProtocolParticipantType
 Types of participants in the Hyperbridge Protocol';

CREATE TYPE app."69ec6d2f99" AS ENUM (
    'REWARD_POINTS_EARNED',
    'ORDER_PLACED_POINTS',
    'ORDER_FILLED_POINTS',
    'TOKEN_TELEPORTED_POINTS',
    'ORDER_REFERRED_POINTS'
);

COMMENT ON TYPE app."69ec6d2f99" IS '@enum
@enumName PointsActivityType
 System activity types related to reward points';

CREATE TYPE app."709fa19eb8" AS ENUM (
    'PENDING',
    'TRACKED'
);

COMMENT ON TYPE app."709fa19eb8" IS '@enum
@enumName TrackedSolverStatus';

CREATE TYPE app."869e90c211" AS ENUM (
    'PLACED',
    'FILLED',
    'REDEEMED',
    'REFUNDED',
    'CANCELLED'
);

COMMENT ON TYPE app."869e90c211" IS '@enum
@enumName OrderStatus
 The status of an order';

CREATE TYPE app.d75f09d1b9 AS ENUM (
    'FILL',
    'WATCHLIST'
);

COMMENT ON TYPE app.d75f09d1b9 IS '@enum
@enumName SolverDiscoveryTrigger';

CREATE TABLE app.accumulated_fees (
    id text NOT NULL,
    relayer text NOT NULL,
    chain_id text NOT NULL,
    lifetime_fees numeric NOT NULL,
    last_updated_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.accumulated_fees.id IS 'The unique ID, format: {relayerAddress}-{chainId}';

COMMENT ON COLUMN app.accumulated_fees.relayer IS 'The relayer''s address.';

COMMENT ON COLUMN app.accumulated_fees.chain_id IS 'A string identifier for the source chain of the fee.';

COMMENT ON COLUMN app.accumulated_fees.lifetime_fees IS 'The aggregated raw amount of fees, in the token''s smallest unit.';

COMMENT ON COLUMN app.accumulated_fees.last_updated_at IS 'The timestamp of the last update.';

CREATE TABLE app.bandwidth_app_daily_consumptions (
    id text NOT NULL,
    app_id text NOT NULL,
    date timestamp without time zone NOT NULL,
    bytes_consumed numeric NOT NULL
);

COMMENT ON COLUMN app.bandwidth_app_daily_consumptions.app_id IS 'Parent app entity.';

COMMENT ON COLUMN app.bandwidth_app_daily_consumptions.date IS 'UTC date bucket for this aggregate.';

COMMENT ON COLUMN app.bandwidth_app_daily_consumptions.bytes_consumed IS 'Sum of `BandwidthConsumed.bytes` events for this app on this date.';

CREATE TABLE app.bandwidth_app_tier_stats (
    id text NOT NULL,
    app_id text NOT NULL,
    tier integer NOT NULL,
    lifetime_subscriptions numeric NOT NULL,
    lifetime_bytes_credited numeric NOT NULL,
    lifetime_bytes_evicted numeric NOT NULL,
    active_subscriptions integer NOT NULL,
    last_purchased_at numeric NOT NULL
);

COMMENT ON COLUMN app.bandwidth_app_tier_stats.app_id IS 'Parent app entity.';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.tier IS 'Tier discriminant on the wire (1..4).';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.lifetime_subscriptions IS 'Total subscriptions of this tier ever bought (purchases + force credits).';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.lifetime_bytes_credited IS 'Total bytes credited via subscriptions of this tier.';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.lifetime_bytes_evicted IS 'Bytes lost to eviction via subscriptions of this tier.';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.active_subscriptions IS 'Running count of live subscriptions of this tier.';

COMMENT ON COLUMN app.bandwidth_app_tier_stats.last_purchased_at IS 'Block timestamp (ms) of the most recent purchase of this tier.';

CREATE TABLE app.bandwidth_apps (
    id text NOT NULL,
    chain text NOT NULL,
    app text NOT NULL,
    first_seen_at numeric NOT NULL,
    last_activity_at numeric NOT NULL,
    lifetime_subscriptions numeric NOT NULL,
    lifetime_bytes_credited numeric NOT NULL,
    lifetime_bytes_consumed numeric NOT NULL,
    lifetime_bytes_evicted numeric NOT NULL,
    active_subscriptions integer NOT NULL
);

COMMENT ON COLUMN app.bandwidth_apps.chain IS 'StateMachine string of the credit chain. Differs from the payer
chain on cross-chain sponsorship purchases.';

COMMENT ON COLUMN app.bandwidth_apps.app IS 'Lowercase hex of the `AppKey` bytes (no 0x prefix).';

COMMENT ON COLUMN app.bandwidth_apps.first_seen_at IS 'Block timestamp (ms) of the first purchase or force-credit that
created a row for this app.';

COMMENT ON COLUMN app.bandwidth_apps.last_activity_at IS 'Block timestamp (ms) of the most recent event that touched this app.';

COMMENT ON COLUMN app.bandwidth_apps.lifetime_subscriptions IS 'Total number of subscriptions ever created (purchases + force credits).';

COMMENT ON COLUMN app.bandwidth_apps.lifetime_bytes_credited IS 'Total bytes ever credited across all subscriptions.';

COMMENT ON COLUMN app.bandwidth_apps.lifetime_bytes_consumed IS 'Total bytes ever drained by the gate via `BandwidthConsumed` events.';

COMMENT ON COLUMN app.bandwidth_apps.lifetime_bytes_evicted IS 'Sum of `lost_bytes` across `SubscriptionEvicted` events — bandwidth
paid for but lost to the 1024-cap eviction.';

COMMENT ON COLUMN app.bandwidth_apps.active_subscriptions IS 'Running estimate of live subscriptions, derived from credit minus
eviction minus full-drain events. Best-effort; resync from chain
storage if exact count is needed.';

CREATE TABLE app.bandwidth_subscriptions (
    id text NOT NULL,
    app_id text NOT NULL,
    tier integer NOT NULL,
    bytes numeric NOT NULL,
    expires_at numeric NOT NULL,
    purchased_at numeric NOT NULL,
    paid_from text NOT NULL,
    forced boolean NOT NULL,
    evicted_at numeric,
    evicted_lost_bytes numeric,
    block_number numeric NOT NULL,
    extrinsic_hash text
);

COMMENT ON COLUMN app.bandwidth_subscriptions.app_id IS 'Parent app entity.';

COMMENT ON COLUMN app.bandwidth_subscriptions.tier IS 'Tier discriminant on the wire (1..4).';

COMMENT ON COLUMN app.bandwidth_subscriptions.bytes IS 'Bytes credited at purchase time. Scaled by months on bulk buys.';

COMMENT ON COLUMN app.bandwidth_subscriptions.expires_at IS 'Unix seconds (from the event payload) at which this sub expires.
Fixed at purchase time, never extends.';

COMMENT ON COLUMN app.bandwidth_subscriptions.purchased_at IS 'Block timestamp (ms) at which the subscription was created.';

COMMENT ON COLUMN app.bandwidth_subscriptions.paid_from IS 'StateMachine string of the chain that paid for this subscription.
Differs from `app.chain` on cross-chain sponsorship.';

COMMENT ON COLUMN app.bandwidth_subscriptions.forced IS 'True if the row was created by an admin `ForceCredited` event
rather than an end-user `BandwidthCredited` purchase.';

COMMENT ON COLUMN app.bandwidth_subscriptions.evicted_at IS 'Block timestamp (ms) at which the 1024-cap evicted this sub. Null
if still live (or naturally expired).';

COMMENT ON COLUMN app.bandwidth_subscriptions.evicted_lost_bytes IS '`lost_bytes` from the matching `SubscriptionEvicted` event. Null if
never evicted.';

COMMENT ON COLUMN app.bandwidth_subscriptions.block_number IS 'Hyperbridge block this sub was created in.';

COMMENT ON COLUMN app.bandwidth_subscriptions.extrinsic_hash IS 'Extrinsic hash of the originating ISMP message handler (or
`force_credit` call).';

CREATE TABLE app.bandwidth_tiers (
    id text NOT NULL,
    tier integer NOT NULL,
    bytes numeric,
    duration_secs numeric,
    is_active boolean NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.bandwidth_tiers.tier IS 'Tier discriminant on the wire (1..4).';

COMMENT ON COLUMN app.bandwidth_tiers.bytes IS 'Bytes per single-month purchase. Null when the tier is revoked.';

COMMENT ON COLUMN app.bandwidth_tiers.duration_secs IS 'Window length in seconds per single-month purchase. Null when revoked.';

COMMENT ON COLUMN app.bandwidth_tiers.is_active IS '`true` if the tier is currently configured (last `TierSet` was non-None).';

COMMENT ON COLUMN app.bandwidth_tiers.last_updated_at IS 'Block timestamp (ms) of the most recent `TierSet` event.';

CREATE TABLE app.bridge_token_supplies (
    id text NOT NULL,
    total_supply numeric NOT NULL,
    circulating_supply numeric NOT NULL,
    last_updated_at numeric NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.bridge_token_supplies.id IS 'Unique identifier for the token supply entry';

COMMENT ON COLUMN app.bridge_token_supplies.total_supply IS 'Total supply of the token';

COMMENT ON COLUMN app.bridge_token_supplies.circulating_supply IS 'Circulating supply of the token (total supply minus locked/staked amounts)';

COMMENT ON COLUMN app.bridge_token_supplies.last_updated_at IS 'Timestamp when this supply data was last updated';

COMMENT ON COLUMN app.bridge_token_supplies.created_at IS 'Record creation timestamp';

CREATE TABLE app.cumulative_dust_collected_per_chains (
    id text NOT NULL,
    chain text NOT NULL,
    amount_u_s_d numeric NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.cumulative_dust_collected_per_chains.id IS 'Unique identifier: chain';

COMMENT ON COLUMN app.cumulative_dust_collected_per_chains.chain IS 'Host state machine';

COMMENT ON COLUMN app.cumulative_dust_collected_per_chains.amount_u_s_d IS 'Cumulative dust collected in USD, scaled by 1e18 (18-decimal fixed-point integer)';

COMMENT ON COLUMN app.cumulative_dust_collected_per_chains.last_updated_at IS 'Last updated at';

CREATE TABLE app.cumulative_dust_swept_per_chains (
    id text NOT NULL,
    chain text NOT NULL,
    amount_u_s_d numeric NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.cumulative_dust_swept_per_chains.id IS 'Unique identifier: chain';

COMMENT ON COLUMN app.cumulative_dust_swept_per_chains.chain IS 'Host state machine';

COMMENT ON COLUMN app.cumulative_dust_swept_per_chains.amount_u_s_d IS 'Cumulative dust swept in USD, scaled by 1e18 (18-decimal fixed-point integer)';

COMMENT ON COLUMN app.cumulative_dust_swept_per_chains.last_updated_at IS 'Last updated at';

CREATE TABLE app.cumulative_intent_gateway_volume_u_s_ds (
    id text NOT NULL,
    chain text NOT NULL,
    volume_type text NOT NULL,
    volume_u_s_d numeric NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.cumulative_intent_gateway_volume_u_s_ds.id IS 'Composite identifier: {chain}-{volumeType}';

COMMENT ON COLUMN app.cumulative_intent_gateway_volume_u_s_ds.chain IS 'Host state machine';

COMMENT ON COLUMN app.cumulative_intent_gateway_volume_u_s_ds.volume_type IS 'PLACED (order inputs on the source chain) or FILLED (order outputs on the destination chain)';

COMMENT ON COLUMN app.cumulative_intent_gateway_volume_u_s_ds.volume_u_s_d IS 'Cumulative volume in USD, scaled by 1e18 (18-decimal fixed-point integer)';

COMMENT ON COLUMN app.cumulative_intent_gateway_volume_u_s_ds.last_updated_at IS 'Last updated at';

CREATE TABLE app.cumulative_volume_u_s_ds (
    id text NOT NULL,
    last_updated_at numeric NOT NULL,
    volume_u_s_d numeric NOT NULL
);

COMMENT ON COLUMN app.cumulative_volume_u_s_ds.id IS 'Unique identifier for the cumulative stats';

COMMENT ON COLUMN app.cumulative_volume_u_s_ds.last_updated_at IS 'Last updated at';

COMMENT ON COLUMN app.cumulative_volume_u_s_ds.volume_u_s_d IS 'Volume in USD, scaled by 1e18 (18-decimal fixed-point integer)';

CREATE TABLE app.daily_treasury_collator_rewards (
    id text NOT NULL,
    date timestamp without time zone NOT NULL,
    daily_reward_amount numeric NOT NULL
);

CREATE TABLE app.daily_treasury_relayer_rewards (
    id text NOT NULL,
    date timestamp without time zone NOT NULL,
    daily_reward_amount numeric NOT NULL
);

CREATE TABLE app.daily_volume_u_s_ds (
    id text NOT NULL,
    last_updated_at numeric NOT NULL,
    created_at timestamp without time zone NOT NULL,
    last24_hours_volume_u_s_d numeric NOT NULL
);

COMMENT ON COLUMN app.daily_volume_u_s_ds.id IS 'Unique identifier for the daily volume stats';

COMMENT ON COLUMN app.daily_volume_u_s_ds.last_updated_at IS 'Last updated at';

COMMENT ON COLUMN app.daily_volume_u_s_ds.created_at IS 'The timestamp when this record was created';

COMMENT ON COLUMN app.daily_volume_u_s_ds.last24_hours_volume_u_s_d IS 'Last 24hrs Volume in USD, scaled by 1e18 (18-decimal fixed-point integer)';

CREATE TABLE app.filler_bids (
    id text NOT NULL,
    commitment text NOT NULL,
    filler text NOT NULL,
    bid text,
    bid_data text,
    extrinsic_hash text,
    block_number numeric NOT NULL
);

COMMENT ON COLUMN app.filler_bids.id IS 'Composite identifier: {commitment}-{filler}-{blockNumber}-{eventIdx}. Unique per BidPlaced event,
so a filler''s repeat bids on the same commitment are kept as separate rows.';

COMMENT ON COLUMN app.filler_bids.commitment IS 'The order commitment bid on — keccak256(abi.encode(Order)).';

COMMENT ON COLUMN app.filler_bids.filler IS 'SS58 address of the filler that placed the bid.';

COMMENT ON COLUMN app.filler_bids.bid IS 'Which of the filler''s bids on the order this is (bytes32). A filler offering several prices on one
order places one bid per price; by convention the identifier is keccak256 of the UserOp''s callData.
Nullable so the column can be added in place; rows indexed before bids carried an identifier have none.';

COMMENT ON COLUMN app.filler_bids.bid_data IS 'The raw bid payload — the SCALE-encoded PackedUserOperation the filler submitted, as hex. Stored
undecoded so a bid stays fully re-interpretable as the fill ABI evolves, and because the pallet''s
own copy expires. Null when neither the extrinsic nor the RPC yielded it.';

COMMENT ON COLUMN app.filler_bids.extrinsic_hash IS 'Hash of the extrinsic that placed the bid, when the event is attributable to one.';

COMMENT ON COLUMN app.filler_bids.block_number IS 'Hyperbridge block number the bid was placed at.';

CREATE TABLE app.get_request_status_metadata (
    id text NOT NULL,
    status app."167d3578e1" NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL,
    request_id text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.get_request_status_metadata.id IS 'The ID of the GetRequestStatusMetadata';

COMMENT ON COLUMN app.get_request_status_metadata.status IS 'The status of the request';

COMMENT ON COLUMN app.get_request_status_metadata.chain IS 'The chain on which the request occured';

COMMENT ON COLUMN app.get_request_status_metadata."timestamp" IS 'The timestamp of the event';

COMMENT ON COLUMN app.get_request_status_metadata.block_number IS 'The number of the block in which the event occured';

COMMENT ON COLUMN app.get_request_status_metadata.block_hash IS 'The hash of the block in which the event occured';

COMMENT ON COLUMN app.get_request_status_metadata.transaction_hash IS 'The hash of the transaction in which the event occured';

COMMENT ON COLUMN app.get_request_status_metadata.request_id IS 'The request that owns this status change';

COMMENT ON COLUMN app.get_request_status_metadata.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.get_request_v2s (
    id text NOT NULL,
    source text NOT NULL,
    dest text NOT NULL,
    "from" text NOT NULL,
    keys jsonb NOT NULL,
    nonce numeric NOT NULL,
    height numeric NOT NULL,
    context text NOT NULL,
    timeout_timestamp numeric NOT NULL,
    fee numeric NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL,
    block_timestamp numeric NOT NULL,
    chain text NOT NULL,
    commitment text NOT NULL,
    created_at timestamp without time zone
);

COMMENT ON COLUMN app.get_request_v2s.id IS 'Unique identifier for the get request, computed as keccak256 hash of the encoded request parameters';

COMMENT ON COLUMN app.get_request_v2s.source IS 'Source chain identifier where the data is being requested from';

COMMENT ON COLUMN app.get_request_v2s.dest IS 'Destination chain identifier where the data will be sent to';

COMMENT ON COLUMN app.get_request_v2s."from" IS 'Address of the account that initiated the get request';

COMMENT ON COLUMN app.get_request_v2s.keys IS 'Array of keys that need to be fetched from the source chain';

COMMENT ON COLUMN app.get_request_v2s.nonce IS 'Sequential number to ensure uniqueness of the request';

COMMENT ON COLUMN app.get_request_v2s.height IS 'Block height at which the request was made';

COMMENT ON COLUMN app.get_request_v2s.context IS 'Additional context or metadata for the request';

COMMENT ON COLUMN app.get_request_v2s.timeout_timestamp IS 'Timestamp after which the request expires';

COMMENT ON COLUMN app.get_request_v2s.fee IS 'Fee paid for processing the request';

COMMENT ON COLUMN app.get_request_v2s.block_number IS 'Block number where the request was made';

COMMENT ON COLUMN app.get_request_v2s.block_hash IS 'Hash of the block containing the request';

COMMENT ON COLUMN app.get_request_v2s.transaction_hash IS 'Hash of the transaction that created the request';

COMMENT ON COLUMN app.get_request_v2s.block_timestamp IS 'Timestamp of the block containing the request';

COMMENT ON COLUMN app.get_request_v2s.chain IS 'Chain on which the request was made';

COMMENT ON COLUMN app.get_request_v2s.commitment IS 'commitment of the request';

COMMENT ON COLUMN app.get_request_v2s.created_at IS 'Creation timestamp of this request';

CREATE TABLE app.get_responses (
    id text NOT NULL,
    commitment text NOT NULL,
    chain text NOT NULL,
    response_message jsonb NOT NULL,
    response_timeout_timestamp numeric,
    request_id text,
    created_at timestamp without time zone NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL
);

COMMENT ON COLUMN app.get_responses.id IS 'Unique identifier of the GetResponse (This is the response commitment)';

COMMENT ON COLUMN app.get_responses.commitment IS 'The commitment of the response';

COMMENT ON COLUMN app.get_responses.chain IS 'The chain on which the response was made';

COMMENT ON COLUMN app.get_responses.response_message IS 'The key-values fetch from hyperbridge host';

COMMENT ON COLUMN app.get_responses.response_timeout_timestamp IS 'The Timestamp at which this response expires in seconds';

COMMENT ON COLUMN app.get_responses.request_id IS 'The request that triggered this response';

COMMENT ON COLUMN app.get_responses.created_at IS 'The timestamp when this record was created';

COMMENT ON COLUMN app.get_responses.block_number IS 'Block number where the response was made';

COMMENT ON COLUMN app.get_responses.block_hash IS 'Hash of the block containing the response';

COMMENT ON COLUMN app.get_responses.transaction_hash IS 'Hash of the transaction that created the response';

CREATE TABLE app.hyper_bridge_stats (
    id text NOT NULL,
    number_of_messages_sent numeric NOT NULL,
    number_of_delivered_messages numeric NOT NULL,
    number_of_failed_deliveries numeric NOT NULL,
    number_of_timed_out_messages numeric NOT NULL,
    total_transfers_in numeric NOT NULL,
    fees_payed_out_to_relayers numeric NOT NULL
);

COMMENT ON COLUMN app.hyper_bridge_stats.id IS 'The chain the metrics are for';

COMMENT ON COLUMN app.hyper_bridge_stats.number_of_messages_sent IS 'Total number of messages sent on Hyperbridge on this chain';

COMMENT ON COLUMN app.hyper_bridge_stats.number_of_delivered_messages IS 'Total number of messages delivered by Hyperbridge on this chain';

COMMENT ON COLUMN app.hyper_bridge_stats.number_of_failed_deliveries IS 'Total number of failed messages sent by Hyperbridge on this chain';

COMMENT ON COLUMN app.hyper_bridge_stats.number_of_timed_out_messages IS 'Total number of timed out messages on this chain';

COMMENT ON COLUMN app.hyper_bridge_stats.total_transfers_in IS 'The total amount of transfers sent into the Hyperbridge Host Contracts (relayer fees)';

COMMENT ON COLUMN app.hyper_bridge_stats.fees_payed_out_to_relayers IS 'Total amount of fees payed out by Hyperbridge to relayers on the chain (relayer fees)';

CREATE TABLE app.hyperbridge_collator_reward_transactions (
    id text NOT NULL,
    collator text NOT NULL,
    amount numeric NOT NULL,
    block_number numeric NOT NULL,
    block_timestamp numeric NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.id IS 'Unique identifier: {collatorAddress}-{blockNumber}';

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.collator IS 'The collator address that received the reward';

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.amount IS 'The amount of the reward';

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.block_number IS 'Block number where the reward was issued (also the block authored by this collator)';

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.block_timestamp IS 'Timestamp of the block';

COMMENT ON COLUMN app.hyperbridge_collator_reward_transactions.created_at IS 'Timestamp when this record was created';

CREATE TABLE app.hyperbridge_collator_rewards (
    id text NOT NULL,
    total_reward_amount numeric,
    total_blocks_authored numeric,
    last_rewarded_at timestamp without time zone
);

COMMENT ON COLUMN app.hyperbridge_collator_rewards.id IS 'Unique identifier for the collator reward entry (collator address)';

COMMENT ON COLUMN app.hyperbridge_collator_rewards.total_reward_amount IS 'The total amount of rewards this collator has ever received';

COMMENT ON COLUMN app.hyperbridge_collator_rewards.total_blocks_authored IS 'Total number of blocks authored by this collator';

COMMENT ON COLUMN app.hyperbridge_collator_rewards.last_rewarded_at IS 'Timestamp of the last reward received';

CREATE TABLE app.hyperbridge_relayer_rewards (
    id text NOT NULL,
    total_reward_amount numeric,
    total_messaging_reward_amount numeric,
    total_consensus_reward_amount numeric,
    reputation_asset_balance numeric
);

COMMENT ON COLUMN app.hyperbridge_relayer_rewards.id IS 'Unique identifier for the incentive entry, this is the relayer address';

COMMENT ON COLUMN app.hyperbridge_relayer_rewards.total_reward_amount IS 'The total amount of rewards this relayer has ever received.';

COMMENT ON COLUMN app.hyperbridge_relayer_rewards.total_messaging_reward_amount IS 'The total amount of messaging rewards this relayer has ever received.';

COMMENT ON COLUMN app.hyperbridge_relayer_rewards.total_consensus_reward_amount IS 'The total amount of consensus rewards this relayer has ever received.';

COMMENT ON COLUMN app.hyperbridge_relayer_rewards.reputation_asset_balance IS 'The balance of reputation assets accrued.';

CREATE TABLE app.i_order_v3_cancellations (
    id text NOT NULL,
    order_id text NOT NULL,
    chain text NOT NULL,
    canceller text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_cancellations.id IS 'Unique identifier for the cancellation event. Format: {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_cancellations.order_id IS 'The order whose cancellation was initiated';

COMMENT ON COLUMN app.i_order_v3_cancellations.chain IS 'The chain on which the cancellation was initiated. The source chain for a same-chain
cancel, the destination chain for the permissionless post-expiry route.';

COMMENT ON COLUMN app.i_order_v3_cancellations.canceller IS 'The account that initiated the cancellation. The destination-side route is
permissionless after expiry, so this is not necessarily the order''s creator.';

COMMENT ON COLUMN app.i_order_v3_cancellations."timestamp" IS 'The timestamp of the cancellation event';

COMMENT ON COLUMN app.i_order_v3_cancellations.block_number IS 'The number of the block in which the cancellation occurred';

COMMENT ON COLUMN app.i_order_v3_cancellations.transaction_hash IS 'The hash of the transaction in which the cancellation occurred';

COMMENT ON COLUMN app.i_order_v3_cancellations.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_escrow_refund_tokens (
    id text NOT NULL,
    refund_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_escrow_refund_tokens.id IS 'Unique identifier: {refundId}-token-{index}';

COMMENT ON COLUMN app.i_order_v3_escrow_refund_tokens.refund_id IS 'The refund event this token belongs to';

COMMENT ON COLUMN app.i_order_v3_escrow_refund_tokens.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_escrow_refund_tokens.amount IS 'Token amount refunded from escrow';

COMMENT ON COLUMN app.i_order_v3_escrow_refund_tokens.index IS 'Position index in the refund tokens array';

CREATE TABLE app.i_order_v3_escrow_refunds (
    id text NOT NULL,
    order_id text NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.id IS 'Unique identifier for the refund event. Format: {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.order_id IS 'The order whose escrow was refunded';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.chain IS 'The chain on which the refund occurred (source chain)';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds."timestamp" IS 'The timestamp of the refund event';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.block_number IS 'The number of the block in which the refund occurred';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.transaction_hash IS 'The hash of the transaction in which the refund occurred';

COMMENT ON COLUMN app.i_order_v3_escrow_refunds.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_escrow_release_tokens (
    id text NOT NULL,
    release_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_escrow_release_tokens.id IS 'Unique identifier: {releaseId}-token-{index}';

COMMENT ON COLUMN app.i_order_v3_escrow_release_tokens.release_id IS 'The release event this token belongs to';

COMMENT ON COLUMN app.i_order_v3_escrow_release_tokens.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_escrow_release_tokens.amount IS 'Token amount released from escrow';

COMMENT ON COLUMN app.i_order_v3_escrow_release_tokens.index IS 'Position index in the release tokens array';

CREATE TABLE app.i_order_v3_escrow_releases (
    id text NOT NULL,
    order_id text NOT NULL,
    solver text,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_escrow_releases.id IS 'Unique identifier for the release event. Format: {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.order_id IS 'The order whose escrow was released';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.solver IS 'The solver the escrow was released to. Null for events emitted before the
contract carried the solver in EscrowReleased.';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.chain IS 'The chain on which the release occurred (source chain)';

COMMENT ON COLUMN app.i_order_v3_escrow_releases."timestamp" IS 'The timestamp of the release event';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.block_number IS 'The number of the block in which the release occurred';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.transaction_hash IS 'The hash of the transaction in which the release occurred';

COMMENT ON COLUMN app.i_order_v3_escrow_releases.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_fill_input_assets (
    id text NOT NULL,
    fill_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_fill_input_assets.id IS 'Unique identifier: {fillId}-input-{index}';

COMMENT ON COLUMN app.i_order_v3_fill_input_assets.fill_id IS 'The fill this input asset belongs to';

COMMENT ON COLUMN app.i_order_v3_fill_input_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_fill_input_assets.amount IS 'Token amount';

COMMENT ON COLUMN app.i_order_v3_fill_input_assets.index IS 'Position index in the fill input assets array';

CREATE TABLE app.i_order_v3_fill_output_assets (
    id text NOT NULL,
    fill_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    amount_received numeric,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.id IS 'Unique identifier: {fillId}-output-{index}';

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.fill_id IS 'The fill this output asset belongs to';

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.amount IS 'Token amount delivered to the beneficiary';

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.amount_received IS 'Token amount actually received by the beneficiary (including any surplus share),
read from the fill transaction''s Transfer logs. Null when it cannot be derived
(e.g. native token outputs, which emit no Transfer log).';

COMMENT ON COLUMN app.i_order_v3_fill_output_assets.index IS 'Position index in the fill output assets array';

CREATE TABLE app.i_order_v3_fills (
    id text NOT NULL,
    order_id text NOT NULL,
    chain text NOT NULL,
    filler text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    user_op_hash text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_fills.id IS 'Unique identifier for the fill event. Format: {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_fills.order_id IS 'The order this fill belongs to';

COMMENT ON COLUMN app.i_order_v3_fills.chain IS 'The chain on which the fill occurred (destination chain for cross-chain orders)';

COMMENT ON COLUMN app.i_order_v3_fills.filler IS 'The address of the filler that executed this fill';

COMMENT ON COLUMN app.i_order_v3_fills."timestamp" IS 'The timestamp of the fill event';

COMMENT ON COLUMN app.i_order_v3_fills.block_number IS 'The number of the block in which the fill occurred';

COMMENT ON COLUMN app.i_order_v3_fills.transaction_hash IS 'The hash of the transaction in which the fill occurred';

COMMENT ON COLUMN app.i_order_v3_fills.user_op_hash IS 'Hash of the ERC-4337 user operation that executed this fill, when the filler
is a smart account (e.g. SolverAccount). Null for plain EOA fills.';

COMMENT ON COLUMN app.i_order_v3_fills.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_input_assets (
    id text NOT NULL,
    order_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL,
    released numeric
);

COMMENT ON COLUMN app.i_order_v3_input_assets.id IS 'Unique identifier: {orderId}-input-{index}';

COMMENT ON COLUMN app.i_order_v3_input_assets.order_id IS 'The order this asset belongs to';

COMMENT ON COLUMN app.i_order_v3_input_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_input_assets.amount IS 'Token amount';

COMMENT ON COLUMN app.i_order_v3_input_assets.index IS 'Position index in the input assets array';

COMMENT ON COLUMN app.i_order_v3_input_assets.released IS 'Cumulative amount of this escrowed input released to solvers across all
(partial) redeems. Equals `amount` once the order is fully redeemed.';

CREATE TABLE app.i_order_v3_output_assets (
    id text NOT NULL,
    order_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL,
    beneficiary text NOT NULL,
    filled numeric
);

COMMENT ON COLUMN app.i_order_v3_output_assets.id IS 'Unique identifier: {orderId}-output-{index}';

COMMENT ON COLUMN app.i_order_v3_output_assets.order_id IS 'The order this asset belongs to';

COMMENT ON COLUMN app.i_order_v3_output_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_output_assets.amount IS 'Token amount';

COMMENT ON COLUMN app.i_order_v3_output_assets.index IS 'Position index in the output assets array';

COMMENT ON COLUMN app.i_order_v3_output_assets.beneficiary IS 'Beneficiary address for this output';

COMMENT ON COLUMN app.i_order_v3_output_assets.filled IS 'Cumulative amount of this output filled across all (partial) fills.
Equals `amount` once the order is fully filled.';

CREATE TABLE app.i_order_v3_partial_fill_input_assets (
    id text NOT NULL,
    partial_fill_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_partial_fill_input_assets.id IS 'Unique identifier: {partialFillId}-input-{index}';

COMMENT ON COLUMN app.i_order_v3_partial_fill_input_assets.partial_fill_id IS 'The partial fill this input asset belongs to';

COMMENT ON COLUMN app.i_order_v3_partial_fill_input_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_partial_fill_input_assets.amount IS 'Token amount released from escrow';

COMMENT ON COLUMN app.i_order_v3_partial_fill_input_assets.index IS 'Position index in the partial fill input assets array';

CREATE TABLE app.i_order_v3_partial_fill_output_assets (
    id text NOT NULL,
    partial_fill_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    amount_received numeric,
    index integer NOT NULL,
    beneficiary text NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.id IS 'Unique identifier: {partialFillId}-output-{index}';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.partial_fill_id IS 'The partial fill this output asset belongs to';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.amount IS 'Token amount delivered in this partial fill';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.amount_received IS 'Token amount actually received by the beneficiary (including any surplus share),
read from the fill transaction''s Transfer logs. Null when it cannot be derived
(e.g. native token outputs, which emit no Transfer log).';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.index IS 'Position index in the partial fill output assets array';

COMMENT ON COLUMN app.i_order_v3_partial_fill_output_assets.beneficiary IS 'Beneficiary address for this output';

CREATE TABLE app.i_order_v3_partial_fills (
    id text NOT NULL,
    order_id text NOT NULL,
    chain text NOT NULL,
    filler text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    user_op_hash text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_partial_fills.id IS 'Unique identifier for the partial fill event. Recommended format: {commitment}.{sequence} or {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_partial_fills.order_id IS 'The order this partial fill belongs to';

COMMENT ON COLUMN app.i_order_v3_partial_fills.chain IS 'The chain on which the partial fill occurred';

COMMENT ON COLUMN app.i_order_v3_partial_fills.filler IS 'The address of the filler that executed this partial fill';

COMMENT ON COLUMN app.i_order_v3_partial_fills."timestamp" IS 'The timestamp of the partial fill event';

COMMENT ON COLUMN app.i_order_v3_partial_fills.block_number IS 'The number of the block in which the partial fill occurred';

COMMENT ON COLUMN app.i_order_v3_partial_fills.transaction_hash IS 'The hash of the transaction in which the partial fill occurred';

COMMENT ON COLUMN app.i_order_v3_partial_fills.user_op_hash IS 'Hash of the ERC-4337 user operation that executed this partial fill, when the filler
is a smart account (e.g. SolverAccount). Null for plain EOA fills.';

COMMENT ON COLUMN app.i_order_v3_partial_fills.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_predispatch_assets (
    id text NOT NULL,
    order_id text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    index integer NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_predispatch_assets.id IS 'Unique identifier: {orderId}-predispatch-{index}';

COMMENT ON COLUMN app.i_order_v3_predispatch_assets.order_id IS 'The order this asset belongs to';

COMMENT ON COLUMN app.i_order_v3_predispatch_assets.token IS 'Token address (bytes32 format)';

COMMENT ON COLUMN app.i_order_v3_predispatch_assets.amount IS 'Token amount';

COMMENT ON COLUMN app.i_order_v3_predispatch_assets.index IS 'Position index in the predispatch assets array';

CREATE TABLE app.i_order_v3_protocol_fee_refunds (
    id text NOT NULL,
    order_id text NOT NULL,
    chain text NOT NULL,
    token text NOT NULL,
    amount numeric NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.id IS 'Unique identifier for the refund event. Format: {transactionHash}.{logIndex}';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.order_id IS 'The order whose protocol fee was refunded';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.chain IS 'The chain on which the protocol fee refund occurred';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.token IS 'The 20-byte token address emitted by ProtocolFeeRefunded';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.amount IS 'Protocol fee amount refunded';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds."timestamp" IS 'The timestamp of the refund event';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.block_number IS 'The number of the block in which the refund occurred';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.transaction_hash IS 'The hash of the transaction in which the refund occurred';

COMMENT ON COLUMN app.i_order_v3_protocol_fee_refunds.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3_status_metadata (
    id text NOT NULL,
    status app."869e90c211" NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    transaction_hash text NOT NULL,
    order_id text,
    filler text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.i_order_v3_status_metadata.id IS 'The ID of the status metadata';

COMMENT ON COLUMN app.i_order_v3_status_metadata.status IS 'The status of the order';

COMMENT ON COLUMN app.i_order_v3_status_metadata.chain IS 'The chain on which the status change occurred';

COMMENT ON COLUMN app.i_order_v3_status_metadata."timestamp" IS 'The timestamp of the event';

COMMENT ON COLUMN app.i_order_v3_status_metadata.block_number IS 'The number of the block in which the event occurred';

COMMENT ON COLUMN app.i_order_v3_status_metadata.transaction_hash IS 'The hash of the transaction in which the event occurred';

COMMENT ON COLUMN app.i_order_v3_status_metadata.order_id IS 'The order that owns this status change';

COMMENT ON COLUMN app.i_order_v3_status_metadata.filler IS 'The filler of the order if the order has been filled';

COMMENT ON COLUMN app.i_order_v3_status_metadata.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.i_order_v3s (
    id text NOT NULL,
    "user" text NOT NULL,
    source_chain text NOT NULL,
    dest_chain text NOT NULL,
    commitment text NOT NULL,
    deadline numeric NOT NULL,
    nonce numeric NOT NULL,
    fees numeric NOT NULL,
    fee_token text,
    fee_token_decimals integer,
    session text,
    input_u_s_d numeric NOT NULL,
    status app."869e90c211" NOT NULL,
    predispatch_calldata text NOT NULL,
    post_dispatch_calldata text NOT NULL,
    referrer text,
    created_at timestamp without time zone NOT NULL,
    block_number numeric NOT NULL,
    block_timestamp numeric NOT NULL,
    transaction_hash text NOT NULL,
    user_op_hash text
);

COMMENT ON COLUMN app.i_order_v3s.id IS 'Unique identifier for the order (commitment)';

COMMENT ON COLUMN app.i_order_v3s."user" IS 'Address of the user who placed the order';

COMMENT ON COLUMN app.i_order_v3s.source_chain IS 'Source chain identifier where the order originates from';

COMMENT ON COLUMN app.i_order_v3s.dest_chain IS 'Destination chain identifier where the order will be executed';

COMMENT ON COLUMN app.i_order_v3s.commitment IS 'Unique commitment hash of the order used for verification';

COMMENT ON COLUMN app.i_order_v3s.deadline IS 'Timestamp after which the order expires';

COMMENT ON COLUMN app.i_order_v3s.nonce IS 'Sequential number to ensure uniqueness of the order';

COMMENT ON COLUMN app.i_order_v3s.fees IS 'Total fees supplied by the user for executing the order';

COMMENT ON COLUMN app.i_order_v3s.fee_token IS 'Address of the source chain host''s fee token at placement. Null for orders indexed before this field was added.';

COMMENT ON COLUMN app.i_order_v3s.fee_token_decimals IS 'Decimals of the fee token. Null for orders indexed before this field was added.';

COMMENT ON COLUMN app.i_order_v3s.session IS 'Optional session key used to select winning solver';

COMMENT ON COLUMN app.i_order_v3s.input_u_s_d IS 'Total input value in USD';

COMMENT ON COLUMN app.i_order_v3s.status IS 'Status of the order';

COMMENT ON COLUMN app.i_order_v3s.predispatch_calldata IS 'Pre dispatch calldata';

COMMENT ON COLUMN app.i_order_v3s.post_dispatch_calldata IS 'Post dispatch calldata';

COMMENT ON COLUMN app.i_order_v3s.referrer IS 'Referrer address that referred the user';

COMMENT ON COLUMN app.i_order_v3s.created_at IS 'Timestamp when the order was created';

COMMENT ON COLUMN app.i_order_v3s.block_number IS 'Block number where the order was placed';

COMMENT ON COLUMN app.i_order_v3s.block_timestamp IS 'Timestamp of the block where the order was placed';

COMMENT ON COLUMN app.i_order_v3s.transaction_hash IS 'Hash of the transaction that placed the order';

COMMENT ON COLUMN app.i_order_v3s.user_op_hash IS 'Hash of the ERC-4337 user operation that placed the order, when attributable from the receipt.
Null for direct transactions, unavailable receipts, and orders indexed before this field was added.';

CREATE TABLE app.intent_gateway_token_volumes (
    id text NOT NULL,
    chain text NOT NULL,
    token_address text NOT NULL,
    token_symbol text NOT NULL,
    decimals integer NOT NULL,
    volume_type text NOT NULL,
    amount numeric NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.intent_gateway_token_volumes.id IS 'Composite identifier: {chain}-{tokenAddress}-{volumeType}';

COMMENT ON COLUMN app.intent_gateway_token_volumes.chain IS 'Host state machine that emitted the event';

COMMENT ON COLUMN app.intent_gateway_token_volumes.token_address IS 'EVM address of the token, as a lowercase 0x-prefixed hex string';

COMMENT ON COLUMN app.intent_gateway_token_volumes.token_symbol IS 'Token symbol';

COMMENT ON COLUMN app.intent_gateway_token_volumes.decimals IS 'Token decimals';

COMMENT ON COLUMN app.intent_gateway_token_volumes.volume_type IS 'PLACED (order inputs on the source chain) or FILLED (order outputs on the destination chain)';

COMMENT ON COLUMN app.intent_gateway_token_volumes.amount IS 'Cumulative token amount in the token''s smallest unit';

COMMENT ON COLUMN app.intent_gateway_token_volumes.last_updated_at IS 'Last updated at';

CREATE TABLE app.pending_status_metadata (
    id text NOT NULL,
    commitment text NOT NULL,
    entity_type text NOT NULL,
    status text NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL,
    filler text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.pending_status_metadata.commitment IS 'The commitment of the parent entity';

COMMENT ON COLUMN app.pending_status_metadata.entity_type IS 'The type of parent entity: RequestV2, ResponseV2, or GetRequestV2';

COMMENT ON COLUMN app.pending_status_metadata.status IS 'The status of the request/response/order';

COMMENT ON COLUMN app.pending_status_metadata.chain IS 'The chain on which the event occurred';

COMMENT ON COLUMN app.pending_status_metadata."timestamp" IS 'The timestamp of the event';

COMMENT ON COLUMN app.pending_status_metadata.block_number IS 'The number of the block in which the event occurred';

COMMENT ON COLUMN app.pending_status_metadata.block_hash IS 'The hash of the block in which the event occurred';

COMMENT ON COLUMN app.pending_status_metadata.transaction_hash IS 'The hash of the transaction in which the event occurred';

COMMENT ON COLUMN app.pending_status_metadata.filler IS 'The filler of the order (only for order status metadata)';

COMMENT ON COLUMN app.pending_status_metadata.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.protocol_dust_collecteds (
    id text NOT NULL,
    chain text NOT NULL,
    token_symbol text NOT NULL,
    amount numeric NOT NULL,
    last_updated timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.protocol_dust_collecteds.id IS 'Unique identifier: chain-tokenAddress';

COMMENT ON COLUMN app.protocol_dust_collecteds.chain IS 'Host state machine that emitted the event';

COMMENT ON COLUMN app.protocol_dust_collecteds.token_symbol IS 'Token symbol';

COMMENT ON COLUMN app.protocol_dust_collecteds.amount IS 'Total amount of dust collected';

COMMENT ON COLUMN app.protocol_dust_collecteds.last_updated IS 'Timestamp when this record was last updated';

CREATE TABLE app.protocol_dust_swepts (
    id text NOT NULL,
    chain text NOT NULL,
    token_symbol text NOT NULL,
    amount numeric NOT NULL,
    last_updated timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.protocol_dust_swepts.id IS 'Unique identifier: chain-tokenAddress';

COMMENT ON COLUMN app.protocol_dust_swepts.chain IS 'Host state machine that emitted the event';

COMMENT ON COLUMN app.protocol_dust_swepts.token_symbol IS 'Token symbol';

COMMENT ON COLUMN app.protocol_dust_swepts.amount IS 'Total amount of dust swept';

COMMENT ON COLUMN app.protocol_dust_swepts.last_updated IS 'Timestamp when this record was last updated';

CREATE TABLE app.relayer_activities (
    id text NOT NULL,
    relayer_id text NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.relayer_activities.id IS 'Unique Identifier for the activity';

COMMENT ON COLUMN app.relayer_activities.relayer_id IS 'Relayer the activities are for';

COMMENT ON COLUMN app.relayer_activities.last_updated_at IS 'Last updated at';

CREATE TABLE app.relayer_reward_transactions (
    id text NOT NULL,
    relayer text NOT NULL,
    chain text NOT NULL,
    amount numeric NOT NULL,
    reward_type app."1b9d21697d" NOT NULL,
    block_number numeric NOT NULL,
    block_timestamp numeric NOT NULL,
    extrinsic_hash text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.relayer_reward_transactions.id IS 'Unique identifier: {relayerAddress}-{blockNumber}-{extrinsicIndex}';

COMMENT ON COLUMN app.relayer_reward_transactions.relayer IS 'The relayer address that received the reward';

COMMENT ON COLUMN app.relayer_reward_transactions.chain IS 'The chain on which the reward was issued';

COMMENT ON COLUMN app.relayer_reward_transactions.amount IS 'The amount of the reward';

COMMENT ON COLUMN app.relayer_reward_transactions.reward_type IS 'The type of reward (consensus, messaging, or fee)';

COMMENT ON COLUMN app.relayer_reward_transactions.block_number IS 'Block number where the reward was issued';

COMMENT ON COLUMN app.relayer_reward_transactions.block_timestamp IS 'Timestamp of the block';

COMMENT ON COLUMN app.relayer_reward_transactions.extrinsic_hash IS 'Hash of the extrinsic that triggered the reward';

COMMENT ON COLUMN app.relayer_reward_transactions.created_at IS 'Timestamp when this record was created';

CREATE TABLE app.relayer_stats_per_chain_v2s (
    id text NOT NULL,
    relayer_id text NOT NULL,
    chain text NOT NULL,
    number_of_successful_messages_delivered numeric NOT NULL,
    gas_used_for_successful_messages numeric NOT NULL,
    gas_fee_for_successful_messages numeric NOT NULL,
    usd_gas_fee_for_successful_messages numeric NOT NULL,
    fees_earned numeric NOT NULL,
    cumulative_withdrawn_amount numeric
);

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.id IS 'Unique identifier of the RelayerStatsPerChainV2 entry.';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.relayer_id IS 'The relayer the metrics are for';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.chain IS 'The chain the metrics are for';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.number_of_successful_messages_delivered IS 'Total number of successful post requests or post responses handled by the Relayer';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.gas_used_for_successful_messages IS 'Amount of gas spent on successful messages (Gas Cost)';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.gas_fee_for_successful_messages IS 'Gas fee spent on successful post requests (Gas Cost * Gas Price)';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.usd_gas_fee_for_successful_messages IS 'The USD value of the gas fee spent on successful post requests';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.fees_earned IS 'Total fees earned by the relayer';

COMMENT ON COLUMN app.relayer_stats_per_chain_v2s.cumulative_withdrawn_amount IS 'Cumulative amount withdrawn by the relayer on this chain';

CREATE TABLE app.relayer_v2s (
    id text NOT NULL
);

COMMENT ON COLUMN app.relayer_v2s.id IS 'Unique identifier of the Relayer (also the relayer''s address)';

CREATE TABLE app.request_status_metadata (
    id text NOT NULL,
    status app."167d3578e1" NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL,
    request_id text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.request_status_metadata.id IS 'The ID of the RequestStatusMetadata';

COMMENT ON COLUMN app.request_status_metadata.status IS 'The status of the request';

COMMENT ON COLUMN app.request_status_metadata.chain IS 'The chain on which the request occured';

COMMENT ON COLUMN app.request_status_metadata."timestamp" IS 'The timestamp of the event';

COMMENT ON COLUMN app.request_status_metadata.block_number IS 'The number of the block in which the event occured';

COMMENT ON COLUMN app.request_status_metadata.block_hash IS 'The hash of the block in which the event occured';

COMMENT ON COLUMN app.request_status_metadata.transaction_hash IS 'The hash of the transaction in which the event occured';

COMMENT ON COLUMN app.request_status_metadata.request_id IS 'The request that owns this status change';

COMMENT ON COLUMN app.request_status_metadata.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.request_v2s (
    id text NOT NULL,
    commitment text NOT NULL,
    chain text NOT NULL,
    source text,
    dest text,
    timeout_timestamp numeric,
    "from" text NOT NULL,
    "to" text NOT NULL,
    nonce numeric NOT NULL,
    body text NOT NULL,
    fee numeric NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.request_v2s.id IS 'Unique identifier of the Request (This is the request commitment)';

COMMENT ON COLUMN app.request_v2s.commitment IS 'The commitment of the request';

COMMENT ON COLUMN app.request_v2s.chain IS 'The chain on which the request was made';

COMMENT ON COLUMN app.request_v2s.source IS 'This is the statemachine id where this request originated from';

COMMENT ON COLUMN app.request_v2s.dest IS 'This is the statemachine where this request is been dispatched to';

COMMENT ON COLUMN app.request_v2s.timeout_timestamp IS 'This is the timestamp when the request would timeout';

COMMENT ON COLUMN app.request_v2s."from" IS 'The address that dispatched this request';

COMMENT ON COLUMN app.request_v2s."to" IS 'The address receiving this request';

COMMENT ON COLUMN app.request_v2s.nonce IS 'Nonce of the request';

COMMENT ON COLUMN app.request_v2s.body IS 'Body of the request';

COMMENT ON COLUMN app.request_v2s.fee IS 'This is the fee set for this request';

COMMENT ON COLUMN app.request_v2s.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.response_status_metadata (
    id text NOT NULL,
    status app."167d3578e1" NOT NULL,
    chain text NOT NULL,
    "timestamp" numeric NOT NULL,
    block_number text NOT NULL,
    block_hash text NOT NULL,
    transaction_hash text NOT NULL,
    response_id text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.response_status_metadata.id IS 'The ID of the ResponseStatusMetadata';

COMMENT ON COLUMN app.response_status_metadata.status IS 'The status of the response';

COMMENT ON COLUMN app.response_status_metadata.chain IS 'The chain on which the response occured';

COMMENT ON COLUMN app.response_status_metadata."timestamp" IS 'The timestamp of the event';

COMMENT ON COLUMN app.response_status_metadata.block_number IS 'The number of the block in which the event occured';

COMMENT ON COLUMN app.response_status_metadata.block_hash IS 'The hash of the block in which the event occured';

COMMENT ON COLUMN app.response_status_metadata.transaction_hash IS 'The hash of the transaction in which the event occured';

COMMENT ON COLUMN app.response_status_metadata.response_id IS 'The response that owns this status change';

COMMENT ON COLUMN app.response_status_metadata.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.response_v2s (
    id text NOT NULL,
    chain text NOT NULL,
    commitment text NOT NULL,
    response_message text,
    response_timeout_timestamp numeric,
    request_id text,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.response_v2s.id IS 'Unique identifier of the Response (This is the response commitment)';

COMMENT ON COLUMN app.response_v2s.chain IS 'The chain on which the response was made';

COMMENT ON COLUMN app.response_v2s.commitment IS 'The commitment of the response';

COMMENT ON COLUMN app.response_v2s.response_message IS 'The response message';

COMMENT ON COLUMN app.response_v2s.response_timeout_timestamp IS 'The Timestamp at which this response expires in seconds';

COMMENT ON COLUMN app.response_v2s.request_id IS 'The request that triggered this response';

COMMENT ON COLUMN app.response_v2s.created_at IS 'The timestamp when this record was created';

CREATE TABLE app.rewards (
    id text NOT NULL,
    address text NOT NULL,
    chain text NOT NULL,
    points numeric NOT NULL,
    earner_type app."1c84cbf86c"
);

CREATE TABLE app.rewards_activity_logs (
    id text NOT NULL,
    chain text NOT NULL,
    points numeric NOT NULL,
    transaction_hash text NOT NULL,
    earner_address text NOT NULL,
    earner_type app."1c84cbf86c",
    activity_type app."69ec6d2f99",
    description text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

CREATE TABLE app.solver_delegations (
    id text NOT NULL,
    chain text NOT NULL,
    solver text NOT NULL,
    delegated boolean NOT NULL,
    delegate text,
    block_number numeric NOT NULL,
    observed_at timestamp without time zone NOT NULL,
    refreshed_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.solver_delegations.id IS 'Composite identifier: {chain}-{solver}.';

COMMENT ON COLUMN app.solver_delegations.chain IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_delegations.solver IS 'Lowercased EVM address of the solver.';

COMMENT ON COLUMN app.solver_delegations.delegated IS 'True when the account''s code is exactly 0xef0100 ‖ addr, with addr a known SolverAccount.

Deliberately not indexed: in historical mode SubQuery rebuilds every index as GIST over
(fields…, _block_range), and btree_gist has no operator class for boolean, so an index here
makes the whole schema fail to create. Filtering on it still works — it is a scan within
whatever the (chain, solver) index has already narrowed.';

COMMENT ON COLUMN app.solver_delegations.delegate IS 'The address the account delegates to, known or not; null when the code is not a delegation.';

COMMENT ON COLUMN app.solver_delegations.block_number IS 'Block of the latest check.';

COMMENT ON COLUMN app.solver_delegations.observed_at IS 'Last time `delegated` or `delegate` changed.';

COMMENT ON COLUMN app.solver_delegations.refreshed_at IS 'Last time the code was checked.';

CREATE TABLE app.solver_discovery_requests (
    id text NOT NULL,
    chain text NOT NULL,
    solver text NOT NULL,
    version integer NOT NULL,
    block_number numeric NOT NULL,
    requested_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.solver_discovery_requests.id IS 'Composite identifier: {chain}-{solver}.';

COMMENT ON COLUMN app.solver_discovery_requests.chain IS 'State machine ID of the chain to track the solver on.';

COMMENT ON COLUMN app.solver_discovery_requests.solver IS 'Lowercased EVM address of the solver.';

COMMENT ON COLUMN app.solver_discovery_requests.version IS 'The SolverWatchlist version this request was queued in.';

COMMENT ON COLUMN app.solver_discovery_requests.block_number IS 'Hyperbridge block the request was queued at.';

COMMENT ON COLUMN app.solver_discovery_requests.requested_at IS 'Timestamp of that block.';

CREATE TABLE app.solver_inventory_heads (
    id text NOT NULL,
    block_number numeric NOT NULL,
    observed_at timestamp without time zone NOT NULL,
    refresh_offset integer NOT NULL
);

COMMENT ON COLUMN app.solver_inventory_heads.id IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_inventory_heads.block_number IS 'Block the chain''s inventory is current as of.';

COMMENT ON COLUMN app.solver_inventory_heads.observed_at IS 'Timestamp of that block.';

COMMENT ON COLUMN app.solver_inventory_heads.refresh_offset IS 'Offset into the chain''s TRACKED solvers where the next revaluation pass resumes.';

CREATE TABLE app.solver_vault_shares (
    id text NOT NULL,
    chain text NOT NULL,
    solver text NOT NULL,
    vault text NOT NULL,
    token_address text NOT NULL,
    shares numeric NOT NULL,
    assets numeric NOT NULL,
    block_number numeric NOT NULL,
    last_log_index integer NOT NULL,
    observed_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.solver_vault_shares.id IS 'Composite identifier: {chain}-{vault}-{solver}.';

COMMENT ON COLUMN app.solver_vault_shares.chain IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_vault_shares.solver IS 'Lowercased EVM address of the solver.';

COMMENT ON COLUMN app.solver_vault_shares.vault IS 'Lowercased EVM address of the ERC-4626 vault.';

COMMENT ON COLUMN app.solver_vault_shares.token_address IS 'Lowercased EVM address of the vault''s underlying token.';

COMMENT ON COLUMN app.solver_vault_shares.shares IS 'Vault shares held.';

COMMENT ON COLUMN app.solver_vault_shares.assets IS '`shares` valued via convertToAssets at the last movement or revaluation.';

COMMENT ON COLUMN app.solver_vault_shares.block_number IS 'Block of the last share Transfer applied, or of the latest storage read.';

COMMENT ON COLUMN app.solver_vault_shares.last_log_index IS 'Log index of that Transfer; the largest Int after a storage read.';

COMMENT ON COLUMN app.solver_vault_shares.observed_at IS 'Timestamp of the last change. Never moves backwards.';

CREATE TABLE app.solver_watchlist_cursors (
    id text NOT NULL,
    chain text NOT NULL,
    version integer NOT NULL
);

COMMENT ON COLUMN app.solver_watchlist_cursors.id IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_watchlist_cursors.chain IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_watchlist_cursors.version IS 'The newest SolverWatchlist version consumed.';

CREATE TABLE app.solver_watchlists (
    id text NOT NULL,
    chain text NOT NULL,
    version integer NOT NULL,
    request_count integer NOT NULL,
    block_number numeric NOT NULL,
    updated_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.solver_watchlists.id IS 'State machine ID of the chain the listed solvers trade on.';

COMMENT ON COLUMN app.solver_watchlists.chain IS 'State machine ID of the chain the listed solvers trade on.';

COMMENT ON COLUMN app.solver_watchlists.version IS 'Batch number of the newest SolverDiscoveryRequest rows for this chain.';

COMMENT ON COLUMN app.solver_watchlists.request_count IS 'Number of solvers ever requested for this chain.';

COMMENT ON COLUMN app.solver_watchlists.block_number IS 'Hyperbridge block the newest batch was queued at.';

COMMENT ON COLUMN app.solver_watchlists.updated_at IS 'Timestamp of that block.';

CREATE TABLE app.state_machine_update_events (
    id text NOT NULL,
    state_machine_id text NOT NULL,
    height integer NOT NULL,
    chain text NOT NULL,
    block_hash text NOT NULL,
    block_number integer NOT NULL,
    transaction_hash text NOT NULL,
    transaction_index integer NOT NULL,
    commitment_timestamp numeric NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.state_machine_update_events.id IS 'The ID of the StateMachineUpdate (hash of concatenation of stateMachineId and height)';

COMMENT ON COLUMN app.state_machine_update_events.state_machine_id IS 'The ID of the StateMachine';

COMMENT ON COLUMN app.state_machine_update_events.height IS 'The height of the block in which the event occured';

COMMENT ON COLUMN app.state_machine_update_events.chain IS 'The chain on which the event occured';

COMMENT ON COLUMN app.state_machine_update_events.block_hash IS 'The hash of the block in which the event occured';

COMMENT ON COLUMN app.state_machine_update_events.block_number IS 'The number of the block in which the event occured';

COMMENT ON COLUMN app.state_machine_update_events.transaction_hash IS 'The hash of the transaction in which the event occured';

COMMENT ON COLUMN app.state_machine_update_events.transaction_index IS 'The index of the transaction in the block';

COMMENT ON COLUMN app.state_machine_update_events.commitment_timestamp IS 'The state machine commitment timestamp';

COMMENT ON COLUMN app.state_machine_update_events.created_at IS 'The timestamp of the event';

CREATE TABLE app.token_price_logs (
    id text NOT NULL,
    symbol text NOT NULL,
    currency text NOT NULL,
    price text NOT NULL,
    provider text NOT NULL,
    "timestamp" numeric NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.token_price_logs.id IS 'Unique identifier: symbol + currency + provider + timestamp';

COMMENT ON COLUMN app.token_price_logs.symbol IS 'Token symbol (e.g., ETH, USDC, DOT)';

COMMENT ON COLUMN app.token_price_logs.currency IS 'Token price currency (e.g USD)';

COMMENT ON COLUMN app.token_price_logs.price IS 'Current price at the specified currency value';

COMMENT ON COLUMN app.token_price_logs.provider IS 'Price data provider (e.g., COINGECKO, CHAINLINK)';

COMMENT ON COLUMN app.token_price_logs."timestamp" IS 'Timestamp when price was recorded';

COMMENT ON COLUMN app.token_price_logs.created_at IS 'Record creation timestamp';

CREATE TABLE app.token_prices (
    id text NOT NULL,
    symbol text NOT NULL,
    address text,
    currency text NOT NULL,
    price text NOT NULL,
    last_updated_at numeric NOT NULL
);

COMMENT ON COLUMN app.token_prices.id IS 'Unique identifier: symbol + currency (e.g., ETH-USD)';

COMMENT ON COLUMN app.token_prices.symbol IS 'Token symbol (e.g., ETH, USDC, USDT, DOT)';

COMMENT ON COLUMN app.token_prices.address IS 'Token contract address (zero address for native tokens)';

COMMENT ON COLUMN app.token_prices.currency IS 'Token price currency (e.g USD)';

COMMENT ON COLUMN app.token_prices.price IS 'Current price at the specified currency value';

COMMENT ON COLUMN app.token_prices.last_updated_at IS 'Timestamp when price was last updated';

CREATE TABLE app.tracked_solvers (
    id text NOT NULL,
    chain text NOT NULL,
    solver text NOT NULL,
    status app."709fa19eb8" NOT NULL,
    discovered_by app.d75f09d1b9 NOT NULL,
    discovered_by_fill text,
    discovery_block numeric NOT NULL,
    discovered_at timestamp without time zone NOT NULL,
    genesis_block numeric,
    revalued_at timestamp without time zone,
    reconciled_at timestamp without time zone
);

COMMENT ON COLUMN app.tracked_solvers.id IS 'Composite identifier: {chain}-{solver}.';

COMMENT ON COLUMN app.tracked_solvers.chain IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.tracked_solvers.solver IS 'Lowercased EVM address of the solver.';

COMMENT ON COLUMN app.tracked_solvers.status IS 'PENDING until the genesis read succeeds.';

COMMENT ON COLUMN app.tracked_solvers.discovered_by IS 'How the solver came to be tracked.';

COMMENT ON COLUMN app.tracked_solvers.discovered_by_fill IS 'Transaction hash of the discovering fill; null when a watchlist request asked for it.';

COMMENT ON COLUMN app.tracked_solvers.discovery_block IS 'Block of `chain` the solver was discovered at.';

COMMENT ON COLUMN app.tracked_solvers.discovered_at IS 'Timestamp of that block.';

COMMENT ON COLUMN app.tracked_solvers.genesis_block IS 'Block of the genesis storage read; null while PENDING.';

COMMENT ON COLUMN app.tracked_solvers.revalued_at IS 'Last time the solver''s vault positions were revalued.';

COMMENT ON COLUMN app.tracked_solvers.reconciled_at IS 'Last time its balances were re-read and its delegation re-checked.';

CREATE TABLE app.transfers (
    id text NOT NULL,
    chain text NOT NULL,
    amount numeric NOT NULL,
    "from" text NOT NULL,
    "to" text NOT NULL
);

COMMENT ON COLUMN app.transfers.id IS 'Unique identifier of the Transfer';

COMMENT ON COLUMN app.transfers.chain IS 'The chain on which the transfer was made';

COMMENT ON COLUMN app.transfers.amount IS 'The amount transferred';

COMMENT ON COLUMN app.transfers."from" IS 'The address that made the transfer (the hyperbridge host address)';

COMMENT ON COLUMN app.transfers."to" IS 'The address that received the transfer (the relayer address)';

CREATE TABLE app.treasuries (
    id text NOT NULL,
    total_amount_transferred_in numeric NOT NULL,
    total_amount_transferred_out numeric NOT NULL,
    total_balance numeric NOT NULL,
    last_updated_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.treasuries.total_amount_transferred_in IS 'The total amount of tokens ever transferred INTO the treasury.';

COMMENT ON COLUMN app.treasuries.total_amount_transferred_out IS 'The total amount of tokens ever transferred OUT of the treasury.';

COMMENT ON COLUMN app.treasuries.total_balance IS 'The current calculated balance of the treasury (In - Out).';

COMMENT ON COLUMN app.treasuries.last_updated_at IS 'The timestamp of the last update.';

CREATE TABLE app.user_activity_v2s (
    id text NOT NULL,
    referrer text,
    total_orders_placed numeric NOT NULL,
    total_filled_orders numeric NOT NULL,
    total_order_placed_volume_u_s_d text NOT NULL,
    total_order_filled_volume_u_s_d text NOT NULL,
    created_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.user_activity_v2s.id IS 'Unique identifier for the user';

COMMENT ON COLUMN app.user_activity_v2s.referrer IS 'Referrer address';

COMMENT ON COLUMN app.user_activity_v2s.total_orders_placed IS 'Total orders placed by the user';

COMMENT ON COLUMN app.user_activity_v2s.total_filled_orders IS 'Filled orders that were placed by the user';

COMMENT ON COLUMN app.user_activity_v2s.total_order_placed_volume_u_s_d IS 'Total volume in USD for orders placed by the user';

COMMENT ON COLUMN app.user_activity_v2s.total_order_filled_volume_u_s_d IS 'Total volume in USD for filled orders that were placed by the user';

COMMENT ON COLUMN app.user_activity_v2s.created_at IS 'Timestamp when the user was created';

CREATE TABLE app.vault_ledger_events (
    id text NOT NULL,
    chain text NOT NULL,
    vault text NOT NULL,
    underlying_token text NOT NULL,
    lp text NOT NULL,
    caller text NOT NULL,
    receiver text,
    event_type app."13ba7d8a6a" NOT NULL,
    assets numeric NOT NULL,
    shares numeric NOT NULL,
    block_number numeric NOT NULL,
    transaction_hash text NOT NULL,
    "timestamp" timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.vault_ledger_events.id IS 'Composite identifier: {chain}-{transactionHash}-{logIndex} for deposits/withdrawals;
transfers append -{lp} because one log can affect two tracked owners.';

COMMENT ON COLUMN app.vault_ledger_events.chain IS 'State machine ID of the chain (e.g. EVM-8453) the vault is deployed on.';

COMMENT ON COLUMN app.vault_ledger_events.vault IS 'EVM address of the ERC-4626 vault, lowercased 0x-prefixed hex.';

COMMENT ON COLUMN app.vault_ledger_events.underlying_token IS 'EVM address of the vault''s underlying asset (the token whose principal is tracked), lowercased hex.';

COMMENT ON COLUMN app.vault_ledger_events.lp IS 'EVM address of the LP whose position changed — the `owner` of the shares (Deposit.owner /
Withdraw.owner), not necessarily the caller. This is the address principal is attributed to.';

COMMENT ON COLUMN app.vault_ledger_events.caller IS 'Deposit.sender / Withdraw.sender, or the previous share owner (Transfer.from).
For transfers this is not necessarily the transaction caller.';

COMMENT ON COLUMN app.vault_ledger_events.receiver IS 'For Withdraw events, the EVM address that received the withdrawn assets (Withdraw.receiver).
Equal to `lp` for a plain `redeem`/`withdraw`; null for Deposit events. Transfer.to for share transfers.';

COMMENT ON COLUMN app.vault_ledger_events.event_type IS 'Whether capital entered/exited through a deposit/withdrawal or a share transfer.';

COMMENT ON COLUMN app.vault_ledger_events.assets IS 'Underlying asset amount in its smallest unit: emitted assets for Deposit/Withdraw,
or the transferred shares valued by convertToAssets at the event block.';

COMMENT ON COLUMN app.vault_ledger_events.shares IS 'Amount of vault shares minted, burned or transferred, in the share token''s smallest unit.';

COMMENT ON COLUMN app.vault_ledger_events.block_number IS 'Block number the event was emitted in.';

COMMENT ON COLUMN app.vault_ledger_events.transaction_hash IS 'Hash of the transaction the event was emitted in.';

COMMENT ON COLUMN app.vault_ledger_events."timestamp" IS 'Timestamp of the block the event was emitted in.';

CREATE TABLE app.vault_lp_positions (
    id text NOT NULL,
    chain text NOT NULL,
    vault text NOT NULL,
    underlying_token text NOT NULL,
    lp text NOT NULL,
    shares numeric NOT NULL,
    opening_shares numeric,
    opening_principal numeric,
    opening_block numeric,
    total_assets_transferred_in numeric,
    total_assets_transferred_out numeric,
    total_assets_deposited numeric NOT NULL,
    total_assets_withdrawn numeric NOT NULL,
    deposit_count integer NOT NULL,
    withdraw_count integer NOT NULL,
    created_at timestamp without time zone NOT NULL,
    last_updated_at timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.vault_lp_positions.id IS 'Composite identifier: {chain}-{vault}-{lp}.';

COMMENT ON COLUMN app.vault_lp_positions.chain IS 'State machine ID of the chain (e.g. EVM-8453) the vault is deployed on.';

COMMENT ON COLUMN app.vault_lp_positions.vault IS 'EVM address of the ERC-4626 vault, lowercased 0x-prefixed hex.';

COMMENT ON COLUMN app.vault_lp_positions.underlying_token IS 'EVM address of the vault''s underlying asset, lowercased hex.';

COMMENT ON COLUMN app.vault_lp_positions.lp IS 'EVM address of the LP this position belongs to (the share owner), lowercased hex.';

COMMENT ON COLUMN app.vault_lp_positions.shares IS 'Vault shares accounted from the opening balance, deposits, withdrawals and ordinary transfers.
Snapshots are withheld when this differs from the onchain balance.';

COMMENT ON COLUMN app.vault_lp_positions.opening_shares IS 'Shares already held before the first tracked event. Null on positions created by older indexers.';

COMMENT ON COLUMN app.vault_lp_positions.opening_principal IS 'Underlying value of openingShares at openingBlock. Yield before this baseline is not attributed
to this LP. Null on legacy positions means no opening principal was recorded.';

COMMENT ON COLUMN app.vault_lp_positions.opening_block IS 'Block of the first tracked event, whose exchange rate values the opening shares.';

COMMENT ON COLUMN app.vault_lp_positions.total_assets_transferred_in IS 'Cumulative underlying value of ordinary incoming share transfers at their event blocks.
Null on legacy positions is treated as zero.';

COMMENT ON COLUMN app.vault_lp_positions.total_assets_transferred_out IS 'Cumulative underlying value of ordinary outgoing share transfers at their event blocks.
Null on legacy positions is treated as zero.';

COMMENT ON COLUMN app.vault_lp_positions.total_assets_deposited IS 'Cumulative underlying assets the LP has ever deposited, smallest unit. Monotonically increasing.';

COMMENT ON COLUMN app.vault_lp_positions.total_assets_withdrawn IS 'Cumulative underlying assets the LP has ever withdrawn/redeemed, smallest unit. Monotonically increasing.
Net principal also includes openingPrincipal and transferred-in minus transferred-out asset values.';

COMMENT ON COLUMN app.vault_lp_positions.deposit_count IS 'Number of Deposit events recorded for this LP on this vault.';

COMMENT ON COLUMN app.vault_lp_positions.withdraw_count IS 'Number of Withdraw events recorded for this LP on this vault.';

COMMENT ON COLUMN app.vault_lp_positions.created_at IS 'Timestamp of the first ledger event that created this position.';

COMMENT ON COLUMN app.vault_lp_positions.last_updated_at IS 'Timestamp of the most recent ledger event applied to this position.';

CREATE TABLE app.vault_position_snapshots (
    id text NOT NULL,
    chain text NOT NULL,
    vault text NOT NULL,
    underlying_token text NOT NULL,
    lp text NOT NULL,
    day_start_timestamp numeric NOT NULL,
    shares numeric NOT NULL,
    asset_value numeric NOT NULL,
    net_principal numeric NOT NULL,
    yield_earned numeric NOT NULL,
    block_number numeric NOT NULL,
    snapshot_time timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.vault_position_snapshots.id IS 'Composite identifier: {chain}-{vault}-{lp}-{dayStartTimestamp}, where dayStartTimestamp is the
UNIX timestamp (seconds) of 00:00 UTC for the snapshot''s day.';

COMMENT ON COLUMN app.vault_position_snapshots.chain IS 'State machine ID of the chain (e.g. EVM-8453) the vault is deployed on.';

COMMENT ON COLUMN app.vault_position_snapshots.vault IS 'EVM address of the ERC-4626 vault, lowercased 0x-prefixed hex.';

COMMENT ON COLUMN app.vault_position_snapshots.underlying_token IS 'EVM address of the vault''s underlying asset, lowercased hex.';

COMMENT ON COLUMN app.vault_position_snapshots.lp IS 'EVM address of the LP this snapshot is for, lowercased hex.';

COMMENT ON COLUMN app.vault_position_snapshots.day_start_timestamp IS 'UNIX timestamp (seconds) of 00:00 UTC for the day this snapshot represents. One snapshot per day bucket.';

COMMENT ON COLUMN app.vault_position_snapshots.shares IS 'The LP''s live vault share balance at snapshot time, read via `balanceOf`. Smallest unit of the share token.';

COMMENT ON COLUMN app.vault_position_snapshots.asset_value IS 'Underlying-asset value of `shares` at snapshot time, read via `convertToAssets(shares)`. Smallest unit.';

COMMENT ON COLUMN app.vault_position_snapshots.net_principal IS 'The LP''s net capital at snapshot time: openingPrincipal + totalAssetsDeposited +
totalAssetsTransferredIn - totalAssetsWithdrawn - totalAssetsTransferredOut.';

COMMENT ON COLUMN app.vault_position_snapshots.yield_earned IS 'Yield earned on the position so far: assetValue - netPrincipal. May be negative if the vault lost value.';

COMMENT ON COLUMN app.vault_position_snapshots.block_number IS 'Block number the snapshot reads were taken against.';

COMMENT ON COLUMN app.vault_position_snapshots.snapshot_time IS 'Timestamp of the block the snapshot was taken at.';

CREATE TABLE app.vault_snapshots (
    id text NOT NULL,
    chain text NOT NULL,
    vault text NOT NULL,
    underlying_token text NOT NULL,
    day_start_timestamp numeric NOT NULL,
    total_assets numeric NOT NULL,
    total_shares numeric NOT NULL,
    assets_per_share numeric NOT NULL,
    block_number numeric NOT NULL,
    snapshot_time timestamp without time zone NOT NULL
);

COMMENT ON COLUMN app.vault_snapshots.id IS 'Composite identifier: {chain}-{vault}-{dayStartTimestamp} (dayStartTimestamp is 00:00 UTC, seconds).';

COMMENT ON COLUMN app.vault_snapshots.chain IS 'State machine ID of the chain (e.g. EVM-8453) the vault is deployed on.';

COMMENT ON COLUMN app.vault_snapshots.vault IS 'EVM address of the ERC-4626 vault, lowercased 0x-prefixed hex.';

COMMENT ON COLUMN app.vault_snapshots.underlying_token IS 'EVM address of the vault''s underlying asset, lowercased hex.';

COMMENT ON COLUMN app.vault_snapshots.day_start_timestamp IS 'UNIX timestamp (seconds) of 00:00 UTC for the day this snapshot represents.';

COMMENT ON COLUMN app.vault_snapshots.total_assets IS 'Total underlying assets backing the vault at snapshot time, read via `totalAssets()`. Smallest unit.
Note: for vest-based vaults this is net of any not-yet-vested yield, matching the value LPs can redeem.';

COMMENT ON COLUMN app.vault_snapshots.total_shares IS 'Total vault shares outstanding at snapshot time, read via `totalSupply()`. Smallest unit of the share token.';

COMMENT ON COLUMN app.vault_snapshots.assets_per_share IS 'Assets redeemable for a single whole share (10^shareDecimals), read via `convertToAssets`. Smallest unit
of the underlying. A normalized price-per-share independent of totalShares for charting the exchange rate.';

COMMENT ON COLUMN app.vault_snapshots.block_number IS 'Block number the snapshot reads were taken against.';

COMMENT ON COLUMN app.vault_snapshots.snapshot_time IS 'Timestamp of the block the snapshot was taken at.';

ALTER TABLE ONLY app.accumulated_fees
    ADD CONSTRAINT accumulated_fees_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bandwidth_app_daily_consumptions
    ADD CONSTRAINT bandwidth_app_daily_consumptions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bandwidth_app_tier_stats
    ADD CONSTRAINT bandwidth_app_tier_stats_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bandwidth_apps
    ADD CONSTRAINT bandwidth_apps_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bandwidth_subscriptions
    ADD CONSTRAINT bandwidth_subscriptions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bandwidth_tiers
    ADD CONSTRAINT bandwidth_tiers_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.bridge_token_supplies
    ADD CONSTRAINT bridge_token_supplies_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.cumulative_dust_collected_per_chains
    ADD CONSTRAINT cumulative_dust_collected_per_chains_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.cumulative_dust_swept_per_chains
    ADD CONSTRAINT cumulative_dust_swept_per_chains_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.cumulative_intent_gateway_volume_u_s_ds
    ADD CONSTRAINT cumulative_intent_gateway_volume_u_s_ds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.cumulative_volume_u_s_ds
    ADD CONSTRAINT cumulative_volume_u_s_ds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.daily_treasury_collator_rewards
    ADD CONSTRAINT daily_treasury_collator_rewards_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.daily_treasury_relayer_rewards
    ADD CONSTRAINT daily_treasury_relayer_rewards_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.daily_volume_u_s_ds
    ADD CONSTRAINT daily_volume_u_s_ds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.filler_bids
    ADD CONSTRAINT filler_bids_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.get_request_status_metadata
    ADD CONSTRAINT get_request_status_metadata_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.get_request_v2s
    ADD CONSTRAINT get_request_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.get_responses
    ADD CONSTRAINT get_responses_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.hyper_bridge_stats
    ADD CONSTRAINT hyper_bridge_stats_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.hyperbridge_collator_reward_transactions
    ADD CONSTRAINT hyperbridge_collator_reward_transactions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.hyperbridge_collator_rewards
    ADD CONSTRAINT hyperbridge_collator_rewards_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.hyperbridge_relayer_rewards
    ADD CONSTRAINT hyperbridge_relayer_rewards_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_cancellations
    ADD CONSTRAINT i_order_v3_cancellations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_escrow_refund_tokens
    ADD CONSTRAINT i_order_v3_escrow_refund_tokens_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_escrow_refunds
    ADD CONSTRAINT i_order_v3_escrow_refunds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_escrow_release_tokens
    ADD CONSTRAINT i_order_v3_escrow_release_tokens_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_escrow_releases
    ADD CONSTRAINT i_order_v3_escrow_releases_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_fill_input_assets
    ADD CONSTRAINT i_order_v3_fill_input_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_fill_output_assets
    ADD CONSTRAINT i_order_v3_fill_output_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_fills
    ADD CONSTRAINT i_order_v3_fills_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_input_assets
    ADD CONSTRAINT i_order_v3_input_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_output_assets
    ADD CONSTRAINT i_order_v3_output_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_partial_fill_input_assets
    ADD CONSTRAINT i_order_v3_partial_fill_input_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_partial_fill_output_assets
    ADD CONSTRAINT i_order_v3_partial_fill_output_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_partial_fills
    ADD CONSTRAINT i_order_v3_partial_fills_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_predispatch_assets
    ADD CONSTRAINT i_order_v3_predispatch_assets_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_protocol_fee_refunds
    ADD CONSTRAINT i_order_v3_protocol_fee_refunds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3_status_metadata
    ADD CONSTRAINT i_order_v3_status_metadata_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.i_order_v3s
    ADD CONSTRAINT i_order_v3s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.intent_gateway_token_volumes
    ADD CONSTRAINT intent_gateway_token_volumes_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.pending_status_metadata
    ADD CONSTRAINT pending_status_metadata_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.protocol_dust_collecteds
    ADD CONSTRAINT protocol_dust_collecteds_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.protocol_dust_swepts
    ADD CONSTRAINT protocol_dust_swepts_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.relayer_activities
    ADD CONSTRAINT relayer_activities_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.relayer_reward_transactions
    ADD CONSTRAINT relayer_reward_transactions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.relayer_stats_per_chain_v2s
    ADD CONSTRAINT relayer_stats_per_chain_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.relayer_v2s
    ADD CONSTRAINT relayer_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.request_status_metadata
    ADD CONSTRAINT request_status_metadata_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.request_v2s
    ADD CONSTRAINT request_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.response_status_metadata
    ADD CONSTRAINT response_status_metadata_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.response_v2s
    ADD CONSTRAINT response_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.rewards_activity_logs
    ADD CONSTRAINT rewards_activity_logs_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.rewards
    ADD CONSTRAINT rewards_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_delegations
    ADD CONSTRAINT solver_delegations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_discovery_requests
    ADD CONSTRAINT solver_discovery_requests_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_inventory_heads
    ADD CONSTRAINT solver_inventory_heads_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_vault_shares
    ADD CONSTRAINT solver_vault_shares_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_watchlist_cursors
    ADD CONSTRAINT solver_watchlist_cursors_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.solver_watchlists
    ADD CONSTRAINT solver_watchlists_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.state_machine_update_events
    ADD CONSTRAINT state_machine_update_events_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.token_price_logs
    ADD CONSTRAINT token_price_logs_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.token_prices
    ADD CONSTRAINT token_prices_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.tracked_solvers
    ADD CONSTRAINT tracked_solvers_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.transfers
    ADD CONSTRAINT transfers_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.treasuries
    ADD CONSTRAINT treasuries_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.user_activity_v2s
    ADD CONSTRAINT user_activity_v2s_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.vault_ledger_events
    ADD CONSTRAINT vault_ledger_events_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.vault_lp_positions
    ADD CONSTRAINT vault_lp_positions_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.vault_position_snapshots
    ADD CONSTRAINT vault_position_snapshots_pkey PRIMARY KEY (id);

ALTER TABLE ONLY app.vault_snapshots
    ADD CONSTRAINT vault_snapshots_pkey PRIMARY KEY (id);

CREATE INDEX "0x0164c5e63419feb4" ON app.protocol_dust_collecteds USING btree (token_symbol);

CREATE INDEX "0x038889582dcdcdfb" ON app.token_price_logs USING btree (provider);

CREATE INDEX "0x03c54d72e0078b42" ON app.token_prices USING btree (last_updated_at, symbol);

CREATE INDEX "0x07a8c567a30ec47a" ON app.i_order_v3_protocol_fee_refunds USING btree (amount);

CREATE INDEX "0x08902cf15fa3fded" ON app.i_order_v3_predispatch_assets USING btree (amount);

CREATE INDEX "0x095f01e6a46468ca" ON app.i_order_v3_output_assets USING btree (beneficiary);

CREATE INDEX "0x09e8e81d92768f72" ON app.vault_position_snapshots USING btree (underlying_token);

CREATE INDEX "0x0a55ff684cbb944b" ON app.i_order_v3_protocol_fee_refunds USING hash (order_id);

CREATE INDEX "0x0b85cbb3a4923eb2" ON app.protocol_dust_collecteds USING btree (chain);

CREATE INDEX "0x0eb177769d04a1a8" ON app.vault_ledger_events USING btree (event_type);

CREATE INDEX "0x0f3d48e903531190" ON app.i_order_v3_escrow_refund_tokens USING hash (refund_id);

CREATE INDEX "0x106783a8d4434f4c" ON app.rewards_activity_logs USING btree (chain, earner_address);

CREATE INDEX "0x13bdd0408875c16d" ON app.token_price_logs USING btree (created_at);

CREATE INDEX "0x169a035f844c01ca" ON app.i_order_v3_escrow_refunds USING btree (transaction_hash);

CREATE INDEX "0x16a1ddf6244cf54a" ON app.transfers USING btree ("from");

CREATE INDEX "0x175583483ebf07d6" ON app.bridge_token_supplies USING btree (created_at);

CREATE INDEX "0x17b57e141e937ad6" ON app.transfers USING btree (amount);

CREATE INDEX "0x19fab0d76bb9f171" ON app.token_price_logs USING btree (symbol);

CREATE INDEX "0x1a60e2a50c4955d9" ON app.vault_lp_positions USING btree (vault);

CREATE INDEX "0x1a72ff92f231fea9" ON app.response_v2s USING btree (created_at);

CREATE INDEX "0x1af7d9c215f98761" ON app.bandwidth_apps USING btree (app);

CREATE INDEX "0x1c309a0f38d2b58f" ON app.response_v2s USING hash (request_id);

CREATE INDEX "0x1cec1c44c09d767e" ON app.relayer_stats_per_chain_v2s USING btree (chain);

CREATE INDEX "0x1dc668a3c1b5d144" ON app.response_v2s USING btree (commitment);

CREATE INDEX "0x1f6e0f514728d4ac" ON app.get_responses USING btree (chain);

CREATE INDEX "0x1f8d582a14ccb630" ON app.i_order_v3_partial_fill_input_assets USING btree (amount);

CREATE INDEX "0x1fa53d48164bfb8b" ON app.solver_watchlist_cursors USING btree (chain);

CREATE INDEX "0x1ff8c243fd9299c4" ON app.filler_bids USING btree (bid);

CREATE INDEX "0x204f305f137c3f6d" ON app.state_machine_update_events USING btree (created_at);

CREATE INDEX "0x20950ee86e864313" ON app.i_order_v3_cancellations USING btree (block_number);

CREATE INDEX "0x20b41cde690a88fa" ON app.hyperbridge_collator_reward_transactions USING btree (created_at);

CREATE INDEX "0x20be02aa94026c67" ON app.i_order_v3_partial_fills USING btree (created_at);

CREATE INDEX "0x20c73f8cc112e5cf" ON app.state_machine_update_events USING btree (chain, state_machine_id);

CREATE INDEX "0x21cfbf99e2d03bb7" ON app.rewards_activity_logs USING btree (chain, earner_type);

CREATE INDEX "0x24a970144386edb6" ON app.relayer_stats_per_chain_v2s USING btree (fees_earned);

CREATE INDEX "0x267b91880ceeaf71" ON app.bridge_token_supplies USING btree (last_updated_at);

CREATE INDEX "0x26e018915b313edb" ON app.vault_ledger_events USING btree (block_number);

CREATE INDEX "0x2a98b8f7b7cd4d71" ON app.user_activity_v2s USING btree (created_at);

CREATE INDEX "0x2afd61d1b0ac2c11" ON app.i_order_v3_escrow_refunds USING btree (block_number);

CREATE INDEX "0x2c1b523119e46741" ON app.i_order_v3_protocol_fee_refunds USING btree (created_at);

CREATE INDEX "0x2ca29faaf7fc2838" ON app.vault_ledger_events USING btree (lp);

CREATE INDEX "0x302e1b623c2b8485" ON app.protocol_dust_collecteds USING btree (amount);

CREATE INDEX "0x31b09b9d458c51cb" ON app.transfers USING btree ("to");

CREATE INDEX "0x33f57522f6cd0d40" ON app.vault_snapshots USING btree (underlying_token);

CREATE INDEX "0x343a8b233a17fd4f" ON app.state_machine_update_events USING btree (chain, height);

CREATE INDEX "0x35a52914934f32ae" ON app.bandwidth_subscriptions USING btree (tier);

CREATE INDEX "0x3672eda73bc2db15" ON app.i_order_v3_fill_input_assets USING btree (amount);

CREATE INDEX "0x38099edb43b5ad7f" ON app.intent_gateway_token_volumes USING btree (token_symbol);

CREATE INDEX "0x38c8a6eb29815254" ON app.vault_position_snapshots USING btree (lp);

CREATE INDEX "0x3a5e714fad708a95" ON app.solver_vault_shares USING btree (chain);

CREATE INDEX "0x3a627c99f56fc9f0" ON app.i_order_v3_protocol_fee_refunds USING btree (transaction_hash);

CREATE INDEX "0x3c796e6902cde45e" ON app.i_order_v3_protocol_fee_refunds USING btree ("timestamp");

CREATE INDEX "0x3cc6ff4376158e03" ON app.rewards USING btree (chain);

CREATE INDEX "0x3d6b8e2925541d42" ON app.relayer_stats_per_chain_v2s USING hash (relayer_id);

CREATE INDEX "0x3f74f064d02cec0d" ON app.i_order_v3_fills USING btree (transaction_hash);

CREATE INDEX "0x409df13846e7fc02" ON app.rewards_activity_logs USING btree (created_at);

CREATE INDEX "0x40c7a30f39a0876b" ON app.token_price_logs USING btree ("timestamp");

CREATE INDEX "0x40eeaa75eef96a6a" ON app.vault_lp_positions USING btree (last_updated_at);

CREATE INDEX "0x413e4af6964d08d5" ON app.i_order_v3s USING btree (block_number);

CREATE INDEX "0x41812965619a412a" ON app.i_order_v3_protocol_fee_refunds USING btree (block_number);

CREATE INDEX "0x42b932a994015d8d" ON app.cumulative_dust_collected_per_chains USING btree (chain);

CREATE INDEX "0x435fae9d37550a6a" ON app.i_order_v3_escrow_releases USING btree ("timestamp");

CREATE INDEX "0x43965c7ba1fb80ef" ON app.vault_snapshots USING btree (block_number);

CREATE INDEX "0x43c13e77e96e5cd2" ON app.i_order_v3_escrow_release_tokens USING btree (amount);

CREATE INDEX "0x43c368fc80c956bd" ON app.i_order_v3_fills USING btree (user_op_hash);

CREATE INDEX "0x447169f21c84d9b0" ON app.i_order_v3_fill_input_assets USING hash (fill_id);

CREATE INDEX "0x4532dc438e7632b6" ON app.vault_position_snapshots USING btree (lp, vault);

CREATE INDEX "0x4bbe5bbe18fb3ce8" ON app.i_order_v3_partial_fill_output_assets USING btree (beneficiary);

CREATE INDEX "0x4c159f5a81c6df92" ON app.rewards USING btree (chain, earner_type);

CREATE INDEX "0x4cf34ce7aae10f9f" ON app.rewards USING btree (address);

CREATE INDEX "0x4e0f5b97131b545d" ON app.hyperbridge_collator_reward_transactions USING btree (block_number);

CREATE INDEX "0x50a67e2adb94ba9d" ON app.i_order_v3_cancellations USING btree (chain);

CREATE INDEX "0x5766e482b04c623e" ON app.i_order_v3_partial_fills USING btree (user_op_hash);

CREATE INDEX "0x579df6b4e5693f04" ON app.rewards USING btree (address, chain);

CREATE INDEX "0x57d2e13da2e5b0e2" ON app.bandwidth_app_daily_consumptions USING hash (app_id);

CREATE INDEX "0x57dca0f289ea0de7" ON app.protocol_dust_swepts USING btree (chain);

CREATE INDEX "0x57f36207d092f7e0" ON app.state_machine_update_events USING btree (block_number, chain);

CREATE INDEX "0x583250ade4084a30" ON app.vault_snapshots USING btree (snapshot_time);

CREATE INDEX "0x585e5864db093e3d" ON app.protocol_dust_swepts USING btree (last_updated);

CREATE INDEX "0x58bbd365fb5cfbfe" ON app.i_order_v3_partial_fills USING btree ("timestamp");

CREATE INDEX "0x59c4fcb7c825582f" ON app.rewards_activity_logs USING btree (transaction_hash);

CREATE INDEX "0x59f7a83d928dae37" ON app.token_price_logs USING btree (provider, "timestamp");

CREATE INDEX "0x5ab0063b445a5a76" ON app.hyper_bridge_stats USING btree (total_transfers_in);

CREATE INDEX "0x5ad40570fe3ed6ea" ON app.vault_position_snapshots USING btree (yield_earned);

CREATE INDEX "0x5b4ce537987ee3e4" ON app.cumulative_intent_gateway_volume_u_s_ds USING btree (chain);

CREATE INDEX "0x5e53245b2a84f2e7" ON app.tracked_solvers USING btree (status);

CREATE INDEX "0x5f03cae314611f33" ON app.i_order_v3_escrow_releases USING btree (block_number);

CREATE INDEX "0x60f5e63bf25a33aa" ON app.transfers USING btree (chain);

CREATE INDEX "0x64700fbd345ecb65" ON app.get_request_status_metadata USING hash (request_id);

CREATE INDEX "0x651b59987fd8cefc" ON app.rewards_activity_logs USING btree (earner_address);

CREATE INDEX "0x662d03e2c1e1d751" ON app.state_machine_update_events USING btree (block_number);

CREATE INDEX "0x667fa98ed98ce88e" ON app.i_order_v3_predispatch_assets USING btree (token);

CREATE INDEX "0x6685d920e52f3819" ON app.i_order_v3_escrow_releases USING btree (transaction_hash);

CREATE INDEX "0x67cf14f2d8b715e4" ON app.vault_ledger_events USING btree ("timestamp");

CREATE INDEX "0x6833c33de2573cd5" ON app.tracked_solvers USING btree (chain);

CREATE INDEX "0x6b159a4ac1966982" ON app.relayer_reward_transactions USING btree (amount);

CREATE INDEX "0x6bdd2b09e5e21e39" ON app.i_order_v3_escrow_releases USING hash (order_id);

CREATE INDEX "0x6c404d763d0b9fe2" ON app.vault_ledger_events USING btree (lp, vault);

CREATE INDEX "0x6c5bfb240775568f" ON app.tracked_solvers USING btree (solver);

CREATE INDEX "0x6cb0ca6f1d7fd13d" ON app.vault_ledger_events USING btree (chain);

CREATE INDEX "0x6d1f55dfa1cccd6f" ON app.token_price_logs USING btree (symbol, "timestamp");

CREATE INDEX "0x6d2cf57c3f94b064" ON app.i_order_v3_predispatch_assets USING hash (order_id);

CREATE INDEX "0x6f37f14a598f2d41" ON app.relayer_reward_transactions USING btree (chain, reward_type);

CREATE INDEX "0x6fbd5419abf704ed" ON app.vault_snapshots USING btree (day_start_timestamp);

CREATE INDEX "0x700355b76c1a661a" ON app.solver_discovery_requests USING btree (chain, version);

CREATE INDEX "0x70d16e0d792cc476" ON app.i_order_v3_status_metadata USING hash (order_id);

CREATE INDEX "0x71b4566b266465a7" ON app.relayer_reward_transactions USING btree (relayer);

CREATE INDEX "0x71ce448c996b8e06" ON app.hyperbridge_collator_reward_transactions USING btree (collator);

CREATE INDEX "0x7313db924cb0f6c2" ON app.i_order_v3_fills USING btree (chain);

CREATE INDEX "0x7789aacacf580981" ON app.intent_gateway_token_volumes USING btree (chain);

CREATE INDEX "0x7840334b4c40c574" ON app.protocol_dust_swepts USING btree (amount);

CREATE INDEX "0x799b5aaef9789ce8" ON app.relayer_reward_transactions USING btree (reward_type);

CREATE INDEX "0x7a2ec72bcb25cf94" ON app.i_order_v3_partial_fill_output_assets USING btree (token);

CREATE INDEX "0x7a72d8121768e1f4" ON app.i_order_v3s USING btree (user_op_hash);

CREATE INDEX "0x7a8623501a761353" ON app.vault_ledger_events USING btree (chain, vault);

CREATE INDEX "0x7b881d4d972f9948" ON app.vault_lp_positions USING btree (chain, vault);

CREATE INDEX "0x7c197fa58d15d816" ON app.accumulated_fees USING btree (relayer);

CREATE INDEX "0x7d0142367c4046bc" ON app.i_order_v3_input_assets USING hash (order_id);

CREATE INDEX "0x7d9346d9a1fe7662" ON app.i_order_v3_partial_fill_input_assets USING hash (partial_fill_id);

CREATE INDEX "0x7ea8c7ef023ff08a" ON app.hyper_bridge_stats USING btree (fees_payed_out_to_relayers);

CREATE INDEX "0x7ebc5e1be8b5881e" ON app.i_order_v3_cancellations USING btree ("timestamp");

CREATE INDEX "0x8003374c6ed6e0bc" ON app.response_v2s USING btree (chain, commitment);

CREATE INDEX "0x81c0ff94714d883d" ON app.vault_lp_positions USING btree (lp);

CREATE INDEX "0x83df5546252ff44a" ON app.transfers USING btree ("from", "to");

CREATE INDEX "0x866b832f865ede4b" ON app.i_order_v3_fill_output_assets USING hash (fill_id);

CREATE INDEX "0x86b4a85660e38303" ON app.solver_discovery_requests USING btree (chain);

CREATE INDEX "0x87bca0563ffd2bde" ON app.vault_ledger_events USING btree (underlying_token);

CREATE INDEX "0x87e16a3a0d36aebf" ON app.vault_snapshots USING btree (chain);

CREATE INDEX "0x88c17570456bfbda" ON app.i_order_v3_fills USING btree (filler);

CREATE INDEX "0x891acf9779f93504" ON app.relayer_reward_transactions USING btree (created_at);

CREATE INDEX "0x89ef006d090dc0d4" ON app.request_status_metadata USING btree (created_at);

CREATE INDEX "0x8b08f2bf2e60b182" ON app.i_order_v3s USING btree (transaction_hash);

CREATE INDEX "0x8b3f0993459104de" ON app.state_machine_update_events USING btree (chain);

CREATE INDEX "0x8b93e40c08ecdca1" ON app.bandwidth_app_tier_stats USING hash (app_id);

CREATE INDEX "0x8b98f58d929a1d8f" ON app.token_prices USING btree (symbol);

CREATE INDEX "0x8bd0c968dd651649" ON app.rewards_activity_logs USING btree (chain);

CREATE INDEX "0x8c049ba651e00d21" ON app.request_status_metadata USING hash (request_id);

CREATE INDEX "0x8c48df99fa59dea5" ON app.solver_delegations USING btree (chain);

CREATE INDEX "0x8d3675445a6fc082" ON app.vault_snapshots USING btree (vault);

CREATE INDEX "0x8d4232259dde8992" ON app.relayer_stats_per_chain_v2s USING btree (chain, relayer_id);

CREATE INDEX "0x8f6403bc88f95a36" ON app.rewards USING btree (points);

CREATE INDEX "0x8ff2adb548c08787" ON app.i_order_v3_output_assets USING btree (amount);

CREATE INDEX "0x90141efe9438dc31" ON app.transfers USING btree (chain, "to");

CREATE INDEX "0x9105ad6ba7686f9f" ON app.i_order_v3_status_metadata USING btree (created_at);

CREATE INDEX "0x9113b2ce042493e7" ON app.i_order_v3_escrow_releases USING btree (created_at);

CREATE INDEX "0x91a82a7a959ae175" ON app.request_v2s USING btree (commitment);

CREATE INDEX "0x92fd9efa1796a6de" ON app.protocol_dust_swepts USING btree (token_symbol);

CREATE INDEX "0x951cb89b5156b90b" ON app.i_order_v3_fill_input_assets USING btree (token);

CREATE INDEX "0x9683c5fc675084bf" ON app.relayer_reward_transactions USING btree (block_number);

CREATE INDEX "0x970da2bef468796e" ON app.vault_position_snapshots USING btree (chain, vault);

CREATE INDEX "0x98395d167a0508fa" ON app.cumulative_dust_swept_per_chains USING btree (chain);

CREATE INDEX "0x98bbbd74ef9033db" ON app.vault_lp_positions USING btree (chain);

CREATE INDEX "0x9918bc591a5ddbce" ON app.bandwidth_app_daily_consumptions USING btree (app_id, date);

CREATE INDEX "0x99f1331f2339252d" ON app.get_responses USING btree (chain, commitment);

CREATE INDEX "0x9aadc28926ce8a96" ON app.solver_vault_shares USING btree (vault);

CREATE INDEX "0x9bb369ddc0b980c9" ON app.i_order_v3_partial_fills USING btree (transaction_hash);

CREATE INDEX "0x9dad71f13afae643" ON app.i_order_v3_partial_fills USING btree (chain);

CREATE INDEX "0x9f016e187906f7b1" ON app.i_order_v3_protocol_fee_refunds USING btree (token);

CREATE INDEX "0xa038fe4bf6e12340" ON app.i_order_v3_partial_fills USING btree (block_number);

CREATE INDEX "0xa1af4c128592e3f5" ON app.state_machine_update_events USING btree (state_machine_id);

CREATE INDEX "0xa1cdd38f54de092c" ON app.i_order_v3_protocol_fee_refunds USING btree (chain);

CREATE INDEX "0xa334ede0a5db09da" ON app.i_order_v3_fills USING btree (created_at);

CREATE INDEX "0xa3c252b6c714ef63" ON app.vault_ledger_events USING btree (vault);

CREATE INDEX "0xa732514f5fac1ec2" ON app.bandwidth_apps USING btree (chain);

CREATE INDEX "0xa95f4c74487598e9" ON app.relayer_reward_transactions USING btree (chain, relayer);

CREATE INDEX "0xab1e88c796c97b21" ON app.vault_position_snapshots USING btree (chain);

CREATE INDEX "0xaede5c0a035583e1" ON app.transfers USING btree (chain, "from");

CREATE INDEX "0xb12d93e5685b454b" ON app.i_order_v3_fills USING btree (block_number);

CREATE INDEX "0xb15e6e7355a51920" ON app.filler_bids USING btree (commitment);

CREATE INDEX "0xb31b2cba0d9652aa" ON app.request_v2s USING btree (chain, commitment);

CREATE INDEX "0xb5e2b2aab5978f8a" ON app.solver_discovery_requests USING btree (solver);

CREATE INDEX "0xb82a61008309e324" ON app.i_order_v3s USING btree (block_timestamp);

CREATE INDEX "0xb96b76940d810642" ON app.hyper_bridge_stats USING btree (number_of_failed_deliveries);

CREATE INDEX "0xb9a851b370a0b78c" ON app.bandwidth_subscriptions USING btree (app_id, tier);

CREATE INDEX "0xba821a691f8a4d50" ON app.solver_delegations USING btree (refreshed_at);

CREATE INDEX "0xbae894989d79a702" ON app.solver_delegations USING btree (chain, solver);

CREATE INDEX "0xbae919c3f50865a3" ON app.bridge_token_supplies USING btree (total_supply);

CREATE INDEX "0xbafa63cb4fe7b9e6" ON app.i_order_v3_escrow_refunds USING btree (chain);

CREATE INDEX "0xbaff156081c4739e" ON app.solver_delegations USING btree (solver);

CREATE INDEX "0xbd7150b866db85ab" ON app.bandwidth_subscriptions USING hash (app_id);

CREATE INDEX "0xbf752e8a71b48ecd" ON app.i_order_v3_partial_fill_input_assets USING btree (token);

CREATE INDEX "0xc02245c0108f7680" ON app.response_status_metadata USING btree (created_at);

CREATE INDEX "0xc1f97e89b357f479" ON app.hyper_bridge_stats USING btree (number_of_timed_out_messages);

CREATE INDEX "0xc2a05c0050ce82c2" ON app.bandwidth_app_tier_stats USING btree (app_id, tier);

CREATE INDEX "0xc444dcb47ab9744e" ON app.bridge_token_supplies USING btree (circulating_supply);

CREATE INDEX "0xc4b0f5751e01e418" ON app.solver_watchlists USING btree (chain);

CREATE INDEX "0xc5947cf89272a384" ON app.i_order_v3_escrow_releases USING btree (solver);

CREATE INDEX "0xc8330388093bae3d" ON app.request_v2s USING btree (chain);

CREATE INDEX "0xc95cf01617290878" ON app.pending_status_metadata USING btree (entity_type);

CREATE INDEX "0xcaa79907eb7e5b64" ON app.relayer_reward_transactions USING btree (chain);

CREATE INDEX "0xcc3171c4c22aeb1f" ON app.i_order_v3_escrow_refund_tokens USING btree (amount);

CREATE INDEX "0xccd4b9775b11c99a" ON app.solver_discovery_requests USING btree (version);

CREATE INDEX "0xce11f6aa70b0ed0f" ON app.i_order_v3_partial_fills USING btree (filler);

CREATE INDEX "0xce994e3f33c78956" ON app.i_order_v3_escrow_release_tokens USING hash (release_id);

CREATE INDEX "0xcec97484bfb296f0" ON app.i_order_v3_fill_output_assets USING btree (token);

CREATE INDEX "0xcfa1df438449723f" ON app.solver_vault_shares USING btree (solver);

CREATE INDEX "0xd0209832bd1f5c93" ON app.i_order_v3_input_assets USING btree (token);

CREATE INDEX "0xd0eea598788d6dc7" ON app.response_v2s USING btree (chain);

CREATE INDEX "0xd1aef609c8dd2d16" ON app.rewards_activity_logs USING btree (points);

CREATE INDEX "0xd2ed15c599e90e25" ON app.protocol_dust_collecteds USING btree (last_updated);

CREATE INDEX "0xd4e69af57019110a" ON app.i_order_v3s USING btree (created_at);

CREATE INDEX "0xd51abb81c05a0e94" ON app.vault_lp_positions USING btree (created_at);

CREATE INDEX "0xd53e186b722f1f0c" ON app.i_order_v3_escrow_refunds USING hash (order_id);

CREATE INDEX "0xd71b30ed308ea272" ON app.intent_gateway_token_volumes USING btree (volume_type);

CREATE INDEX "0xd79fb57ba3b97565" ON app.i_order_v3_escrow_refund_tokens USING btree (token);

CREATE INDEX "0xd8e1c9f0e2682a54" ON app.i_order_v3_escrow_refunds USING btree (created_at);

CREATE INDEX "0xd9317fa3413b9914" ON app.state_machine_update_events USING btree (height);

CREATE INDEX "0xd9e869ae458171b1" ON app.accumulated_fees USING btree (chain_id);

CREATE INDEX "0xdb4801871045cfe6" ON app.pending_status_metadata USING btree (commitment);

CREATE INDEX "0xdc0e94f0f2d83237" ON app.i_order_v3_cancellations USING btree (canceller);

CREATE INDEX "0xdcbe2e078c0a5e69" ON app.get_responses USING btree (commitment);

CREATE INDEX "0xdd132400873270f7" ON app.i_order_v3_output_assets USING btree (token);

CREATE INDEX "0xdde0c45ad62b5348" ON app.token_prices USING btree (last_updated_at);

CREATE INDEX "0xdfb36f967c7c04d8" ON app.filler_bids USING btree (block_number);

CREATE INDEX "0xe1b5567f3b04ac01" ON app.intent_gateway_token_volumes USING btree (token_address);

CREATE INDEX "0xe2a91d51c0517a85" ON app.solver_delegations USING btree (observed_at);

CREATE INDEX "0xe2efd16bd36ca343" ON app.get_request_status_metadata USING btree (created_at);

CREATE INDEX "0xe3cdd42f6cf67b6a" ON app.i_order_v3_input_assets USING btree (amount);

CREATE INDEX "0xe4dead1627c1c90d" ON app.vault_position_snapshots USING btree (snapshot_time);

CREATE INDEX "0xe52ec8ab6205546b" ON app.bandwidth_app_tier_stats USING btree (tier);

CREATE INDEX "0xe594bd127726bd7c" ON app.get_request_v2s USING btree (created_at);

CREATE INDEX "0xe620c1472b0601ce" ON app.bandwidth_subscriptions USING btree (app_id, purchased_at);

CREATE INDEX "0xe8ed5303aea46b92" ON app.i_order_v3_cancellations USING hash (order_id);

CREATE INDEX "0xe9055c9cd89ae79f" ON app.solver_vault_shares USING btree (chain, solver);

CREATE INDEX "0xe97f3a066b40a5e2" ON app.i_order_v3_fills USING hash (order_id);

CREATE INDEX "0xea63f97466f64101" ON app.filler_bids USING btree (filler);

CREATE INDEX "0xeaa03317960929fd" ON app.rewards_activity_logs USING btree (activity_type, chain);

CREATE INDEX "0xeb6f6e7969f49465" ON app.i_order_v3_cancellations USING btree (transaction_hash);

CREATE INDEX "0xed3e0f3acf5dd642" ON app.i_order_v3_partial_fill_output_assets USING btree (amount);

CREATE INDEX "0xedd88ec345cfea29" ON app.request_v2s USING btree (created_at);

CREATE INDEX "0xee56f0d73b09728d" ON app.i_order_v3_partial_fills USING hash (order_id);

CREATE INDEX "0xee8b036f5b342cda" ON app.vault_position_snapshots USING btree (day_start_timestamp);

CREATE INDEX "0xefe96d90dfe2c274" ON app.i_order_v3_partial_fill_output_assets USING hash (partial_fill_id);

CREATE INDEX "0xf0e334f7e1c0aa77" ON app.response_status_metadata USING hash (response_id);

CREATE INDEX "0xf12a02003425fa5a" ON app.i_order_v3_escrow_refunds USING btree ("timestamp");

CREATE INDEX "0xf1c5184ab6e0db3b" ON app.cumulative_intent_gateway_volume_u_s_ds USING btree (volume_type);

CREATE INDEX "0xf1f196ae7d4c234d" ON app.i_order_v3_output_assets USING hash (order_id);

CREATE INDEX "0xf281fab1244e2c89" ON app.hyper_bridge_stats USING btree (number_of_messages_sent);

CREATE INDEX "0xf424ded524e78697" ON app.i_order_v3_cancellations USING btree (created_at);

CREATE INDEX "0xf5927225f9337f8b" ON app.relayer_activities USING hash (relayer_id);

CREATE INDEX "0xf642a520032fceed" ON app.hyper_bridge_stats USING btree (number_of_delivered_messages);

CREATE INDEX "0xf7f720f769e6008c" ON app.bandwidth_app_daily_consumptions USING btree (date);

CREATE INDEX "0xf80d1893ec70b2cc" ON app.i_order_v3_escrow_release_tokens USING btree (token);

CREATE INDEX "0xf820d7ca1b6675b4" ON app.tracked_solvers USING btree (chain, status);

CREATE INDEX "0xf97eaf019b9b3cc5" ON app.i_order_v3_fills USING btree ("timestamp");

CREATE INDEX "0xfa52005a7add3787" ON app.relayer_stats_per_chain_v2s USING btree (chain, fees_earned);

CREATE INDEX "0xfaa56b200c32ce13" ON app.i_order_v3_fill_output_assets USING btree (amount);

CREATE INDEX "0xfb2220aa5186c64d" ON app.i_order_v3_escrow_releases USING btree (chain);

CREATE INDEX "0xfbc1f31e14f97683" ON app.hyperbridge_collator_reward_transactions USING btree (amount);

CREATE INDEX "0xfbcb0c0ff5ac51d0" ON app.vault_position_snapshots USING btree (block_number);

CREATE INDEX "0xfdae6e5678aeed02" ON app.vault_snapshots USING btree (chain, vault);

CREATE INDEX "0xfeb81c774a2c171f" ON app.vault_lp_positions USING btree (underlying_token);

CREATE INDEX "0xfefa49c044c748a2" ON app.get_responses USING hash (request_id);

CREATE INDEX "0xff278e6fae33946f" ON app.vault_position_snapshots USING btree (vault);

CREATE INDEX "0xfff64b5a7c32ab6d" ON app.get_responses USING btree (created_at);

CREATE UNIQUE INDEX relayer_activities_relayer_id_uindex ON app.relayer_activities USING btree (relayer_id);

ALTER TABLE ONLY app.bandwidth_app_daily_consumptions
    ADD CONSTRAINT bandwidth_app_daily_consumptions_app_id_fkey FOREIGN KEY (app_id) REFERENCES app.bandwidth_apps(id) ON UPDATE CASCADE DEFERRABLE;

ALTER TABLE ONLY app.bandwidth_app_tier_stats
    ADD CONSTRAINT bandwidth_app_tier_stats_app_id_fkey FOREIGN KEY (app_id) REFERENCES app.bandwidth_apps(id) ON UPDATE CASCADE DEFERRABLE;

ALTER TABLE ONLY app.bandwidth_subscriptions
    ADD CONSTRAINT bandwidth_subscriptions_app_id_fkey FOREIGN KEY (app_id) REFERENCES app.bandwidth_apps(id) ON UPDATE CASCADE DEFERRABLE;

ALTER TABLE ONLY app.get_request_status_metadata
    ADD CONSTRAINT get_request_status_metadata_request_id_fkey FOREIGN KEY (request_id) REFERENCES app.get_request_v2s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT get_request_status_metadata_request_id_fkey ON app.get_request_status_metadata IS '@foreignFieldName statusMetadata';

ALTER TABLE ONLY app.get_responses
    ADD CONSTRAINT get_responses_request_id_fkey FOREIGN KEY (request_id) REFERENCES app.get_request_v2s(id) ON UPDATE CASCADE ON DELETE SET NULL DEFERRABLE;

ALTER TABLE ONLY app.i_order_v3_cancellations
    ADD CONSTRAINT i_order_v3_cancellations_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_cancellations_order_id_fkey ON app.i_order_v3_cancellations IS '@foreignFieldName cancellations';

ALTER TABLE ONLY app.i_order_v3_escrow_refund_tokens
    ADD CONSTRAINT i_order_v3_escrow_refund_tokens_refund_id_fkey FOREIGN KEY (refund_id) REFERENCES app.i_order_v3_escrow_refunds(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_escrow_refund_tokens_refund_id_fkey ON app.i_order_v3_escrow_refund_tokens IS '@foreignFieldName tokens';

ALTER TABLE ONLY app.i_order_v3_escrow_refunds
    ADD CONSTRAINT i_order_v3_escrow_refunds_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_escrow_refunds_order_id_fkey ON app.i_order_v3_escrow_refunds IS '@foreignFieldName escrowRefunds';

ALTER TABLE ONLY app.i_order_v3_escrow_release_tokens
    ADD CONSTRAINT i_order_v3_escrow_release_tokens_release_id_fkey FOREIGN KEY (release_id) REFERENCES app.i_order_v3_escrow_releases(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_escrow_release_tokens_release_id_fkey ON app.i_order_v3_escrow_release_tokens IS '@foreignFieldName tokens';

ALTER TABLE ONLY app.i_order_v3_escrow_releases
    ADD CONSTRAINT i_order_v3_escrow_releases_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_escrow_releases_order_id_fkey ON app.i_order_v3_escrow_releases IS '@foreignFieldName escrowReleases';

ALTER TABLE ONLY app.i_order_v3_fill_input_assets
    ADD CONSTRAINT i_order_v3_fill_input_assets_fill_id_fkey FOREIGN KEY (fill_id) REFERENCES app.i_order_v3_fills(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_fill_input_assets_fill_id_fkey ON app.i_order_v3_fill_input_assets IS '@foreignFieldName inputAssets';

ALTER TABLE ONLY app.i_order_v3_fill_output_assets
    ADD CONSTRAINT i_order_v3_fill_output_assets_fill_id_fkey FOREIGN KEY (fill_id) REFERENCES app.i_order_v3_fills(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_fill_output_assets_fill_id_fkey ON app.i_order_v3_fill_output_assets IS '@foreignFieldName outputAssets';

ALTER TABLE ONLY app.i_order_v3_fills
    ADD CONSTRAINT i_order_v3_fills_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_fills_order_id_fkey ON app.i_order_v3_fills IS '@foreignFieldName fills';

ALTER TABLE ONLY app.i_order_v3_input_assets
    ADD CONSTRAINT i_order_v3_input_assets_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_input_assets_order_id_fkey ON app.i_order_v3_input_assets IS '@foreignFieldName inputAssets';

ALTER TABLE ONLY app.i_order_v3_output_assets
    ADD CONSTRAINT i_order_v3_output_assets_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_output_assets_order_id_fkey ON app.i_order_v3_output_assets IS '@foreignFieldName outputAssets';

ALTER TABLE ONLY app.i_order_v3_partial_fill_input_assets
    ADD CONSTRAINT i_order_v3_partial_fill_input_assets_partial_fill_id_fkey FOREIGN KEY (partial_fill_id) REFERENCES app.i_order_v3_partial_fills(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_partial_fill_input_assets_partial_fill_id_fkey ON app.i_order_v3_partial_fill_input_assets IS '@foreignFieldName inputAssets';

ALTER TABLE ONLY app.i_order_v3_partial_fill_output_assets
    ADD CONSTRAINT i_order_v3_partial_fill_output_assets_partial_fill_id_fkey FOREIGN KEY (partial_fill_id) REFERENCES app.i_order_v3_partial_fills(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_partial_fill_output_assets_partial_fill_id_fkey ON app.i_order_v3_partial_fill_output_assets IS '@foreignFieldName outputAssets';

ALTER TABLE ONLY app.i_order_v3_partial_fills
    ADD CONSTRAINT i_order_v3_partial_fills_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_partial_fills_order_id_fkey ON app.i_order_v3_partial_fills IS '@foreignFieldName partialFills';

ALTER TABLE ONLY app.i_order_v3_predispatch_assets
    ADD CONSTRAINT i_order_v3_predispatch_assets_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_predispatch_assets_order_id_fkey ON app.i_order_v3_predispatch_assets IS '@foreignFieldName predispatchAssets';

ALTER TABLE ONLY app.i_order_v3_protocol_fee_refunds
    ADD CONSTRAINT i_order_v3_protocol_fee_refunds_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_protocol_fee_refunds_order_id_fkey ON app.i_order_v3_protocol_fee_refunds IS '@foreignFieldName protocolFeeRefunds';

ALTER TABLE ONLY app.i_order_v3_status_metadata
    ADD CONSTRAINT i_order_v3_status_metadata_order_id_fkey FOREIGN KEY (order_id) REFERENCES app.i_order_v3s(id) ON UPDATE CASCADE ON DELETE SET NULL DEFERRABLE;

COMMENT ON CONSTRAINT i_order_v3_status_metadata_order_id_fkey ON app.i_order_v3_status_metadata IS '@foreignFieldName statusMetadata';

ALTER TABLE ONLY app.relayer_activities
    ADD CONSTRAINT relayer_activities_relayer_id_fkey FOREIGN KEY (relayer_id) REFERENCES app.relayer_v2s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT relayer_activities_relayer_id_fkey ON app.relayer_activities IS '@singleForeignFieldName latestActivity';

ALTER TABLE ONLY app.relayer_stats_per_chain_v2s
    ADD CONSTRAINT relayer_stats_per_chain_v2s_relayer_id_fkey FOREIGN KEY (relayer_id) REFERENCES app.relayer_v2s(id) ON UPDATE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT relayer_stats_per_chain_v2s_relayer_id_fkey ON app.relayer_stats_per_chain_v2s IS '@foreignFieldName perChainStats';

ALTER TABLE ONLY app.request_status_metadata
    ADD CONSTRAINT request_status_metadata_request_id_fkey FOREIGN KEY (request_id) REFERENCES app.request_v2s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT request_status_metadata_request_id_fkey ON app.request_status_metadata IS '@foreignFieldName statusMetadata';

ALTER TABLE ONLY app.response_status_metadata
    ADD CONSTRAINT response_status_metadata_response_id_fkey FOREIGN KEY (response_id) REFERENCES app.response_v2s(id) ON UPDATE CASCADE ON DELETE CASCADE DEFERRABLE;

COMMENT ON CONSTRAINT response_status_metadata_response_id_fkey ON app.response_status_metadata IS '@foreignFieldName statusMetadata';

ALTER TABLE ONLY app.response_v2s
    ADD CONSTRAINT response_v2s_request_id_fkey FOREIGN KEY (request_id) REFERENCES app.request_v2s(id) ON UPDATE CASCADE ON DELETE SET NULL DEFERRABLE;

CREATE TABLE app.solver_inventories (
    id text NOT NULL,
    chain text NOT NULL,
    solver text NOT NULL,
    token_address text NOT NULL,
    balance numeric NOT NULL,
    wallet numeric NOT NULL,
    vaults numeric NOT NULL,
    vault_shares numeric NOT NULL,
    block_number numeric NOT NULL,
    last_log_index integer NOT NULL,
    observed_at timestamp without time zone NOT NULL,
    refreshed_at timestamp without time zone NOT NULL,
    genesis_block numeric NOT NULL,
    last_read_block numeric NOT NULL,
    last_reconciled_at timestamp without time zone,
    last_reconciled_drift numeric,
    discovered_by app.d75f09d1b9 NOT NULL,
    _id uuid NOT NULL,
    _block_range int8range NOT NULL
);

COMMENT ON COLUMN app.solver_inventories.id IS 'Composite identifier: {chain}-{tokenAddress}-{solver}.';

COMMENT ON COLUMN app.solver_inventories.chain IS 'State machine ID of the chain.';

COMMENT ON COLUMN app.solver_inventories.solver IS 'Lowercased EVM address of the solver.';

COMMENT ON COLUMN app.solver_inventories.token_address IS 'Lowercased EVM address of the token.';

COMMENT ON COLUMN app.solver_inventories.balance IS 'wallet + vaults, in raw token units.';

COMMENT ON COLUMN app.solver_inventories.wallet IS 'Running ERC-20 balance: the latest storage read plus every Transfer since.';

COMMENT ON COLUMN app.solver_inventories.vaults IS 'The solver''s shares in this token''s vaults, valued in assets at the last movement or revaluation.';

COMMENT ON COLUMN app.solver_inventories.vault_shares IS 'Share balance behind `vaults`, summed across the token''s vaults, so a consumer can tell a movement from a revaluation.';

COMMENT ON COLUMN app.solver_inventories.block_number IS 'Block of the last Transfer applied, or of the latest storage read.';

COMMENT ON COLUMN app.solver_inventories.last_log_index IS 'Log index of that Transfer. After a storage read it is the largest Int, since the read already includes every log of its block.';

COMMENT ON COLUMN app.solver_inventories.observed_at IS 'Timestamp of the last change to the balance. Never moves backwards.';

COMMENT ON COLUMN app.solver_inventories.refreshed_at IS 'Last time the value was confirmed by a read: the genesis read, a vault revaluation, or a reconciliation.';

COMMENT ON COLUMN app.solver_inventories.genesis_block IS 'Block of the genesis storage read.';

COMMENT ON COLUMN app.solver_inventories.last_read_block IS 'Block of the latest storage read; Transfers at or before it are already included.';

COMMENT ON COLUMN app.solver_inventories.last_reconciled_at IS 'Timestamp of the latest reconciliation.';

COMMENT ON COLUMN app.solver_inventories.last_reconciled_drift IS 'What the latest reconciliation found the running total off by (read minus tracked), in raw token units. Non-zero means an event was missed or the token moves balances without a Transfer.';

COMMENT ON COLUMN app.solver_inventories.discovered_by IS 'How the solver came to be tracked.';

ALTER TABLE ONLY app.solver_inventories
    ADD CONSTRAINT solver_inventories_pkey PRIMARY KEY (_id);

CREATE INDEX "0x02b3c9562ab87479" ON app.solver_inventories USING gist (observed_at, _block_range);

CREATE INDEX "0x20823d714375a62a" ON app.solver_inventories USING gist (chain, _block_range);

CREATE INDEX "0x4f96d0739e46c71f" ON app.solver_inventories USING gist (chain, solver, _block_range);

CREATE INDEX "0x50bf00107888b57c" ON app.solver_inventories USING gist (refreshed_at, _block_range);

CREATE INDEX "0x7caf80aad1f608fb" ON app.solver_inventories USING gist (token_address, _block_range);

CREATE INDEX "0x993b3432cfc2996b" ON app.solver_inventories USING gist (solver, _block_range);

CREATE INDEX "0xa7c0825e405a0152" ON app.solver_inventories USING btree (id);

CREATE TABLE app._metadata (
    key character varying(255) NOT NULL PRIMARY KEY,
    value jsonb,
    "createdAt" timestamp with time zone NOT NULL,
    "updatedAt" timestamp with time zone NOT NULL
);
