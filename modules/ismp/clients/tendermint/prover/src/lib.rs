pub use tendermint::{
	block::{signed_header::SignedHeader, Header, Height},
	chain::Id as ChainId,
	hash::Hash,
	time::Time,
	validator::{Info as Validator, Set as ValidatorSet},
};

pub mod error;
pub mod prover;
pub mod rpc_client;

pub use error::ProverError;
pub use prover::{prove_header_update, prove_misbehaviour_header};
pub use rpc_client::TendermintRpcClient;

pub use tendermint_verifier::{
	ConsensusProof, TrustedState, UpdatedTrustedState, VerificationOptions,
};
