use tesseract_evm::EvmConfig;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TendermintEvmClientConfig {
	/// EVM config
	#[serde(flatten)]
	pub evm_config: EvmConfig,
	/// RPC URL
	pub rpc_url: String,
}
