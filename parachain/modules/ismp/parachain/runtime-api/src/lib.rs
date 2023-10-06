//! Runtime API for parachains.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;

sp_api::decl_runtime_apis! {
    /// Ismp Parachain Runtime Apis
    pub trait IsmpParachainApi {
        /// Return all the para_ids this runtime is interested in. Used by the inherent provider
        fn para_ids() -> Vec<u32>;
    }
}
