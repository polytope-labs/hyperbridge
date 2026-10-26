use alloy_primitives::Bloom;

use crate::types::EthLog;

pub fn block_bloom(logs: &[EthLog]) -> Bloom {
	let mut bloom = Bloom::default();
	for log in logs {
		bloom.accrue_raw_log(log.address, &log.topics);
	}
	bloom
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::{BloomInput, Bytes, B256};

	#[test]
	fn bloom_contains_address_and_topics() {
		let address = crate::types::pallet_address("ismp");
		let topics = vec![B256::repeat_byte(0x01), B256::repeat_byte(0x02)];
		let log = EthLog {
			address,
			topics: topics.clone(),
			data: Bytes::new(),
			block_hash: B256::ZERO,
			block_number: 1,
			block_timestamp: 0,
			transaction_hash: B256::ZERO,
			transaction_index: 0,
			log_index: 0,
		};
		let bloom = block_bloom(&[log]);
		assert!(bloom.contains_input(BloomInput::Raw(address.as_slice())));
		for topic in &topics {
			assert!(bloom.contains_input(BloomInput::Raw(topic.as_slice())));
		}
		assert!(!bloom.contains_input(BloomInput::Raw(B256::repeat_byte(0x03).as_slice())));
		assert!(block_bloom(&[]).is_zero());
	}
}
