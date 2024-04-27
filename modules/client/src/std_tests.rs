#![cfg(not(target_arch = "wasm32"))]
use crate::testing::{subscribe_to_request_status, test_timeout_request};

pub fn setup_logging() {
    use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};
    let filter =
        tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
    let _ = tracing_subscriber::fmt().with_env_filter(filter).finish().try_init();
}

#[tokio::test]
async fn std_tests() -> Result<(), anyhow::Error> {
    setup_logging();
    test_timeout_request().await?;
    subscribe_to_request_status().await?;
    Ok(())
}
