#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod verifier;
pub use cometbft::{
	PublicKey as PubKey,
	block::{
		Commit, CommitSig, Header, Height, Id, parts::Header as PartSetHeader,
		signed_header::SignedHeader,
	},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub use cometbft_proto::types::v1;

pub use verifier::{
	CodecConsensusProof, CodecSignedHeader, CodecTrustedState, CodecValidator, ConsensusProof,
	TendermintCodecHeader, TrustedState, UpdatedTrustedState, VerificationError,
	VerificationOptions,
};

pub mod prover;

pub use prover::{Client, ProverError};
pub mod keys;
pub use keys::{DefaultEvmKeys, EvmStoreKeys, SeiEvmKeys};
