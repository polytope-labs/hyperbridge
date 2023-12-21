#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
use anyhow::anyhow;
use ismp::host::IsmpHost;
use primitives::{get_signature, parse_validators, CodecHeader, Header, VerifierState};
use sp_core::{H160, H256};
pub mod primitives;
use alloc::vec::Vec;

extern crate alloc;

pub trait EcdsaRecover {
    fn recover(sig: [u8; 65], msg: H256) -> Result<H160, anyhow::Error>;
}

pub struct VerificationResult {
    pub hash: H256,
    pub header: CodecHeader,
    pub next_validators: Option<Vec<H160>>,
}
/// This function simply verifies a polygon block header
pub fn verify_polygon_header<I: IsmpHost, E: EcdsaRecover>(
    trusted_state: VerifierState,
    header: CodecHeader,
) -> Result<VerificationResult, anyhow::Error> {
    if header.number.as_u64() <= trusted_state.finalized_height {
        Err(anyhow!("Expired update"))?
    }
    let signature = get_signature(&header.extra_data)?;
    let next_validators = parse_validators(&header.extra_data)?;
    let rlp_header: Header = (&header).into();
    let hash = rlp_header.hash::<I>()?;
    let signer = <E as EcdsaRecover>::recover(signature, hash)?;
    if !trusted_state.validators.contains(&signer) {
        Err(anyhow!("Header is signed by unknown validator"))?
    }
    Ok(VerificationResult { hash, header, next_validators })
}
