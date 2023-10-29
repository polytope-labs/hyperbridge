mod types;

use crate::types::{
    handler::StateMachineUpdatedFilter,
    ismp_host::PostRequestEventFilter,
    ping_module::{PingMessage, PingModule, PostReceivedFilter},
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
    // types::TransactionReceipt,
};
use futures::StreamExt;
use hex_literal::hex;
use ismp::host::{Ethereum, StateMachine};
use jsonrpsee::{
    core::{client::SubscriptionClientT, params::ObjectParams, traits::ToRpcParams},
    ws_client::WsClientBuilder,
};
use sp_core::{
    crypto::Pair,
    keccak_256,
    storage::{StorageChangeSet, StorageKey},
    H160,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use subxt::{
    config::{polkadot::PolkadotExtrinsicParams, substrate::SubstrateHeader, Hasher},
    rpc_params,
    utils::{AccountId32, MultiAddress, MultiSignature, H256},
    OnlineClient,
};
trait ChainInfo {
    fn execution_rpc(&self) -> String;

    fn chain_id(&self) -> u64;

    fn ping_module(&self) -> Address;

    fn handler(&self) -> Address;
}

impl ChainInfo for Ethereum {
    fn execution_rpc(&self) -> String {
        match self {
            Ethereum::Base =>
                "wss://base-goerli.g.alchemy.com/v2/T61-8vjm3pbzXg8qv-xgCFuceiH-yKx8".to_string(),
            Ethereum::ExecutionLayer =>
                "wss://eth-goerli.g.alchemy.com/v2/ExCoqYRMmgK6D-XonUfuMfr8UJYuvH-q".to_string(),
            Ethereum::Optimism =>
                "wss://opt-goerli.g.alchemy.com/v2/K5J-ceP4ULgjvQuO2gPvikNoLE4oeuSO".to_string(),
            Ethereum::Arbitrum =>
                "wss://arb-goerli.g.alchemy.com/v2/_8F-Kfgm9ETkRHKII67WmQDgClbl7TJC".to_string(),
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

    fn ping_module(&self) -> Address {
        match self {
            Ethereum::ExecutionLayer => H160(hex!("be094ba30775301FDc5ABE6095e1457073825b40")),
            Ethereum::Arbitrum => H160(hex!("2Fc23c39Bd341ba467349725e6ab61B2DA9D49c1")),
            Ethereum::Optimism => H160(hex!("aA505C51C975ee19c5A2BB080245c20CCE6D3E51")),
            Ethereum::Base => H160(hex!("02b20A2db3c97203Da489a53ed3316D37389a779")),
        }
    }

    fn handler(&self) -> Address {
        match self {
            Ethereum::ExecutionLayer => H160(hex!("1df0f722a40aaFB36B10edc6641201eD6ce37d91")),
            Ethereum::Arbitrum => H160(hex!("11f6d0323B4b8154b0b8874FB4183970bdd64C23")),
            Ethereum::Optimism => H160(hex!("394e341299A928bC72b01A56f22125921707D7F7	")),
            Ethereum::Base => H160(hex!("A3002B1a247Fd8E2a2A5A4abFe76ca49A03B4063")),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();
    let source: Ethereum = args.source.into();
    let destination: Ethereum = args.destination.into();

    let bytes = hex::decode(args.signer.as_str())?;
    let signer = sp_core::ecdsa::Pair::from_seed_slice(&bytes)?;
    let signer = LocalWallet::from(SecretKey::from_slice(signer.seed().as_slice())?)
        .with_chain_id(source.chain_id());
    let provider = Provider::<Ws>::connect_with_reconnects(source.execution_rpc(), 1000).await?;
    let substrate = OnlineClient::<KeccakSubstrateChain>::from_url(
        "wss://hyperbridge-rpc.blockops.network:443",
    )
    .await?;
    let rpc_client = WsClientBuilder::default().build(&destination.execution_rpc()).await?;
    let signer = Arc::new(provider.clone().with_signer(signer));

    let messenger = PingModule::new(source.ping_module(), signer.clone());

    let receipt = messenger
        .ping(PingMessage {
            dest: StateMachine::Ethereum(destination).to_string().as_bytes().to_vec().into(),
            module: destination.ping_module(),
            timeout: 3 * 60 * 60,
        })
        .gas(100_000)
        .send()
        .await?
        .await?
        .ok_or_else(|| anyhow!("transaction failed i guess"))?;
    let block_number = receipt.block_number.unwrap().as_u64();
    dbg!(block_number);

    let request = parse_log::<PostRequestEventFilter>(receipt.logs[0].clone())?;
    dbg!(&request);

    let subscription = substrate
        .rpc()
        .subscribe::<StorageChangeSet<H256>>(
            "state_subscribeStorage",
            rpc_params![vec![system_events_key()]],
            "state_unsubscribeStorage",
        )
        .await
        .expect("Storage subscription failed");
    let mut debounced_sub = Debounced::new(subscription, Duration::from_secs(4));

    'outer: loop {
        let change_set = match debounced_sub.next().await {
            Some(Ok(change_set)) => {
                println!("Got changeset for state machine");
                change_set
            },
            Some(Err(e)) => {
                println!("Some error {e:?}");
                continue
            },
            None => {
                panic!("Got None");
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

                dbg!(&events);

                for event in events {
                    match event {
                       Ev::StateMachineUpdated {
                            state_machine_id,
                            latest_height,
                        } => {
                            if let types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                network,
                            ) = state_machine_id.state_id
                            {
                                let ethereum: Ethereum = network.into();
                                if ethereum == source && latest_height >= block_number {
                                    println!("Found StateMachineUpdated");
                                    break 'outer
                                }
                            }
                        },
                        _ => continue
                    }
                }
            }
        }
    }

    let header = 'outer: loop {
        let change_set = match debounced_sub.next().await {
            Some(Ok(change_set)) => {
                println!("Got changeset for request");
                change_set
            },
            Some(Err(e)) => {
                println!("Some error {e:?}");
                continue
            },
            None => {
                panic!("Got None");
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

                dbg!(&events);

                if events.is_empty() {
                    continue
                }

                for event in events {
                    match event {
                        Ev::Request { source_chain, dest_chain, request_nonce } => {
                            let (source, dest) = match (source_chain, dest_chain) {
                                    (
                                        types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                            source,
                                        ),
                                        types::runtime::api::runtime_types::ismp::host::StateMachine::Ethereum(
                                            dest,
                                        ),
                                    ) =>
                                        (StateMachine::Ethereum(source.into()), StateMachine::Ethereum(dest.into())),
                                    _ => continue,
                                };

                            let event_source = StateMachine::from_str(&String::from_utf8_lossy(
                                request.source.as_ref(),
                            ))
                            .map_err(|e| anyhow!("{e}"))?;
                            let event_dest = StateMachine::from_str(&String::from_utf8_lossy(
                                request.dest.as_ref(),
                            ))
                            .map_err(|e| anyhow!("{e}"))?;
                            dbg!((&event_source, &event_dest));
                            if source == event_source &&
                                dest == event_dest &&
                                request_nonce == request.nonce.as_u64()
                            {
                                println!("Found Request");
                                let header = substrate
                                    .rpc()
                                    .header(Some(change_set.block))
                                    .await?
                                    .expect("Block is known; qed");

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
        let mut obj = ObjectParams::new();
        let address = format!("{:?}", destination.handler());
        obj.insert("address", address.as_str())
            .expect("handler address should be valid");
        let param = obj.to_rpc_params().ok().flatten().expect("Failed to serialize rpc params");
        let sub = rpc_client
            .subscribe::<Log, _>(
                "eth_subscribe",
                jsonrpsee::rpc_params!("logs", param),
                "eth_unsubscribe",
            )
            .await
            .expect("Failed to susbcribe");
        let stream = sub.filter_map(|log| async move {
            log.ok().and_then(|log| {
                parse_log::<StateMachineUpdatedFilter>(log).map(|ev| ev.height.as_u32()).ok()
            })
        });
        Box::pin(stream)
    };

    loop {
        let height = match stream.next().await {
            Some(height) => {
                println!("Got new height");
                height
            },
            None => {
                panic!("Got None");
            },
        };

        if height >= header.number {
            println!("Found parachain height");
            break
        }
    }

    let mut stream = {
        let mut obj = ObjectParams::new();
        let address = format!("{:?}", destination.ping_module());
        obj.insert("address", address.as_str())
            .expect("handler address should be valid");
        let param = obj.to_rpc_params().ok().flatten().expect("Failed to serialize rpc params");
        let sub = rpc_client
            .subscribe::<Log, _>(
                "eth_subscribe",
                jsonrpsee::rpc_params!("logs", param),
                "eth_unsubscribe",
            )
            .await
            .expect("Failed to susbcribe");
        let stream = sub.filter_map(|log| async move {
            log.ok().and_then(|log| {
                parse_log::<PostReceivedFilter>(log.clone())
                    .map(|ev| (log.block_hash, ev.message))
                    .ok()
            })
        });
        Box::pin(stream)
    };

    loop {
        let (_block_hash, message) = match stream.next().await {
            Some(message) => {
                println!("Got new message: {}", message.1);
                message
            },
            None => {
                panic!("Got None");
            },
        };

        if args.message == message {
            println!("Found message");
            break
        }
    }

    Ok(())
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

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
pub struct Cli {
    /// Raw account secret key
    #[arg(short, long)]
    signer: String,

    /// The source chain for the cross-chain message
    #[arg(value_enum, short, long)]
    source: Network,

    /// The destination chain for the cross-chain message
    #[arg(value_enum, short, long)]
    destination: Network,

    /// Cross-chain message to be sent.
    #[arg(short, long)]
    message: String,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Network {
    /// Ethereum Execution layer
    Goerli,
    /// The optimism state machine
    OpGoerli,
    /// The Arbitrum state machine
    ArbGoerli,
    /// The Base state machine
    BaseGoerli,
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
            Network::Goerli => Ethereum::ExecutionLayer,
            Network::ArbGoerli => Ethereum::Arbitrum,
            Network::OpGoerli => Ethereum::Optimism,
            Network::BaseGoerli => Ethereum::Base,
        }
    }
}

// The storage key needed to access events.
pub fn system_events_key() -> StorageKey {
    let mut storage_key = sp_core::twox_128(b"System").to_vec();
    storage_key.extend(sp_core::twox_128(b"Events").to_vec());
    StorageKey(storage_key)
}
