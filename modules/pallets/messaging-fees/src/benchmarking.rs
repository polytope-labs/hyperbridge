use super::*;

use alloc::vec;
use codec::{Decode, Encode};
use frame_benchmarking::v2::*;
use frame_support::{
	storage::{storage_prefix, unhashed},
	traits::Get,
};
use frame_system::RawOrigin;
use ismp::host::StateMachine;
use polkadot_sdk::sp_core::U256;
use sp_runtime::Saturating;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn migrate_evm_fees() {
		let fee_storage_prefix = storage_prefix(b"Relayer", b"Fees");

		let evm_chain = StateMachine::Evm(1);
		let relayer_addr_32 = vec![1u8; 32];

		let fee_decimals = 6u8;
		let fee_18_decimals = U256::from(100_000_000_000_000_000_000u128);
		let scaling_power = 18u32.saturating_sub(fee_decimals as u32);
		let divisor = U256::from(10u128).pow(U256::from(scaling_power));
		let expected_fee = fee_18_decimals.checked_div(divisor).unwrap();

		let key1_hash = sp_io::hashing::blake2_128(&evm_chain.encode());
		let key2_hash = sp_io::hashing::blake2_128(&relayer_addr_32.encode());

		let key_suffix = [
			key1_hash.as_slice(),
			&evm_chain.encode(),
			key2_hash.as_slice(),
			&relayer_addr_32.encode(),
		]
		.concat();
		let full_key = [fee_storage_prefix.as_slice(), key_suffix.as_slice()].concat();

		unhashed::put(&full_key, &fee_18_decimals);
		pallet_ismp_host_executive::FeeTokenDecimals::<T>::insert(&evm_chain, fee_decimals);

		assert_eq!(unhashed::get::<U256>(&full_key), Some(fee_18_decimals));

		#[block]
		{
			let mut key_part = &key_suffix[16..];
			if let Ok(state_machine) = StateMachine::decode(&mut key_part) {
				if state_machine.is_evm() {
					if key_part.len() > 16 {
						let mut relayer_address_bytes = &key_part[16..];
						if let Ok(relayer_address) = Vec::<u8>::decode(&mut relayer_address_bytes) {
							if relayer_address.len() == 32 {
								let current_fee =
									unhashed::get::<U256>(&full_key).unwrap_or_default();
								if let Some(decimals) = pallet_ismp_host_executive::FeeTokenDecimals::<
									T,
								>::get(&state_machine)
								{
									let decimals_u32 = decimals as u32;
									let scaling_power = 18u32.saturating_sub(decimals_u32);

									if scaling_power > 0 {
										let divisor =
											U256::from(10u128).pow(U256::from(scaling_power));
										let new_fee = current_fee
											.checked_div(divisor)
											.unwrap_or(U256::zero());
										unhashed::put(&full_key, &new_fee);
									}
								}
							}
						}
					}
				}
			}
		}

		assert_eq!(unhashed::get::<U256>(&full_key), Some(expected_fee));
	}
}
