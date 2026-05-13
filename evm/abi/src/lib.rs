// Copyright (C) 2022 Polytope Labs.
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

//! Generated types for the ismp-solidity ABI.
//!
//! Every `sol!` binding under [`mod@generated`] and the conversion impls in
//! [`mod@conversions`] compile in both `std` and `no_std`. Only `#[sol(rpc)]` (the
//! provider-backed contract-call bindings, via `alloy-contract` / `alloy-provider` /
//! `alloy-network` / `alloy-transport`) is gated on `std`, since those crates are std-only.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod generated;

pub mod conversions;

pub use conversions::*;
pub use generated::*;
