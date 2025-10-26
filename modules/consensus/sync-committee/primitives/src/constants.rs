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
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_BLS_TO_EXECUTION_CHANGES: usize = 16;
pub const MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP: usize = 16384;

pub const MAX_COMMITTEES_PER_SLOT: usize = 64;
pub const MAX_VALIDATORS_PER_COMMITTEE: usize = 2048 * MAX_COMMITTEES_PER_SLOT;
pub const EPOCHS_PER_ETH1_VOTING_PERIOD: Epoch = 64;
pub const SLOTS_PER_HISTORICAL_ROOT: usize = 8192;
pub const EPOCHS_PER_HISTORICAL_VECTOR: usize = 65536;
pub const EPOCHS_PER_SLASHINGS_VECTOR: usize = 8192;
pub const HISTORICAL_ROOTS_LIMIT: usize = 16_777_216;
pub const VALIDATOR_REGISTRY_LIMIT: usize = 2usize.saturating_pow(40);
pub const MAX_PROPOSER_SLASHINGS: usize = 16;
pub const MAX_ATTESTER_SLASHINGS: usize = 1;
pub const MAX_ATTESTATIONS: usize = 8;
pub const MAX_DEPOSITS: usize = 16;
pub const MAX_VOLUNTARY_EXITS: usize = 16;
pub const JUSTIFICATION_BITS_LENGTH: usize = 4;

pub const MAX_BYTES_PER_TRANSACTION: usize = 1_073_741_824;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1_048_576;
pub const BYTES_PER_LOGS_BLOOM: usize = 256;
pub const MAX_EXTRA_DATA_BYTES: usize = 32;

pub const DEPOSIT_PROOF_LENGTH: usize = 33;

pub const DOMAIN_SYNC_COMMITTEE: DomainType = DomainType::SyncCommittee;

pub const FINALIZED_ROOT_INDEX: u64 = 52;
pub const EXECUTION_PAYLOAD_INDEX: u64 = 56;
pub const NEXT_SYNC_COMMITTEE_INDEX: u64 = 55;

pub const FINALIZED_ROOT_INDEX_LOG2: u64 = 5;
pub const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 5;
pub const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 5;

pub const ETH1_DATA_VOTES_BOUND_ETH: usize = (EPOCHS_PER_ETH1_VOTING_PERIOD * 32) as usize;
pub const ETH1_DATA_VOTES_BOUND_GNO: usize = (EPOCHS_PER_ETH1_VOTING_PERIOD * 16) as usize;

pub const BEACON_CONSENSUS_ID: [u8; 4] = *b"BEAC";
pub const GNOSIS_CONSENSUS_ID: [u8; 4] = *b"GNOS";

pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 2usize.saturating_pow(13);
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 2usize.saturating_pow(16);
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 2usize.saturating_pow(3);

pub const PENDING_DEPOSITS_LIMIT: usize = 2usize.saturating_pow(27);
pub const PENDING_PARTIAL_WITHDRAWALS_LIMIT: usize = 2usize.saturating_pow(27);
pub const PENDING_CONSOLIDATIONS_LIMIT: usize = 2usize.saturating_pow(18);

pub const PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM: usize = 64;
pub const PROPOSER_LOOK_AHEAD_LIMIT_GNO: usize = 32;

pub trait Config {
	const SLOTS_PER_EPOCH: Slot;
	const GENESIS_VALIDATORS_ROOT: [u8; 32];
	const BELLATRIX_FORK_VERSION: Version;
	const ALTAIR_FORK_VERSION: Version;
	const GENESIS_FORK_VERSION: Version;
	const ALTAIR_FORK_EPOCH: Epoch;
	const BELLATRIX_FORK_EPOCH: Epoch;
	const CAPELLA_FORK_EPOCH: Epoch;
	const CAPELLA_FORK_VERSION: Version;
	const DENEB_FORK_EPOCH: Epoch;
	const DENEB_FORK_VERSION: Version;
	const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch;
	const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64;
	const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64;
	const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64;
	const EXECUTION_PAYLOAD_INDEX: u64;
	const NEXT_SYNC_COMMITTEE_INDEX: u64;
	const FINALIZED_ROOT_INDEX: u64;
	const FINALIZED_ROOT_INDEX_LOG2: u64;
	const EXECUTION_PAYLOAD_INDEX_LOG2: u64;
	const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64;
	const ELECTRA_FORK_VERSION: Version;
	const ELECTRA_FORK_EPOCH: Epoch;
	const FULU_FORK_VERSION: Version;
	const FULU_FORK_EPOCH: Epoch;
	const ID: [u8; 4];
}

use crate::ssz::ByteVector;

pub mod sepolia {
	use super::*;
	use hex_literal::hex;

