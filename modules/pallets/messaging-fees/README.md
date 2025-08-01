# Messaging Fees Pallet

This pallet aims to reward and generates protocol fees for relayers and the hyperbridge protocol.

## Overview

The Messaging Fees pallet provides a dual-mechanism system to reward relayers for delivering ISMP `Request` and `Response` messages. It acts as a `FeeHandler` for `pallet-ismp` to process all relayed messages and distribute incentives accordingly.

The pallet features two types of incentives:
1.  **Bridge Rewards**: Relayers are rewarded from the local treasury for delivering messages on specified routes. The reward amount is calculated based on the size of the message body and follows a quadratic decay curve, which resets every session. 
2.  **Protocol Fees**: The pallet captures protocol fees paid by users on source chains. It then delegates the accounting of these fees to `pallet-ismp-relayer`, allowing relayers to accumulate these fees and withdraw them from respective source chains.

## Features

-   Dual incentive system: local Bridge Rewards and cross-chain Protocol Fees.
-   Configurable, whitelisted routes (Between a source and destination) for Bridge Rewards.
-   A quadratic decay reward curve to incentivize immediate message relay.
-   State reset is automatically synced with the network's session changes.
-   Cryptographic signature verification to authenticate the relayer of each message.
-   Seamless integration with `pallet-ismp-relayer` for protocol fee management.

## Interface

### Dispatchable Functions

-   `set_supported_route`: Whitelists a route to make it eligible for Bridge Rewards.

### Storage

-   `TotalBytesProcessed`: Tracks the total size of message bodies processed in the current session. This is used as input for the quadratic decay reward curve.
-   `IncentivizedRoutes`: A `StorageMap` that holds the whitelisted state machines for Bridge Rewards.
-   `CommitmentFees`: A temporary storage that maps request commitments to protocol fees captured from the `hyperbridge-client-machine` hook. These entries are consumed and removed as messages are processed.

## Usage

This pallet can be integrated into a runtime to create a comprehensive economic model for relayers. It works alongside `pallet-ismp`, `pallet-ismp-host-executive`, and `pallet-ismp-relayer` to ensure relayers are compensated both from the bridge's own treasury and from fees paid by end-users.

## Integration

To use this pallet in your runtime, you need to implement its `Config` trait and include it in your runtime's `construct_runtime!` macro.

```rust

impl pallet_messaging_fees::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IsmpHost = Ismp;
    type TreasuryAccount = TreasuryAccount;
    type IncentivesOrigin = EnsureRoot<AccountId>;
    type PriceOracle = YourPriceOracle;
    type TargetMessageSize = ConstU32<4096>; // 4KB
    type WeightInfo = ();
}


construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        MessagingFees: pallet_messaging_fees,
    }
);

```