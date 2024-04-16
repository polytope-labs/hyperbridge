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
use pallet_ismp::host::Host;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::types::{ConsensusState, L2Consensus};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::{consensus::StateMachineId, host::IsmpHost};

    use sp_core::{H160, H256};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// Origin allowed to add or remove parachains in Consensus State
        type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Contract Address Already Exists
        ContractAddressAlreadyExists,
        /// Error fetching consensus state
        ErrorFetchingConsensusState,
        /// Error decoding consensus state
        ErrorDecodingConsensusState,
        /// Error storing consensus state
        ErrorStoringConsensusState,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        /// Add an ismp host contract address for a new chain
        #[pallet::call_index(0)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        pub fn add_ismp_address(
            origin: OriginFor<T>,
            contract_address: H160,
            state_machine_id: StateMachineId,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();
            let StateMachineId { consensus_state_id, state_id: state_machine } = state_machine_id;
            let encoded_consensus_state = ismp_host
                .consensus_state(consensus_state_id)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;
            ensure!(
                !consensus_state.ismp_contract_addresses.contains_key(&state_machine),
                Error::<T>::ContractAddressAlreadyExists
            );
            consensus_state.ismp_contract_addresses.insert(state_machine, contract_address);

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }

        /// Add a new l2 consensus to the sync committee consensus state
        #[pallet::call_index(1)]
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        pub fn add_l2_consensus(
            origin: OriginFor<T>,
            state_machine_id: StateMachineId,
            l2_consensus: L2Consensus,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;

            let ismp_host = Host::<T>::default();
            let StateMachineId { consensus_state_id, state_id: state_machine } = state_machine_id;

            let encoded_consensus_state = ismp_host
                .consensus_state(consensus_state_id)
                .map_err(|_| Error::<T>::ErrorFetchingConsensusState)?;
            let mut consensus_state: ConsensusState =
                codec::Decode::decode(&mut &encoded_consensus_state[..])
                    .map_err(|_| Error::<T>::ErrorDecodingConsensusState)?;

            consensus_state.l2_consensus.insert(state_machine, l2_consensus);

            let encoded_consensus_state = consensus_state.encode();
            ismp_host
                .store_consensus_state(consensus_state_id, encoded_consensus_state)
                .map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
            Ok(())
        }
    }
}
