use crate::types::{ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, SubstrateConfig};
use anyhow::anyhow;
use core::str::FromStr;
use ismp::{
    host::StateMachine,
    router::{Post, PostResponse},
};
use primitive_types::H160;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct JsChainConfig {
    pub rpc_url: String,
    pub state_machine: String,
    pub host_address: Vec<u8>,
    pub handler_address: Vec<u8>,
    pub consensus_state_id: Vec<u8>,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct JsHyperbridgeConfig {
    pub rpc_url: String,
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct JsClientConfig {
    pub source: JsChainConfig,
    pub dest: JsChainConfig,
    pub hyperbridge: JsHyperbridgeConfig,
}

impl TryFrom<JsClientConfig> for ClientConfig {
    type Error = anyhow::Error;

    fn try_from(value: JsClientConfig) -> Result<Self, Self::Error> {
        let to_config = |val: &JsChainConfig| {
            if !val.host_address.is_empty() && !val.handler_address.is_empty() {
                let conf = EvmConfig {
                    rpc_url: val.rpc_url.clone(),
                    state_machine: StateMachine::from_str(&val.state_machine)
                        .map_err(|e| anyhow!("{e:?}"))?,
                    host_address: {
                        if val.host_address.len() != 20 {
                            Err(anyhow!("Invalid host address"))?
                        }
                        H160::from_slice(&val.host_address)
                    },
                    handler_address: {
                        if val.handler_address.len() != 20 {
                            Err(anyhow!("Invalid handler address"))?
                        }
                        H160::from_slice(&val.handler_address)
                    },
                    consensus_state_id: {
                        if val.consensus_state_id.len() != 4 {
                            Err(anyhow!("Invalid consensus state id"))?
                        }
                        let mut dest = [0u8; 4];
                        dest.copy_from_slice(&val.consensus_state_id);
                        dest
                    },
                };

                Ok::<_, anyhow::Error>(ChainConfig::Evm(conf))
            } else {
                let conf = SubstrateConfig {
                    rpc_url: val.rpc_url.clone(),
                    consensus_state_id: {
                        if val.consensus_state_id.len() != 4 {
                            Err(anyhow!("Invalid consensus state id"))?
                        }
                        let mut dest = [0u8; 4];
                        dest.copy_from_slice(&val.consensus_state_id);
                        dest
                    },
                    hash_algo: HashAlgorithm::Keccak,
                };

                Ok(ChainConfig::Substrate(conf))
            }
        };

        let to_hyperbridge_config = |val: &JsHyperbridgeConfig| {
            let conf = SubstrateConfig {
                rpc_url: val.rpc_url.clone(),
                consensus_state_id: [0u8; 4],
                hash_algo: HashAlgorithm::Keccak,
            };

            Ok::<ChainConfig, Self::Error>(ChainConfig::Substrate(conf))
        };

        let source_config = to_config(&value.source)?;
        let dest_config = to_config(&value.dest)?;
        let hyperbridge = to_hyperbridge_config(&value.hyperbridge)?;

        Ok(ClientConfig { source: source_config, dest: dest_config, hyperbridge })
    }
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Eq, PartialEq)]
pub struct JsPost {
    /// The source state machine of this request.
    pub source: String,
    /// The destination state machine of this request.
    pub dest: String,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Module ID of the receiving module
    pub to: Vec<u8>,
    /// Timestamp which this request expires in seconds.
    pub timeout_timestamp: u64,
    /// Encoded Request.
    pub data: Vec<u8>,
    /// Gas limit for executing the request on destination
    /// This value should be zero if destination module is not a contract
    pub gas_limit: u64,
}

impl TryFrom<JsPost> for Post {
    type Error = anyhow::Error;

    fn try_from(value: JsPost) -> Result<Self, Self::Error> {
        let post = Post {
            source: StateMachine::from_str(&value.source).map_err(|e| anyhow!("{e:?}"))?,
            dest: StateMachine::from_str(&value.dest).map_err(|e| anyhow!("{e:?}"))?,
            nonce: value.nonce,
            from: value.from,
            to: value.to,
            timeout_timestamp: value.timeout_timestamp,
            data: value.data,
            gas_limit: value.gas_limit,
        };
        Ok(post)
    }
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Clone, Eq, PartialEq)]
pub struct JsResponse {
    /// The request that triggered this response.
    pub post: JsPost,
    /// The response message.
    pub response: Vec<u8>,
    /// Timestamp at which this response expires in seconds.
    pub timeout_timestamp: u64,
    /// Gas limit for executing the response on destination, only used for solidity modules.
    pub gas_limit: u64,
}

