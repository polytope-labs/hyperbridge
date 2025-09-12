# Pallet Consensus Incentives

This pallet provides a mechanism to reward consensus relayers for submitting ISMP `ConsensusMessages`, which are essential for keeping the local chain's view of other state machines up-to-date.

***

## Overview

The **Consensus Incentives pallet** integrates with `pallet-ismp` by implementing the `FeeHandler` trait. Its primary role is to calculate and distribute rewards to relayers who successfully deliver consensus updates. These updates are critical for the cross-chain communication system, and this pallet ensures that relayers are economically incentivized for their contribution.

Rewards are funded from a central treasury and are calculated based on the number of blocks a consensus update covers for a specific state machine. In addition to a token reward, relayers also earn a non-fungible reputation asset.

***

## How It Works

The pallet's core logic revolves around a reward mechanism triggered by successful consensus updates of a State machine.

### Reward Mechanism

* **Trigger**: The incentive process is triggered when the `on_executed` hook from the `FeeHandler` trait is called. The pallet filters for `ConsensusMessage` types.
* **Relayer Identification**: The pallet cryptographically recovers the relayer's public key from the signature attached to the `ConsensusMessage` to identify who should receive the reward.
* **Reward Calculation**: The reward is calculated based on how much "progress" a consensus update represents. The formula is:
  
  ```latex
    Reward=(LatestHeight - PreviousHeight) * CostPerBlock
  ```
    * **LatestHeight & PreviousHeight**: These values are fetched from the `IsmpHost` and represent the latest and previously known heights of the remote state machine.
    * **CostPerBlock**: This is a configurable value stored in the `StateMachinesCostPerBlock` map that defines the reward amount per block for each state machine.
* **Reward Distribution**: The calculated reward amount is transferred from the `TreasuryAccount` to the identified relayer.
* **Reputation Minting**: Alongside the token reward, the pallet mints a `ReputationAsset` of the same value to the relayer as an on-chain record of their contributions.

### Dispatchable Functions

* `update_cost_per_block(origin, state_machine_id, cost_per_block)`: A privileged extrinsic used to set or update the reward cost per block for a given state machine.

***

## Storage

* **`StateMachinesCostPerBlock`**: A `StorageMap` that holds the configured reward amount per block for each `StateMachineId`. This is the core configuration for the reward calculation.

***

## Events

* **`RelayerRewarded`**: Emitted when a relayer has been successfully rewarded. It includes the relayer's account, the reward amount, and the state machine height.
* **`StateMachineCostPerBlockUpdated`**: Emitted when the cost per block for a state machine is updated via the `update_cost_per_block` extrinsic.

***

## Example Code

To integrate this pallet into a runtime, you need to implement its `Config` trait.

```rust
use frame_support::{parameter_types, PalletId};

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
}

impl pallet_consensus_incentives::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IsmpHost = Ismp;
    type TreasuryAccount = TreasuryPalletId;
    type IncentivesOrigin = EnsureRoot<AccountId>;
    type ReputationAsset = Assets;
    type WeightInfo = ();
}
```