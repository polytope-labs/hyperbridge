// Copyright (c) 2024 Polytope Labs.
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

use crate::{weights::get_weight, Config};
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use ismp::messaging::Message;

pub trait HandleFee<T: Config> {
	fn handle_fee(messages: &[Message], signer: T::AccountId) -> DispatchResultWithPostInfo;
}

impl<T: Config> HandleFee<T> for () {
	fn handle_fee(messages: &[Message], _signer: T::AccountId) -> DispatchResultWithPostInfo {
		//Todo: reward the signer
		Ok(PostDispatchInfo {
			actual_weight: Some(get_weight::<T>(&messages)),
			pays_fee: Pays::Yes,
		})
	}
}
