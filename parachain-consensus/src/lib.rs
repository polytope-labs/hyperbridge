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
// See the License for the specific language governing permissions and
// limitations under the License.

//! ISMP Parachain Consensus Client
//!
//! This allows parachains communicate over ISMP leveraging the relay chain as a consensus oracle.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

pub mod consensus;

use cumulus_primitives_core::relay_chain;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use cumulus_primitives_core::relay_chain;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use parachain_system::{RelaychainDataProvider, RelaychainStateProvider};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_ismp::Config + parachain_system::Config
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Mapping of relay chain heights to it's state root. Gotten from parachain-system.
    #[pallet::storage]
    #[pallet::getter(fn relay_chain_state)]
    pub type RelayChainState<T: Config> =
        StorageMap<_, Blake2_128Concat, relay_chain::BlockNumber, relay_chain::Hash, OptionQuery>;

    #[pallet::event]
    pub enum Event<T: Config> {}

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(_n: T::BlockNumber) {
            let state = RelaychainDataProvider::<T>::current_relay_chain_state();
            if !RelayChainState::<T>::contains_key(state.number) {
                RelayChainState::<T>::insert(state.number, state.state_root);

                let digest = sp_runtime::generic::DigestItem::Consensus(
                    consensus::PARACHAIN_CONSENSUS_ID,
                    state.number.encode(),
                );
                <frame_system::Pallet<T>>::deposit_log(digest);
            }
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {}

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            GenesisConfig {}
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            // insert empty bytes
            pallet_ismp::ConsensusStates::<T>::insert(
                consensus::PARACHAIN_CONSENSUS_ID,
                Vec::<u8>::new(),
            );

            pallet_ismp::ConsensusClientUpdateTime::<T>::insert(
                consensus::PARACHAIN_CONSENSUS_ID,
                // parachains have no challenge period
                0,
            );
        }
    }
}

/// Interface that exposes the relay chain state roots.
pub trait RelayChainOracle {
    /// Returns the state root for a given height if it exists.
    fn state_root(height: relay_chain::BlockNumber) -> Option<relay_chain::Hash>;
}

impl<T: Config> RelayChainOracle for Pallet<T> {
    fn state_root(height: relay_chain::BlockNumber) -> Option<relay_chain::Hash> {
        RelayChainState::<T>::get(height)
    }
}
