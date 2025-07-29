# Consensus Incentives Pallet

A FRAME pallet for providing incentives to consensus relayers in the Hyperbridge network.

## Overview

The Consensus Incentives pallet provides a mechanism to reward relayers for their services in the Hyperbridge network. It manages the cost per block for each consensus Block Message, and it is also responsible for distibuting rewards to relayers the consensus message processed.

## Features

- Configurable cost per block per State machine
- Tracking of rewards per relayer

## Interface

### Dispatchable Functions

- `update_cost_per_block` - Updates the cost per block for a state machine

### Storage

- `ProcessedMessages` - Holds already processed consensus messages
- `RelayerRewards` - Mapping of relayer accounts to their accumulated rewards
- `StateMachinesCostPerBlock` - Mapping of State machine to each Block cost

## Usage

This pallet can be integrated into a runtime to provide economic incentives for relayers participating in the Hyperbridge network. It works alongside other Hyperbridge pallets to create a complete incentive system.

## Integration

To use this pallet in your runtime, you need to implement its Config trait and include it in your runtime's construct_runtime macro.

```rust
parameter_types! {
    // Define your parameters here
}

impl pallet_consensus_incentives::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IsmpHost = IsmpHost;
    type TreasuryAccount = TreasuryAccount;
    type WeightInfo = ();
}

// In construct_runtime
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Other pallets
        ConsensusIncentives: pallet_consensus_incentives,
    }
);
```
