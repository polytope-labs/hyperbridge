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

use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use frame_support::traits::{fungibles, Get};
pub use pallet::*;
use pallet_ismp::dispatcher::Dispatcher;
use sp_core::{H160, H256, U256};
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
    use ismp::{host::StateMachine, router::DispatchPost};
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

        /// TokenGateWay address on evm chains
        #[pallet::constant]
        type TokenGateWay: Get<H160>;

        /// The 32 bytes Asset Id used to identify the DOT token on Token Gateway deployments
        #[pallet::constant]
        type DotAssetId: Get<H256>;

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
    pub enum Error<T> {
        /// Error encountered while dispatching post request
        DispatchPostError
    }

    /// Events emiited by the relayer pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An XCM transfer from the relay chain has been transformed into a crosschain message
        TransferInitiated {
            /// Source account on the relaychain
            from: T::AccountId,
            /// beneficiary account on destination
            to: T::EvmAccountId,
            /// Amount transferred
            amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
            /// Destination chain
            dest: StateMachine
        }
    }
}

impl<T: Config> Pallet<T>
where
    u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
    T::AccountId: Into<[u8; 32]>,
    T::EvmAccountId: Into<[u8; 20]>,
{
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

    pub fn protocol_account_id() -> T::AccountId {
        T::ProtocolAccount::get().into_account_truncating()
    }

    /// Dispatch ismp request to token gateway on destination chain
    pub fn dispatch_request(
        multi_account: MultiAccount<T::AccountId, T::EvmAccountId>,
        amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
    ) -> Result<(), Error<T>> {
        let amount: u128 = amount.into();
        let dispatcher = Dispatcher::<T>::default();

        let to: [u8; 20] = multi_account.evm_account.into();

        let asset_id = T::DotAssetId::get().0.into();
        let body = Body {
            amount: {
                let mut bytes = [0u8; 32];
                U256::from(amount).to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            asset_id,
            redeem: false,
            from: Default::default(),
            to: to.into(),
        };

        // let dispatch_post =  DispatchPost {

        // };

        // We don't have signed transactions yet on our chain, so we cannot allow user funds to be
        // stuck on our chain during timeouts, so we have to maintain a map of destination
        // state machine and nonce to user's substrate account

        Ok(())
    }
}

#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct Body {
    // amount to be sent
    pub amount: alloy_primitives::U256,
    // The token identifier
    pub asset_id: alloy_primitives::B256,
    // flag to redeem the erc20 asset on the destination
    pub redeem: bool,
    // sender address
    pub from: alloy_primitives::Address,
    // recipient address
    pub to: alloy_primitives::Address,
}
