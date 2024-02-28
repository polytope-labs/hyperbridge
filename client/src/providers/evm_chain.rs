use crate::{
    providers::global::Client,
    types::{
        request_commitment_key, response_commitment_key, BoxStream, EvmHost, EvmStateProof,
        PostRequestHandledFilter, ResponseReceipt,
    },
};
use anyhow::Error;
use codec::Encode;
use ethers::{
    middleware::Middleware,
    prelude::{H160, H256},
    providers::{Provider, Ws},
    types::Address,
    utils::keccak256,
};
use ismp::{
    consensus::{ConsensusStateId, StateMachineHeight, StateMachineId},
    events::Event,
    host::StateMachine,
    messaging::Proof,
};
use std::{str::FromStr, sync::Arc};
use std::time::Duration;
use ethers::prelude::ProviderExt;
use ethers::providers::Http;
use futures::stream;
use gloo_timers::future::TimeoutFuture;
use ismp::events::StateMachineUpdated;
use crate::types::{EvmHostEvents, HandlerV1, to_ismp_event, to_state_machine_updated};
use crate::types::StateMachineUpdatedFilter;






#[derive(Debug, Clone)]
pub struct EvmClient {
    // A WS rpc url of the EVM chain
    pub rpc_url: String,
    // Ethers provider instance
    pub client: Arc<Provider<Http>>,
    // Identifies the state machine this EVM client represents
    pub state_machine: StateMachine,
    // This is the Consensus State ID of the chain (e.g. BSC0)
    pub consensus_state_id: ConsensusStateId,
    // Address of the ISMP host of this state machine
    pub host_address: Address,
    // The ISMP handler address
    pub ismp_handler: H160,
}

impl EvmClient {
    // Creates an instance of an EVM client
    pub async fn new(
        rpc_url: String,
        consensus_state_id: ConsensusStateId,
        host_address: H160,
        handler_address: H160,
        state_machine: String,
    ) -> Result<Self, anyhow::Error> {
        let client =
            Arc::new(Provider::<Http>::connect(&rpc_url.clone()).await);
        let state_machine: StateMachine = StateMachine::from_str(&state_machine).unwrap();
        Ok(Self {
            rpc_url,
            client,
            state_machine,
            consensus_state_id,
            host_address: Address::from(host_address),
            ismp_handler: handler_address,
        })
    }
}


impl Client for EvmClient {
    async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error> {
        Ok(self.client.get_block_number().await?.as_u64())
    }

    fn state_machine_id(&self) -> Result<StateMachineId, anyhow::Error> {
        Ok(StateMachineId {
            state_id: self.state_machine,
            consensus_state_id: self.consensus_state_id,
        })
    }

