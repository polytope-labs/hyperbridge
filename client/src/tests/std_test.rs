use std::sync::Arc;

use crate::{internals::query_request_status_internal, mock::ping_module::{PingMessage, PingModule}, providers::evm_chain::EvmClient, types::MessageStatus};
use ethers::{core::k256::SecretKey, middleware::MiddlewareBuilder, prelude::{H160, U256}, providers::Middleware, signers::{LocalWallet, Signer}, types::Bytes};
use hex_literal::hex;
use ismp::{host::{Ethereum, StateMachine}, router::Post};
use sp_core::Pair;


// Ethereum chain constants
const ISMP_HANDLER_ETHEREUM: H160 = H160(hex!("574f5260097C90c30427846A560Ae7696A287C56"));
const TEST_HOST_ETHEREUM: H160 = H160(hex!("3C51029d8b53f00384272AaFd92BA5c50F94EE6E"));
const MOCK_MODULE_ETHEREUM: H160 = H160(hex!("3F076aE33723b2F61656166D40a78d409e350625"));



// OPTIMISM chain constants
const ISMP_HANDLER_OPTIMISM: H160 = H160(hex!("574f5260097C90c30427846A560Ae7696A287C56"));
const TEST_HOST_OPTIMISM: H160 = H160(hex!("3C51029d8b53f00384272AaFd92BA5c50F94EE6E"));
const MOCK_MODULE_OPTIMISM: H160 = H160(hex!("3F076aE33723b2F61656166D40a78d409e350625"));



#[tokio::test]
async fn test_query_request_status_internal() -> Result<(), anyhow::Error> {
    let eth_stat_machine_raw = StateMachine::Ethereum(Ethereum::ExecutionLayer);
    let eth_State_chaine_bytes = eth_stat_machine_raw.to_string().as_bytes().to_vec();

    
    let ethereum_state_machine = EvmClient::new(
        "https://eth-sepolia.g.alchemy.com/v2/tKtJs47xn9LPe8d99J0L06Ixg3bsHGIR".to_string(), 
        *b"ETH0", 
        TEST_HOST_ETHEREUM, 
        ISMP_HANDLER_ETHEREUM, 
        "ETHE".to_string()
    ).await?;


    let signer = sp_core::ecdsa::Pair::from_seed_slice(&hex!(
        "6456101e79abe59d2308d63314503446857d4f1f949468bf5627e86e3d6adebd"
    ))?;
    let signer =
        LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
            .with_chain_id(ethereum_state_machine.client.clone().get_chainid().await?.low_u64());
    let client_with_signer = Arc::new(ethereum_state_machine.client.clone().with_signer(signer));
    let ping_module_instance = PingModule::new(MOCK_MODULE_ETHEREUM, client_with_signer.clone());


    println!("Pinging.....");
    let ping_message = PingMessage {
        dest: eth_State_chaine_bytes.into(),
        module: ISMP_HANDLER_ETHEREUM,
        timeout: 10 * 60 * 60,
        count: U256::from(100),
        fee: U256::from(900_000_000_000_000_000u128)
    };

    let response = ping_module_instance.ping(ping_message).send().await?;
    let ping_tx_event = 
    let post: Post = Post { 
        source: todo!(), 
        dest: todo!(), 
        nonce: todo!(), 
        from: todo!(), 
        to: todo!(), 
        timeout_timestamp: todo!(), 
        data: todo!(), 
        gas_limit: todo!() 
    };




    loop {
        let current_ping_status = query_request_status_internal(
            post, 
            config
        ).await?;


        match current_ping_status {
            MessageStatus::Destination => {
                println!("Ping message has reached the destination chain [breaking -->]");
                break;
            },
            MessageStatus::Hyperbridge => {
                println!("Ping message has reached the hyperbridge");
            },
            MessageStatus::Timeout => {
                println!("Ping message has timed out [breaking -->]");
                break;
            },
            MessageStatus::Pending => {
                println!("Ping message is still pending");
            },
            MessageStatus::HyperbridgeFinalized => {
                println!("Ping message has been finalized on the hyperbridge");
            },
            MessageStatus::NotTimedOut => {
                println!("Ping message has not timed out");
            }
        }
    }


    Ok(())
}


#[tokio::test]
async fn test_query_response_status_internal() -> Result<(), anyhow::Error> {

    Ok(())
}


#[tokio::test]
async fn test_timeout_request_internal() -> Result<(), anyhow::Error> {

    Ok(())
}

#[tokio::test]
async fn test_timeout_response_internal() -> Result<(), anyhow::Error> {

    Ok(())
}