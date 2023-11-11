mod types;

use crate::types::{
    cross_chain_messenger::{CrossChainMessage, CrossChainMessenger, PostReceivedFilter},
    handler::StateMachineUpdatedFilter,
    ismp_host::PostRequestEventFilter,
    runtime::api::{
        ismp::Event as Ev,
        runtime_types::{frame_system::EventRecord, hyperbridge_runtime::RuntimeEvent},
    },
};
use anyhow::anyhow;
use clap::Parser;
use codec::Encode;
use debounced::Debounced;
use ethers::{
    abi::Address,
    contract::parse_log,
    core::k256::SecretKey,
    middleware::MiddlewareBuilder,
    providers::{Provider, Ws},
    signers::{LocalWallet, Signer},
    types::Log,
};
use futures::StreamExt;
use hex_literal::hex;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use ismp::host::{Ethereum, StateMachine};
use jsonrpsee::{
    core::{client::SubscriptionClientT, params::ObjectParams, traits::ToRpcParams},
    ws_client::{WsClient, WsClientBuilder},
};
use sp_core::{
    crypto::Pair,
    keccak_256,
    storage::{StorageChangeSet, StorageKey},
    H160,
};
use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    rpc_params,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    OnlineClient,
};
use types::{
    runtime::api::runtime_types::ismp::consensus::StateMachineId,
    token_faucet::TokenFaucet,
    token_gateway::{AssetReceivedFilter, SendParams, TokenGateway},
};

