use sp_core::H256;

/// The block root and state root for every slot are stored in the state for `SLOTS_PER_HISTORICAL_ROOT` slots.
/// When that list is full, both lists are Merkleized into a single Merkle root,
/// which is added to the ever-growing state.historical_roots list.
/// [source](https://eth2book.info/bellatrix/part3/config/preset/#slots_per_historical_root)
const SLOTS_PER_HISTORICAL_ROOT: u64 = 2 ^ 13; // 8,192

/// Every `SLOTS_PER_HISTORICAL_ROOT` slots, the list of block roots and the list of state roots in the beacon state
/// are Merkleized and added to state.historical_roots list. Although state.historical_roots is in principle unbounded,
/// all SSZ lists must have maximum sizes specified.
///
/// The size `HISTORICAL_ROOTS_LIMIT` will be fine for the next few millennia, after which it will be somebody else's problem.
/// The list grows at less than 10 KB per year. Storing past roots like this allows Merkle proofs to be constructed
/// about anything in the beacon chain's history if required.
/// [source](https://eth2book.info/bellatrix/part3/config/preset/#historical_roots_limit)
const HISTORICAL_ROOTS_LIMIT: u64 = 2 ^ 24; // 16,777,216

/// Generalized merkle tree index for the latest finalized header
/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/light-client/sync-protocol.md#constants)
const FINALIZED_ROOT_INDEX: u64 = 105;

/// Generalized merkle tree index for the next sync committee
/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/light-client/sync-protocol.md#constants)
const NEXT_SYNC_COMMITTEE_INDEX: u64 = 55;

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/beacon-chain.md#domain-types)
pub const DOMAIN_SYNC_COMMITTEE: [u8; 4] = [7, 0, 0, 0];

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/beacon-chain.md#sync-committee)
const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 = 2 ^ 8;

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#time-parameters)
const SLOTS_PER_EPOCH: u64 = 2 ^ 5;

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/fork.md#configuration)
const ALTAIR_FORK_EPOCH: u64 = 74240;

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/altair/fork.md#configuration)
const ALTAIR_FORK_VERSION: [u8; 4] = [1, 0, 0, 0];

/// [source](https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md#genesis-settings)
const GENESIS_FORK_VERSION: [u8; 4] = [0, 0, 0, 0];

const GENESIS_VALIDATORS_ROOT: H256 = H256([0u8; 32]);

pub const MAX_PROPOSER_SLASHINGS: u64 = 2 ^ 4;
pub const MAX_ATTESTER_SLASHINGS: u64 = 2 ^ 4;
pub const MAX_ATTESTATIONS: u64 = 2 ^ 4;
pub const MAX_DEPOSITS: u64 = 2 ^ 4;
pub const MAX_VOLUNTARY_EXITS: u64 = 2 ^ 4;
pub const MAX_VALIDATORS_PER_COMMITTEE: u64 = 2 ^ 11;

pub const SYNC_COMMITTEE_SIZE: u64 = 2 ^ 9;

pub const DEPOSIT_CONTRACT_TREE_DEPTH: u64 = 2 ^ 5;