	#[derive(Default)]
	pub struct Sepolia;

	impl Config for Sepolia {
		const SLOTS_PER_EPOCH: Slot = 32;
		const GENESIS_VALIDATORS_ROOT: [u8; 32] =
			hex_literal::hex!("d8ea171f3c94aea21ebc42a1ed61052acf3f9209c00e4efbaaddac09ed9b8078");
		const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("90000071");
		const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("90000070");
		const GENESIS_FORK_VERSION: Version = hex_literal::hex!("90000069");
		const ALTAIR_FORK_EPOCH: Epoch = 50;
		const BELLATRIX_FORK_EPOCH: Epoch = 100;
		const CAPELLA_FORK_EPOCH: Epoch = 56832;
		const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("90000072");
		const DENEB_FORK_EPOCH: Epoch = 132608;
		const DENEB_FORK_VERSION: Version = hex!("90000073");
		const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 256;
		const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 34;
		const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 38;
		const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 41;
		const EXECUTION_PAYLOAD_INDEX: u64 = 88;
		const NEXT_SYNC_COMMITTEE_INDEX: u64 = 87;
		const FINALIZED_ROOT_INDEX: u64 = 84;
		const FINALIZED_ROOT_INDEX_LOG2: u64 = 6;
		const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 6;
		const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 6;
		const ELECTRA_FORK_VERSION: Version = hex_literal::hex!("90000074");
		const ELECTRA_FORK_EPOCH: Epoch = 222464;
		const FULU_FORK_EPOCH: Epoch = 272640;
		const FULU_FORK_VERSION: Version = hex_literal::hex!("90000075");
		const ID: [u8; 4] = BEACON_CONSENSUS_ID;
	}
}

pub mod mainnet {
	use super::*;

	#[derive(Default)]
	pub struct Mainnet;

	impl Config for Mainnet {
		const SLOTS_PER_EPOCH: Slot = 32;
		const GENESIS_VALIDATORS_ROOT: [u8; 32] =
			hex_literal::hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95");
		const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("02000000");
		const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("01000000");
		const GENESIS_FORK_VERSION: Version = hex_literal::hex!("00000000");
		const ALTAIR_FORK_EPOCH: Epoch = 74240;
		const BELLATRIX_FORK_EPOCH: Epoch = 144896;
		const CAPELLA_FORK_EPOCH: Epoch = 194048;
		const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("03000000");
		const DENEB_FORK_EPOCH: Epoch = 269568;
		const DENEB_FORK_VERSION: Version = hex_literal::hex!("04000000");
		const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 256;
		const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 34;
		const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 38;
		const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 41;
		const EXECUTION_PAYLOAD_INDEX: u64 = 88;
		const NEXT_SYNC_COMMITTEE_INDEX: u64 = 87;
		const FINALIZED_ROOT_INDEX: u64 = 84;
		const FINALIZED_ROOT_INDEX_LOG2: u64 = 6;
		const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 6;
		const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 6;
		const ELECTRA_FORK_VERSION: Version = hex_literal::hex!("05000000");
		const ELECTRA_FORK_EPOCH: Epoch = 364032;
		const FULU_FORK_EPOCH: Epoch = u64::MAX;
		const FULU_FORK_VERSION: Version = hex_literal::hex!("06000000");
		const ID: [u8; 4] = BEACON_CONSENSUS_ID;
	}
}

pub mod gnosis {
	use super::*;

	#[derive(Default)]
	pub struct Mainnet;

	impl Config for Mainnet {
		const SLOTS_PER_EPOCH: Slot = 16;
		const GENESIS_VALIDATORS_ROOT: [u8; 32] =
			hex_literal::hex!("f5dcb5564e829aab27264b9becd5dfaa017085611224cb3036f573368dbb9d47");
		const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("02000064");
		const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("01000064");
		const GENESIS_FORK_VERSION: Version = hex_literal::hex!("00000064");
		const ALTAIR_FORK_EPOCH: Epoch = 512;
		const BELLATRIX_FORK_EPOCH: Epoch = 385536;
		const CAPELLA_FORK_EPOCH: Epoch = 648704;
		const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("03000064");
		const DENEB_FORK_EPOCH: Epoch = 889856;
		const DENEB_FORK_VERSION: Version = hex_literal::hex!("04000064");
		const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 512;
		const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 34;
		const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 38;
		const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 41;
		const EXECUTION_PAYLOAD_INDEX: u64 = 88;
		const NEXT_SYNC_COMMITTEE_INDEX: u64 = 87;
		const FINALIZED_ROOT_INDEX: u64 = 84;
		const FINALIZED_ROOT_INDEX_LOG2: u64 = 6;
		const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 6;
		const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 6;
		const ELECTRA_FORK_VERSION: Version = hex_literal::hex!("05000064");
		const ELECTRA_FORK_EPOCH: Epoch = 1337856;
		const FULU_FORK_EPOCH: Epoch = u64::MAX;
		const FULU_FORK_VERSION: Version = hex_literal::hex!("06000064");
		const ID: [u8; 4] = GNOSIS_CONSENSUS_ID;
	}