static CROSS_CHAIN_MESSENGER_ADDRESS: H160 = H160(hex!("96ae1E0309C38C594b3721a1256fC080ca3fE061"));
static GATEWAY_ADDRESS: H160 = H160(hex!("29311a33601ab2352d813992fa5cefe969ba45b1"));
static FAUCET_ADDRESS: H160 = H160(hex!("501d6bb926600cd5347d51d217e322999978cc1d"));
static MULTICHAIN_TOKEN: H160 = H160(hex!("87c686875dD4d74F32D6eF399d17425F0d9F77cc"));

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::builder()
        .filter_module("messenger", log::LevelFilter::Info)
        .format_module_path(false)
        .init();
    let args = Cli::parse();
    let source: Ethereum = args.source.into();

    // initialize clients
    let bytes = hex::decode(args.signer.as_str())?;
    let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
    let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
        .with_chain_id(source.chain_id());
    let provider = Provider::<Ws>::connect_with_reconnects(source.execution_rpc(), 1000).await?;
    let mut substrate =
        OnlineClient::<KeccakSubstrateChain>::from_url("ws://34.22.152.185:9933").await?;
    let signer = Arc::new(provider.clone().with_signer(signer));

    // initiate transaction
    let pb = progress_bar(format!("Sending transaction .."));
    let now = Instant::now();
    let (receipt, destination) = match args.action.clone() {
        Action::Gateway { to, amount, destination } => {
            let destination: Ethereum = destination.into();
            let gateway = TokenGateway::new(GATEWAY_ADDRESS, signer.clone());
            let receipt = gateway
                    .send(SendParams {
                        amount: (amount * 10u128.pow(18)).into(),
                        to: {
                            let to = to
                                .split("0x")
                                .last()
                                .ok_or_else(|| anyhow!("Invalid destination adress"))?;
                            let bytes = hex::decode(to)?;
                            if bytes.len() != 20 {
                                Err(anyhow!("Invalid destination adress"))?
                            }
                            H160::from_slice(&bytes)
                        },
                        dest: StateMachine::Ethereum(destination)
                            .to_string()
                            .as_bytes()
                            .to_vec()
                            .into(),
                        token_contract: MULTICHAIN_TOKEN,
                        timeout: 3 * 60 * 60,
                    })
                    .gas(100_000)
                    .send()
                    .await?
                    .await?
                    .ok_or_else(|| anyhow!("Transaction failed, please ensure you have some balance in your account, if not use the `drip` command first"))?;
            (receipt, destination)
        },
        Action::Drip => {
            let faucet = TokenFaucet::new(FAUCET_ADDRESS, signer.clone());
            let receipt = faucet
                .drip()
                .gas(100_000)
                .send()
                .await?
                .await?
                .ok_or_else(|| anyhow!("Transaction submission failed, try again"))?;

            if receipt.status == Some(0u64.into()) {
                pb.finish_with_message(format!(
                    "You can only use the token faucet once every 24 hours: {}, took: {}",
                    source.etherscan(receipt.transaction_hash),
                    HumanDuration(now.elapsed())
                ));
            } else {
                pb.finish_with_message(format!(
                    "Drip completed: {}, took: {}",
                    source.etherscan(receipt.transaction_hash),
                    HumanDuration(now.elapsed())
                ));
            }
            return Ok(())
        },
        Action::Post { destination, body } => {
            let destination: Ethereum = destination.into();
            let messenger = CrossChainMessenger::new(CROSS_CHAIN_MESSENGER_ADDRESS, signer.clone());
            let receipt = messenger
                .teleport(CrossChainMessage {
                    dest: StateMachine::Ethereum(destination)
                        .to_string()
                        .as_bytes()
                        .to_vec()
                        .into(),
                    message: body.as_bytes().to_vec().into(),
                    timeout: 3 * 60 * 60,
                })
                .gas(100_000)
                .send()
                .await?
                .await?
                .ok_or_else(|| anyhow!("Transaction failed i guess"))?;
            (receipt, destination)
        },
    };

    let mut rpc_client = WsClientBuilder::default().build(&destination.execution_rpc()).await?;

    let block_number = receipt.block_number.unwrap().as_u64();

    pb.finish_with_message(format!(
        "Cross chain message sent: {}, took: {}",
        source.etherscan(receipt.transaction_hash),
        HumanDuration(now.elapsed())
    ));
    let request = receipt
        .logs
        .iter()
        .filter_map(|log| parse_log::<PostRequestEventFilter>(log.clone()).ok())
        .collect::<Vec<_>>()
        .get(0)
        .cloned()
        .ok_or_else(|| anyhow!("Post Request was not sent, Ensure you have some balance by using the `drip` command before performing a token transfer"))?;

    // wait for Ethereum finality
    let now = Instant::now();
    let subscription = subscribe_storage(&mut substrate).await;
    let mut debounced_sub = Debounced::new(subscription, Duration::from_secs(4));
    let pb = progress_bar("Waiting for Ethereum to finalize your transaction".into());
    'outer: loop {
        let change_set = match debounced_sub.next().await {
            Some(Ok(change_set)) => change_set,
            Some(Err(_e)) => {
                log::error!("Error encountered in Ethereum finality stream: {_e:?}");
                continue
            },
            None => {
                panic!("Ethereum finality stream terminated unexpectedly");
            },
        };

        for (_key, change) in change_set.changes {
            if let Some(data) = change {
                let events = <Vec<EventRecord<RuntimeEvent, H256>> as codec::Decode>::decode(
                    &mut data.0.as_slice(),
                )?
                .into_iter()
                .filter_map(|ev| match ev.event {
                    RuntimeEvent::Ismp(event @ Ev::StateMachineUpdated { .. }) => Some(event),
                    _ => None,
                })
                .collect::<Vec<_>>();

                if events.is_empty() {
                    continue
                }

                for event in events {
                    match event {
                        Ev::StateMachineUpdated {
                            state_machine_id:
                                StateMachineId {state_id: types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                    network,
                                ), ..},
                            latest_height,
                        } => {
                            let event_source: Ethereum = network.into();
                            if event_source == source {
                                if latest_height >= block_number {
                                    pb.finish_with_message(format!("Ethereum has finalized your transaction at block: {latest_height}, took: {}",
                                        HumanDuration(now.elapsed())
                                    ));
                                    break 'outer
                                } else {
                                    pb.set_message(format!("Waiting for Ethereum to finalize your transaction, finalized: {latest_height}, your tx: {block_number}"));
                                }
                            }
                        },
                        _ => {},
                    }
                }
            }
        }
    }

    // wait for confirmation
    let now = Instant::now();
    let pb = progress_bar("Waiting for Hyperbridge to confirm your transaction".into());
    let header = 'outer: loop {
        let change_set = match debounced_sub.next().await {
            Some(Ok(change_set)) => change_set,
            Some(Err(_e)) => {
                log::error!("Error encountered in transaction confirmation stream: {_e:?}");
                continue
            },
            None => {
                panic!("Transaction confirmation stream terminated unexpectedly");
            },
        };

        for (_key, change) in change_set.changes {
            if let Some(data) = change {
                let events = <Vec<EventRecord<RuntimeEvent, H256>> as codec::Decode>::decode(
                    &mut data.0.as_slice(),
                )?
                .into_iter()
                .filter_map(|ev| match ev.event {
                    RuntimeEvent::Ismp(event @ Ev::Request { .. }) => Some(event),
                    _ => None,
                })
                .collect::<Vec<_>>();

                if events.is_empty() {
                    continue
                }

                for event in events {
                    match event {
                        Ev::Request {
                            source_chain:
                                types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                    source,
                                ),
                            dest_chain:
                                types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                    dest,
                                ),
                            request_nonce,
                        } => {
                            let event_source = StateMachine::from_str(&String::from_utf8_lossy(
                                request.source.as_ref(),
                            ))
                            .map_err(|e| anyhow!("{e}"))?;
                            let event_dest = StateMachine::from_str(&String::from_utf8_lossy(
                                request.dest.as_ref(),
                            ))
                            .map_err(|e| anyhow!("{e}"))?;

                            if StateMachine::Ethereum(source.into()) == event_source &&
                                StateMachine::Ethereum(dest.into()) == event_dest &&
                                request_nonce == request.nonce.as_u64()
                            {
                                let header = substrate
                                    .rpc()
                                    .header(Some(change_set.block))
                                    .await?
                                    .expect("Block is known; qed");
                                pb.finish_with_message(format!(
                                    "Hyperbridge has confirmed your transaction: https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fhyperbridge-rpc.blockops.network#/explorer/query/{}, took: {}", header.number,
                                    HumanDuration(now.elapsed())

                                ));

                                break 'outer header
                            }
                        },
                        _ => continue,
                    };
                }
            }
        }
    };

    let mut stream = {
        let stream =
            subscribe_logs(&mut rpc_client, &destination.execution_rpc(), destination.handler())
                .await
                .filter_map(|log| async move {
                    log.ok().and_then(|log| {
                        parse_log::<StateMachineUpdatedFilter>(log.clone())
                            .map(|ev| (ev.height.as_u32(), log.transaction_hash))
                            .ok()
                    })
                });
        Box::pin(stream)
    };

    // wait for hyperbridge finality
    let now = Instant::now();
    let pb = progress_bar("Waiting for Hyperbridge to finalize your transaction".into());
    loop {
        let (height, hash) = match stream.next().await {
            Some(height) => height,
            None => {
                panic!("Hyperbridge finality stream terminated unexpectedly");
            },
        };

        if height >= header.number {
            pb.finish_with_message(format!(
                "Hyperbridge has finalized your transaction: {}, took: {}",
                destination.etherscan(hash.unwrap()),
                HumanDuration(now.elapsed())
            ));
            break
        }
    }

    // wait for hyperbridge delivery
    let now = Instant::now();
    let pb = progress_bar("Waiting for Hyperbridge to deliver your transaction".into());
    match args.action {
        Action::Gateway { .. } => {
            let mut stream = {
                let stream =
                    subscribe_logs(&mut rpc_client, &destination.execution_rpc(), GATEWAY_ADDRESS)
                        .await
                        .filter_map(|log| async move {
                            log.ok().and_then(|log| {
                                parse_log::<AssetReceivedFilter>(log.clone())
                                    .map(|ev| (log.transaction_hash, ev))
                                    .ok()
                            })
                        });
                Box::pin(stream)
            };

            loop {
                let (hash, event) = match stream.next().await {
                    Some(message) => message,
                    None => {
                        panic!("Hyperbridge delivery stream terminated unexpectedly");
                    },
                };

                if event.nonce == request.nonce && request.source == event.source {
                    pb.finish_with_message(format!(
                        "Hyperbridge has delivered your transaction, check your wallet balance: {}, took: {}",
                        destination.etherscan(hash.unwrap()),
                        HumanDuration(now.elapsed())
                    ));

                    break
                }
            }
        },
        Action::Post { body, .. } => {
            let mut stream = {
                let stream = subscribe_logs(
                    &mut rpc_client,
                    &destination.execution_rpc(),
                    CROSS_CHAIN_MESSENGER_ADDRESS,
                )
                .await
                .filter_map(|log| async move {
                    log.ok().and_then(|log| {
                        parse_log::<PostReceivedFilter>(log.clone())
                            .map(|ev| (log.transaction_hash, ev))
                            .ok()
                    })
                });
                Box::pin(stream)
            };

            loop {
                let (hash, event) = match stream.next().await {
                    Some(message) => message,
                    None => {
                        panic!("Hyperbridge delivery stream terminated unexpectedly");
                    },
                };

                if body == event.message &&
                    event.nonce == request.nonce &&
                    request.source == event.source
                {
                    pb.finish_with_message(format!(
                        "Hyperbridge has delivered your transaction: {}, took: {}",
                        destination.etherscan(hash.unwrap()),
                        HumanDuration(now.elapsed())
                    ));

                    break
                }
            }
        },
        _ => unreachable!(),
    }

    Ok(())
}

