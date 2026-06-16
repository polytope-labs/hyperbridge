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

//! Library surface for the consolidated Hyperbridge relayer.
//!
//! Module shape mirrors what `src/main.rs` used to declare directly. Lifting
//! the modules into a lib lets other crates (notably the collator-side
//! fisherman wrapper) consume `HyperbridgeConfig` and friends without
//! re-implementing the same toml parser.

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "tesseract";

pub mod claim_rewards;
pub mod cli;
pub mod config;
pub mod fees;
pub mod monitor;
pub mod provider;
