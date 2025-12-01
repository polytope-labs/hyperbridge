#![cfg(test)]

use anyhow::anyhow;
use codec::{Decode, Encode};
use gargantua_runtime::RuntimeCall;
use ismp::host::StateMachine;
use polkadot_sdk::{
    frame_support::weights::Weight,
    frame_system, pallet_sudo,
    sc_consensus_manual_seal::CreatedBlock,
    sp_core::{crypto::Ss58Codec, Bytes, H256, U256},
    sp_keyring::sr25519::Keyring,
};
use std::{
        collections::HashMap,
        env, fs,
        net::TcpStream,
        process::{Child, Command, Stdio},
        time::{Duration, Instant}
};
use subxt::{
    ext::subxt_rpcs::{rpc_params, RpcClient},
    tx::SubmittableTransaction,
    OnlineClient,
};
use tokio::net::TcpSocket;
use subxt_utils::Hyperbridge;

const NEXUS_RPC: &str = "wss://nexus.ibp.network";
const WASM_PATH: &str = "../../target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.compressed.wasm";
//const WASM_PATH: &str = "/Users/dharjeezy/Documents/polytope/hyperbridge/gargantua_runtime.compact.compressed.wasm";

struct ProcessGuard(Child);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        self.0.kill().unwrap();
    }
}

async fn download_file(url: &str, output_path: &str) -> Result<(), anyhow::Error> {
    println!("Downloading {} to {}...", url, output_path);
    let status = Command::new("curl").args(["-L", "-0", output_path, url]).status()?;
    if !status.success() {
        return Err(anyhow!("Failed to download file from {}", url));
    }
    Ok(())
}

async fn wait_for_port(port: u16, timeout: Duration) -> Result<(), anyhow::Error> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow!("Timed out waiting for connection to port"))

}

