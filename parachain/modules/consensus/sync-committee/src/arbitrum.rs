use crate::{
    beacon_client::BEACON_CONSENSUS_ID,
    utils::{derive_map_key, get_contract_storage_root, to_bytes_32},
};

use crate::{prelude::*, presets::NODES_SLOT, utils::get_value_from_proof};
use alloc::{format, string::ToString};
use alloy_primitives::{Address, FixedBytes, B256};
use alloy_rlp::Decodable;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use ethabi::ethereum_types::{Bloom, H160, H256, H64, U256};
use ismp::{
    consensus::{IntermediateState, StateCommitment, StateMachineHeight, StateMachineId},
    error::Error,
    host::{Ethereum, IsmpHost, StateMachine},
};

/// https://github.com/OffchainLabs/go-ethereum/blob/8c5b9339ca9043d2b8fb5e35814a64e7e9ff7c9b/core/types/block.go#L70
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
#[rlp(trailing)]
pub struct Header {
    pub parent_hash: B256,
    pub uncle_hash: B256,
    pub coinbase: Address,
    pub state_root: B256,
    pub transactions_root: B256,
    pub receipts_root: B256,
    pub logs_bloom: FixedBytes<256>,
    pub difficulty: alloy_primitives::U256,
    pub number: alloy_primitives::U256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    // This is the sendRoot, a 32 byte hash
    // https://github.com/OffchainLabs/go-ethereum/blob/8c5b9339ca9043d2b8fb5e35814a64e7e9ff7c9b/core/types/arb_types.go#L457
    pub extra_data: alloy_primitives::Bytes,
    pub mix_hash: B256,
    pub nonce: FixedBytes<8>,
    pub base_fee_per_gas: Option<alloy_primitives::U256>,
}

/// https://github.com/OffchainLabs/go-ethereum/blob/8c5b9339ca9043d2b8fb5e35814a64e7e9ff7c9b/core/types/block.go#L70
#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct CodecHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    pub number: U256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    // This is the sendRoot, a 32 byte hash
    // https://github.com/OffchainLabs/go-ethereum/blob/8c5b9339ca9043d2b8fb5e35814a64e7e9ff7c9b/core/types/arb_types.go#L457
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub nonce: H64,
    pub base_fee_per_gas: Option<U256>,
}

