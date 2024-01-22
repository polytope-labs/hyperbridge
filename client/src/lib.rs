use std::str::FromStr;
use std::sync::Arc;
use ethers::prelude::H256;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, H160, U256};
use ismp::router::{Post, PostResponse, Request};
use wasm_bindgen::prelude::*;
use crate::types::{HyperClientConfig, MessageStatus, WasmPost, WasmPostResponse};
use subxt::{client::OnlineClient, PolkadotConfig, rpc_params};
use ismp::LeafIndexQuery;
use ismp_solidity_abi::evm_host::EvmHost;
use subxt::ext::codec;
use subxt::ext::sp_core::keccak_256;
use ismp::host::{Ethereum, StateMachine};
use ismp::util::{hash_request, Keccak256};
use hex_literal::hex;


pub mod types;
type HyperClientApiConfig = PolkadotConfig;






#[wasm_bindgen]
pub async fn query_request_status(
    request: WasmPost,
    // config
    hyper_bridge_rpc_url: String,
    destination_ismp_host_address: String,
    source_ismp_host_address: String,
    destination_rpc_ws: String,
    source_rpc_ws: String
) -> Result<MessageStatus, anyhow::Error> {
    let post: Post = request.into();
    let config = HyperClientConfig {
        hyper_bridge_rpc_url,
        destination_ismp_host_address: Address::from_str(destination_ismp_host_address.into()).unwrap(),
        source_ismp_host_address: Address::from_str(source_ismp_host_address.into()).unwrap(),
        destination_rpc_ws,
        source_rpc_ws
    };
    let provider = Arc::new(Provider::<Ws>::connect_with_reconnects(config.destination_rpc_ws.clone(), 1000).await?);
    let dest_host = EvmHost::new(config.destination_ismp_host_address, provider);

    let destination_timeout = match post.dest {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let destination_current_timestamp = dest_host.timestamp().call().await?;
            Some(destination_current_timestamp)
        },
        StateMachine::Polkadot(chain_id) => {
            Some(U256::zero())
        },
        StateMachine::Kusama(chain_id) => {
            Some(U256::zero())
        }
        _ => {
            None
        }
    };




    let destination_receipt = match post.dest {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let req = Request::Post(post.clone());
            let hash = hash_request::<Hasher>(&req);
            let request_commitment = dest_host.request_receipts(hash.into()).call().await?;
            Some(request_commitment)
        },
        StateMachine::Polkadot(chain_id) => {
            Some(H160::zero())
        },
        StateMachine::Kusama(chain_id) => {
            Some(H160::zero())
        }
        _ => {
            None
        }
    };

    // check if the message has reached the destination
    if let Some(addr) = destination_receipt {
        if addr != Address::from([0u8; 20]) {
            return Ok(MessageStatus::Destination);
        } else {
            if let Some(timestamp) = destination_timeout {
                if timestamp > post.timeout_timestamp.into() {
                    return Ok(MessageStatus::Timeout);
                }
            }
        }
    }





    let api = OnlineClient::<HyperClientApiConfig>::from_url(config.hyper_bridge_rpc_url).await?;
    let addr: [u8; 32] =
        hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb");
    let timestamp = api.rpc().storage(&addr, None).await.unwrap().unwrap();
    let hyperbridge_timestamp: u64 = codec::Decode::decode(&mut &*timestamp.0).unwrap();


    let build_leaf_index_query = LeafIndexQuery {
        source_chain: post.source,
        dest_chain: post.dest,
        nonce: post.nonce
    };
    let leaf_index_query = rpc_params![build_leaf_index_query];
    let hyper_bridge_response: Vec<Request> = api.rpc().request("ismp_queryRequests", leaf_index_query).await?;


    // check is the message is on hyper-bridge
    if let Some(request_) = hyper_bridge_response.get(0) {
        return Ok(MessageStatus::Hyperbridge);
    } else {
        if hyperbridge_timestamp > post.timeout_timestamp {
            return Ok(MessageStatus::Timeout);
        }
    }


    Ok(MessageStatus::Pending)
}


