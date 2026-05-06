// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Collator-side fisherman task. Subscribes to hyperbridge's
//! `StateMachineUpdated` events for each L2, queries the same height across
//! multiple independent L2 RPC providers, and submits a veto when responding
//! providers disagree among themselves or with hyperbridge's recorded root.
//! Transport errors and timeouts never produce a veto on their own.

pub mod config;
mod task;

pub use config::{is_l2, ChainSection, ConsensusSection, FishermanConfig, HyperbridgeSection};
pub use task::{spawn, LOG_TARGET};
