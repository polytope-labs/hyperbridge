use polkadot_sdk::sp_runtime::Weight;

/// Weight information for pallet operations
pub trait WeightInfo {
	fn set_mint_per_byte() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn set_mint_per_byte() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
