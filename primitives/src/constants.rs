use crate::domains::DomainType;
use ssz_rs::Node;

pub type BlsPublicKey = ByteVector<BLS_PUBLIC_KEY_BYTES_LEN>;
pub type BlsSignature = ByteVector<BLS_SIGNATURE_BYTES_LEN>;

pub type Epoch = u64;
pub type Slot = u64;
pub type Root = Node;
pub type ParticipationFlags = u8;

pub type CommitteeIndex = u64;
pub type ValidatorIndex = u64;
pub type WithdrawalIndex = u64;
pub type Gwei = u64;
pub type Hash32 = Bytes32;

pub type Version = [u8; 4];
pub type ForkDigest = [u8; 4];
pub type Domain = [u8; 32];

pub type ExecutionAddress = ByteVector<20>;

pub type ChainId = usize;
pub type NetworkId = usize;

pub type RandaoReveal = BlsSignature;
pub type Bytes32 = ByteVector<32>;

pub const BLS_PUBLIC_KEY_BYTES_LEN: usize = 48;
pub const BLS_SECRET_KEY_BYTES_LEN: usize = 32;
pub const BLS_SIGNATURE_BYTES_LEN: usize = 96;

pub const SYNC_COMMITTEE_SIZE: usize = 512;
pub const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 256;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_BLS_TO_EXECUTION_CHANGES: usize = 16;
pub const MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP: usize = 16384;

pub const MAX_VALIDATORS_PER_COMMITTEE: usize = 2048;
pub const EPOCHS_PER_ETH1_VOTING_PERIOD: Epoch = 64;
pub const SLOTS_PER_HISTORICAL_ROOT: usize = 8192;
pub const EPOCHS_PER_HISTORICAL_VECTOR: usize = 65536;
pub const EPOCHS_PER_SLASHINGS_VECTOR: usize = 8192;
pub const HISTORICAL_ROOTS_LIMIT: usize = 16_777_216;
pub const VALIDATOR_REGISTRY_LIMIT: usize = 2usize.saturating_pow(40);
pub const MAX_PROPOSER_SLASHINGS: usize = 16;
pub const MAX_ATTESTER_SLASHINGS: usize = 2;
pub const MAX_ATTESTATIONS: usize = 128;
pub const MAX_DEPOSITS: usize = 16;
pub const MAX_VOLUNTARY_EXITS: usize = 16;
pub const JUSTIFICATION_BITS_LENGTH: usize = 4;

pub const MAX_BYTES_PER_TRANSACTION: usize = 1_073_741_824;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1_048_576;
pub const BYTES_PER_LOGS_BLOOM: usize = 256;
pub const MAX_EXTRA_DATA_BYTES: usize = 32;

pub const DEPOSIT_PROOF_LENGTH: usize = 33;

pub const ETH1_DATA_VOTES_BOUND: usize = (EPOCHS_PER_ETH1_VOTING_PERIOD * SLOTS_PER_EPOCH) as usize;
pub const DOMAIN_SYNC_COMMITTEE: DomainType = DomainType::SyncCommittee;
pub const FINALIZED_ROOT_INDEX: u64 = 52;
pub const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 18;
pub const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 22;
pub const EXECUTION_PAYLOAD_INDEX: u64 = 56;
pub const NEXT_SYNC_COMMITTEE_INDEX: u64 = 55;
pub const BLOCK_ROOTS_INDEX: u64 = 37;
pub const HISTORICAL_ROOTS_INDEX: u64 = 39;
pub const HISTORICAL_BATCH_BLOCK_ROOTS_INDEX: u64 = 2;
pub const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 25;
pub const FINALIZED_ROOT_INDEX_LOG2: u64 = 5;
pub const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 5;
pub const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 5;
pub const BLOCK_ROOTS_INDEX_LOG2: u64 = 5;
pub const HISTORICAL_ROOTS_INDEX_LOG2: u64 = 5;

#[cfg(feature = "goerli")]
pub use goerli::*;

#[cfg(feature = "mainnet")]
pub use mainnet::*;

use crate::ssz::ByteVector;
#[cfg(all(not(feature = "mainnet"), not(feature = "goerli")))]
pub use devnet::*;

#[cfg(feature = "goerli")]
pub mod goerli {
	use super::*;
	pub const SLOTS_PER_EPOCH: Slot = 32;
	pub const GENESIS_VALIDATORS_ROOT: [u8; 32] =
		hex_literal::hex!("043db0d9a83813551ee2f33450d23797757d430911a9320530ad8a0eabc43efb");
	pub const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("02001020");
	pub const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("01001020");
	pub const GENESIS_FORK_VERSION: Version = hex_literal::hex!("00001020");
	pub const ALTAIR_FORK_EPOCH: Epoch = 36660;
	pub const BELLATRIX_FORK_EPOCH: Epoch = 112260;
	pub const CAPELLA_FORK_EPOCH: Epoch = 162304;
	pub const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("03001020");
}

#[cfg(feature = "mainnet")]
pub mod mainnet {
	use super::*;
	pub const SLOTS_PER_EPOCH: Slot = 32;
	pub const GENESIS_VALIDATORS_ROOT: [u8; 32] =
		hex_literal::hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95");
	pub const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("02000000");
	pub const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("01000000");
	pub const GENESIS_FORK_VERSION: Version = hex_literal::hex!("00000000");
	pub const ALTAIR_FORK_EPOCH: Epoch = 74240;
	pub const BELLATRIX_FORK_EPOCH: Epoch = 144896;
	pub const CAPELLA_FORK_EPOCH: Epoch = 194048;
	pub const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("03000000");
}

#[cfg(all(not(feature = "mainnet"), not(feature = "goerli")))]
pub mod devnet {
	use super::*;
	use hex_literal::hex;
	pub const SLOTS_PER_EPOCH: Slot = 6;
	pub const GENESIS_VALIDATORS_ROOT: [u8; 32] =
		hex_literal::hex!("83431ec7fcf92cfc44947fc0418e831c25e1d0806590231c439830db7ad54fda");
	pub const BELLATRIX_FORK_VERSION: Version = hex!("52525502");
	pub const ALTAIR_FORK_VERSION: Version = hex!("52525501");
	pub const GENESIS_FORK_VERSION: Version = hex!("52525500");
	pub const ALTAIR_FORK_EPOCH: Epoch = 0;
	pub const BELLATRIX_FORK_EPOCH: Epoch = 0;
	pub const CAPELLA_FORK_EPOCH: Epoch = 2;
	pub const CAPELLA_FORK_VERSION: Version = hex!("52525503");
}
