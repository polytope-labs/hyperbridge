# Pallet Messaging Fees

This pallet implements a dual-mechanism system to manage incentives for ISMP message relayers. It acts as a **FeeHandler** for `pallet-ismp`, processing all relayed messages to distribute rewards, and account for protocol-level charges.

---

## Overview

The **Messaging Fees** pallet is designed to create a balanced economic model for messaging relayers within the Hyperbridge ecosystem.  
It ensures that relayers are compensated for their service for delivering ISMP `Request` and `Response` messages.

The pallet features two primary incentive mechanisms:

1. **Bridge Rewards/Incentives**  
   Relayers are rewarded from the local treasury for delivering messages on whitelisted routes.  
   The incentive amount is calculated based on the message size and follows a dynamic quadratic/decay curve.  
   This mechanism can either reward relayers or charge them a fee, depending on network traffic within the current session.

2. **Protocol Fees**  
   The pallet captures fees paid by users on various source chains (both Substrate and EVM).  
   It integrates with `pallet-ismp-relayer` to handle the accounting of these fees, allowing relayers to accumulate and eventually withdraw them from the respective source chains.

---

## Key Concepts

### Bridge Incentives (Rewards & Fees)

This mechanism incentivizes relaying when network traffic is low and disincentivizes it during periods of high congestion.  
It operates on a **session-by-session** basis.

- A `TargetMessageSize` is configured for each session.
- When a relayer processes a message on a whitelisted route, the pallet calculates a `base_reward` based on the message size and a price oracle.

**Reward Scenario**  
If `TotalBytesProcessed` in the current session is **less than** `TargetMessageSize`, the relayer receives a reward from the treasury:

```latex
Reward=BaseReward×((TargetSize−TotalBytes)/TargetSize)^2
```

**Fee Scenario**  
If `TotalBytesProcessed` **exceeds** `TargetMessageSize`, the relayer is charged a fee equal to the `base_reward`, which is transferred to the treasury.

**Reputation**  
In both scenarios, a **reputation asset** is minted to the relayer, equal to the calculated reward or fee amount.

**Session Reset**  
`TotalBytesProcessed` is reset to zero at the beginning of each new session, restarting the incentive curve.

---

### Protocol Fees

This mechanism allows relayers to collect fees paid by users for cross-chain interactions.

- **Substrate Chains**  
  Fees are captured via the `OnRequestProcessed` hook (from the hyperbridge client machine) and stored temporarily in the `CommitmentFees` map.

- **EVM Chains**  
  Fees are calculated based on a `per_byte_fee` defined in the `HostParams` for the destination chain.

All collected fees are funneled to `pallet-ismp-relayer` for proper accounting and to enable withdrawal by the relayer.

---

### Relayer Identification

The pallet identifies the relayer of a message by recovering their public key from the cryptographic signature attached to the ISMP `Request` or `Response`.

---

## Interface

### Dispatchable Functions

- `set_supported_route(origin, state_machine)`  
  A privileged extrinsic to whitelist a `StateMachine` (i.e., a chain) for Messaging Incentives.

---

### Storage

- **TotalBytesProcessed**  
  A `StorageValue` that tracks the cumulative size of message bodies processed on incentivized routes within the current session.

- **IncentivizedRoutes**  
  A `StorageMap` that holds the set of whitelisted state machines eligible for Messaging Incentives.

- **CommitmentFees**  
  A temporary `StorageMap` that links a request commitment hash to the protocol fees paid on a source Substrate chain.  
  Entries are consumed when the corresponding message is processed.

---

### Events

- `RouteSupported` — Emitted when a new state machine is whitelisted.
- `FeeRewarded` — Emitted when a relayer receives a reward from the treasury.
- `FeePaid` — Emitted when a relayer is charged a fee that is paid to the treasury.
- `IncentivesReset` — Emitted at the start of a new session when `TotalBytesProcessed` is reset.

---

## Configuration

To integrate this pallet into a runtime, implement its `Config` trait:

```rust
use frame_support::{parameter_types, PalletId};
use sp_core::ConstU32;

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
}

impl pallet_messaging_fees::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IsmpHost = Ismp;
    type TreasuryAccount = TreasuryPalletId;
    type IncentivesOrigin = EnsureRoot<AccountId>;
    type PriceOracle = YourPriceOracle;
    type TargetMessageSize = ConstU32<4096>; // 4KB
    type ReputationAsset = Assets;
    type WeightInfo = ();
}
```