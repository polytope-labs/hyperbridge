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
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod consensus;

use alloc::{vec, vec::Vec};
use cumulus_primitives_core::relay_chain;
use ismp::handlers;
pub use pallet::*;
use pallet_ismp::host::Host;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use cumulus_primitives_core::relay_chain;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp::messaging::{ConsensusMessage, CreateConsensusState, Message};
    use parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
    use primitive_types::H256;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_ismp::Config + parachain_system::Config
    {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Mapping of relay chain heights to it's state root. Gotten from parachain-system.
    #[pallet::storage]
    #[pallet::getter(fn relay_chain_state)]
    pub type RelayChainState<T: Config> =
        StorageMap<_, Blake2_128Concat, relay_chain::BlockNumber, relay_chain::Hash, OptionQuery>;

    /// Tracks whether we've already seen the `update_parachain_consensus` inherent
    #[pallet::storage]
    pub type ConsensusUpdated<T: Config> = StorageValue<_, bool>;

    /// List of parachains who's headers will be inserted in the `update_parachain_consensus`
    /// inherent
    #[pallet::storage]
    pub type Parachains<T: Config> = StorageMap<_, Identity, u32, ()>;

    /// Events emitted by this pallet
    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        /// Rather than users manually submitting consensus updates for sibling parachains, we
        /// instead make it the responsibility of the block builder to insert the consensus
        /// updates as an inherent.
        #[pallet::call_index(0)]
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn update_parachain_consensus(
            origin: OriginFor<T>,
            data: ConsensusMessage,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;
            assert!(
                !<ConsensusUpdated<T>>::exists(),
                "ValidationData must be updated only once in a block",
            );

            assert_eq!(
                data.consensus_state_id,
                consensus::PARACHAIN_CONSENSUS_ID,
                "Only parachain consensus updates should be passed in the inherents!"
            );

            pallet_ismp::Pallet::<T>::handle_messages(vec![Message::Consensus(data)])?;

            Ok(Pays::No.into())
        }

        /// Add some new parachains to the list of parachains we care about
        #[pallet::call_index(1)]
        #[pallet::weight(0)]
        pub fn add_parachain(origin: OriginFor<T>, para_ids: Vec<u32>) -> DispatchResult {
            ensure_root(origin)?;
            for id in para_ids {
                Parachains::<T>::insert(id, ());
            }

            Ok(())
        }

        /// Remove some parachains from the list of parachains we care about
        #[pallet::call_index(2)]
        #[pallet::weight(0)]
        pub fn remove_parachain(origin: OriginFor<T>, para_ids: Vec<u32>) -> DispatchResult {
            ensure_root(origin)?;
            for id in para_ids {
                Parachains::<T>::remove(id);
            }

            Ok(())
        }
    }

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

        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // kill the storage, since this is the beginning of a new block.
            ConsensusUpdated::<T>::kill();

            Weight::from_parts(0, 0)
        }
    }

    /// The identifier for the parachain consensus update inherent.
    pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"paraismp";

    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        type Call = Call<T>;
        type Error = sp_inherents::MakeFatalError<()>;
        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            let data: ConsensusMessage =
                data.get_data(&Self::INHERENT_IDENTIFIER).ok().flatten()?;

            Some(Call::update_parachain_consensus { data })
        }

        fn is_inherent(call: &Self::Call) -> bool {
            matches!(call, Call::update_parachain_consensus { .. })
        }
    }

    /// The genesis config
    #[pallet::genesis_config]
    pub struct GenesisConfig {
        /// List of parachains to track at genesis
        pub parachains: Vec<u32>,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            GenesisConfig { parachains: vec![] }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        fn build(&self) {
            let host = Host::<T>::default();

            let message = CreateConsensusState {
                // insert empty bytes
                consensus_state: vec![],
                unbonding_period: u64::MAX,
                consensus_state_id: consensus::PARACHAIN_CONSENSUS_ID,
                consensus_client_id: consensus::PARACHAIN_CONSENSUS_ID,
                state_machine_commitments: vec![],
            };
            handlers::create_client(&host, message)
                .expect("Failed to initialize parachain consensus client");

            // insert the parachain ids
            for id in &self.parachains {
                Parachains::<T>::insert(id, ());
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Returns the list of parachains who's consensus updates will be inserted by the inherent
    /// data provider
    pub fn para_ids() -> Vec<u32> {
        Parachains::<T>::iter_keys().collect()
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
