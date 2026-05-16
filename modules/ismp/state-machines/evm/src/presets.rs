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

//! EvmHost storage slot indices. Values must match `forge inspect EvmHost
//! storage`. Verified for `evm/src/core/EvmHost.sol` after PR #840 removed
//! `_responseCommitments` (which used to occupy slot 1).

/// Slot index for `_requestCommitments`.
pub const REQUEST_COMMITMENTS_SLOT: u64 = 0;
/// Slot index for `_requestReceipts`.
pub const REQUEST_RECEIPTS_SLOT: u64 = 1;
