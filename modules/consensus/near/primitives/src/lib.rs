#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use async_trait::async_trait;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

pub mod prover;
pub mod verifier;

pub use prover::{Client, ConsensusProof, ProverError, TrustedState};

pub use near_primitives::{
	hash::CryptoHash,
	types::{BlockHeight, BlockReference, TransactionOrReceiptId},
	views::{
		validator_stake_view::ValidatorStakeView, BlockHeaderView, LightClientBlockLiteView,
		LightClientBlockView,
	},
};

pub type BlockHash = CryptoHash;
pub type EpochId = CryptoHash;
pub type ValidatorStake = ValidatorStakeView;