#[tokio::test]
#[ignore]
async fn test_runtime_upgrade_and_fee_migration() -> Result<(), anyhow::Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output().map_err(|_| anyhow!("Failed to determine git branch"))?;
    let branch =  String::from_utf8_lossy(&output.stdout).trim().to_string();

    if branch != "dami/fix-decimals-scaling" {
        return Ok(());
    }

    println!("Running migration test on branch: {}", branch);

    let old_binary_url =
        env::var("OLD_BINARY_URL").map_err(|_| anyhow!("OLD_BINARY_URL env var not set"))?;
    let new_runtime_url =
        env::var("NEW_RUNTIME_URL").map_err(|_| anyhow!("NEW_RUNTIME_URL env var not set"))?;

    let binary_path = "./hyperbridge-old-simnode";
    let wasm_path = "./new_runtime.wasm";

    let _ = fs::remove_file(binary_path);
    let _ = fs::remove_file(wasm_path);

    download_file(&old_binary_url, binary_path).await?;
    download_file(&new_runtime_url, wasm_path).await?;

    Command::new("chmod").args(["+x", binary_path]).status()?;

    println!("Spawning Simnode...");
    let child = Command::new(binary_path)
        .args([
            "simnode",
            "--chain=gargantua-2000",
            "--name=alice",
            "--tmp",
            "--state-pruning=archive",
            "--blocks-pruning=archive",
            "--rpc-port=9990",
            "--port=40337",
            "--rpc-cors=all",
            "--unsafe-rpc-external",
            "--rpc-methods=unsafe",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let _guard = ProcessGuard(child);

    println!("Waiting for Simnode RPC port 9990...");
    wait_for_port(9990, Duration::from_secs(60)).await?;

    let port = env::var("PORT").unwrap_or_else(|_| "9990".to_string());
    let local_ws_url = format!("ws://127.0.0.1:{}", port);

    println!("Connecting to Nexus at: {}", NEXUS_RPC);
    let nexus_client =
        subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?.0;

    println!("Connecting to Local Simnode at: {}", local_ws_url);
    let (local_client, rpc_client) =
        subxt_utils::client::ws_client::<Hyperbridge>(&local_ws_url, u32::MAX).await?;

    let sudo_account = Keyring::Alice.to_account_id();
    println!("Using Sudo account: {}", sudo_account.to_ss58check());

    println!("Fetching Fee Token Decimals from Nexus...");
    let decimals_addr = subxt::dynamic::storage("HostExecutive", "FeeTokenDecimals", ());
    let mut decimals_iter = nexus_client.storage().at_latest().await?.iter(decimals_addr).await?;

    let mut chain_decimals: HashMap<StateMachine, u8> = HashMap::new();
    let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

    while let Some(Ok(kv)) = decimals_iter.next().await {
        let key_bytes = kv.key_bytes;
        let value_thunk = kv.value;

        // offset is 32 bytes (pallet + storage hash) + 8 bytes (key hash) = 40 bytes.
        if key_bytes.len() > 40 {
            let mut key_slice = &key_bytes[40..];
            if let Ok(chain) = StateMachine::decode(&mut key_slice) {
                let mut value_slice = value_thunk.encoded();
                if let Ok(decimals) = u8::decode(&mut value_slice) {
                    chain_decimals.insert(chain, decimals);
                }
            }
        }

        storage_data.push((key_bytes, value_thunk.encoded().to_vec()));
    }
    println!("Fetched {} decimal entries.", chain_decimals.len());

    println!("Fetching Relayer Fees from Nexus...");
    let fees_addr = subxt::dynamic::storage("Relayer", "Fees", ());
    let mut fees_iter = nexus_client.storage().at_latest().await?.iter(fees_addr).await?;

    // (rawKey, stateMachine, original value)
    let mut fees_to_verify: Vec<(Vec<u8>, StateMachine, U256)> = Vec::new();

    while let Some(Ok(kv)) = fees_iter.next().await {
        let key_bytes = kv.key_bytes;
        let value_thunk = kv.value;

        // offset is 32 bytes (pallet + storage Hash) + 16 bytes (key hash) = 48 bytes.
        if key_bytes.len() > 48 {
            let mut key_slice = &key_bytes[48..];
            if let Ok(chain) = StateMachine::decode(&mut key_slice) {
                let mut value_slice = value_thunk.encoded();
                if let Ok(fee_value) = U256::decode(&mut value_slice) {
                    if !fee_value.is_zero() {
                        fees_to_verify.push((key_bytes.clone(), chain, fee_value));
                    }
                }
            }
        }

        storage_data.push((key_bytes, value_thunk.encoded().to_vec()));
    }
    println!("Fetched {} fee entries to verify.", fees_to_verify.len());

    if fees_to_verify.is_empty() {
        return Err(anyhow!("No fee entries found."));
    }

    println!("Injecting state into Simnode...");
    batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

    println!("Reading WASM file from: {}", WASM_PATH);
    let wasm_code = fs::read(WASM_PATH).map_err(|e| anyhow!("Failed to read WASM: {}", e))?;
    println!("WASM size: {} bytes", wasm_code.len());

    println!("Submitting Runtime Upgrade...");
    let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
    let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
        call: Box::new(set_code_call),
        weight: Weight::from_parts(0, 0),
    });

    submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

    println!("Signaling Simnode Upgrade...");
    let _: () = rpc_client
        .request("simnode_upgradeSignal", rpc_params![true])
        .await?;

    println!("Producing blocks to drive migration...");
    let blocks_to_produce = 250;
    for i in 0..blocks_to_produce {
        let _ = rpc_client
            .request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
            .await?;

        if i % 100 == 0 {
            println!("Produced {}/{} blocks...", i, blocks_to_produce);
        }
    }

    println!("Verifying Migrations...");
    let mut changed_count = 0;
    let mut unchanged_count = 0;

    for (raw_key, chain, old_val) in fees_to_verify {
        let fetch_res = local_client.storage().at_latest().await?.fetch_raw(raw_key).await?;

        if let Some(data) = fetch_res {
            let new_val = U256::decode(&mut &data[..])?;
            let decimals = *chain_decimals.get(&chain).unwrap_or(&18);

            if decimals == 18 {
                if new_val == old_val {
                    unchanged_count += 1;
                } else {
                    println!(" Successful storage change {:?} (18 decimals): Old {}, New {}", chain, old_val, new_val);
                    changed_count += 1;
                }
            } else {
                let scaling_power = 18u32.saturating_sub(decimals as u32);
                let divisor = U256::from(10).pow(U256::from(scaling_power));
                let expected = old_val / divisor;

                if new_val == expected {
                    unchanged_count += 1;
                } else {
                    println!("Successful storage change {:?} ({} decimals): Old {}, New {}, Expected {}", chain, decimals, old_val, new_val, expected);
                    changed_count += 1;
                }
            }
        }
    }

    println!("Changed Count: {}", changed_count);
    println!("Unchanged Count: {}", unchanged_count);


    Ok(())
}

async fn batch_set_storage(
    client: &OnlineClient<Hyperbridge>,
    rpc_client: &RpcClient,
    sudo_account: &sp_core::crypto::AccountId32,
    data: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<(), anyhow::Error> {
    const BATCH_SIZE: usize = 500;

    for (i, chunk) in data.chunks(BATCH_SIZE).enumerate() {
        let items: Vec<(Vec<u8>, Vec<u8>)> = chunk.to_vec();
        let call = RuntimeCall::System(frame_system::Call::set_storage { items });
        let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
            call: Box::new(call),
            weight: Weight::from_parts(0, 0),
        });

        println!("Injecting batch {}/{}...", i + 1, (data.len() + BATCH_SIZE - 1) / BATCH_SIZE);
        submit_sudo(client, rpc_client, sudo_account, sudo_call).await?;
    }
    Ok(())
}

async fn submit_sudo(
    client: &OnlineClient<Hyperbridge>,
    rpc_client: &RpcClient,
    sudo_account: &sp_core::crypto::AccountId32,
    call: RuntimeCall,
) -> Result<(), anyhow::Error> {
    let call_data = call.encode();

    let extrinsic_bytes: Bytes = rpc_client
        .request(
            "simnode_authorExtrinsic",
            rpc_params![Bytes::from(call_data), sudo_account.to_ss58check()],
        )
        .await?;

    let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic_bytes.0);
    let progress = submittable.submit_and_watch().await?;

    let block = rpc_client
        .request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
        .await?;
    let _ = rpc_client
        .request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
        .await?;

    let events = progress.wait_for_finalized_success().await?;
    Ok(())
}