fn progress_bar(msg: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&[
                "( ●    )",
                "(  ●   )",
                "(   ●  )",
                "(    ● )",
                "(     ●)",
                "(    ● )",
                "(   ●  )",
                "(  ●   )",
                "( ●    )",
                "(●     )",
            ]),
    );

    pb.set_message(msg);

    pb
}

async fn subscribe_logs(
    rpc_client: &mut WsClient,
    rpc_addr: &str,
    address: H160,
) -> jsonrpsee::core::client::Subscription<Log> {
    let mut obj = ObjectParams::new();
    let address = format!("{:?}", address);
    obj.insert("address", address.as_str())
        .expect("handler address should be valid");
    let param = obj.to_rpc_params().ok().flatten().expect("Failed to serialize rpc params");
    let reconnects = 10;
    for _ in 0..reconnects {
        let res = rpc_client
            .subscribe::<Log, _>(
                "eth_subscribe",
                jsonrpsee::rpc_params!("logs", param.clone()),
                "eth_unsubscribe",
            )
            .await;
        match res {
            Ok(sub) => return sub,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Ok(client) = WsClientBuilder::default().build(&rpc_addr).await {
                    *rpc_client = client
                }
            },
        }
    }
    panic!("Rpc provider is down, Your message will still be delivered by the relayer, check your wallet balance in about 30mins")
}

