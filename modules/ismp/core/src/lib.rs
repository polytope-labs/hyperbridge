// Copyright (C) Polytope Labs Ltd.
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

//! # The Interoperable State Machine Protocol
//!
//! Rust implementation of the Interoperable State Machine Protocol. This library is intended to aid
//! state machines communicate over ISMP with other ISMP supported state machines.
//!
//! ## Overview
//!
//! This repo provides an implementation of the neccessary components laid out in the [ISMP protocol specification](https://docs.hyperbridge.network/protocol/ismp).
//!
//! - [`Message` Definitions](messaging)
//! - [`Message` Handlers](handlers)
//! - [`ConsensusClient` and `StateMachineClient` definitions](consensus)
//! - [`IsmpHost` definitions](host)
//! - [`IsmpRouter` definitions](router)
//! - [`IsmpDispatcher` definitions](dispatcher)
//!
//! **NOTE**: All timestamps are denominated in seconds

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod consensus;
pub mod dispatcher;
pub mod error;
pub mod events;
pub mod handlers;
pub mod host;
pub mod messaging;
pub mod module;
pub mod router;

pub use error::Error;
pub mod prelude {
	//! Some useful imports in the crate prelude.
	pub use alloc::{format, str::FromStr, string::String, vec, vec::Vec};
}
