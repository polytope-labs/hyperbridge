//! Canonical ISMP Events

use crate::{
    consensus::StateMachineId,
    router::{Get, Post, PostResponse},
};
use alloc::vec::Vec;
use codec::{Decode, Encode};
use primitive_types::H256;
use scale_info::TypeInfo;

/// Emitted when a state machine is successfully updated to a new height after the challenge period
/// has elapsed
#[derive(Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
pub struct StateMachineUpdated {
    /// State machine id
    pub state_machine_id: StateMachineId,
    /// Latest height
    pub latest_height: u64,
}

/// Emitted when a post request is successfully handled.
#[derive(Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
pub struct PostRequestHandled {
    /// The commitment to the request
    pub commitment: H256,
    /// The address of the relayer responsible for relaying the request
    pub relayer: Vec<u8>,
}

/// This represents events that should be emitted by ismp-rs wrappers
#[derive(Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
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
    /// Emitted when a post request is handled
    PostRequestHandled(PostRequestHandled),
}
