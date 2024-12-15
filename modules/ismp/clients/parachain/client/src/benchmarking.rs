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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;
	use ismp::messaging::{ConsensusMessage, Message};

	#[benchmark]
	fn add_parachain() -> Result<(), BenchmarkError> {
		let state_machines: Vec<ParachainData> =
			(0..10).map(|i| ParachainData { id: i, slot_duration: 6000 }).collect();

		#[block]
		{
			Pallet::<T>::add_parachain(RawOrigin::Root.into(), state_machines)?;
		}

		Ok(())
	}

	#[benchmark]
	fn remove_parachain() -> Result<(), BenchmarkError> {
		let state_machines: Vec<ParachainData> =
			(0..10).map(|i| ParachainData { id: i, slot_duration: 6000 }).collect();

		#[block]
		{
			Pallet::<T>::add_parachain(RawOrigin::Root.into(), state_machines)?;
			Pallet::<T>::remove_parachain(RawOrigin::Root.into(), vec![0, 1, 2, 3, 4])?;
		}

		Ok(())
	}

	#[benchmark]
	fn update_parachain_consensus() -> Result<(), BenchmarkError> {
		let consensus_message = ConsensusMessage {
			consensus_proof: vec![],
			consensus_state_id: *b"PARA",
			signer: vec![],
		};

		#[block]
		{
			Pallet::<T>::update_parachain_consensus(RawOrigin::None.into(), consensus_message)?;
		}

		Ok(())
	}
}
