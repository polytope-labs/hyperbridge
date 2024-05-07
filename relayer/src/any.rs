use arb_host::ArbConfig;
use ismp::host::StateMachine;
use op_host::OpConfig;
use serde::{Deserialize, Serialize};
use tesseract_bsc::BscPosConfig;
use tesseract_sync_committee::SyncCommitteeConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Various chain configurations supported by consensus task
pub enum AnyConfig {
    /// Ethereum Sepolia sync committee config
    Sepolia(SyncCommitteeConfig),
    /// Ethereum Mainnet sync committee config
    Ethereum(SyncCommitteeConfig),
    /// Any Arbitrum orbit chain config
    ArbitrumOrbit(ArbConfig),
    /// Any Opstack chain config
    OpStack(OpConfig),
    /// Bsc chain config
    Bsc(BscPosConfig),
}

impl AnyConfig {
    /// Returns the state machine for the config
    pub fn state_machine(&self) -> StateMachine {
        match self {
            AnyConfig::Sepolia(config) => config.evm_config.state_machine,
            AnyConfig::Ethereum(config) => config.evm_config.state_machine,
            AnyConfig::ArbitrumOrbit(config) => config.evm_config.state_machine,
            AnyConfig::OpStack(config) => config.evm_config.state_machine,
            AnyConfig::Bsc(config) => config.evm_config.state_machine,
        }
    }
}
