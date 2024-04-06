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

use core::marker::PhantomData;

use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use frame_support::{
    ensure,
    traits::{fungibles, Get},
};
use ismp::{
    events::Meta,
    host::StateMachine,
    module::IsmpModule,
    router::{DispatchPost, DispatchRequest, IsmpDispatcher, Request, Timeout},
    util::hash_request,
};
pub use pallet::*;
use pallet_ismp::{dispatcher::Dispatcher, host::Host};
use sp_core::{H160, H256, U256};
use sp_runtime::traits::AccountIdConversion;
use staging_xcm::{
    v3::{AssetId, Fungibility, Junction, MultiAsset, MultiAssets, MultiLocation, WeightLimit},
    VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_utilities::MultiAccount;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec;
    use frame_support::{pallet_prelude::*, traits::fungibles, PalletId, Parameter};
    use sp_runtime::Percent;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config + pallet_xcm::Config {
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

    #[pallet::error]
    pub enum Error<T> {
        /// Error encountered while dispatching post request
        DispatchPostError,
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
            dest: StateMachine,
        },
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
        let dispatcher = Dispatcher::<T>::default();

        let to: [u8; 20] = multi_account.evm_account.clone().into();

        let asset_id = T::DotAssetId::get().0.into();
        let body = Body {
            amount: {
                let amount: u128 = amount.into();
                let mut bytes = [0u8; 32];
                U256::from(amount).to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            asset_id,
            redeem: false,
            from: Default::default(),
            to: to.into(),
        };

        let dispatch_post = DispatchPost {
            dest: multi_account.dest_state_machine,
            from: T::TokenGateWay::get().0.to_vec(),
            to: T::TokenGateWay::get().0.to_vec(),
            // 1 hour timeout
            timeout_timestamp: 60 * 60,
            data: alloy_rlp::encode(body),
        };

        dispatcher
            .dispatch_request(
                DispatchRequest::Post(dispatch_post),
                multi_account.substrate_account.clone(),
                Default::default(),
            )
            .map_err(|_| Error::<T>::DispatchPostError)?;

        Self::deposit_event(Event::<T>::TransferInitiated {
            from: multi_account.substrate_account,
            to: multi_account.evm_account,
            dest: multi_account.dest_state_machine,
            amount,
        });

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

#[derive(Clone)]
pub struct Module<T>(PhantomData<T>);

impl<T: Config> Default for Module<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Config> IsmpModule for Module<T>
where
    <T::Assets as fungibles::Inspect<T::AccountId>>::Balance: From<u128>,
    u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
    T::AccountId: Into<[u8; 32]>,
    T::EvmAccountId: Into<[u8; 20]>,
{
    fn on_accept(&self, _request: ismp::router::Post) -> Result<(), ismp::error::Error> {
        // We can't custody user funds since there would be not signed transactions at launch
        // and they would not be able to send an xcm back to the relaychain, xcm implementation for
        // substrate wallets would use signed transactions We send the dot back to the
        // relaychain on timeout
        todo!()
    }

    fn on_response(&self, response: ismp::router::Response) -> Result<(), ismp::error::Error> {
        Err(ismp::error::Error::ModuleDispatchError {
            msg: "Token Gateway does not accept responses".to_string(),
            meta: Meta {
                source: response.source_chain(),
                dest: response.dest_chain(),
                nonce: response.nonce(),
            },
        })
    }

    fn on_timeout(&self, request: ismp::router::Timeout) -> Result<(), ismp::error::Error> {
        // We can't custody user funds since there would be not signed transactions at launch
        // and they would not be able to send an xcm back to the relaychain, xcm implementation for
        // substrate wallets would use signed transactions We send the dot back to the
        // relaychain on timeout

        match request {
            Timeout::Request(Request::Post(post)) => {
                let request = Request::Post(post.clone());
                ensure!(
                    request.source_module() == T::TokenGateWay::get().0.to_vec(),
                    ismp::error::Error::ModuleDispatchError {
                        msg: "Token Gateway: Unknown source contract address".to_string(),
                        meta: Meta {
                            source: request.source_chain(),
                            dest: request.dest_chain(),
                            nonce: request.nonce(),
                        },
                    }
                );
                let commitment = hash_request::<Host<T>>(&request);
                let fee_metadata = pallet_ismp::child_trie::RequestCommitments::<T>::get(
                    commitment,
                )
                .ok_or_else(|| ismp::error::Error::ModuleDispatchError {
                    msg: "Token Gateway: Fee metadata could not be found for request".to_string(),
                    meta: Meta {
                        source: request.source_chain(),
                        dest: request.dest_chain(),
                        nonce: request.nonce(),
                    },
                })?;
                let beneficiary = fee_metadata.meta.origin;
                let body: Body = alloy_rlp::Decodable::decode(&mut &*post.data).map_err(|_| {
                    ismp::error::Error::ModuleDispatchError {
                        msg: "Token Gateway: Failed to decode request body".to_string(),
                        meta: Meta {
                            source: request.source_chain(),
                            dest: request.dest_chain(),
                            nonce: request.nonce(),
                        },
                    }
                })?;
                // Send xcm back to relaychain

                let amount = { U256::from_big_endian(&body.amount.to_be_bytes::<32>()).low_u128() };
                // We do an xcm limited reserve transfer from the pallet custody account to the user
                // on the relaychain;
                let xcm_beneficiary: MultiLocation =
                    Junction::AccountId32 { network: None, id: beneficiary.into() }.into();
                let xcm_dest = VersionedMultiLocation::V3(MultiLocation::parent());
                let fee_asset_item = 0;
                let weight_limit = WeightLimit::Unlimited;
                let asset = MultiAsset {
                    id: AssetId::Concrete(MultiLocation::parent()),
                    fun: Fungibility::Fungible(amount),
                };

                let mut assets = MultiAssets::new();
                assets.push(asset);
                pallet_xcm::Pallet::<T>::limited_reserve_transfer_assets(
                    frame_system::RawOrigin::Signed(Pallet::<T>::account_id()).into(),
                    Box::new(xcm_dest),
                    Box::new(xcm_beneficiary.into()),
                    Box::new(VersionedMultiAssets::V3(assets)),
                    fee_asset_item,
                    weight_limit,
                )
                .map_err(|_| ismp::error::Error::ModuleDispatchError {
                    msg: "Token Gateway: Failed execute xcm to relay chain".to_string(),
                    meta: Meta {
                        source: request.source_chain(),
                        dest: request.dest_chain(),
                        nonce: request.nonce(),
                    },
                })?;

                Ok(())
            },
            Timeout::Request(Request::Get(get)) => Err(ismp::error::Error::ModuleDispatchError {
                msg: "Tried to timeout unsupported request type".to_string(),
                meta: Meta { source: get.source, dest: get.dest, nonce: get.nonce },
            }),

            Timeout::Response(response) => Err(ismp::error::Error::ModuleDispatchError {
                msg: "Tried to timeout unsupported request type".to_string(),
                meta: Meta {
                    source: response.source_chain(),
                    dest: response.dest_chain(),
                    nonce: response.nonce(),
                },
            }),
        }
    }
}
