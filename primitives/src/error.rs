use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
	InvalidRoot,
	InvalidPublicKey,
	InvalidProof,
	InvalidBitVec,
	ErrorConvertingAncestorBlock,
	InvalidNodeBytes,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		match self {
			Error::InvalidRoot => write!(f, "Invalid root",),
			Error::InvalidPublicKey => write!(f, "Invalid public key",),
			Error::InvalidProof => write!(f, "Invalid proof",),
			Error::InvalidBitVec => write!(f, "Invalid bit vec",),
			Error::InvalidNodeBytes => write!(f, "Invalid node bytes",),
			Error::ErrorConvertingAncestorBlock => write!(f, "Error deriving ancestor block",),
		}
	}
}