#[wasm_bindgen]
pub async fn query_response_status(
    res: WasmPostResponse,
    // config
    hyper_bridge_rpc_url: String,
    destination_ismp_host_address: String,
    source_ismp_host_address: String,
    destination_rpc_ws: String,
    source_rpc_ws: String
) -> Result<MessageStatus, anyhow::Error> {
    let post_response: PostResponse = res.into();
    let config = HyperClientConfig {
        hyper_bridge_rpc_url,
        destination_ismp_host_address: Address::from_str(destination_ismp_host_address.into()).unwrap(),
        source_ismp_host_address: Address::from_str(source_ismp_host_address.into()).unwrap(),
        destination_rpc_ws,
        source_rpc_ws
    };
    let source_provider = Arc::new(Provider::<Ws>::connect_with_reconnects(config.source_rpc_ws.clone(), 1000).await?);
    let source_host = EvmHost::new(config.source_ismp_host_address, source_provider);




    let source_timeout_timestamp = match post_response.dest_chain() {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let source_current_timestamp = source_host.timestamp().call().await?;
            Some(source_current_timestamp)
        },
        StateMachine::Polkadot(chain_id) => {
            Some(U256::zero())
        },
        StateMachine::Kusama(chain_id) => {
            Some(U256::zero())
        }
        _ => {
            None
        }
    };

    let source_response_commitment = match post_response.dest_chain() {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let req = Request::Post(post_response.post.clone());
            let hash = hash_request::<Hasher>(&req);
            let request_commitment = source_host.response_receipts(hash.into()).call().await?;
            Some(request_commitment)
        },
        // StateMachine::Polkadot(chain_id) => {
        //
        // },
        // StateMachine::Kusama(chain_id) => {
        //
        // }
        _ => {
            None
        }
    };


    if let Some(response_receipt) = source_response_commitment {
        if response_receipt.relayer != Address::from([0u8; 20]) {
            return Ok(MessageStatus::Destination);
        } else {
            if let Some(timestamp) = source_timeout_timestamp {
                if timestamp > post_response.timeout_timestamp.into() {
                    return Ok(MessageStatus::Timeout);
                }
            }
        }
    }


    let api = OnlineClient::<HyperClientApiConfig>::from_url(config.hyper_bridge_rpc_url).await?;
    let addr: [u8; 32] =
        hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb");
    let timestamp = api.rpc().storage(&addr, None).await.unwrap().unwrap();
    let hyperbridge_timestamp: u64 = codec::Decode::decode(&mut &*timestamp.0).unwrap();




    let build_leaf_index_query = LeafIndexQuery {
        source_chain: post_response.source_chain(),
        dest_chain: post_response.dest_chain(),
        nonce: post_response.nonce()
    };
    let leaf_index_query = rpc_params![build_leaf_index_query];
    let hyper_bridge_response: Vec<Request> = api.rpc().request("ismp_queryRequests", leaf_index_query).await?;


    // check is the message is on hyper-bridge
    if let Some(request_) = hyper_bridge_response.get(0) {
        return Ok(MessageStatus::Hyperbridge);
    } else {
        if hyperbridge_timestamp > post_response.timeout_timestamp {
            return Ok(MessageStatus::Timeout);
        }
    }


    Ok(MessageStatus::Pending)
}



#[wasm_bindgen]
pub fn timeout_request() {
    todo!()
}


#[wasm_bindgen]
pub fn timeout_response() {
    todo!()
}







pub struct Hasher;

impl Keccak256 for Hasher {
    fn keccak256(bytes: &[u8]) -> H256 {
        keccak_256(bytes).into()
    }
}


// flow
// 1. check if the request has been be submitted on the destination chain -> ProccessComplete
// 2. check if the request has been seen by hyperbridge -> On the Hyperbridge Hub
// 3. check is it was ever sent from the source chain -> MessageNeverLeftSource
// 4. return the progress status enum