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
// See the License for the specific lang
use alloc::collections::BTreeMap;
use codec::{Decode, Encode};
use primitives::{FinalityProof, ParachainHeaderProofs};
use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;

/// Relay chain substrate header type
pub type SubstrateHeader = sp_runtime::generic::Header<u32, BlakeTwo256>;

/// [`ClientMessage`] definition
#[derive(Clone, Debug, Encode, Decode)]
pub enum ConsensusMessage {
    /// This is the variant representing the standalone chain
    StandaloneChainMessage(StandaloneChainMessage),
    /// This is the variant representing the relay chain
    RelayChainMessage(RelayChainMessage),
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct StandaloneChainMessage {
    /// finality proof
    pub finality_proof: FinalityProof<SubstrateHeader>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct RelayChainMessage {
    /// finality proof
    pub finality_proof: FinalityProof<SubstrateHeader>,
    /// parachain headers
    pub parachain_headers: BTreeMap<H256, ParachainHeaderProofs>,
}
