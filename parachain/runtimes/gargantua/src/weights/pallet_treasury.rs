#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_treasury`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_treasury::WeightInfo for WeightInfo<T> {
	/// Storage: Treasury ProposalCount (r:1 w:1)
	/// Proof: Treasury ProposalCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: Treasury Approvals (r:1 w:1)
	/// Proof: Treasury Approvals (max_values: Some(1), max_size: Some(402), added: 897, mode: MaxEncodedLen)
	/// Storage: Treasury Proposals (r:0 w:1)
	/// Proof: Treasury Proposals (max_values: None, max_size: Some(108), added: 2583, mode: MaxEncodedLen)
	fn spend_local() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `1887`
		// Minimum execution time: 177_000_000 picoseconds.
		Weight::from_parts(191_000_000, 0)
			.saturating_add(Weight::from_parts(0, 1887))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: Treasury Approvals (r:1 w:1)
	/// Proof: Treasury Approvals (max_values: Some(1), max_size: Some(402), added: 897, mode: MaxEncodedLen)
	fn remove_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127`
		//  Estimated: `1887`
		// Minimum execution time: 80_000_000 picoseconds.
		Weight::from_parts(82_000_000, 0)
			.saturating_add(Weight::from_parts(0, 1887))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Treasury Deactivated (r:1 w:1)
	/// Proof: Treasury Deactivated (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Balances InactiveIssuance (r:1 w:1)
	/// Proof: Balances InactiveIssuance (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: Treasury Approvals (r:1 w:1)
	/// Proof: Treasury Approvals (max_values: Some(1), max_size: Some(402), added: 897, mode: MaxEncodedLen)
	/// Storage: Treasury Proposals (r:99 w:99)
	/// Proof: Treasury Proposals (max_values: None, max_size: Some(108), added: 2583, mode: MaxEncodedLen)
	/// Storage: System Account (r:199 w:199)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Bounties BountyApprovals (r:1 w:1)
	/// Proof: Bounties BountyApprovals (max_values: Some(1), max_size: Some(402), added: 897, mode: MaxEncodedLen)
	/// The range of component `p` is `[0, 99]`.
	fn on_initialize_proposals(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331 + p * (251 ±0)`
		//  Estimated: `3593 + p * (5206 ±0)`
		// Minimum execution time: 887_000_000 picoseconds.
		Weight::from_parts(828_616_021, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			// Standard Error: 695_351
			.saturating_add(Weight::from_parts(566_114_524, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes(5))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 5206).saturating_mul(p.into()))
	}
	/// Storage: AssetRate ConversionRateToNative (r:1 w:0)
	/// Proof: AssetRate ConversionRateToNative (max_values: None, max_size: Some(1237), added: 3712, mode: MaxEncodedLen)
	/// Storage: Treasury SpendCount (r:1 w:1)
	/// Proof: Treasury SpendCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: Treasury Spends (r:0 w:1)
	/// Proof: Treasury Spends (max_values: None, max_size: Some(1848), added: 4323, mode: MaxEncodedLen)
	fn spend() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `114`
		//  Estimated: `4702`
		// Minimum execution time: 208_000_000 picoseconds.
		Weight::from_parts(222_000_000, 0)
			.saturating_add(Weight::from_parts(0, 4702))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Treasury Spends (r:1 w:1)
	/// Proof: Treasury Spends (max_values: None, max_size: Some(1848), added: 4323, mode: MaxEncodedLen)
	/// Storage: XcmPallet QueryCounter (r:1 w:1)
	/// Proof Skipped: XcmPallet QueryCounter (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Configuration ActiveConfig (r:1 w:0)
	/// Proof Skipped: Configuration ActiveConfig (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Dmp DeliveryFeeFactor (r:1 w:0)
	/// Proof Skipped: Dmp DeliveryFeeFactor (max_values: None, max_size: None, mode: Measured)
	/// Storage: XcmPallet SupportedVersion (r:1 w:0)
	/// Proof Skipped: XcmPallet SupportedVersion (max_values: None, max_size: None, mode: Measured)
	/// Storage: XcmPallet VersionDiscoveryQueue (r:1 w:1)
	/// Proof Skipped: XcmPallet VersionDiscoveryQueue (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: XcmPallet SafeXcmVersion (r:1 w:0)
	/// Proof Skipped: XcmPallet SafeXcmVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Dmp DownwardMessageQueues (r:1 w:1)
	/// Proof Skipped: Dmp DownwardMessageQueues (max_values: None, max_size: None, mode: Measured)
	/// Storage: Dmp DownwardMessageQueueHeads (r:1 w:1)
	/// Proof Skipped: Dmp DownwardMessageQueueHeads (max_values: None, max_size: None, mode: Measured)
	/// Storage: XcmPallet Queries (r:0 w:1)
	/// Proof Skipped: XcmPallet Queries (max_values: None, max_size: None, mode: Measured)
	fn payout() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `737`
		//  Estimated: `5313`
		// Minimum execution time: 551_000_000 picoseconds.
		Weight::from_parts(569_000_000, 0)
			.saturating_add(Weight::from_parts(0, 5313))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: Treasury Spends (r:1 w:1)
	/// Proof: Treasury Spends (max_values: None, max_size: Some(1848), added: 4323, mode: MaxEncodedLen)
	/// Storage: XcmPallet Queries (r:1 w:1)
	/// Proof Skipped: XcmPallet Queries (max_values: None, max_size: None, mode: Measured)
	fn check_status() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `442`
		//  Estimated: `5313`
		// Minimum execution time: 245_000_000 picoseconds.
		Weight::from_parts(281_000_000, 0)
			.saturating_add(Weight::from_parts(0, 5313))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Treasury Spends (r:1 w:1)
	/// Proof: Treasury Spends (max_values: None, max_size: Some(1848), added: 4323, mode: MaxEncodedLen)
	fn void_spend() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `172`
		//  Estimated: `5313`
		// Minimum execution time: 147_000_000 picoseconds.
		Weight::from_parts(160_000_000, 0)
			.saturating_add(Weight::from_parts(0, 5313))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}