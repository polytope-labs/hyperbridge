
//! Autogenerated weights for `pallet_collective`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-08-23, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Lukambas-M2-MAX`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("gargantua-2000")`, DB CACHE: 1024

// Executed Command:
// ./target/release/hyperbridge
// benchmark
// pallet
// --chain=gargantua-2000
// --pallet
// pallet_collective
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --output
// parachain/runtimes/gargantua/src/weights/pallet_collective.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_collective`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collective::WeightInfo for WeightInfo<T> {
	/// Storage: `TechnicalCollective::Members` (r:1 w:1)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:0)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Voting` (r:100 w:100)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Prime` (r:0 w:1)
	/// Proof: `TechnicalCollective::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[0, 10]`.
	/// The range of component `n` is `[0, 10]`.
	/// The range of component `p` is `[0, 100]`.
	fn set_members(m: u32, _n: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + m * (3232 ±0) + p * (309 ±0)`
		//  Estimated: `7119 + m * (1848 ±25) + p * (2643 ±2)`
		// Minimum execution time: 6_000_000 picoseconds.
		Weight::from_parts(6_000_000, 0)
			.saturating_add(Weight::from_parts(0, 7119))
			// Standard Error: 311_524
			.saturating_add(Weight::from_parts(8_075_528, 0).saturating_mul(m.into()))
			// Standard Error: 31_842
			.saturating_add(Weight::from_parts(2_977_511, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 1848).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 2643).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 10]`.
	fn execute(b: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `104 + m * (32 ±0)`
		//  Estimated: `1588 + m * (32 ±0)`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(9_865_645, 0)
			.saturating_add(Weight::from_parts(0, 1588))
			// Standard Error: 46
			.saturating_add(Weight::from_parts(1_829, 0).saturating_mul(b.into()))
			// Standard Error: 4_952
			.saturating_add(Weight::from_parts(12_615, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:1 w:0)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 10]`.
	fn propose_execute(b: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `104 + m * (32 ±0)`
		//  Estimated: `3568 + m * (32 ±0)`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_253_091, 0)
			.saturating_add(Weight::from_parts(0, 3568))
			// Standard Error: 51
			.saturating_add(Weight::from_parts(1_948, 0).saturating_mul(b.into()))
			// Standard Error: 5_515
			.saturating_add(Weight::from_parts(57_107, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalCount` (r:1 w:1)
	/// Proof: `TechnicalCollective::ProposalCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Voting` (r:0 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[2, 10]`.
	/// The range of component `p` is `[1, 100]`.
	fn propose_proposed(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `482 + m * (32 ±0) + p * (35 ±0)`
		//  Estimated: `3797 + m * (40 ±0) + p * (36 ±0)`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(15_842_342, 0)
			.saturating_add(Weight::from_parts(0, 3797))
			// Standard Error: 75
			.saturating_add(Weight::from_parts(2_370, 0).saturating_mul(b.into()))
			// Standard Error: 8_916
			.saturating_add(Weight::from_parts(125_732, 0).saturating_mul(m.into()))
			// Standard Error: 776
			.saturating_add(Weight::from_parts(132_769, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 40).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Voting` (r:1 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[5, 10]`.
	fn vote(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `974 + m * (64 ±0)`
		//  Estimated: `4439 + m * (64 ±0)`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(14_632_768, 0)
			.saturating_add(Weight::from_parts(0, 4439))
			// Standard Error: 7_985
			.saturating_add(Weight::from_parts(63_993, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 64).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCollective::Voting` (r:1 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[4, 10]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_early_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `397 + m * (64 ±0) + p * (36 ±0)`
		//  Estimated: `3824 + m * (77 ±1) + p * (37 ±0)`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(17_139_338, 0)
			.saturating_add(Weight::from_parts(0, 3824))
			// Standard Error: 9_804
			.saturating_add(Weight::from_parts(128_887, 0).saturating_mul(m.into()))
			// Standard Error: 652
			.saturating_add(Weight::from_parts(128_836, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 77).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 37).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Voting` (r:1 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 10]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_early_approved(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `742 + b * (1 ±0) + m * (64 ±0) + p * (40 ±0)`
		//  Estimated: `4331 + b * (1 ±0) + m * (44 ±2) + p * (41 ±0)`
		// Minimum execution time: 23_000_000 picoseconds.
		Weight::from_parts(25_159_047, 0)
			.saturating_add(Weight::from_parts(0, 4331))
			// Standard Error: 91
			.saturating_add(Weight::from_parts(1_164, 0).saturating_mul(b.into()))
			// Standard Error: 14_136
			.saturating_add(Weight::from_parts(2_173, 0).saturating_mul(m.into()))
			// Standard Error: 943
			.saturating_add(Weight::from_parts(150_308, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 44).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 41).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Voting` (r:1 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Prime` (r:1 w:0)
	/// Proof: `TechnicalCollective::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[4, 10]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `417 + m * (64 ±0) + p * (36 ±0)`
		//  Estimated: `3844 + m * (77 ±1) + p * (37 ±0)`
		// Minimum execution time: 18_000_000 picoseconds.
		Weight::from_parts(18_233_398, 0)
			.saturating_add(Weight::from_parts(0, 3844))
			// Standard Error: 12_334
			.saturating_add(Weight::from_parts(185_588, 0).saturating_mul(m.into()))
			// Standard Error: 820
			.saturating_add(Weight::from_parts(128_618, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 77).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 37).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Voting` (r:1 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Members` (r:1 w:0)
	/// Proof: `TechnicalCollective::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Prime` (r:1 w:0)
	/// Proof: `TechnicalCollective::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 10]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_approved(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `762 + b * (1 ±0) + m * (64 ±0) + p * (40 ±0)`
		//  Estimated: `4351 + b * (1 ±0) + m * (44 ±2) + p * (41 ±0)`
		// Minimum execution time: 26_000_000 picoseconds.
		Weight::from_parts(26_509_353, 0)
			.saturating_add(Weight::from_parts(0, 4351))
			// Standard Error: 83
			.saturating_add(Weight::from_parts(1_239, 0).saturating_mul(b.into()))
			// Standard Error: 12_829
			.saturating_add(Weight::from_parts(39_709, 0).saturating_mul(m.into()))
			// Standard Error: 856
			.saturating_add(Weight::from_parts(150_131, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 44).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 41).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCollective::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCollective::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::Voting` (r:0 w:1)
	/// Proof: `TechnicalCollective::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCollective::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCollective::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `p` is `[1, 100]`.
	fn disapprove_proposal(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `260 + p * (32 ±0)`
		//  Estimated: `1745 + p * (32 ±0)`
		// Minimum execution time: 10_000_000 picoseconds.
		Weight::from_parts(10_964_967, 0)
			.saturating_add(Weight::from_parts(0, 1745))
			// Standard Error: 789
			.saturating_add(Weight::from_parts(120_532, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(p.into()))
	}
}