use alloc::collections::BTreeSet;
use anyhow::anyhow;
use ethabi::ethereum_types::{H160, H256};

use geth_primitives::Header;
use ismp::util::Keccak256;

const EXTRA_VANITY_LENGTH: usize = 32;
const EXTRA_SEAL_LENGTH: usize = 65;
pub const SPAN_LENGTH: u64 = 400 * 16;

pub fn hash_without_sig<H: Keccak256>(mut header: Header) -> Result<H256, anyhow::Error> {
    if header.extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
        Err(anyhow!("Invalid extra data"))?
    }
    let slice = header.extra_data.len() - EXTRA_SEAL_LENGTH;
    header.extra_data = {
        let bytes = header.extra_data[..slice].to_vec();
        bytes.into()
    };
    let encoding = alloy_rlp::encode(header);
    Ok(H::keccak256(&encoding))
}

pub fn get_signature(extra_data: &[u8]) -> Result<[u8; EXTRA_SEAL_LENGTH], anyhow::Error> {
    if extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
        Err(anyhow!("Invalid extra data"))?
    }

    let mut sig = [0u8; 65];
    sig.copy_from_slice(&extra_data[extra_data.len() - EXTRA_SEAL_LENGTH..]);
    Ok(sig)
}

pub fn parse_validators(extra_data: &[u8]) -> Result<Option<BTreeSet<H160>>, anyhow::Error> {
    if extra_data.len() < (EXTRA_VANITY_LENGTH + EXTRA_SEAL_LENGTH) {
        Err(anyhow!("Invalid extra data"))?
    }

    let slice = &extra_data[EXTRA_VANITY_LENGTH..(extra_data.len() - EXTRA_SEAL_LENGTH)];

    if slice.len() == 0 {
        return Ok(None)
    }

    if slice.len() % 20 != 0 {
        Err(anyhow!("Invalid block extra data"))?
    }
    let mut validators = BTreeSet::new();
    for chunk in slice.chunks(20) {
        let address = H160::from_slice(&chunk[..]);
        validators.insert(address);
    }
    Ok(Some(validators))
}
