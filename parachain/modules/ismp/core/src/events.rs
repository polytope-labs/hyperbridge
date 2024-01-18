//! Canonical ISMP Events

use crate::{
    consensus::StateMachineId,
    router::{Get, Post, PostResponse},
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Emitted when a state machine is successfully updated to a new height after the challenge period
/// has elapsed
#[derive(Clone, Debug, TypeInfo, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct StateMachineUpdated {
    /// State machine id
    pub state_machine_id: StateMachineId,
    /// Latest height
    pub latest_height: u64,
}

/// This represents events that should be emitted by ismp-rs wrappers
#[derive(Clone, Debug, TypeInfo, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    /// Emitted when a state machine is successfully updated to a new height after the challenge
    /// period has elapsed
    StateMachineUpdated(StateMachineUpdated),
    /// An event that is emitted when a post request is dispatched
    PostRequest(Post),
    /// An event that is emitted when a post response is dispatched
    PostResponse(PostResponse),
    /// An event that is emitted when a get request is dispatched
    GetRequest(Get),
}
