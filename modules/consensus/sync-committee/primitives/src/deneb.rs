use crate::ssz::ByteVector;

pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const BYTES_PER_COMMITMENT: usize = 48;
pub type KzgCommitment = ByteVector<ssz_types::typenum::U48>;

/// Type level counterparts of the bounds above, sharing their names. See `constants::bounds`.
#[allow(non_camel_case_types)]
mod bounds {
	pub type MAX_BLOB_COMMITMENTS_PER_BLOCK = ssz_types::typenum::U4096;
	pub type BYTES_PER_COMMITMENT = ssz_types::typenum::U48;
}

pub use bounds::*;
