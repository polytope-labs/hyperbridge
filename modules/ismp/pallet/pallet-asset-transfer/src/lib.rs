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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod xcm_utilities;

extern crate alloc;

use frame_support::traits::{fungibles, Get};
pub use pallet::*;
use sp_runtime::traits::AccountIdConversion;
use xcm_utilities::MultiAccount;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec;
    use frame_support::{
        pallet_prelude::{OptionQuery, *},
        traits::fungibles,
        PalletId, Parameter,
    };
    use ismp::host::StateMachine;
    use sp_runtime::Percent;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The asset tranfer's pallet id, used for deriving its sovereign account ID.
        /// All escrowed funds will be custodied by this account
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Protocol fees will be custodied by this account
        #[pallet::constant]
        type ProtocolAccount: Get<PalletId>;

        /// Percentage to be taken as protocol fees
        #[pallet::constant]
        type ProtocolFees: Get<Percent>;

        /// Evm account id type
        type EvmAccountId: Parameter;

        /// Fungible asset implementation
        type Assets: fungibles::Mutate<Self::AccountId> + fungibles::Inspect<Self::AccountId>;
    }

    /// Here we map the destination evm
    #[pallet::storage]
    #[pallet::getter(fn account_map)]
    pub type AccountMap<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        StateMachine,
        Twox64Concat,
        u64,
        T::AccountId,
        OptionQuery,
    >;

    #[pallet::error]
    pub enum Error<T> {}

    /// Events emiited by the relayer pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}
}

impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

    pub fn protocol_account_id() -> T::AccountId {
        T::ProtocolAccount::get().into_account_truncating()
    }

    /// Dispatch ismp request to token gateway on destination chain
    pub fn dispatch_request(
        _multi_account: MultiAccount<T::AccountId, T::EvmAccountId>,
        _amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
    ) -> Result<(), Error<T>> {
        Ok(())
    }
}