async fn subscribe_storage(
    substrate: &mut OnlineClient<KeccakSubstrateChain>,
) -> subxt::rpc::Subscription<StorageChangeSet<H256>> {
    let reconnects = 10;
    for _ in 0..reconnects {
        let res = substrate
            .rpc()
            .subscribe::<StorageChangeSet<H256>>(
                "state_subscribeStorage",
                rpc_params![vec![system_events_key()]],
                "state_unsubscribeStorage",
            )
            .await;
        match res {
            Ok(sub) => return sub,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Ok(client) =
                    OnlineClient::<KeccakSubstrateChain>::from_url("ws://34.22.152.185:9933").await
                {
                    *substrate = client
                }
            },
        }
    }
    panic!("Rpc provider is down, Your message will still be delivered by the relayer")
}

/// Some metadata about each chain
trait ChainInfo {
    fn execution_rpc(&self) -> String;

    fn chain_id(&self) -> u64;

    fn handler(&self) -> Address;

    fn etherscan(&self, transaction: H256) -> String;
}

impl ChainInfo for Ethereum {
    fn execution_rpc(&self) -> String {
        match self {
            Ethereum::Base =>
                "wss://small-compatible-yard.base-sepolia.quiknode.pro/ca5a8b1907ef065b10c260ecb13802b574e82220".to_string(),
            Ethereum::ExecutionLayer =>
                "wss://eth-sepolia.g.alchemy.com/v2/YfHY8davK9lcmyjydissrUxrc_gUbFjZ".to_string(),
            Ethereum::Optimism =>
                "wss://yolo-billowing-daylight.optimism-sepolia.quiknode.pro/ad2efd5fe6b1422db784640f7702552797ac12e0".to_string(),
            Ethereum::Arbitrum =>
                "wss://arb-sepolia.g.alchemy.com/v2/7cwLO5j3I9qI5KvLMjr_BmwXxaIPbRZi".to_string(),
        }
    }

    fn chain_id(&self) -> u64 {
        match self {
            Ethereum::ExecutionLayer => 5,
            Ethereum::Arbitrum => 421613,
            Ethereum::Optimism => 420,
            Ethereum::Base => 84531,
        }
    }

