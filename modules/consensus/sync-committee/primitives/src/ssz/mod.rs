mod byte_list;
use core::fmt;

fn write_bytes_to_lower_hex<T: AsRef<[u8]>>(f: &mut fmt::Formatter<'_>, data: T) -> fmt::Result {
	if f.alternate() {
		write!(f, "0x")?;
	}
	for i in data.as_ref() {
		write!(f, "{i:02x}")?;
	}
	Ok(())
}

pub use bls_utils::ByteVector;
#[cfg(feature = "glamsterdam")]
mod state_list;
#[cfg(feature = "glamsterdam")]
pub use state_list::StateList;
mod uint256;
pub use uint256::U256;
pub use byte_list::ByteList;