impl TryFrom<JsResponse> for PostResponse {
    type Error = anyhow::Error;

    fn try_from(value: JsResponse) -> Result<Self, Self::Error> {
        let response = PostResponse {
            post: value.post.try_into()?,
            response: value.response,
            timeout_timestamp: value.timeout_timestamp,
            gas_limit: value.gas_limit,
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        interfaces::{JsChainConfig, JsClientConfig, JsHyperbridgeConfig, JsPost, JsResponse},
        types::{ChainConfig, ClientConfig, EvmConfig, HashAlgorithm, SubstrateConfig},
    };
    use ethers::prelude::H160;
    use hex_literal::hex;
    use ismp::{
        host::{Ethereum, StateMachine},
        router::{Post, PostResponse},
    };
    const OP_HOST: H160 = H160(hex!("1B58A47e61Ca7604b634CBB00b4e275cCd7c9E95"));
    const BSC_HOST: H160 = H160(hex!("022DDE07A21d8c553978b006D93CDe68ac83e677"));
    const OP_HANDLER: H160 = H160(hex!("a25151598Dc180fc03635858f37bDF8427f47845"));
    const BSC_HANDLER: H160 = H160(hex!("43a0BcC347894303f93905cE137CB3b804bE990d"));
    #[test]
    fn test_chain_config_conversion() {
        let source_chain = EvmConfig {
            rpc_url: "https://127.0.0.1:9990".to_string(),
            state_machine: StateMachine::Bsc,
            host_address: BSC_HOST,
            handler_address: BSC_HANDLER,
            consensus_state_id: *b"BSC0",
        };

        let dest_chain = EvmConfig {
            rpc_url: "https://127.0.0.1:9990".to_string(),
            state_machine: StateMachine::Ethereum(Ethereum::Optimism),
            host_address: OP_HOST,
            handler_address: OP_HANDLER,
            consensus_state_id: *b"ETH0",
        };

        let hyperbrige_config = SubstrateConfig {
            rpc_url: "ws://127.0.0.1:9990".to_string(),
            consensus_state_id: [0u8; 4],
            hash_algo: HashAlgorithm::Keccak,
        };
        let config = ClientConfig {
            source: ChainConfig::Evm(source_chain.clone()),
            dest: ChainConfig::Evm(dest_chain.clone()),
            hyperbridge: ChainConfig::Substrate(hyperbrige_config),
        };

        let js_source = JsChainConfig {
            rpc_url: "https://127.0.0.1:9990".to_string(),
            state_machine: "BSC".to_string(),
            host_address: BSC_HOST.0.to_vec(),
            handler_address: BSC_HANDLER.0.to_vec(),
            consensus_state_id: b"BSC0".to_vec(),
        };

        let js_dest = JsChainConfig {
            rpc_url: "https://127.0.0.1:9990".to_string(),
            state_machine: "OPTI".to_string(),
            host_address: OP_HOST.0.to_vec(),
            handler_address: OP_HANDLER.0.to_vec(),
            consensus_state_id: b"ETH0".to_vec(),
        };

        let js_hyperbridge = JsHyperbridgeConfig { rpc_url: "ws://127.0.0.1:9990".to_string() };

        let js_client_conf =
            JsClientConfig { source: js_source, dest: js_dest, hyperbridge: js_hyperbridge };

        assert_eq!(config, js_client_conf.try_into().unwrap());
    }

    #[test]
    fn test_post_conversion() {
        let post_response = PostResponse {
            post: Post {
                source: StateMachine::Bsc,
                dest: StateMachine::Kusama(2000),
                nonce: 100,
                from: vec![30; 20],
                to: vec![15; 20],
                timeout_timestamp: 1_600_000,
                data: vec![40; 256],
                gas_limit: 5000,
            },
            response: vec![80; 256],
            timeout_timestamp: 4_500_000,
            gas_limit: 6000,
        };

        let js_post_response = JsResponse {
            post: JsPost {
                source: "BSC".to_string(),
                dest: "KUSAMA-2000".to_string(),
                nonce: 100,
                from: vec![30; 20],
                to: vec![15; 20],
                timeout_timestamp: 1_600_000,
                data: vec![40; 256],
                gas_limit: 5000,
            },
            response: vec![80; 256],
            timeout_timestamp: 4_500_000,
            gas_limit: 6000,
        };

        assert_eq!(post_response, js_post_response.try_into().unwrap())
    }
}