    fn handler(&self) -> Address {
        match self {
            Ethereum::ExecutionLayer => H160(hex!("1df0f722a40aaFB36B10edc6641201eD6ce37d91")),
            Ethereum::Arbitrum => H160(hex!("11f6d0323B4b8154b0b8874FB4183970bdd64C23")),
            Ethereum::Optimism => H160(hex!("394e341299A928bC72b01A56f22125921707D7F7")),
            Ethereum::Base => H160(hex!("A3002B1a247Fd8E2a2A5A4abFe76ca49A03B4063")),
        }
    }

    fn etherscan(&self, transaction: H256) -> String {
        match self {
            Ethereum::Base => format!("https://goerli.basescan.org/tx/{transaction:?}"),
            Ethereum::ExecutionLayer => format!("https://goerli.etherscan.io/tx/{transaction:?}"),
            Ethereum::Optimism => {
                format!("https://goerli-optimism.etherscan.io/tx/{transaction:?}")
            },
            Ethereum::Arbitrum => format!("https://testnet.arbiscan.io/tx/{transaction:?}"),
        }
    }
}

/// Implements [`subxt::Config`] for substrate chains with keccak as their hashing algorithm
#[derive(Clone)]
pub struct KeccakSubstrateChain;

/// A type that can hash values using the keccak_256 algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode)]
pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    type Output = H256;
    fn hash(s: &[u8]) -> Self::Output {
        keccak_256(s).into()
    }
}

impl subxt::Config for KeccakSubstrateChain {
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = KeccakHasher;
    type Header = SubstrateHeader<u32, KeccakHasher>;
    type ExtrinsicParams = PolkadotExtrinsicParams<Self>;
}

/// A simple CLI application for sending arbitrary messages through Hyperbridge
#[derive(Parser, Debug)]
pub struct Cli {
    /// Sub commands for the messenger
    #[command(subcommand)]
    action: Action,

    /// Raw, hex-encoded account secret key
    #[arg(short, long)]
    signer: String,

    /// The source network for the transaction
    #[arg(value_enum, short, long)]
    source: Network,
}

#[derive(Debug, clap::Subcommand, Clone)]
pub enum Action {
    /// Transfer tokens through the token gateway,
    /// Ensure you have some balance by running the `drip` command first
    Gateway {
        /// Account to receive this transfer on the destination chain
        #[arg(short, long)]
        to: String,
        /// Amount to be transferred
        #[arg(short, long)]
        amount: u128,
        /// The destination network for the token transfer
        #[arg(value_enum, short, long)]
        destination: Network,
    },
    /// Send a POST request with a custom body
    Post {
        /// The destination chain for the cross-chain message
        #[arg(value_enum, short, long)]
        destination: Network,

        /// Request body to be sent.
        #[arg(short, long)]
        body: String,
    },
    /// Get daily drip from the token faucet
    Drip,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Network {
    /// Goerli
    Sepolia,
    /// Optimism Sepolia
    OpSepolia,
    /// Arbitrum Sepolia
    ArbSepolia,
    /// Base Sepolia
    BaseSepolia,
}

impl From<types::runtime::api::runtime_types::ismp::host::Ethereum> for Ethereum {
    fn from(value: types::runtime::api::runtime_types::ismp::host::Ethereum) -> Self {
        match value {
            types::runtime::api::runtime_types::ismp::host::Ethereum::ExecutionLayer =>
                Ethereum::ExecutionLayer,

            types::runtime::api::runtime_types::ismp::host::Ethereum::Arbitrum =>
                Ethereum::Arbitrum,

            types::runtime::api::runtime_types::ismp::host::Ethereum::Optimism =>
                Ethereum::Optimism,

            types::runtime::api::runtime_types::ismp::host::Ethereum::Base => Ethereum::Base,
        }
    }
}

impl From<Network> for Ethereum {
    fn from(value: Network) -> Self {
        match value {
            Network::Sepolia => Ethereum::ExecutionLayer,
            Network::ArbSepolia => Ethereum::Arbitrum,
            Network::OpSepolia => Ethereum::Optimism,
            Network::BaseSepolia => Ethereum::Base,
        }
    }
}

// The storage key needed to access events.
pub fn system_events_key() -> StorageKey {
    let mut storage_key = sp_core::twox_128(b"System").to_vec();
    storage_key.extend(sp_core::twox_128(b"Events").to_vec());
    StorageKey(storage_key)
}
