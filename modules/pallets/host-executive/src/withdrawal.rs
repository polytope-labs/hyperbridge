use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use polkadot_sdk::*;
use sp_core::U256;

#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct WithdrawalParams {
	pub beneficiary_address: Vec<u8>,
	pub amount: U256,
	pub native: bool,
}

impl WithdrawalParams {
	pub fn abi_encode(&self) -> Vec<u8> {
		let mut data = vec![0];
		let tokens = [
			ethabi::Token::Address(ethabi::ethereum_types::H160::from_slice(
				&self.beneficiary_address,
			)),
			ethabi::Token::Uint(ethabi::ethereum_types::U256::from_big_endian(
				&self.amount.to_big_endian(),
			)),
			ethabi::Token::Bool(self.native),
		];
		let params = ethabi::encode(&tokens);
		data.extend_from_slice(&params);
		data
	}
}

#[cfg(test)]
mod test {
	use crate::withdrawal::WithdrawalParams;
	use polkadot_sdk::*;
	use sp_core::{H160, U256};
	#[test]
	fn check_decoding() {
		let params = WithdrawalParams {
			beneficiary_address: H160::random().0.to_vec(),
			amount: U256::from(500_00_000_000u128),
			native: false,
		};

		let encoding = params.abi_encode();

		assert_eq!(encoding.len(), 97);
	}
}
