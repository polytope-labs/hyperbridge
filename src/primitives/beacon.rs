use crate::primitives::consts::{
    DEPOSIT_CONTRACT_TREE_DEPTH, MAX_ATTESTATIONS, MAX_ATTESTER_SLASHINGS, MAX_DEPOSITS,
    MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_VOLUNTARY_EXITS, SYNC_COMMITTEE_SIZE,
};
use sp_core::H256;

struct ValidatorIndex(u64);
struct Slot(u64);
struct BLSSignature([u8; 24]);
struct BLSPubkey([u8; 12]);
struct Root([u8; 32]);
struct CommitteeIndex(u64);
struct Epoch(u64);
struct Gwei(u64);

struct Eth1Data {
    deposit_root: H256,
    deposit_count: u64,
    block_hash: H256,
}

struct ProposerSlashing {
    signed_header_1: SignedBeaconBlockHeader,
    signed_header_2: SignedBeaconBlockHeader,
}

struct SignedBeaconBlockHeader {
    message: BeaconBlockHeader,
    signature: BLSSignature,
}

struct AttesterSlashing {
    attestation_1: IndexedAttestation,
    attestation_2: IndexedAttestation,
}

struct IndexedAttestation {
    attesting_indices: Vec<[ValidatorIndex; MAX_VALIDATORS_PER_COMMITTEE as usize]>,
    data: AttestationData,
    signature: BLSSignature,
}

struct AttestationData {
    slot: Slot,
    index: CommitteeIndex,
    //LMD GHOST vote,
    beacon_block_root: H256,
    //FFG vote,
    source: Checkpoint,
    target: Checkpoint,
}

struct Checkpoint {
    epoch: Epoch,
    root: H256,
}

struct Attestation {
    aggregation_bits: Vec<[u64; MAX_VALIDATORS_PER_COMMITTEE as usize]>,
    data: AttestationData,
    signature: BLSSignature,
}

pub struct SignedVoluntaryExit {
    message: VoluntaryExit,
    signature: BLSSignature,
}

struct VoluntaryExit {
    epoch: Epoch,
    validator_index: ValidatorIndex,
}

struct SyncAggregate {
    sync_committee_bits: Vec<[u64; SYNC_COMMITTEE_SIZE as usize]>,
    sync_committee_signature: BLSSignature,
}

struct Deposit {
    proof: Vec<([u8; 4], [u64; (DEPOSIT_CONTRACT_TREE_DEPTH + 1) as usize])>,
    data: DepositData,
}

struct DepositData {
    pubkey: BLSPubkey,
    withdrawal_credentials: [u8; 32],
    amount: Gwei,
    signature: BLSSignature,
}

/// The beacon block header
/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#beaconblockheader)
pub struct BeaconBlockHeader {
    /// current slot for this block
    slot: Slot,
    /// validator index
    proposer_index: ValidatorIndex,
    /// ssz root of parent block
    parent_root: H256,
    /// ssz root of associated [`BeaconState`]
    state_root: H256,
    /// ssz root of associated [`BeaconBlockBody`]
    body_root: H256,
}

/// The beacon block body
/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/beacon-chain.md#beaconblockbody)
struct BeaconBlockBody {
    randao_reveal: BLSSignature,
    eth1_data: Eth1Data, // Eth1 data vote
    graffiti: H256,      // Arbitrary data
    // Operations
    proposer_slashings: [ProposerSlashing; MAX_PROPOSER_SLASHINGS as usize],
    attester_slashings: [AttesterSlashing; MAX_ATTESTER_SLASHINGS as usize],
    attestations: [Attestation; MAX_ATTESTATIONS as usize],
    deposits: [Deposit; MAX_DEPOSITS as usize],
    voluntary_exits: [SignedVoluntaryExit; MAX_VOLUNTARY_EXITS as usize],
    sync_aggregate: SyncAggregate,
}
