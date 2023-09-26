use crate::{
    abi::{
        beefy::BeefyConsensusState,
        ismp_handler::{
            GetRequest, GetResponseMessage, GetTimeoutMessage, IsmpHandler, PostRequest,
            PostRequestLeaf, PostRequestMessage, PostResponse, PostResponseLeaf,
            PostResponseMessage, PostTimeoutMessage, Proof,
        },
        IIsmpHost,
    },
    EvmClient,
};
use anyhow::{anyhow, Error};
use beefy_verifier_primitives::{BeefyNextAuthoritySet, ConsensusState};
use codec::Encode;
use consensus_client::types::EvmStateProof;
use ethers::{abi::AbiDecode, providers::Middleware};
use futures::stream::StreamExt;
use ismp::{
    consensus::{ConsensusStateId, StateMachineId},
    events::Event,
    host::StateMachine,
    messaging::{Message, ResponseMessage, TimeoutMessage},
    router::{Get, Request, Response},
};
use ismp_primitives::{MembershipProof, SubstrateStateProof};
use merkle_mountain_range_labs::mmr_position_to_k_index;
use pallet_ismp::NodesUtils;
use patricia_merkle_trie::StorageProof;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider, Query, StateMachineUpdated};

#[async_trait::async_trait]
impl<I: IsmpHost> IsmpProvider for EvmClient<I>
where
    I: Send + Sync,
{
    async fn query_consensus_state(
        &self,
        at: Option<u64>,
        _: ConsensusStateId,
    ) -> Result<Vec<u8>, Error> {
        let contract = IIsmpHost::new(self.ismp_host_address, self.client.clone());
        let value = {
            let call = if let Some(block) = at {
                contract.consensus_state().block(block)
            } else {
                contract.consensus_state()
            };
            call.call().await?
        };

        let beefy_consensus_state = BeefyConsensusState::decode(&value.0)?;
        // Convert this bytes into BeefyConsensusState for rust and scale encode
        let consensus_state = ConsensusState {
            latest_beefy_height: beefy_consensus_state.latest_height.as_u32(),
            mmr_root_hash: Default::default(),
            beefy_activation_block: beefy_consensus_state.beefy_activation_block.as_u32(),
            current_authorities: BeefyNextAuthoritySet {
                id: beefy_consensus_state.current_authority_set.id.as_u64(),
                len: beefy_consensus_state.current_authority_set.len.as_u32(),
                keyset_commitment: H256::from_slice(
                    beefy_consensus_state.current_authority_set.root.as_slice(),
                ),
            },
            next_authorities: BeefyNextAuthoritySet {
                id: beefy_consensus_state.next_authority_set.id.as_u64(),
                len: beefy_consensus_state.next_authority_set.len.as_u32(),
                keyset_commitment: H256::from_slice(
                    beefy_consensus_state.next_authority_set.root.as_slice(),
                ),
            },
        };
        Ok(consensus_state.encode())
    }

    async fn query_latest_state_machine_height(&self, _id: StateMachineId) -> Result<u32, Error> {
        let contract = IIsmpHost::new(self.ismp_host_address, self.client.clone());
        let value = contract.latest_state_machine_height().call().await?;
        Ok(value.low_u64() as u32)
    }

    async fn query_consensus_update_time(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
        let contract = IIsmpHost::new(self.ismp_host_address, self.client.clone());
        let value = contract.consensus_update_time().call().await?;
        Ok(Duration::from_secs(value.low_u64()))
    }

    async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
        let contract = IIsmpHost::new(self.ismp_host_address, self.client.clone());
        let value = contract.challenge_period().call().await?;
        Ok(Duration::from_secs(value.low_u64()))
    }

    async fn query_timestamp(&self) -> Result<Duration, Error> {
        let client = Arc::new(self.client.clone());
        let contract = IIsmpHost::new(self.ismp_host_address, client);
        let value = contract.timestamp().call().await?;
        Ok(Duration::from_secs(value.low_u64()))
    }

    async fn query_requests_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
        let keys =
            keys.into_iter().map(|query| self.request_commitment_key(query.commitment)).collect();

        let proof = self.client.get_proof(self.ismp_host_address, keys, Some(at.into())).await?;
        let proof = EvmStateProof {
            contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
            storage_proof: proof
                .storage_proof
                .into_iter()
                .map(|proof| {
                    (
                        sp_core::keccak_256(&proof.key.0).to_vec(),
                        proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
                    )
                })
                .collect(),
        };
        Ok(proof.encode())
    }

    async fn query_responses_proof(&self, at: u64, keys: Vec<Query>) -> Result<Vec<u8>, Error> {
        let keys =
            keys.into_iter().map(|query| self.response_commitment_key(query.commitment)).collect();
        let proof = self.client.get_proof(self.ismp_host_address, keys, Some(at.into())).await?;
        let proof = EvmStateProof {
            contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
            storage_proof: proof
                .storage_proof
                .into_iter()
                .map(|proof| {
                    (
                        sp_core::keccak_256(&proof.key.0).to_vec(),
                        proof.proof.into_iter().map(|bytes| bytes.0.into()).collect(),
                    )
                })
                .collect(),
        };
        Ok(proof.encode())
    }

    async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
        let mut contract_proofs: Vec<_> = vec![];
        let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
        for key in keys {
            if key.len() != 52 {
                Err(anyhow!("Invalid key supplied, keys should be 52 bytes"))?
            }

            let contract_address = H160::from_slice(&key[..20]);
            let slot_hash = H256::from_slice(&key[20..]);
            let proof =
                self.client.get_proof(contract_address, vec![slot_hash], Some(at.into())).await?;
            contract_proofs
                .push(StorageProof::new(proof.account_proof.into_iter().map(|node| node.0.into())));
            map.insert(
                key,
                proof
                    .storage_proof
                    .get(0)
                    .cloned()
                    .ok_or_else(|| {
                        anyhow!("Invalid key supplied, storage proof could not be retrieved")
                    })?
                    .proof
                    .into_iter()
                    .map(|bytes| bytes.0.into())
                    .collect(),
            );
        }

        let contract_proof = StorageProof::merge(contract_proofs);

        let state_proof = EvmStateProof {
            contract_proof: contract_proof.into_nodes().into_iter().collect(),
            storage_proof: map,
        };
        Ok(state_proof.encode())
    }

    async fn query_ismp_events(&self, event: StateMachineUpdated) -> Result<Vec<Event>, Error> {
        let latest_state_machine_height = Arc::clone(&self.latest_state_machine_height);
        let previous_height = *latest_state_machine_height.lock() + 1;
        let events = self.events(previous_height, event.latest_height).await?;
        *latest_state_machine_height.lock() = event.latest_height;
        Ok(events)
    }

    async fn query_pending_get_requests(&self, _height: u64) -> Result<Vec<Get>, Error> {
        Ok(Default::default())
    }

    fn name(&self) -> String {
        self.state_machine.to_string()
    }

    fn state_machine_id(&self) -> StateMachineId {
        StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
    }

    fn block_max_gas(&self) -> u64 {
        self.gas_limit
    }

    async fn estimate_gas(&self, _msg: Vec<Message>) -> Result<u64, Error> {
        todo!()
    }

    async fn state_machine_update_notification(
        &self,
        _counterparty_state_id: StateMachineId,
    ) -> BoxStream<StateMachineUpdated> {
        let (sender, receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<StateMachineUpdated, Error>>();
        tokio::spawn({
            let events = self.events.clone();
            async move {
                let mut stream = events.stream().await.expect("Stream creation failed");
                while let Some(res) = stream.next().await {
                    match res {
                        Ok(ev) => sender.send(Ok(ev.into())).expect("Event stream panicked"),
                        _ => break,
                    }
                }
            }
        });

        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

        Box::pin(stream)
    }

    async fn submit(&self, messages: Vec<Message>) -> Result<(), Error> {
        use codec::Decode;
        let contract = IsmpHandler::new(self.handler_address, self.signer.clone());

        for msg in messages {
            match msg {
                Message::Consensus(msg) => {
                    contract
                        .handle_consensus(self.ismp_host_address, msg.consensus_proof.into())
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                Message::Request(msg) => {
                    let membership_proof =
                        MembershipProof::decode(&mut msg.proof.proof.as_slice())?;
                    let k_indexes = mmr_position_to_k_index(
                        membership_proof.leaf_indices.clone(),
                        NodesUtils::new(membership_proof.mmr_size).size(),
                    );

                    let leaves = msg
                        .requests
                        .into_iter()
                        .zip(k_indexes)
                        .map(|(post, (_, k_index))| PostRequestLeaf {
                            request: PostRequest {
                                source: post.source.to_string().as_bytes().to_vec().into(),
                                dest: post.dest.to_string().as_bytes().to_vec().into(),
                                nonce: post.nonce,
                                from: post.from.into(),
                                to: post.to.into(),
                                timeout_timestamp: post.timeout_timestamp,
                                body: post.data.into(),
                                gaslimit: post.gas_limit.into(),
                            },
                            index: post.nonce.into(),
                            k_index: k_index.into(),
                        })
                        .collect();

                    let post_message = PostRequestMessage {
                        proof: Proof {
                            height: crate::abi::ismp_handler::StateMachineHeight {
                                state_machine_id: {
                                    match msg.proof.height.id.state_id {
                                        StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                                            id.into()
                                        }
                                        _ => Err(anyhow!(
                                            "Expected polkadot or kusama state machines"
                                        ))?,
                                    }
                                },
                                height: msg.proof.height.height.into(),
                            },
                            multiproof: membership_proof
                                .proof
                                .into_iter()
                                .map(|node| node.0)
                                .collect(),
                            leaf_count: membership_proof.mmr_size.into(),
                        },
                        requests: leaves,
                    };

                    contract
                        .handle_post_requests(self.ismp_host_address, post_message)
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                Message::Response(ResponseMessage::Post { responses, proof }) => {
                    let membership_proof = MembershipProof::decode(&mut proof.proof.as_slice())?;
                    let k_indexes = mmr_position_to_k_index(
                        membership_proof.leaf_indices.clone(),
                        NodesUtils::new(membership_proof.mmr_size).size(),
                    );

                    let leaves = responses
                        .into_iter()
                        .zip(k_indexes)
                        .filter_map(|(res, (_, k_index))| match res {
                            Response::Post(res) => Some(PostResponseLeaf {
                                response: PostResponse {
                                    request: PostRequest {
                                        source: res
                                            .post
                                            .source
                                            .to_string()
                                            .as_bytes()
                                            .to_vec()
                                            .into(),
                                        dest: res.post.dest.to_string().as_bytes().to_vec().into(),
                                        nonce: res.post.nonce,
                                        from: res.post.from.into(),
                                        to: res.post.to.into(),
                                        timeout_timestamp: res.post.timeout_timestamp,
                                        body: res.post.data.into(),
                                        gaslimit: res.post.gas_limit.into(),
                                    },
                                    response: res.response.into(),
                                },
                                index: res.post.nonce.into(),
                                k_index: k_index.into(),
                            }),
                            _ => None,
                        })
                        .collect();

                    let message = PostResponseMessage {
                        proof: Proof {
                            height: crate::abi::ismp_handler::StateMachineHeight {
                                state_machine_id: {
                                    match proof.height.id.state_id {
                                        StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                                            id.into()
                                        }
                                        _ => Err(anyhow!(
                                            "Expected polkadot or kusama state machines"
                                        ))?,
                                    }
                                },
                                height: proof.height.height.into(),
                            },
                            multiproof: membership_proof
                                .proof
                                .into_iter()
                                .map(|node| node.0)
                                .collect(),
                            leaf_count: membership_proof.mmr_size.into(),
                        },
                        responses: leaves,
                    };
                    contract
                        .handle_post_responses(self.ismp_host_address, message)
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                Message::Response(ResponseMessage::Get { requests, proof }) => {
                    let requests = requests
                        .into_iter()
                        .map(|req| {
                            let get =
                                req.get_request().map_err(|_| anyhow!("Expected get request"))?;
                            Ok(GetRequest {
                                source: get.source.to_string().as_bytes().to_vec().into(),
                                dest: get.dest.to_string().as_bytes().to_vec().into(),
                                nonce: get.nonce,
                                from: get.from.into(),
                                keys: get.keys.into_iter().map(|key| key.into()).collect(),
                                timeout_timestamp: get.timeout_timestamp,
                                gaslimit: get.gas_limit.into(),
                                height: get.height.into(),
                            })
                        })
                        .collect::<Result<Vec<_>, Error>>()?;
                    let state_proof: SubstrateStateProof =
                        codec::Decode::decode(&mut proof.proof.as_slice())?;
                    let message = GetResponseMessage {
                        proof: state_proof
                            .storage_proof
                            .into_iter()
                            .map(|key| key.into())
                            .collect(),
                        height: crate::abi::ismp_handler::StateMachineHeight {
                            state_machine_id: {
                                match proof.height.id.state_id {
                                    StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                                        id.into()
                                    }
                                    _ => {
                                        Err(anyhow!("Expected polkadot or kusama state machines"))?
                                    }
                                }
                            },
                            height: proof.height.height.into(),
                        },
                        requests,
                    };

                    contract
                        .handle_get_responses(self.ismp_host_address, message)
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                Message::Timeout(TimeoutMessage::Post { timeout_proof, requests }) => {
                    let post_requests = requests
                        .into_iter()
                        .filter_map(|req| match req {
                            Request::Post(post) => Some(PostRequest {
                                source: post.source.to_string().as_bytes().to_vec().into(),
                                dest: post.dest.to_string().as_bytes().to_vec().into(),
                                nonce: post.nonce,
                                from: post.from.into(),
                                to: post.to.into(),
                                timeout_timestamp: post.timeout_timestamp,
                                body: post.data.into(),
                                gaslimit: post.gas_limit.into(),
                            }),
                            Request::Get(_) => None,
                        })
                        .collect();

                    let state_proof: SubstrateStateProof =
                        codec::Decode::decode(&mut timeout_proof.proof.as_slice())?;
                    let message = PostTimeoutMessage {
                        timeouts: post_requests,
                        height: crate::abi::ismp_handler::StateMachineHeight {
                            state_machine_id: {
                                match timeout_proof.height.id.state_id {
                                    StateMachine::Polkadot(id) | StateMachine::Kusama(id) => {
                                        id.into()
                                    }
                                    _ => {
                                        Err(anyhow!("Expected polkadot or kusama state machines"))?
                                    }
                                }
                            },
                            height: timeout_proof.height.height.into(),
                        },
                        proof: state_proof
                            .storage_proof
                            .into_iter()
                            .map(|key| key.into())
                            .collect(),
                    };

                    contract
                        .handle_post_timeouts(self.ismp_host_address, message)
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                Message::Timeout(TimeoutMessage::Get { requests }) => {
                    let get_requests = requests
                        .into_iter()
                        .filter_map(|req| match req {
                            Request::Get(get) => Some(GetRequest {
                                source: get.source.to_string().as_bytes().to_vec().into(),
                                dest: get.dest.to_string().as_bytes().to_vec().into(),
                                nonce: get.nonce,
                                from: get.from.into(),
                                keys: get.keys.into_iter().map(|key| key.into()).collect(),
                                timeout_timestamp: get.timeout_timestamp,
                                gaslimit: get.gas_limit.into(),
                                height: get.height.into(),
                            }),
                            _ => None,
                        })
                        .collect();

                    let message = GetTimeoutMessage { timeouts: get_requests };

                    contract
                        .handle_get_timeouts(self.ismp_host_address, message)
                        .gas(10_000_000)
                        .send()
                        .await?
                        .await?;
                }
                _ => {
                    log::debug!(target: "tesseract", "Message handler not implemented in solidity abi")
                }
            }
        }
        Ok(())
    }
}
