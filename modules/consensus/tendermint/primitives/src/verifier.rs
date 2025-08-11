use alloc::{
	format,
	string::{String, ToString},
	vec::Vec,
};
use core::time::Duration;

use crate::{SignedHeader, Validator};
use codec::{Decode, Encode};
use cometbft::validator::ProposerPriority;
use cometbft_proto::version::v1::Consensus;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedState {
	/// Chain ID
	pub chain_id: String,
	/// Block height
	pub height: u64,
	/// Block timestamp
	pub timestamp: u64,
	/// Hash of the finalized header
	pub finalized_header_hash: [u8; 32],
	/// Current validator set
	pub validators: Vec<Validator>,
	/// Next validator set
	pub next_validators: Vec<Validator>,
	/// Hash of the next validator set
	pub next_validators_hash: [u8; 32],
	/// Trusting period in seconds
	pub trusting_period: u64,
	/// Verification options for this consensus state
	pub verification_options: VerificationOptions,
}

impl TrustedState {
	pub fn new(
		chain_id: String,
		height: u64,
		timestamp: u64,
		finalized_header_hash: [u8; 32],
		validators: Vec<Validator>,
		next_validators: Vec<Validator>,
		next_validators_hash: [u8; 32],
		trusting_period: u64,
		verification_options: VerificationOptions,
	) -> Self {
		Self {
			chain_id,
			height,
			timestamp,
			finalized_header_hash,
			validators,
			next_validators,
			next_validators_hash,
			trusting_period,
			verification_options,
		}
	}

	pub fn trusting_period_duration(&self) -> Duration {
		Duration::from_secs(self.trusting_period)
	}

	/// Validate the trusted state
	pub fn validate(&self) -> Result<(), String> {
		if self.chain_id.is_empty() {
			return Err("Chain ID cannot be empty".to_string());
		}
		if self.height == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.timestamp == 0 {
			return Err("Timestamp cannot be zero".to_string());
		}
		if self.finalized_header_hash == [0u8; 32] {
			return Err("Finalized header hash cannot be zero".to_string());
		}
		if self.validators.is_empty() {
			return Err("Validator set cannot be empty".to_string());
		}
		if self.next_validators.is_empty() {
			return Err("Next validator set cannot be empty".to_string());
		}
		if self.trusting_period == 0 {
			return Err("Trusting period cannot be zero".to_string());
		}
		// Validate verification options
		self.verification_options.validate()?;
		Ok(())
	}

	/// Check if the state is valid for a given height
	pub fn is_valid_for_height(&self, height: u64) -> bool {
		height <= self.height
	}

	/// Update the finalized header hash
	pub fn update_finalized_header_hash(&mut self, new_hash: [u8; 32]) {
		self.finalized_header_hash = new_hash;
	}

	/// Get the finalized header hash
	pub fn get_finalized_header_hash(&self) -> [u8; 32] {
		self.finalized_header_hash
	}

	/// Get verification options
	pub fn get_verification_options(&self) -> &VerificationOptions {
		&self.verification_options
	}

	/// Update verification options
	pub fn update_verification_options(&mut self, options: VerificationOptions) {
		self.verification_options = options;
	}
}

