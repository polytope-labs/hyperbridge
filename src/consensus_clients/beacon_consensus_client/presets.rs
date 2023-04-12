use ismp_rs::{host::ChainID, router::RequestResponse};
#[cfg(not(feature = "testnet"))]
pub use mainnet::*;
#[cfg(feature = "testnet")]
pub use testnet::*;

#[cfg(not(feature = "testnet"))]
mod mainnet {
    use hex_literal::hex;

    pub const L2_ORACLE_ADDRESS: [u8; 20] = hex!("47bBB9054823f27B9B6A71F5cb0eBc785692FF2E");
    /// Contract address on optimism
    pub const ISMP_CONTRACT_ADDRESS_OPTIMISM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on gnosis
    pub const ISMP_CONTRACT_ADDRESS_GNOSIS: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on arbitrum
    pub const ISMP_CONTRACT_ADDRESS_ARB: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on base
    pub const ISMP_CONTRACT_ADDRESS_BASE: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on moonbeam
    pub const ISMP_CONTRACT_ADDRESS_MOONBEAM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on ethereum
    pub const ISMP_CONTRACT_ADDRESS_ETHEREUM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Unbonding period for ethereum after which unstaked validators can withdraw their funds
    /// https://ethos.dev/beacon-chain
    pub const UNBONDING_PERIOD_HOURS: u64 = 27;
}

#[cfg(feature = "testnet")]
mod testnet {
    use hex_literal::hex;

    pub const L2_ORACLE_ADDRESS: [u8; 20] = hex!("47bBB9054823f27B9B6A71F5cb0eBc785692FF2E");
    /// Contract address on optimism
    pub const ISMP_CONTRACT_ADDRESS_OPTIMISM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on gnosis
    pub const ISMP_CONTRACT_ADDRESS_GNOSIS: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on arbitrum
    pub const ISMP_CONTRACT_ADDRESS_ARB: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on base
    pub const ISMP_CONTRACT_ADDRESS_BASE: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on moonbeam
    pub const ISMP_CONTRACT_ADDRESS_MOONBEAM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Contract address on ethereum
    pub const ISMP_CONTRACT_ADDRESS_ETHEREUM: [u8; 20] =
        hex!("b856af30b938b6f52e5bff365675f358cd52f91b");
    /// Unbonding period for ethereum after which unstaked validators can withdraw their funds
    pub const UNBONDING_PERIOD_HOURS: u64 = 27;
}

pub fn ismp_contract_address(item: &RequestResponse) -> Option<[u8; 20]> {
    let chain_id = match item {
        RequestResponse::Request(req) => req.source_chain(),
        RequestResponse::Response(res) => res.request.dest_chain(),
    };

    match chain_id {
        ChainID::ETHEREUM => Some(ISMP_CONTRACT_ADDRESS_ETHEREUM),
        ChainID::GNOSIS => Some(ISMP_CONTRACT_ADDRESS_GNOSIS),
        ChainID::ARBITRUM => Some(ISMP_CONTRACT_ADDRESS_ARB),
        ChainID::OPTIMISM => Some(ISMP_CONTRACT_ADDRESS_OPTIMISM),
        ChainID::BASE => Some(ISMP_CONTRACT_ADDRESS_BASE),
        ChainID::MOONBEAM => Some(ISMP_CONTRACT_ADDRESS_MOONBEAM),
        _ => None,
    }
}