    async fn host_timestamp(&self) -> Result<u64, anyhow::Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let current_host_time = host.timestamp().call().await?;
        Ok(current_host_time.as_u64())
    }

    async fn query_request_receipts(&self, request_hash: &H256) -> Result<H160, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let request_receipt = host.request_receipts(request_hash.0).call().await?;
        Ok(request_receipt)
    }

    async fn query_request_proof(
        &self,
        request_query_commitment: &H256,
        at: u64,
    ) -> Result<Proof, anyhow::Error> {
        let key = vec![request_commitment_key(request_query_commitment)];
        let proof = self.client.get_proof(self.host_address.clone(), key, Some(at.into())).await?;
        let proof = EvmStateProof {
            contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
            storage_proof: proof
                .storage_proof
                .into_iter()
                .map(|proof| {
                    (
                        keccak256(&proof.key.0).to_vec(),
                        proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
                    )
                })
                .collect(),
        };

        let query_proof = proof.encode();

        let proof = Proof {
            height: StateMachineHeight { id: self.state_machine_id()?, height: at },
            proof: query_proof,
        };

        Ok(proof)
    }

    async fn query_response_proof(
        &self,
        response_query_commitment: &H256,
        at: u64,
    ) -> Result<Proof, Error> {
        let key = vec![response_commitment_key(response_query_commitment)];
        let proof = self.client.get_proof(self.host_address.clone(), key, Some(at.into())).await?;
        let proof = EvmStateProof {
            contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
            storage_proof: proof
                .storage_proof
                .into_iter()
                .map(|proof| {
                    (
                        keccak256(&proof.key.0).to_vec(),
                        proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
                    )
                })
                .collect(),
        };

        let query_proof = proof.encode();

        let proof = Proof {
            height: StateMachineHeight { id: self.state_machine_id()?, height: at },
            proof: query_proof,
        };

        Ok(proof)
    }

    async fn query_response_receipts(
        &self,
        response_hash: &H256,
    ) -> Result<ResponseReceipt, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let response_receipt = host.response_receipts(response_hash.0).call().await.unwrap();

        Ok(response_receipt)
    }



    async fn event_stream(&self) -> Result<BoxStream<Event>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let client = self.clone();


        let stream = stream::unfold(
            (initial_height, client),
            move |(mut latest_height, client)|

                async move {
                    loop {
                        // wait for 15 seconds
                        TimeoutFuture::new(15000u32).await;
                        let block_number = client.client.get_block_number().await.ok()?.as_u64();

                        // see that the block number is not less than the latest height
                        if block_number < latest_height {
                            continue;
                        }

                        let contract_host = EvmHost::new(client.host_address, client.client.clone());
                        let results_host = contract_host
                            .events()
                            .address(client.host_address.into())
                            .from_block(latest_height)
                            .to_block(block_number)
                            .query()
                            .await.ok()?;

                        let mut events = results_host
                            .into_iter()
                            .filter_map(|ev| {
                                let event = to_ismp_event(ev.clone());

                                match event {
                                    Ok(event) => Some(event),
                                    Err(_) => None
                                }
                            })
                            .collect::<Vec<Event>>();


                        if let Some(event) = events.last() {
                            return Some((Ok(event.clone()), (block_number + 1, client)))
                        } else {
                            latest_height = block_number + 1;
                        }
                    }
                }
        );

        Ok(Box::pin(stream))
    }

    async fn post_request_handled_stream(
        &self,
    ) -> Result<BoxStream<PostRequestHandledFilter>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let client = self.clone();


        let stream = stream::unfold(
            (initial_height, client),
            move |(mut latest_height, client)|

                async move {
                    loop {
                        // wait for 15 seconds
                        TimeoutFuture::new(15000u32).await;
                        let block_number = client.client.get_block_number().await.ok()?.as_u64();

                        // see that the block number is not less than the latest height
                        if block_number < latest_height {
                            continue;
                        }

                        let contract = EvmHost::new(client.ismp_handler(), client.client.clone());
                        let results = contract
                            .events()
                            .address(client.ismp_handler().into())
                            .from_block(latest_height)
                            .to_block(block_number)
                            .query()
                            .await.ok()?;

                        let mut events = results
                            .into_iter()
                            .filter_map(|ev| {
                                if let EvmHostEvents::PostRequestHandledFilter(filter) = ev {
                                    Some(filter)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<PostRequestHandledFilter>>();

                        if let Some(event) = events.last() {
                            return Some((Ok(event.clone()), (block_number + 1, client)))
                        } else {
                            latest_height = block_number + 1;
                        }
                    }
                }
        );

        Ok(Box::pin(stream))
    }

    async fn query_state_machine_height(&self) -> Result<u64, Error> {
        let host = EvmHost::new(self.host_address, self.client.clone());
        let latest_state_machine_height = host.latest_state_machine_height().call().await.unwrap();

        Ok(latest_state_machine_height.as_u64())
    }

    async fn state_machine_update_notification(&self) -> Result<BoxStream<StateMachineUpdated>, Error> {
        let initial_height = self.client.get_block_number().await?.as_u64();
        let client = self.clone();


        let stream = stream::unfold(
            (initial_height, client),
            move |(mut latest_height, client)|

                async move {
                    loop {
                        TimeoutFuture::new(15000u32).await;
                        let block_number = client.client.get_block_number().await.ok()?.as_u64();

                        // see that the block number is not less than the latest height
                        if block_number < latest_height {
                            continue;
                        }

                        let contract = HandlerV1::new(client.ismp_handler(), client.client.clone());
                        let results = contract
                            .events()
                            .address(client.ismp_handler().into())
                            .from_block(latest_height)
                            .to_block(block_number)
                            .query()
                            .await.ok()?;

                        let mut events = results
                            .into_iter()
                            .map(|ev| ev.into())
                            .collect::<Vec<StateMachineUpdated>>();

                        // we only want the highest event
                        events.sort_by(|a, b| a.latest_height.cmp(&b.latest_height));

                        if let Some(event) = events.last() {
                            return Some((Ok(event.clone()), (block_number + 1, client)))
                        } else {
                            latest_height = block_number + 1;
                        }
                    }
                }
        );

        Ok(Box::pin(stream))
    }

    fn ismp_handler(&self) -> H160 {
        self.ismp_handler
    }
}


impl From<StateMachineUpdatedFilter> for StateMachineUpdated  {
    fn from(filter: StateMachineUpdatedFilter) -> Self {
        StateMachineUpdated {
            latest_height: filter.height.as_u64(),
            state_machine_id: StateMachineId {
                state_id: StateMachine::Kusama(filter.state_machine_id.as_u32()),
                consensus_state_id: *b"PARA",
            },
        }
    }
}













