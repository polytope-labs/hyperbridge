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

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use geth_primitives::CodecHeader;
    use sp_core::H256;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {}

    /// Polygon block headers
    #[pallet::storage]
    #[pallet::getter(fn headers)]
    pub type Headers<T: Config> = StorageMap<_, Identity, H256, CodecHeader, OptionQuery>;
}
