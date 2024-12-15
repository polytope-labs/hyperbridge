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

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weights for ismp_grandpa
pub struct WeightInfo<T>(PhantomData<T>);

/// Weight functions for ismp-parachain pallet extrinsics.
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
    /// Weight for adding state machines, scaled by the number of machines.
    /// Values based on measured benchmarks:
    /// - Base Weight: 5.525 µs
    /// - Additional Weight per item: 1.458 µs
    /// - DB Weight: n writes
    fn add_state_machines(n: u32) -> Weight {
        Weight::from_parts(5_525, 0)
            .saturating_add(Weight::from_parts(1_458, 0).saturating_mul(n as u64))
            .saturating_add(T::DbWeight::get().writes(n as u64))
    }

    /// Weight for removing state machines, scaled by the number of machines.
    /// Values based on measured benchmarks:
    /// - Base Weight: 4.914 µs
    /// - Additional Weight per item: 1.419 µs
    /// - DB Weight: n writes
    fn remove_state_machines(n: u32) -> Weight {
        Weight::from_parts(4_914, 0)
            .saturating_add(Weight::from_parts(1_419, 0).saturating_mul(n as u64))
            .saturating_add(T::DbWeight::get().writes(n as u64))
    }
}