# Relayer Incentives Pallet

A FRAME pallet for providing incentives to relayers in the Hyperbridge network.

## Overview

The Relayer Incentives pallet provides a mechanism to reward relayers for their services in the Hyperbridge network. It manages incentive parameters, tracks rewards for relayers, and provides functionality for claiming accumulated rewards.

## Features

- Configurable incentive parameters (base rewards, priority multipliers, etc.)
- Tracking of rewards per relayer
- Secure reward claiming mechanism
- Admin control over incentive parameters

## Interface

### Dispatchable Functions

- `update_incentive_parameters` - Update the parameters used for calculating relayer incentives
- `reward_relayer` - Reward a specific relayer for their services
- `claim_rewards` - Allow a relayer to claim their accumulated rewards

### Storage

- `IncentiveParams` - Global parameters for the incentive mechanism
- `RelayerRewards` - Mapping of relayer accounts to their accumulated rewards

## Usage

This pallet can be integrated into a runtime to provide economic incentives for relayers participating in the Hyperbridge network. It works alongside other Hyperbridge pallets to create a complete incentive system.

## Integration

To use this pallet in your runtime, you need to implement its Config trait and include it in your runtime's construct_runtime macro.

```rust
parameter_types! {
    // Define your parameters here
}

impl pallet_relayer_incentives::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AdminOrigin = EnsureRoot<AccountId>;
}

// In construct_runtime
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Other pallets
        RelayerIncentives: pallet_relayer_incentives,
    }
);
```