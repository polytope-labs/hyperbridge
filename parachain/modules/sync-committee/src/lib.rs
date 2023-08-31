// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Implementation of the ethereum beacon consensus client for ISMP
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod prelude {
    pub use alloc::{boxed::Box, vec, vec::Vec};
}

pub use beacon_client::*;

pub mod arbitrum;
pub mod beacon_client;
pub mod optimism;
pub mod presets;
#[cfg(test)]
mod tests;
pub mod types;
pub mod utils;
