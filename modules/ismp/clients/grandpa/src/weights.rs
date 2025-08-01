// This file is part of Hyperbridge.

// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
use polkadot_sdk::*;

use frame_support::weights::Weight;

/// The weight information provider trait for dispatchable extrinsics
pub trait WeightInfo {
	/// Weight for adding state machines, scaled by the number of machines
	/// * n: The number of machines being added
	fn add_state_machines(n: u32) -> Weight;

	/// Weight for removing state machines, scaled by the number of machines
	/// * n: The number of machines being removed
	fn remove_state_machines(n: u32) -> Weight;
}

impl WeightInfo for () {
	fn add_state_machines(_: u32) -> Weight {
		Weight::from_parts(10_000_000, 0)
	}

	fn remove_state_machines(_: u32) -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
