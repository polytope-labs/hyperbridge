use frame_support::weights::Weight;

// ============================== INTERFACE ============================================ //
/// Weight functions needed for `pallet_token_gateway.
pub trait WeightInfo {
    fn create_erc6160_asset() -> Weight;
    fn teleport() -> Weight;
    fn set_token_gateway_addresses(x: u32) -> Weight;
    fn update_erc6160_asset() -> Weight;
}

impl WeightInfo for () {
    fn create_erc6160_asset() -> Weight {
        Weight::zero()
    }

    fn teleport() -> Weight {
        Weight::zero()
    }

    fn set_token_gateway_addresses(_x: u32) -> Weight {
        Weight::zero()
    }

    fn update_erc6160_asset() -> Weight {
        Weight::zero()
    }
}
