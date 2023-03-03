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

use crate::{
    error::Error,
    proto::{
        self, client_message, ClientMessage as RawClientMessage, Misbehaviour as RawMisbehaviour,
    },
};
use alloc::{collections::BTreeMap, vec::Vec};
use anyhow::anyhow;
use codec::{Decode, Encode};
use tendermint_proto::Protobuf;

/// Protobuf type url for GRANDPA header
pub const ETHEREUM_CLIENT_MESSAGE_TYPE_URL: &str = "/ibc.lightclients.ethereum.v1.ClientMessage";

#[derive(Clone, Debug)]
pub struct Header {}

#[derive(Clone, Debug)]
pub struct Misbehaviour {}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    /// This is the variant for header updates
    Header(Header),
    /// This is for submitting misbehaviors.
    Misbehaviour(Misbehaviour),
}

impl ibc::core::ics02_client::client_message::ClientMessage for ClientMessage {
    fn encode_to_vec(&self) -> Result<Vec<u8>, tendermint_proto::Error> {
        self.encode_vec()
    }
}

impl Protobuf<RawClientMessage> for ClientMessage {}

impl TryFrom<RawClientMessage> for ClientMessage {
    type Error = Error;

    fn try_from(raw_client_message: RawClientMessage) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

impl From<ClientMessage> for RawClientMessage {
    fn from(client_message: ClientMessage) -> Self {
        unimplemented!()
    }
}
