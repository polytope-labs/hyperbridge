mod error;
mod responses;
mod routes;
#[cfg(test)]
mod test;

use ethereum_consensus::altair::Validator;
use ethereum_consensus::bellatrix::{
    BeaconBlock, BeaconBlockHeader, SignedBeaconBlock, SignedBeaconBlockHeader, SyncCommittee,
};
use reqwest::Client;

use crate::error::Error;
use crate::responses::sync_committee_response::NodeSyncCommittee;
use crate::routes::*;
use ethereum_consensus::bellatrix::mainnet::{
    BYTES_PER_LOGS_BLOOM, MAX_BYTES_PER_TRANSACTION, MAX_EXTRA_DATA_BYTES,
    MAX_TRANSACTIONS_PER_PAYLOAD, SYNC_COMMITTEE_SIZE,
};
use ethereum_consensus::crypto::{aggregate, eth_aggregate_public_keys, PublicKey};
use ethereum_consensus::phase0::mainnet::{
    EPOCHS_PER_HISTORICAL_VECTOR, EPOCHS_PER_SLASHINGS_VECTOR, ETH1_DATA_VOTES_BOUND,
    HISTORICAL_ROOTS_LIMIT, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS, MAX_DEPOSITS,
    MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS,
    SLOTS_PER_HISTORICAL_ROOT, VALIDATOR_REGISTRY_LIMIT,
};
use ethereum_consensus::primitives::{BlsPublicKey, ValidatorIndex};
use ssz_rs::{List, Vector};

pub struct SyncCommitteeProver {
    pub node_url: String,
    pub client: Client,
}

impl SyncCommitteeProver {
    pub fn new(node_url: String) -> Self {
        let client = reqwest::Client::new();

        SyncCommitteeProver { node_url, client }
    }

    pub async fn fetch_header(
        &self,
        block_id: String,
    ) -> Result<BeaconBlockHeader, reqwest::Error> {
        let path = header_route(block_id);
        let full_url = format!("{}{}", self.node_url.clone(), path);
        let response = self.client.get(full_url).send().await?;
        let response_data = response
            .json::<responses::beacon_block_header_response::Response>()
            .await?;

        let beacon_block_header = response_data.data.header.message;

        Ok(beacon_block_header)
    }
    pub async fn fetch_block(
        &self,
        block_id: String,
    ) -> Result<
        BeaconBlock<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        >,
        reqwest::Error,
    > {
        let path = block_route(block_id);
        let full_url = format!("{}/{}", self.node_url.clone(), path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::beacon_block_response::Response>()
            .await?;

        let beacon_block = response_data.data.message;

        Ok(beacon_block)
    }
    pub async fn fetch_sync_committee(
        &self,
        state_id: String,
    ) -> Result<NodeSyncCommittee, reqwest::Error> {
        let path = sync_committee_route(state_id);
        let full_url = format!("{}/{}", self.node_url.clone(), path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::sync_committee_response::Response>()
            .await
            .unwrap();

        let sync_committee = response_data.data;

        Ok(sync_committee)
    }
    pub async fn fetch_validator(
        &self,
        state_id: String,
        validator_index: String,
    ) -> Result<Validator, reqwest::Error> {
        let path = validator_route(state_id, validator_index);
        let full_url = format!("{}/{}", self.node_url.clone(), path);

        let response = self.client.get(full_url).send().await?;

        let response_data = response
            .json::<responses::validator_response::Response>()
            .await
            .unwrap();

        let validator = response_data.data.validator;

        Ok(validator)
    }

    pub async fn fetch_processed_sync_committee(
        &self,
        state_id: String,
    ) -> Result<SyncCommittee<SYNC_COMMITTEE_SIZE>, reqwest::Error> {
        // fetches sync committee from Node
        let node_sync_committee = self.fetch_sync_committee(state_id.clone()).await?;

        let mut validators: List<Validator, VALIDATOR_REGISTRY_LIMIT> = Default::default();
        let mut validator_indexes: Vec<ValidatorIndex> = Vec::new();

        for mut validator_index in node_sync_committee.validators {
            // fetches validator based on validator index
            let validator = self
                .fetch_validator(state_id.clone(), validator_index.clone())
                .await?;
            validators.push(validator);
            validator_indexes.push(validator_index.parse().unwrap());
        }

        let public_keys_vector = validator_indexes
            .into_iter()
            .map(|i| validators[i].public_key.clone())
            .collect::<Vector<_, SYNC_COMMITTEE_SIZE>>();

        let aggregate_public_key = eth_aggregate_public_keys(&public_keys_vector).unwrap();

        let sync_committee = SyncCommittee::<SYNC_COMMITTEE_SIZE> {
            public_keys: public_keys_vector,
            aggregate_public_key,
        };

        Ok(sync_committee)
    }

    pub fn signed_beacon_block(
        &self,
        beacon_block: BeaconBlock<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        >,
    ) -> Option<
        SignedBeaconBlock<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        >,
    > {
        let attestations = beacon_block.body.attestations.clone();
        let signatures: Vec<_> = attestations
            .iter()
            .map(|sig| sig.signature.clone())
            .collect();

        let aggregate_signature =
            aggregate(signatures.as_ref()).map_err(|_| Error::AggregateSignatureError);

        let signed_beacon_block = SignedBeaconBlock::<
            MAX_PROPOSER_SLASHINGS,
            MAX_VALIDATORS_PER_COMMITTEE,
            MAX_ATTESTER_SLASHINGS,
            MAX_ATTESTATIONS,
            MAX_DEPOSITS,
            MAX_VOLUNTARY_EXITS,
            SYNC_COMMITTEE_SIZE,
            BYTES_PER_LOGS_BLOOM,
            MAX_EXTRA_DATA_BYTES,
            MAX_BYTES_PER_TRANSACTION,
            MAX_TRANSACTIONS_PER_PAYLOAD,
        > {
            message: beacon_block,
            signature: aggregate_signature.unwrap(),
        };

        Some(signed_beacon_block)
    }

    pub fn signed_beacon_block_header(
        &self,
        signed_beacon_block: Option<
            SignedBeaconBlock<
                MAX_PROPOSER_SLASHINGS,
                MAX_VALIDATORS_PER_COMMITTEE,
                MAX_ATTESTER_SLASHINGS,
                MAX_ATTESTATIONS,
                MAX_DEPOSITS,
                MAX_VOLUNTARY_EXITS,
                SYNC_COMMITTEE_SIZE,
                BYTES_PER_LOGS_BLOOM,
                MAX_EXTRA_DATA_BYTES,
                MAX_BYTES_PER_TRANSACTION,
                MAX_TRANSACTIONS_PER_PAYLOAD,
            >,
        >,
        beacon_block_header: BeaconBlockHeader,
    ) -> Result<SignedBeaconBlockHeader, Error> {
        if signed_beacon_block.is_none() {
            return Err(Error::EmptySignedBeaconBlock);
        }

        let signed_beacon_block_header = SignedBeaconBlockHeader {
            message: beacon_block_header,
            signature: signed_beacon_block.unwrap().signature,
        };

        Ok(signed_beacon_block_header)
    }
}
