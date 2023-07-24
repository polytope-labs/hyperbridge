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
// See the License for the specific lang

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod consensus;
pub mod consensus_message;

use alloc::{vec, vec::Vec};
pub use pallet::*;
use pallet_ismp::host::Host;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::host::IsmpHost;
    use primitive_types::H256;
    use primitives::ConsensusState;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin allowed to add or remove parachains in Consensus State
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    /// Events emitted by this pallet
    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        /// Standalone Consensus State Already Exists
        StandaloneConsensusStateAlreadyExists,
        /// Standalone Consensus Does not Exist
        StandaloneConsensusStateDontExists,
        /// Error fetching consensus state
        ErrorFetchingConsensusState,
        /// Error decoding consensus state
        ErrorDecodingConsensusState,
        /// Incorrect consensus state id length
        IncorrectConsensusStateIdLength,
        /// Error storing consensus state
        ErrorStoringConsensusState,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        /// Add some new parachains to the list of parachains in the relay chain consensus state
        #[pallet::call_index(0)]
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn add_parachains(
            origin: OriginFor<T>,
            consensus_state_id_vec: Vec<u8>,
            para_ids: Vec<u32>,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();
            let consensus_state_id = consensus_state_id_vec
                .as_slice()
                .try_into()
                .map_err(|_| Error::<T>::IncorrectConsensusStateIdLength)?;

            let encoded_consensus_state = ismp_host
                .consensus_state(consensus_state_id)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;

            let mut stored_para_ids = consensus_state.para_ids;
            para_ids.iter().for_each(|para_id| {
                stored_para_ids.entry(*para_id).or_insert(true);
            });
            consensus_state.para_ids = stored_para_ids;

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }

        /// Remove some parachains from the list of parachains in the relay chain consensus state
        #[pallet::call_index(1)]
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn remove_parachains(
            origin: OriginFor<T>,
            consensus_state_id_vec: Vec<u8>,
            para_ids: Vec<u32>,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();
            let consensus_state_id = consensus_state_id_vec
                .as_slice()
                .try_into()
                .map_err(|_| Error::<T>::IncorrectConsensusStateIdLength)?;

            let encoded_consensus_state = ismp_host
                .consensus_state(consensus_state_id)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;

            let mut stored_para_ids = consensus_state.para_ids;
            stored_para_ids.retain(|&key, _| !para_ids.contains(&key));
            consensus_state.para_ids = stored_para_ids;

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }
    }
}
