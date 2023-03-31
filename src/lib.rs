#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
pub mod consensus_client;
pub mod error;
pub mod handlers;
pub mod host;
pub mod messaging;
pub mod module;
pub mod paths;
pub mod router;

pub mod prelude {
    pub use alloc::{format, str::FromStr, string::String, vec, vec::Vec};
}