impl Default for TrustedState {
	fn default() -> Self {
		Self {
			chain_id: "test-chain".to_string(),
			height: 1,
			timestamp: 0,
			finalized_header_hash: [0u8; 32],
			validators: Vec::new(),
			next_validators: Vec::new(),
			next_validators_hash: [0u8; 32],
			trusting_period: 3600, // 1 hour default
			verification_options: VerificationOptions::default(),
		}
	}
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub struct CodecTrustedState {
	/// Chain ID
	pub chain_id: String,
	/// Block height
	pub height: u64,
	/// Block timestamp
	pub timestamp: u64,
	/// Hash of the finalized header
	pub finalized_header_hash: [u8; 32],
	/// Current validator set
	pub validators: Vec<CodecValidator>,
	/// Next validator set
	pub next_validators: Vec<CodecValidator>,
	/// Hash of the next validator set
	pub next_validators_hash: [u8; 32],
	/// Trusting period in seconds
	pub trusting_period: u64,
	/// Verification options for this consensus state
	pub verification_options: VerificationOptions,
}

impl From<CodecTrustedState> for TrustedState {
	fn from(codec_state: CodecTrustedState) -> Self {
		Self {
			chain_id: codec_state.chain_id,
			height: codec_state.height,
			timestamp: codec_state.timestamp,
			finalized_header_hash: codec_state.finalized_header_hash,
			validators: codec_state
				.validators
				.into_iter()
				.map(|v| v.to_validator().expect("Failed to convert CodecValidator to Validator"))
				.collect(),
			next_validators: codec_state
				.next_validators
				.into_iter()
				.map(|v| v.to_validator().expect("Failed to convert CodecValidator to Validator"))
				.collect(),
			next_validators_hash: codec_state.next_validators_hash,
			trusting_period: codec_state.trusting_period,
			verification_options: codec_state.verification_options,
		}
	}
}

impl From<&TrustedState> for CodecTrustedState {
	fn from(trusted_state: &TrustedState) -> Self {
		Self {
			chain_id: trusted_state.chain_id.clone(),
			height: trusted_state.height,
			timestamp: trusted_state.timestamp,
			finalized_header_hash: trusted_state.finalized_header_hash,
			validators: trusted_state
				.validators
				.iter()
				.map(|validators| CodecValidator::from(validators))
				.collect(),
			next_validators: trusted_state
				.next_validators
				.iter()
				.map(|validators| CodecValidator::from(validators))
				.collect(),
			next_validators_hash: trusted_state.next_validators_hash,
			trusting_period: trusted_state.trusting_period,
			verification_options: trusted_state.verification_options.clone(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusProof {
	/// Signed header containing the block header and commit
	pub signed_header: SignedHeader,
	/// Next validator set  (optional) - target height + 1
	pub next_validators: Option<Vec<Validator>>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecConsensusProof {
	/// Signed header containing the block header and commit
	pub signed_header: CodecSignedHeader,
	/// Next validator set  (optional) - target height + 1
	pub next_validators: Option<Vec<CodecValidator>>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecSignedHeader {
	/// Block header
	pub header: TendermintCodecHeader,
	/// Commit containing signatures for the header
	pub commit: CodecCommit,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct TendermintCodecHeader {
	/// Header version
	pub version: CodecVersion,
	/// Chain ID
	pub chain_id: String,
	/// Current block height
	pub height: u64,
	/// Current timestamp (RFC 3339 format)
	pub time: String,
	/// Previous block info
	pub last_block_id: Option<CodecBlockId>,
	/// Commit from validators from the last block
	pub last_commit_hash: Option<Vec<u8>>,
	/// Merkle root of transaction hashes
	pub data_hash: Option<Vec<u8>>,
	/// Validators for the current block
	pub validators_hash: Vec<u8>,
	/// Validators for the next block
	pub next_validators_hash: Vec<u8>,
	/// Consensus params for the current block
	pub consensus_hash: Vec<u8>,
	/// State after txs from the previous block
	pub app_hash: Vec<u8>,
	/// Root hash of all results from the txs from the previous block
	pub last_results_hash: Option<Vec<u8>>,
	/// Hash of evidence included in the block
	pub evidence_hash: Option<Vec<u8>>,
	/// Original proposer of the block
	pub proposer_address: Vec<u8>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecVersion {
	pub block: u64,
	pub app: u64,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecBlockId {
	/// The block's main hash is the Merkle root of all the fields in the block header.
	pub hash: Vec<u8>,
	/// Parts header (if available) is used for secure gossipping of the block during consensus.
	pub part_set_header: CodecPartSetHeader,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecPartSetHeader {
	/// Number of parts in this block
	pub total: u32,
	/// Hash of the parts set header
	pub hash: Vec<u8>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub struct CodecCommit {
	/// Block height
	pub height: u64,
	/// Round
	pub round: u32,
	/// Block ID
	pub block_id: CodecBlockId,
	/// Signatures
	pub signatures: Vec<CodecCommitSig>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo)]
pub enum CodecCommitSig {
	/// no vote was received from a validator.
	BlockIdFlagAbsent,
	/// voted for the Commit.BlockID.
	BlockIdFlagCommit {
		/// Validator address
		validator_address: Vec<u8>,
		/// Timestamp of vote (RFC 3339 format)
		timestamp: String,
		/// Signature of vote
		signature: Option<Vec<u8>>,
	},
	/// voted for nil.
	BlockIdFlagNil {
		/// Validator address
		validator_address: Vec<u8>,
		/// Timestamp of vote (RFC 3339 format)
		timestamp: String,
		/// Signature of vote
		signature: Option<Vec<u8>>,
	},
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub struct CodecValidator {
	/// Validator account address
	pub address: Vec<u8>,
	/// Validator public key
	pub pub_key: CodecPublicKey,
	/// Validator voting power
	pub power: u64,
	/// Validator name
	pub name: Option<String>,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub enum CodecPublicKey {
	/// Ed25519 keys
	Ed25519(Vec<u8>),
	/// Secp256k1 keys
	Secp256k1(Vec<u8>),
}

impl AsRef<CodecConsensusProof> for CodecConsensusProof {
	fn as_ref(&self) -> &CodecConsensusProof {
		&self
	}
}

impl CodecConsensusProof {
	/// Convert back to ConsensusProof
	pub fn to_consensus_proof(&self) -> Result<ConsensusProof, String> {
		let signed_header = self.signed_header.to_signed_header()?;
		let next_validators = if let Some(ref validators) = self.next_validators {
			Some(
				validators
					.iter()
					.map(|codec_validator| codec_validator.to_validator())
					.collect::<Result<Vec<_>, _>>()?,
			)
		} else {
			None
		};

		Ok(ConsensusProof { signed_header, next_validators })
	}
}

impl CodecSignedHeader {
	/// Convert back to SignedHeader
	pub fn to_signed_header(&self) -> Result<SignedHeader, String> {
		let header = self.header.to_header()?;
		let commit = self.commit.to_commit()?;

		let signed_header = SignedHeader::new(header, commit).map_err(|e| e.to_string())?;

		Ok(signed_header)
	}
}

impl TendermintCodecHeader {
	/// Convert back to Header
	pub fn to_header(&self) -> Result<crate::Header, String> {
		let version = Consensus { block: self.version.block, app: self.version.app }.into();
		let chain_id = cometbft::chain::Id::try_from(self.chain_id.as_str())
			.map_err(|e| format!("Invalid chain ID: {}", e))?;
		let height = cometbft::block::Height::try_from(self.height)
			.map_err(|e| format!("Invalid height: {}", e))?;
		let time = cometbft::Time::parse_from_rfc3339(&self.time)
			.map_err(|e| format!("Invalid timestamp: {}", e))?;

		let last_block_id = if let Some(ref block_id) = self.last_block_id {
			Some(block_id.to_block_id()?)
		} else {
			None
		};

		let last_commit_hash = if let Some(ref hash_bytes) = self.last_commit_hash {
			Some(
				cometbft::Hash::try_from(hash_bytes.to_vec())
					.map_err(|e| format!("Invalid last commit hash: {}", e))?,
			)
		} else {
			None
		};

		let data_hash = if let Some(ref hash_bytes) = self.data_hash {
			Some(
				cometbft::Hash::try_from(hash_bytes.to_vec())
					.map_err(|e| format!("Invalid data hash: {}", e))?,
			)
		} else {
			None
		};

		let validators_hash = cometbft::Hash::try_from(self.validators_hash.to_vec())
			.map_err(|e| format!("Invalid validators hash: {}", e))?;
		let next_validators_hash = cometbft::Hash::try_from(self.next_validators_hash.to_vec())
			.map_err(|e| format!("Invalid next validators hash: {}", e))?;
		let consensus_hash = cometbft::Hash::try_from(self.consensus_hash.to_vec())
			.map_err(|e| format!("Invalid consensus hash: {}", e))?;
		let app_hash = cometbft::AppHash::try_from(self.app_hash.to_vec())
			.map_err(|e| format!("Invalid app hash: {}", e))?;

		let last_results_hash = if let Some(ref hash_bytes) = self.last_results_hash {
			Some(
				cometbft::Hash::try_from(hash_bytes.to_vec())
					.map_err(|e| format!("Invalid last results hash: {}", e))?,
			)
		} else {
			None
		};

		let evidence_hash = if let Some(ref hash_bytes) = self.evidence_hash {
			Some(
				cometbft::Hash::try_from(hash_bytes.to_vec())
					.map_err(|e| format!("Invalid evidence hash: {}", e))?,
			)
		} else {
			None
		};

		let proposer_address = cometbft::account::Id::try_from(self.proposer_address.to_vec())
			.map_err(|e| format!("Invalid proposer address: {}", e))?;

		Ok(crate::Header {
			version,
			chain_id,
			height,
			time,
			last_block_id,
			last_commit_hash,
			data_hash,
			validators_hash,
			next_validators_hash,
			consensus_hash,
			app_hash,
			last_results_hash,
			evidence_hash,
			proposer_address,
		})
	}
}

impl CodecBlockId {
	/// Convert back to block::Id
	pub fn to_block_id(&self) -> Result<crate::Id, String> {
		let hash = cometbft::Hash::try_from(self.hash.to_vec())
			.map_err(|e| format!("Invalid block hash: {}", e))?;
		let part_set_header = self.part_set_header.to_part_set_header()?;
		Ok(crate::Id { hash, part_set_header })
	}
}

impl CodecPartSetHeader {
	/// Convert back to block::parts::Header
	pub fn to_part_set_header(&self) -> Result<crate::PartSetHeader, String> {
		let hash = cometbft::Hash::try_from(self.hash.to_vec())
			.map_err(|e| format!("Invalid part set header hash: {}", e))?;
		let part_set_header =
			crate::PartSetHeader::new(self.total, hash).map_err(|e| e.to_string())?;
		Ok(part_set_header)
	}
}

impl CodecCommit {
	/// Convert back to Commit
	pub fn to_commit(&self) -> Result<crate::Commit, String> {
		let height = cometbft::block::Height::try_from(self.height)
			.map_err(|e| format!("Invalid commit height: {}", e))?;
		let round = cometbft::block::Round::try_from(self.round)
			.map_err(|e| format!("Invalid commit round: {}", e))?;
		let block_id = self.block_id.to_block_id()?;
		let signatures = self
			.signatures
			.iter()
			.map(|codec_sig| codec_sig.to_commit_sig())
			.collect::<Result<Vec<_>, _>>()?;
		Ok(crate::Commit { height, round, block_id, signatures })
	}
}

impl CodecCommitSig {
	/// Convert back to CommitSig
	pub fn to_commit_sig(&self) -> Result<crate::CommitSig, String> {
		match self {
			CodecCommitSig::BlockIdFlagAbsent => Ok(crate::CommitSig::BlockIdFlagAbsent),
			CodecCommitSig::BlockIdFlagCommit { validator_address, timestamp, signature } => {
				let validator_address = cometbft::account::Id::try_from(validator_address.to_vec())
					.map_err(|e| format!("Invalid validator address: {}", e))?;
				let timestamp = cometbft::Time::parse_from_rfc3339(timestamp)
					.map_err(|e| format!("Invalid timestamp: {}", e))?;
				let signature = if let Some(sig_bytes) = signature {
					Some(
						cometbft::Signature::try_from(sig_bytes.as_slice())
							.map_err(|e| format!("Invalid signature: {}", e))?,
					)
				} else {
					None
				};
				Ok(crate::CommitSig::BlockIdFlagCommit { validator_address, timestamp, signature })
			},
			CodecCommitSig::BlockIdFlagNil { validator_address, timestamp, signature } => {
				let validator_address = cometbft::account::Id::try_from(validator_address.to_vec())
					.map_err(|e| format!("Invalid validator address: {}", e))?;
				let timestamp = cometbft::Time::parse_from_rfc3339(timestamp)
					.map_err(|e| format!("Invalid timestamp: {}", e))?;
				let signature = if let Some(sig_bytes) = signature {
					Some(
						cometbft::Signature::try_from(sig_bytes.as_slice())
							.map_err(|e| format!("Invalid signature: {}", e))?,
					)
				} else {
					None
				};
				Ok(crate::CommitSig::BlockIdFlagNil { validator_address, timestamp, signature })
			},
		}
	}
}

impl CodecValidator {
	/// Convert back to Validator
	pub fn to_validator(&self) -> Result<Validator, String> {
		let address = cometbft::account::Id::try_from(self.address.to_vec())
			.map_err(|e| format!("Invalid validator address: {}", e))?;
		let pub_key = self.pub_key.to_public_key()?;
		let power = cometbft::vote::Power::try_from(self.power)
			.map_err(|e| format!("Invalid validator power: {}", e))?;
		Ok(Validator {
			address,
			pub_key,
			power,
			name: self.name.clone(),
			proposer_priority: ProposerPriority::from(self.power as i64),
		})
	}
}

impl CodecPublicKey {
	/// Convert back to PublicKey
	pub fn to_public_key(&self) -> Result<cometbft::public_key::PublicKey, String> {
		match self {
			CodecPublicKey::Ed25519(key_bytes) =>
				cometbft::public_key::PublicKey::from_raw_ed25519(key_bytes)
					.ok_or_else(|| format!("Invalid Ed25519 public key")),
			CodecPublicKey::Secp256k1(key_bytes) =>
				cometbft::public_key::PublicKey::from_raw_secp256k1(key_bytes)
					.ok_or_else(|| format!("Invalid Secp256k1 public key")),
		}
	}
}

impl From<&ConsensusProof> for CodecConsensusProof {
	fn from(proof: &ConsensusProof) -> Self {
		CodecConsensusProof {
			signed_header: CodecSignedHeader::from(&proof.signed_header),
			next_validators: proof
				.next_validators
				.as_ref()
				.map(|validators| validators.iter().map(CodecValidator::from).collect()),
		}
	}
}

impl From<&SignedHeader> for CodecSignedHeader {
	fn from(signed_header: &SignedHeader) -> Self {
		CodecSignedHeader {
			header: TendermintCodecHeader::from(&signed_header.header),
			commit: CodecCommit::from(&signed_header.commit),
		}
	}
}

impl From<&crate::Header> for TendermintCodecHeader {
	fn from(header: &crate::Header) -> Self {
		TendermintCodecHeader {
			version: CodecVersion { block: header.version.block, app: header.version.app },
			chain_id: header.chain_id.to_string(),
			height: header.height.value(),
			time: header.time.to_rfc3339(),
			last_block_id: header.last_block_id.as_ref().map(CodecBlockId::from),
			last_commit_hash: header.last_commit_hash.as_ref().map(|hash| hash.as_bytes().to_vec()),
			data_hash: header.data_hash.as_ref().map(|hash| hash.as_bytes().to_vec()),
			validators_hash: header.validators_hash.as_bytes().to_vec(),
			next_validators_hash: header.next_validators_hash.as_bytes().to_vec(),
			consensus_hash: header.consensus_hash.as_bytes().to_vec(),
			app_hash: header.app_hash.as_bytes().to_vec(),
			last_results_hash: header
				.last_results_hash
				.as_ref()
				.map(|hash| hash.as_bytes().to_vec()),
			evidence_hash: header.evidence_hash.as_ref().map(|hash| hash.as_bytes().to_vec()),
			proposer_address: header.proposer_address.as_bytes().to_vec(),
		}
	}
}

impl From<&crate::Id> for CodecBlockId {
	fn from(block_id: &crate::Id) -> Self {
		CodecBlockId {
			hash: block_id.hash.as_bytes().to_vec(),
			part_set_header: CodecPartSetHeader::from(&block_id.part_set_header),
		}
	}
}

impl From<&crate::PartSetHeader> for CodecPartSetHeader {
	fn from(part_set_header: &crate::PartSetHeader) -> Self {
		CodecPartSetHeader {
			total: part_set_header.total,
			hash: part_set_header.hash.as_bytes().to_vec(),
		}
	}
}

impl From<&crate::Commit> for CodecCommit {
	fn from(commit: &crate::Commit) -> Self {
		CodecCommit {
			height: commit.height.value(),
			round: commit.round.value(),
			block_id: CodecBlockId::from(&commit.block_id),
			signatures: commit.signatures.iter().map(CodecCommitSig::from).collect(),
		}
	}
}

impl From<&crate::CommitSig> for CodecCommitSig {
	fn from(commit_sig: &crate::CommitSig) -> Self {
		match commit_sig {
			crate::CommitSig::BlockIdFlagAbsent => CodecCommitSig::BlockIdFlagAbsent,
			crate::CommitSig::BlockIdFlagCommit { validator_address, timestamp, signature } =>
				CodecCommitSig::BlockIdFlagCommit {
					validator_address: validator_address.as_bytes().to_vec(),
					timestamp: timestamp.to_rfc3339(),
					signature: signature.as_ref().map(|sig| sig.as_bytes().to_vec()),
				},
			crate::CommitSig::BlockIdFlagNil { validator_address, timestamp, signature } =>
				CodecCommitSig::BlockIdFlagNil {
					validator_address: validator_address.as_bytes().to_vec(),
					timestamp: timestamp.to_rfc3339(),
					signature: signature.as_ref().map(|sig| sig.as_bytes().to_vec()),
				},
		}
	}
}

impl From<&Validator> for CodecValidator {
	fn from(validator: &Validator) -> Self {
		CodecValidator {
			address: validator.address.as_bytes().to_vec(),
			pub_key: CodecPublicKey::from(&validator.pub_key),
			power: validator.power.value(),
			name: validator.name.clone(),
		}
	}
}

impl From<&cometbft::public_key::PublicKey> for CodecPublicKey {
	fn from(pub_key: &cometbft::public_key::PublicKey) -> Self {
		match pub_key {
			cometbft::public_key::PublicKey::Ed25519(key) =>
				CodecPublicKey::Ed25519(key.as_bytes().to_vec()),
			cometbft::public_key::PublicKey::Secp256k1(key) => {
				let key_bytes = key.to_encoded_point(false);
				CodecPublicKey::Secp256k1(key_bytes.as_bytes().to_vec())
			},
			_ => CodecPublicKey::Ed25519(pub_key.to_bytes().to_vec()),
		}
	}
}

impl ConsensusProof {
	pub fn new(signed_header: SignedHeader, next_validators: Option<Vec<Validator>>) -> Self {
		Self { signed_header, next_validators }
	}

	pub fn height(&self) -> u64 {
		self.signed_header.header.height.value()
	}

	pub fn timestamp(&self) -> u64 {
		self.signed_header.header.time.unix_timestamp() as u64
	}

	pub fn chain_id(&self) -> &str {
		self.signed_header.header.chain_id.as_str()
	}

	/// Validate the consensus proof
	pub fn validate(&self) -> Result<(), String> {
		// Validate that if next_validators_hash is not empty, next_validators must be provided
		let header_next_validators_hash = &self.signed_header.header.next_validators_hash;
		if !header_next_validators_hash.is_empty() {
			// Hash is not empty, so next_validators must be provided
			if self.next_validators.is_none() {
				return Err("Header has non-empty next_validators_hash but consensus proof has no next_validators".to_string());
			}
			if self.next_validators.as_ref().unwrap().is_empty() {
				return Err("Header has non-empty next_validators_hash but consensus proof has empty next_validators".to_string());
			}
		}

		if self.height() == 0 {
			return Err("Height cannot be zero".to_string());
		}
		if self.timestamp() == 0 {
			return Err("Timestamp cannot be zero".to_string());
		}
		if self.chain_id().is_empty() {
			return Err("Chain ID cannot be empty".to_string());
		}
		Ok(())
	}

	/// Check if the proof has next validators
	pub fn has_next_validators(&self) -> bool {
		self.next_validators.is_some() && !self.next_validators.as_ref().unwrap().is_empty()
	}

	/// Get the next validators if available
	pub fn get_next_validators(&self) -> Option<&Vec<Validator>> {
		self.next_validators.as_ref()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, TypeInfo)]
pub struct VerificationOptions {
	/// Trust threshold as a fraction (numerator/denominator)
	/// Default is 2/3
	pub trust_threshold_numerator: u64,
	pub trust_threshold_denominator: u64,
	/// Clock drift tolerance in seconds
	pub clock_drift: u64,
}

impl VerificationOptions {
	pub fn new(
		trust_threshold_numerator: u64,
		trust_threshold_denominator: u64,
		clock_drift: u64,
	) -> Self {
		Self { trust_threshold_numerator, trust_threshold_denominator, clock_drift }
	}

	pub fn trust_threshold_fraction(&self) -> f64 {
		self.trust_threshold_numerator as f64 / self.trust_threshold_denominator as f64
	}

	pub fn clock_drift_duration(&self) -> Duration {
		Duration::from_secs(self.clock_drift)
	}

	/// Validate the verification options
	pub fn validate(&self) -> Result<(), String> {
		if self.trust_threshold_numerator == 0 {
			return Err("Trust threshold numerator cannot be zero".to_string());
		}
		if self.trust_threshold_denominator == 0 {
			return Err("Trust threshold denominator cannot be zero".to_string());
		}
		if self.trust_threshold_numerator > self.trust_threshold_denominator {
			return Err("Trust threshold numerator cannot be greater than denominator".to_string());
		}
		if self.trust_threshold_fraction() < 0.5 {
			return Err("Trust threshold must be at least 0.5 (50%)".to_string());
		}
		if self.trust_threshold_fraction() > 1.0 {
			return Err("Trust threshold cannot be greater than 1.0 (100%)".to_string());
		}
		Ok(())
	}

	/// Create default verification options (2/3 trust threshold, 180 seconds clock drift)
	pub fn create_default() -> Self {
		Self { trust_threshold_numerator: 2, trust_threshold_denominator: 3, clock_drift: 180 }
	}

	/// Create verification options with custom trust threshold
	pub fn with_trust_threshold(
		trust_threshold_numerator: u64,
		trust_threshold_denominator: u64,
	) -> Self {
		Self {
			trust_threshold_numerator,
			trust_threshold_denominator,
			clock_drift: 5, // Default 5 seconds
		}
	}
}

impl Default for VerificationOptions {
	fn default() -> Self {
		Self::create_default()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatedTrustedState {
	/// The new trusted state
	pub trusted_state: TrustedState,
	/// Height of the verified header
	pub verified_height: u64,
	/// Timestamp of the verified header
	pub verified_timestamp: u64,
}

impl UpdatedTrustedState {
	pub fn new(trusted_state: TrustedState, verified_height: u64, verified_timestamp: u64) -> Self {
		Self { trusted_state, verified_height, verified_timestamp }
	}

	/// Get the height difference between the old and new trusted state
	pub fn height_difference(&self) -> u64 {
		self.verified_height.saturating_sub(self.trusted_state.height)
	}

	/// Get the time difference between the old and new trusted state
	pub fn time_difference(&self) -> u64 {
		self.verified_timestamp.saturating_sub(self.trusted_state.timestamp)
	}

	/// Check if the update was successful
	pub fn is_successful(&self) -> bool {
		self.verified_height > self.trusted_state.height
	}
}

use thiserror::Error;

/// Errors that can occur during Tendermint verification
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
	/// Verification failed due to insufficient voting power
	#[error("Not enough validators signed the block: {0}")]
	NotEnoughTrust(String),

	/// Verification failed due to invalid data
	#[error("Invalid verification data: {0}")]
	Invalid(String),

	/// Time-related verification error
	#[error("Time verification failed: {0}")]
	TimeError(String),

	/// Height-related verification error
	#[error("Height verification failed: {0}")]
	HeightError(String),

	/// Chain ID mismatch
	#[error("Chain ID mismatch: expected {expected}, got {got}")]
	ChainIdMismatch { expected: String, got: String },

	/// Validator set verification failed
	#[error("Validator set verification failed: {0}")]
	ValidatorSetError(String),

	/// Commit verification failed
	#[error("Commit verification failed: {0}")]
	CommitError(String),

	/// Trust period expired
	#[error("Trust period expired: {0}")]
	TrustPeriodExpired(String),

	/// Header from the future
	#[error("Header timestamp is in the future: {0}")]
	HeaderFromFuture(String),

	/// Conversion error
	#[error("Conversion error: {0}")]
	ConversionError(String),

	/// State validation error
	#[error("State validation failed: {0}")]
	StateValidationError(String),

	/// Configuration error
	#[error("Configuration error: {0}")]
	ConfigurationError(String),
}
