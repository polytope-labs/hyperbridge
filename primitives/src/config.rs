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

//! Relayer configuration options

use serde::{Deserialize, Serialize};
use sp_core::Bytes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    /// Relays only responses to GET requests
    GetResponse,
    /// Relays POST requests to destination
    PostRequest,
    /// Relays POST responses to destination
    PostResponse,
    /// Relays consensus updates
    Consensus,
}

/// Configuration options for the relayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
    /// Types of messages to be relayed.
    pub messages: Vec<MessageKind>,
    /// Modules we're interested in relaying
    pub module_filter: Vec<Bytes>,
}
