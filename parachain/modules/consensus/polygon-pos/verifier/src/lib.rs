#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
use anyhow::anyhow;
use geth_primitives::{CodecHeader, Header};
use ismp::util::Keccak256;
use primitives::{get_signature, hash_without_sig, parse_validators};
use sp_core::{H160, H256};
pub mod primitives;
use alloc::vec::Vec;

extern crate alloc;
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub hash: H256,
    pub header: CodecHeader,
    pub signer: H160,
    pub next_validators: Option<Vec<H160>>,
}
/// This function simply verifies a polygon block header
pub fn verify_polygon_header<I: Keccak256>(
    validators: &Vec<H160>,
    header: CodecHeader,
) -> Result<VerificationResult, anyhow::Error> {
    let signature = get_signature(&header.extra_data)?;
    let next_validators = parse_validators(&header.extra_data)?;
    let rlp_header: Header = (&header).into();
    let msg = hash_without_sig::<I>(rlp_header.clone())?;
    let address = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &msg.0)
        .map_err(|_| anyhow!("Signature verification failed"))?;
    let signer = H160::from_slice(&I::keccak256(&address[..]).0[12..]);
    log::info!(target: "pallet-ismp", "{signer:?}");
    log::info!(target: "pallet-ismp", "Validators {:#?}", validators);
    if !validators.contains(&signer) {
        Err(anyhow!("Header is signed by unknown validator"))?
    }
    let hash = rlp_header.hash::<I>();
    Ok(VerificationResult { hash, header, signer, next_validators })
}