impl From<CodecHeader> for Header {
    fn from(value: CodecHeader) -> Self {
        Header {
            parent_hash: value.parent_hash.0.into(),
            uncle_hash: value.uncle_hash.0.into(),
            coinbase: value.coinbase.0.into(),
            state_root: value.state_root.0.into(),
            transactions_root: value.transactions_root.0.into(),
            receipts_root: value.receipts_root.0.into(),
            logs_bloom: value.logs_bloom.0.into(),
            difficulty: {
                let mut bytes = [0u8; 32];
                value.difficulty.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            number: {
                let mut bytes = [0u8; 32];
                value.number.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            gas_limit: value.gas_limit,
            gas_used: value.gas_used,
            timestamp: value.timestamp,
            extra_data: value.extra_data.into(),
            mix_hash: value.mix_hash.0.into(),
            nonce: value.nonce.0.into(),
            base_fee_per_gas: value.base_fee_per_gas.map(|val| {
                let mut bytes = [0u8; 32];
                val.to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            }),
        }
    }
}

impl Header {
    pub fn hash<H: IsmpHost>(self) -> H256 {
        let encoding = alloy_rlp::encode(self);
        H::keccak256(&encoding)
    }
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct GlobalState {
    pub block_hash: H256,
    pub send_root: H256,
    pub inbox_position: u64,
    pub position_in_message: u64,
}

impl GlobalState {
    /// https://github.com/OffchainLabs/nitro/blob/5e9f4228e6418b114a5aea0aa7f2f0cc161b67c0/contracts/src/state/GlobalState.sol#L16
    pub fn hash<H: IsmpHost>(&self) -> H256 {
        // abi encode packed
        let mut buf = Vec::new();
        buf.extend_from_slice("Global state:".as_bytes());
        buf.extend_from_slice(&self.block_hash[..]);
        buf.extend_from_slice(&self.send_root[..]);
        buf.extend_from_slice(&self.inbox_position.to_be_bytes()[..]);
        buf.extend_from_slice(&self.position_in_message.to_be_bytes()[..]);
        H::keccak256(&buf)
    }
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub enum MachineStatus {
    Running = 0,
    Finished = 1,
    Errored = 2,
    TooFar = 3,
}

impl TryFrom<u8> for MachineStatus {
    type Error = &'static str;

    fn try_from(status: u8) -> Result<Self, Self::Error> {
        if status == 0 {
            Ok(MachineStatus::Running)
        } else if status == 1 {
            Ok(MachineStatus::Finished)
        } else if status == 2 {
            Ok(MachineStatus::Errored)
        } else if status == 3 {
            Ok(MachineStatus::TooFar)
        } else {
            Err("Invalid machine status received")
        }
    }
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct ArbitrumPayloadProof {
    /// Arbitrum header that corresponds to the node being created
    pub arbitrum_header: CodecHeader,
    /// Global State as recorded in the NodeCreated event that was emitted for this node
    pub global_state: GlobalState,
    /// Machine status as recorded in the NodeCreated event that was emitted for this node
    pub machine_status: MachineStatus,
    /// Inbox max count as recorded in the NodeCreated event that was emitted for this node
    pub inbox_max_count: U256,
    /// Key used to store the node  in the _nodes mapping in the RollupCore as recorded in the
    /// latestNodeCreated field of the NodeCreated event
    pub node_number: u64,
    /// Proof for the state_hash field in the Node struct inside the _nodes mapping in the
    /// RollupCore
    pub storage_proof: Vec<Vec<u8>>,
    /// RollupCore contract proof in the ethereum world trie
    pub contract_proof: Vec<Vec<u8>>,
}

/// https://github.com/OffchainLabs/nitro/blob/5e9f4228e6418b114a5aea0aa7f2f0cc161b67c0/contracts/src/rollup/RollupLib.sol#L59
fn get_state_hash<H: IsmpHost>(
    global_state: GlobalState,
    machine_status: MachineStatus,
    inbox_max_count: U256,
) -> H256 {
    // abi encode packed
    let mut buf = Vec::new();
    buf.extend_from_slice(&global_state.hash::<H>()[..]);
    let mut inbox = [0u8; 32];
    inbox_max_count.to_big_endian(&mut inbox);
    buf.extend_from_slice(&inbox);
    buf.extend_from_slice((machine_status as u8).to_be_bytes().as_slice());
    H::keccak256(&buf)
}

pub fn verify_arbitrum_payload<H: IsmpHost + Send + Sync>(
    payload: ArbitrumPayloadProof,
    root: &[u8],
    rollup_core_address: H160,
) -> Result<IntermediateState, Error> {
    let root = to_bytes_32(root)?;
    let root = H256::from_slice(&root[..]);

    let storage_root =
        get_contract_storage_root::<H>(payload.contract_proof, rollup_core_address, root)?;

    let header: Header = payload.arbitrum_header.clone().into();
    if &payload.global_state.send_root[..] != &payload.arbitrum_header.extra_data {
        Err(Error::ImplementationSpecific(
            "Arbitrum header extra data does not match send root in global state".to_string(),
        ))?
    }

    let block_number = payload.arbitrum_header.number.low_u64();
    let timestamp = payload.arbitrum_header.timestamp;
    let state_root = payload.arbitrum_header.state_root.0.into();

    let header_hash = header.hash::<H>();
    if payload.global_state.block_hash != header_hash {
        Err(Error::ImplementationSpecific(
            "Arbitrum header hash does not match block hash in global state".to_string(),
        ))?
    }

    let state_hash =
        get_state_hash::<H>(payload.global_state, payload.machine_status, payload.inbox_max_count);

    let mut key = [0u8; 32];
    U256::from(payload.node_number).to_big_endian(&mut key);
    let state_hash_key = derive_map_key::<H>(key.to_vec(), NODES_SLOT);
    let proof_value = match get_value_from_proof::<H>(
        state_hash_key.0.to_vec(),
        storage_root,
        payload.storage_proof,
    )? {
        Some(value) => value.clone(),
        _ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
    };

    let proof_value = <B256 as Decodable>::decode(&mut &*proof_value).map_err(|_| {
        Error::ImplementationSpecific(format!("Error decoding state hash {:?}", &proof_value))
    })?;

    if proof_value.0 != state_hash.0 {
        Err(Error::MembershipProofVerificationFailed(
            "State hash from proof does not match calculated state hash".to_string(),
        ))?
    }

    Ok(IntermediateState {
        height: StateMachineHeight {
            id: StateMachineId {
                state_id: StateMachine::Ethereum(Ethereum::Arbitrum),
                consensus_state_id: BEACON_CONSENSUS_ID,
            },
            height: block_number,
        },
        commitment: StateCommitment { timestamp, overlay_root: None, state_root },
    })
}
