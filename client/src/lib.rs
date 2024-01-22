use std::sync::Arc;
use ethers::prelude::H256;
use ethers::providers::{Provider, Ws};
use ethers::types::{Address, H160};
use ismp::router::{Post, Request};
use wasm_bindgen::prelude::*;
use crate::types::{build_post, MessageStatus, WasmPost};
use subxt::{client::OnlineClient, PolkadotConfig, rpc_params};
use ismp::LeafIndexQuery;
use ismp_solidity_abi::evm_host::EvmHost;
use subxt::ext::sp_core::keccak_256;
use ismp::host::{Ethereum, StateMachine};
use ismp::util::{hash_request, Keccak256};


pub mod types;
type HyperClientApiConfig = PolkadotConfig;

struct HyperClientConfig {
    hyper_bridge_rpc_url: String,
    destination_ismp_host_address: H160,
    destination_rpc_ws: String
}




#[wasm_bindgen]
pub async fn query_request_status(
    request: WasmPost,
    config: HyperClientConfig,
) -> Result<MessageStatus, anyhow::Error>{
    let mut return_data = MessageStatus::Pending;


    let post = build_post(request);
    let provider = Arc::new(Provider::<Ws>::connect_with_reconnects(config.destination_rpc_ws.clone(), 1000).await?);
    let dest_host = EvmHost::new(config.destination_ismp_host_address, provider);

    let destination_timeout = match post.dest {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let destination_current_timestamp = dest_host.timestamp().call().await?;
            Some(destination_current_timestamp)
        },
        StateMachine::Polkadot(chain_id) => {

        },
        StateMachine::Kusama(chain_id) => {

        }
        _ => {
            None
        }
    };

    // checking if the request has timed-out
    if let Some(timestamp) = destination_timeout {
        if timestamp > post.timeout_timestamp.into() {
            return_data = MessageStatus::Timeout;
        }
    }


    let destination_receipt = match post.dest {
        StateMachine::Bsc | StateMachine::Polygon | StateMachine::Ethereum(Ethereum::Arbitrum) | StateMachine::Ethereum(Ethereum::Base) | StateMachine::Ethereum(Ethereum::Optimism) | StateMachine::Ethereum(Ethereum::ExecutionLayer) => {
            let req = Request::Post(post.clone());
            let hash = hash_request::<Hasher>(&req);
            let request_commitment = dest_host.request_receipts(hash.into()).call().await?;
            Some(request_commitment)
        },
        StateMachine::Polkadot(chain_id) => {

        },
        StateMachine::Kusama(chain_id) => {

        }
        _ => {
            None
        }
    };

    // check if the message has reached the destination
    if let Some(addr) = destination_receipt {
        if addr == Address::from([0u8; 20]) {
            return_data = MessageStatus::Destination;
        }
    }



    let api = OnlineClient::<HyperClientConfig>::from_url(config.hyper_bridge_rpc_url).await?;
    let build_leaf_index_query = LeafIndexQuery {
        source_chain: post.source,
        dest_chain: post.dest,
        nonce: post.nonce
    };
    let leaf_index_query = rpc_params![build_leaf_index_query];
    let hyper_bridge_response: Vec<Request> = api.rpc().request("ismp_queryRequests", leaf_index_query).await?;


    // check is the message is on hyper-bridge
    if let Some(request_) = hyper_bridge_response.get(0) {
        return_data = MessageStatus::Hyperbridge
    }


    Ok(return_data)
}


#[wasm_bindgen]
pub fn query_response_status() {
    todo!()
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