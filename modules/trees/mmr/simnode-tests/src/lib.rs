#![cfg(test)]

mod runtime;
use sp_core::crypto::Ss58Codec;
use std::env;
use subxt::{OnlineClient, SubstrateConfig};

#[tokio::test]
async fn test_all_features() -> Result<(), anyhow::Error> {
    dispatch_requests().await?;
    Ok(())
}

async fn dispatch_requests() -> Result<(), anyhow::Error> {
    let port = env::var("PORT").unwrap_or("9944".into());
    let client =
        OnlineClient::<SubstrateConfig>::from_url(format!("ws://127.0.0.1:{}", port)).await?;

    // Generate some requests and proofs for a substrate state machine

    // Set state machine commitment on chain

    // Submit requests

    Ok(())
}
