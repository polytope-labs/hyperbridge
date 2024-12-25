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
// See the License for the specific lang

//! Benchmarks for the ISMP GRANDPA pallet operations

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Benchmark for add_state_machines extrinsic
	/// The benchmark creates n state machines and measures the time to add them
	/// to the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of state machines to add in a single call
	#[benchmark]
	fn add_state_machines(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let state_machines: Vec<AddStateMachine> = (0..n)
			.map(|i| {
				let id = [i as u8, 0, 0, 0]; // Create unique 4-byte identifier
				AddStateMachine {
					state_machine: StateMachine::Substrate(id),
					slot_duration: 6000u64,
				}
			})
			.collect();

		#[extrinsic_call]
		_(RawOrigin::Root, state_machines);

		// Verify operation was successful
		assert!(SupportedStateMachines::<T>::iter().count() == n as usize);
		Ok(())
	}

	/// Benchmark for remove_state_machines extrinsic
	/// The benchmark first adds n state machines, then measures the time to remove them
	/// from the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of state machines to remove in a single call
	#[benchmark]
	fn remove_state_machines(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		// Setup: First add state machines that we'll remove
		let setup_machines: Vec<AddStateMachine> = (0..n)
			.map(|i| {
				let id = [i as u8, 0, 0, 0]; // Create unique 4-byte identifier
				AddStateMachine {
					state_machine: StateMachine::Substrate(id),
					slot_duration: 6000u64,
				}
			})
			.collect();

		// Add the machines using root origin
		Pallet::<T>::add_state_machines(RawOrigin::Root.into(), setup_machines.clone())?;

		// Create removal list
		let remove_machines: Vec<StateMachine> =
			setup_machines.into_iter().map(|m| m.state_machine).collect();

		// Verify initial state
		assert!(SupportedStateMachines::<T>::iter().count() == n as usize);

		#[extrinsic_call]
		_(RawOrigin::Root, remove_machines);

		// Verify all machines were removed
		assert!(SupportedStateMachines::<T>::iter().count() == 0);
		Ok(())
	}
}
