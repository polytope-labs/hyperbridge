use crate::ssz::ByteVector;

pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const BYTES_PER_COMMITMENT: usize = 48;
pub type KzgCommitment = ByteVector<ssz_types::typenum::U48>;
