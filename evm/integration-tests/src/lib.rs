#![allow(unused_parens)]

// #[cfg(test)]
// mod tests;

pub use ethers::{abi::Token, types::U256, utils::keccak256};
use merkle_mountain_range::{util::MemMMR, Error, Merge};
use pallet_ismp::offchain::Leaf;
use pallet_mmr_tree::mmr::Hasher;
use polkadot_sdk::*;
use primitive_types::H256;
use sp_runtime::traits;

#[derive(Clone, Default)]
pub struct Keccak256;

pub type DataOrHash = mmr_primitives::DataOrHash<traits::Keccak256, Leaf>;

impl rs_merkle::Hasher for Keccak256 {
	type Hash = [u8; 32];

	fn hash(data: &[u8]) -> [u8; 32] {
		keccak256(data)
	}
}

impl ismp::messaging::Keccak256 for Keccak256 {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		keccak256(bytes).into()
	}
}

struct MergeKeccak;

impl Merge for MergeKeccak {
	type Item = NumberHash;
	fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item, Error> {
		let mut concat = vec![];
		concat.extend(&lhs.0);
		concat.extend(&rhs.0);
		let hash = keccak256(&concat);
		Ok(NumberHash(hash.to_vec().into()))
	}
}

#[derive(Eq, PartialEq, Clone, Debug, Default)]
struct NumberHash(pub Vec<u8>);

impl From<u32> for NumberHash {
	fn from(num: u32) -> Self {
		let hash = keccak256(&num.to_le_bytes());
		NumberHash(hash.to_vec())
	}
}

pub type Mmr = MemMMR<DataOrHash, Hasher<traits::Keccak256, Leaf>>;
