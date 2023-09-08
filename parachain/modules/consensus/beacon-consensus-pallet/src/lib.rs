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

use alloc::{vec, vec::Vec};
pub use pallet::*;
use pallet_ismp::host::Host;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use ethabi::ethereum_types::H160;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::host::{IsmpHost, StateMachine};
    use ismp_sync_committee::types::ConsensusState;
    use primitive_types::H256;

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
        /// Contract Address Already Exists
        ContractAddressAlreadyExists,
        /// Contract Address Consensus Does not Exist
        ContractAddressDontExists,
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
        /// Add contract address
        #[pallet::call_index(0)]
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn add_contract_address(
            origin: OriginFor<T>,
            consensus_state_id_vec: Vec<u8>,
            contract_address: H160,
            state_machine: StateMachine,
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

            let mut stored_contract_addresses = consensus_state.ismp_contract_addresses;
            stored_contract_addresses.insert(state_machine, contract_address);
            consensus_state.ismp_contract_addresses = stored_contract_addresses;

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }

        /// Remove contract address
        #[pallet::call_index(1)]
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn remove_contract_address(
            origin: OriginFor<T>,
            consensus_state_id_vec: Vec<u8>,
            state_machine: StateMachine,
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

            let mut stored_contract_addresses = consensus_state.ismp_contract_addresses;
            stored_contract_addresses.remove(&state_machine);
            consensus_state.ismp_contract_addresses = stored_contract_addresses;

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }
    }
}
