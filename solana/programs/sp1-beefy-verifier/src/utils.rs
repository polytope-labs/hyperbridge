//! Vendored from `succinctlabs/sp1-solana` @ master `verifier/src/utils.rs` (MIT).
//!
//! We copy the proof-parsing and VK-loading helpers verbatim; the only thing we
//! *don't* bring over is the `verify_proof` / `verify_proof_raw` entry points
//! (those are SP1-v5-specific — we supply our own v6 entry point in
//! `verifier.rs`).
//!
//! Original copyright: Copyright (c) 2024 Bhargav Annem. MIT.

use ark_bn254::{Fq, G1Affine};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum Error {
    G1CompressionError,
    G2CompressionError,
    VerificationError,
    InvalidPublicInputsLength,
    InvalidProof,
    InvalidInput,
    InvalidProgramVkeyHash,
    Groth16VkeyHashMismatch,
    VkRootMismatch,
    ExitCodeMismatch,
}

pub struct Proof {
    pub pi_a: [u8; 64],
    pub pi_b: [u8; 128],
    pub pi_c: [u8; 64],
}

pub struct VerificationKey {
    pub nr_pubinputs: u32,
    pub vk_alpha_g1: [u8; 64],
    pub vk_beta_g2: [u8; 128],
    pub vk_gamme_g2: [u8; 128],
    pub vk_delta_g2: [u8; 128],
    pub vk_ic: Vec<[u8; 64]>,
}

pub struct PublicInputs<const N: usize> {
    pub inputs: [[u8; 32]; N],
}

fn convert_endianness<const CHUNK_SIZE: usize, const ARRAY_SIZE: usize>(
    bytes: &[u8; ARRAY_SIZE],
) -> [u8; ARRAY_SIZE] {
    let reversed: [_; ARRAY_SIZE] = bytes
        .chunks_exact(CHUNK_SIZE)
        .flat_map(|chunk| chunk.iter().rev().copied())
        .enumerate()
        .fold([0u8; ARRAY_SIZE], |mut acc, (i, v)| {
            acc[i] = v;
            acc
        });
    reversed
}

fn decompress_g1(g1_bytes: &[u8; 32]) -> Result<[u8; 64], Error> {
    let g1_bytes = gnark_compressed_x_to_ark_compressed_x(g1_bytes)?;
    let g1_bytes = convert_endianness::<32, 32>(&g1_bytes.as_slice().try_into().unwrap());
    groth16_solana::decompression::decompress_g1(&g1_bytes).map_err(|_| Error::G1CompressionError)
}

fn decompress_g2(g2_bytes: &[u8; 64]) -> Result<[u8; 128], Error> {
    let g2_bytes = gnark_compressed_x_to_ark_compressed_x(g2_bytes)?;
    let g2_bytes = convert_endianness::<64, 64>(&g2_bytes.as_slice().try_into().unwrap());
    groth16_solana::decompression::decompress_g2(&g2_bytes).map_err(|_| Error::G2CompressionError)
}

const GNARK_MASK: u8 = 0b11 << 6;
const GNARK_COMPRESSED_POSITIVE: u8 = 0b10 << 6;
const GNARK_COMPRESSED_NEGATIVE: u8 = 0b11 << 6;
const GNARK_COMPRESSED_INFINITY: u8 = 0b01 << 6;

const ARK_MASK: u8 = 0b11 << 6;
const ARK_COMPRESSED_POSITIVE: u8 = 0b00 << 6;
const ARK_COMPRESSED_NEGATIVE: u8 = 0b10 << 6;
const ARK_COMPRESSED_INFINITY: u8 = 0b01 << 6;

fn gnark_flag_to_ark_flag(msb: u8) -> Result<u8, Error> {
    let gnark_flag = msb & GNARK_MASK;
    let ark_flag = match gnark_flag {
        GNARK_COMPRESSED_POSITIVE => ARK_COMPRESSED_POSITIVE,
        GNARK_COMPRESSED_NEGATIVE => ARK_COMPRESSED_NEGATIVE,
        GNARK_COMPRESSED_INFINITY => ARK_COMPRESSED_INFINITY,
        _ => return Err(Error::InvalidInput),
    };
    Ok(msb & !ARK_MASK | ark_flag)
}

