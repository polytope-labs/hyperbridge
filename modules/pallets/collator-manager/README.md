# Pallet Collator Manager

This pallet introduces a reputation-based collator selection mechanism. The primary goal is to select collators based on their accrued revenue, which is treated as a reputation score held in a `pallet-assets` token.
This system is designed to incentivize reliable and active relayer participation in the network.

This pallet provides:

* An implementation of the `SessionManager` trait.
* Core logic for selecting a new set of collators at the beginning of each session based on their reputation score.
* A mechanism for resetting the reputation scores of outgoing collators to ensure a fair and dynamic selection process.

## Usage

This pallet must be configured as the `SessionManager` in your runtime's `pallet_session::Config`. It depends on `pallet-collator-selection` to provide the list of candidates and `pallet-assets` (with a holder pallet like `pallet-assets-holder`) to manage the reputation token.

```rust
use polkadot_sdk::sp_core::H256;
use frame_support::parameter_types;

// 1. Define a CandidateProvider to get the list of candidates.
pub struct CollatorSelectionProvider;
impl pallet_collator_manager::CandidateProvider<AccountId> for CollatorSelectionProvider {
    fn candidates() -> Vec<AccountId> {
        pallet_collator_selection::CandidateList::<Runtime>::get()
            .into_iter()
            .map(|info| info.who)
            .collect()
    }
}

// 2. Configure the pallet in your runtime.
parameter_types! {
    pub const DesiredCollators: u32 = 10;
    pub const ReputationAssetId: H256 = H256([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
}

impl pallet_collator_manager::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ReputationCurrency = ReputationCurrency; // AssetCurrencyAdapter
    type CandidateProvider = CollatorSelectionProvider;
    type ReputationAssetId = ReputationAssetId;
    type ReputationAssets = Assets; // pallet-assets instance
    type DesiredCollators = DesiredCollators;
}

// 3. Set it as the SessionManager.
impl pallet_session::Config for Runtime {
    // ... other session config ...
    type SessionManager = CollatorManager;
}