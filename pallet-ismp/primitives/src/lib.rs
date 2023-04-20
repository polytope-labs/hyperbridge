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

#![cfg_attr(not(feature = "std"), no_std)]

use ismp::host::StateMachine;

pub mod mmr;

#[derive(codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
    pub source_chain: StateMachine,
    pub dest_chain: StateMachine,
    pub nonce: u64,
}
