//! # The Interoperable State Machine Protocol
//!
//! This library is intended to aid state machines communicate over ISMP with other
//! ISMP supported state machines.
//!
//! Note: All timestamps are denominated in seconds

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod consensus;
pub mod error;
pub mod events;
pub mod handlers;
pub mod host;
pub mod messaging;
pub mod module;
pub mod router;
pub mod util;

pub use error::Error;
pub mod prelude {
    //! Some useful imports in the crate prelude.
    pub use alloc::{format, str::FromStr, string::String, vec, vec::Vec};
}
