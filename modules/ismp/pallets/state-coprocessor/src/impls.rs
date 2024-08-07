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

//! Pallet method definitions

use super::{Config, Pallet};
use ismp::{
	host::IsmpHost,
	messaging::Proof,
	router::{GetRequest, Request},
	Error,
};

/// Message for processing state queries
pub struct GetRequestsWithProof {
	/// The associated Get requests
	pub requests: Vec<GetRequest>,
	/// Proof of these requests on the source chain
	pub source: Proof,
	/// State proof of the requested values in the Get requests.
	pub response: Proof,
}

impl<T: Config> Pallet<T> {
	pub fn handle_get_requests(
		GetRequestsWithProof { requests, source, response }: GetRequestsWithProof,
	) -> Result<(), Error> {
		// 1. Verify source proofs
		// 2. Extract fees
		// 3. Verify response proof
		// 4. insert GetResponse into mmr and request receipts
		// 5. emit Response events
		let mut checked = vec![];
		let host = T::IsmpHost::default();
		for req in requests.iter() {
			let full = Request::Get(req.clone());

			// Get requests time out are relative to Hyperbridge
			if full.timed_out(host.timestamp()) {
				Err(Error::RequestTimeout { meta: full.clone().into() })?
			}

			// Source of the request must match the proof
			if full.source_chain() != source.height.id.state_id {
				Err(Error::RequestProofMetadataNotValid { meta: full.clone().into() })?
			}

			// This request has already been previously processed
			if host.request_receipt(&full).is_some() {
				Err(Error::DuplicateResponse { meta: full.into() })?
			}

			checked.push(req.clone());
		}

		// Ensure the proof height is equal to each retrieval height specified in the Get
		// requests
		if !checked.iter().all(|get| get.height == source.height.height) {
			Err(Error::InsufficientProofHeight)?
		}

		Ok(())
	}
}
