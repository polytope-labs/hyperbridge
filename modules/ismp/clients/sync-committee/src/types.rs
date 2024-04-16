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
use alloc::collections::BTreeMap;
use arbitrum_verifier::ArbitrumPayloadProof;
use codec::{Decode, Encode};
use ethabi::ethereum_types::H160;
use ismp::host::StateMachine;
use op_verifier::{OptimismDisputeGameProof, OptimismPayloadProof};
use sync_committee_primitives::types::{VerifierState, VerifierStateUpdate};

#[derive(Debug, Encode, Decode, Clone)]
pub struct ConsensusState {
    pub frozen_height: Option<u64>,
    pub light_client_state: VerifierState,
    pub ismp_contract_addresses: BTreeMap<StateMachine, H160>,
    pub l2_consensus: BTreeMap<StateMachine, L2Consensus>,
}

#[derive(Encode, Decode)]
pub struct BeaconClientUpdate {
    pub consensus_update: VerifierStateUpdate,
    pub l2_oracle_payload: BTreeMap<StateMachine, OptimismPayloadProof>,
    pub dispute_game_payload: BTreeMap<StateMachine, OptimismDisputeGameProof>,
    pub arbitrum_payload: BTreeMap<StateMachine, ArbitrumPayloadProof>,
}

/// Description of the various consensus mechanics supported for ethereum L2s
#[derive(Encode, Decode, Debug, Clone, scale_info::TypeInfo, Eq, PartialEq)]
pub enum L2Consensus {
    /// Arbitrum orbit chains Rollup Core Address
    ArbitrumOrbit(H160),
    /// Op Stack L2 Oracle Address
    OpL2Oracle(H160),
    /// Op Stack Dispute game factory address
    OpFaultProofs(H160),
}