	#[derive(Default)]
	pub struct Testnet;

	impl Config for Testnet {
		const SLOTS_PER_EPOCH: Slot = 16;
		const GENESIS_VALIDATORS_ROOT: [u8; 32] =
			hex_literal::hex!("9d642dac73058fbf39c0ae41ab1e34e4d889043cb199851ded7095bc99eb4c1e");
		const BELLATRIX_FORK_VERSION: Version = hex_literal::hex!("0200006f");
		const ALTAIR_FORK_VERSION: Version = hex_literal::hex!("0100006f");
		const GENESIS_FORK_VERSION: Version = hex_literal::hex!("0000006f");
		const ALTAIR_FORK_EPOCH: Epoch = 90;
		const BELLATRIX_FORK_EPOCH: Epoch = 180;
		const CAPELLA_FORK_EPOCH: Epoch = 244224;
		const CAPELLA_FORK_VERSION: Version = hex_literal::hex!("0300006f");
		const DENEB_FORK_EPOCH: Epoch = 516608;
		const DENEB_FORK_VERSION: Version = hex_literal::hex!("0400006f");
		const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 512;
		const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 34;
		const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 38;
		const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 41;
		const EXECUTION_PAYLOAD_INDEX: u64 = 88;
		const NEXT_SYNC_COMMITTEE_INDEX: u64 = 87;
		const FINALIZED_ROOT_INDEX: u64 = 84;
		const FINALIZED_ROOT_INDEX_LOG2: u64 = 6;
		const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 6;
		const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 6;
		const ELECTRA_FORK_VERSION: Version = hex_literal::hex!("0500006f");
		const ELECTRA_FORK_EPOCH: Epoch = 948224;
		const FULU_FORK_EPOCH: Epoch = u64::MAX;
		const FULU_FORK_VERSION: Version = hex_literal::hex!("0600006f");
		const ID: [u8; 4] = GNOSIS_CONSENSUS_ID;
	}
}

pub mod devnet {
	use super::*;
	use hex_literal::hex;

	#[derive(Default)]
	pub struct ElectraDevnet;

	impl Config for ElectraDevnet {
		const SLOTS_PER_EPOCH: Slot = 32;
		const GENESIS_VALIDATORS_ROOT: [u8; 32] =
			hex_literal::hex!("83431ec7fcf92cfc44947fc0418e831c25e1d0806590231c439830db7ad54fda");
		const BELLATRIX_FORK_VERSION: Version = hex!("52525502");
		const ALTAIR_FORK_VERSION: Version = hex!("52525501");
		const GENESIS_FORK_VERSION: Version = hex!("52525500");
		const ALTAIR_FORK_EPOCH: Epoch = 0;
		const BELLATRIX_FORK_EPOCH: Epoch = 0;
		const CAPELLA_FORK_EPOCH: Epoch = 0;
		const CAPELLA_FORK_VERSION: Version = hex!("52525503");
		const DENEB_FORK_EPOCH: Epoch = 0;
		const DENEB_FORK_VERSION: Version = hex!("52525504");
		const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: Epoch = 4;
		const EXECUTION_PAYLOAD_STATE_ROOT_INDEX: u64 = 34;
		const EXECUTION_PAYLOAD_BLOCK_NUMBER_INDEX: u64 = 38;
		const EXECUTION_PAYLOAD_TIMESTAMP_INDEX: u64 = 41;
		const EXECUTION_PAYLOAD_INDEX: u64 = 88;
		const NEXT_SYNC_COMMITTEE_INDEX: u64 = 87;
		const FINALIZED_ROOT_INDEX: u64 = 84;
		const FINALIZED_ROOT_INDEX_LOG2: u64 = 6;
		const EXECUTION_PAYLOAD_INDEX_LOG2: u64 = 6;
		const NEXT_SYNC_COMMITTEE_INDEX_LOG2: u64 = 6;
		const ELECTRA_FORK_VERSION: Version = hex_literal::hex!("52525505");
		const ELECTRA_FORK_EPOCH: Epoch = 0;
		const FULU_FORK_EPOCH: Epoch = u64::MAX;
		const FULU_FORK_VERSION: Version = hex_literal::hex!("52525506");
		const ID: [u8; 4] = BEACON_CONSENSUS_ID;
	}
}