fn gnark_compressed_x_to_ark_compressed_x(x: &[u8]) -> Result<Vec<u8>, Error> {
    if x.is_empty() {
        return Err(Error::InvalidInput);
    }
    let mut x_copy = x.to_vec();
    let msb = gnark_flag_to_ark_flag(x_copy[0])?;
    x_copy[0] = msb;
    x_copy.reverse();
    Ok(x_copy)
}

fn uncompressed_bytes_to_g1_point(buf: &[u8]) -> Result<G1Affine, Error> {
    if buf.len() != 64 {
        return Err(Error::InvalidInput);
    }
    let (x_bytes, y_bytes) = buf.split_at(32);
    let x = Fq::from_be_bytes_mod_order(x_bytes);
    let y = Fq::from_be_bytes_mod_order(y_bytes);
    Ok(G1Affine::new_unchecked(x, y))
}

fn negate_g1(g1_bytes: &[u8; 64]) -> Result<[u8; 64], Error> {
    let g1 = -uncompressed_bytes_to_g1_point(g1_bytes)?;
    let mut g1_bytes = [0u8; 64];
    g1.serialize_uncompressed(&mut g1_bytes[..])
        .map_err(|_| Error::G1CompressionError)?;
    Ok(convert_endianness::<32, 64>(&g1_bytes))
}

pub fn load_proof_from_bytes(buffer: &[u8]) -> Result<Proof, Error> {
    Ok(Proof {
        pi_a: negate_g1(
            &buffer[..64].try_into().map_err(|_| Error::G1CompressionError)?,
        )?,
        pi_b: buffer[64..192].try_into().map_err(|_| Error::G2CompressionError)?,
        pi_c: buffer[192..256].try_into().map_err(|_| Error::G1CompressionError)?,
    })
}

pub fn load_groth16_verifying_key_from_bytes(buffer: &[u8]) -> Result<VerificationKey, Error> {
    // Note: g1_beta and g1_delta are not used in verification.
    let g1_alpha = decompress_g1(buffer[..32].try_into().unwrap())?;
    let g2_beta = decompress_g2(buffer[64..128].try_into().unwrap())?;
    let g2_gamma = decompress_g2(buffer[128..192].try_into().unwrap())?;
    let g2_delta = decompress_g2(buffer[224..288].try_into().unwrap())?;

    let num_k = u32::from_be_bytes([buffer[288], buffer[289], buffer[290], buffer[291]]);
    let mut k = Vec::new();
    let mut offset = 292;
    for _ in 0..num_k {
        let point = decompress_g1(buffer[offset..offset + 32].try_into().unwrap())?;
        k.push(point);
        offset += 32;
    }

    // Skip any trailing "commitment-committed" arrays (present in some gnark VKs; unused here).
    let _ = offset;

    Ok(VerificationKey {
        nr_pubinputs: num_k,
        vk_alpha_g1: g1_alpha,
        vk_beta_g2: g2_beta,
        vk_gamme_g2: g2_gamma,
        vk_delta_g2: g2_delta,
        vk_ic: k,
    })
}

/// Hashes the public inputs to a 32-byte digest that fits in BN254 Fr.
pub fn hash_public_inputs(public_inputs: &[u8]) -> [u8; 32] {
    let mut result = Sha256::digest(public_inputs);
    // Zero top 3 bits so result < 2^253 < BN254 Fr modulus.
    result[0] &= 0x1F;
    result.into()
}

pub fn decode_sp1_vkey_hash(sp1_vkey_hash: &str) -> Result<[u8; 32], Error> {
    let stripped = sp1_vkey_hash
        .strip_prefix("0x")
        .ok_or(Error::InvalidProgramVkeyHash)?;
    let bytes = hex::decode(stripped).map_err(|_| Error::InvalidProgramVkeyHash)?;
    bytes.try_into().map_err(|_| Error::InvalidProgramVkeyHash)
}
