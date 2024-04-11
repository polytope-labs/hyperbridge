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

use alloc::{boxed::Box, string::ToString};
use core::marker::PhantomData;

use alloc::vec;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use frame_support::{
    ensure,
    traits::{
        fungibles::{self, Mutate},
        tokens::Preservation,
        Get,
    },
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
use sp_runtime::{traits::AccountIdConversion, Percent};
use staging_xcm::{
    v3::{AssetId, Fungibility, Junction, MultiAsset, MultiAssets, MultiLocation, WeightLimit},
    VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_utilities::MultiAccount;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec;
    use frame_support::{pallet_prelude::*, traits::fungibles, PalletId};
    use frame_system::pallet_prelude::OriginFor;
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

        /// Pallet parameters
        #[pallet::constant]
        type Params: Get<TokenGatewayParams>;

        /// Fungible asset implementation
        type Assets: fungibles::Mutate<Self::AccountId> + fungibles::Inspect<Self::AccountId>;
    }

    #[pallet::storage]
    #[pallet::getter(fn params)]
    pub type Params<T> = StorageValue<_, TokenGatewayParams, OptionQuery>;

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
        AssetTeleported {
            /// Source account on the relaychain
            from: T::AccountId,
            /// beneficiary account on destination
            to: H160,
            /// Amount transferred
            amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
            /// Destination chain
            dest: StateMachine,
        },

        /// An asset has been received and transferred to the beneficiary's account on the
        /// relaychain
        AssetReceived {
            /// beneficiary account on relaychain
            beneficiary: T::AccountId,
            /// Amount transferred
            amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
            /// Destination chain
            source: StateMachine,
        },

        /// An asset has been refunded and transferred to the beneficiary's account on the
        /// relaychain
        AssetRefunded {
            /// beneficiary account on relaychain
            beneficiary: T::AccountId,
            /// Amount transferred
            amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
            /// Destination chain
            source: StateMachine,
        },
    }

    #[derive(Clone, Encode, Decode, scale_info::TypeInfo, Eq, PartialEq, RuntimeDebug)]
    pub struct TokenGatewayParams {
        /// Percentage to be taken as protocol fees
        pub protocol_fee_percentage: Percent,
        /// TokenGateWay address on evm chains
        pub token_gateway_address: H160,
        /// The 32 bytes Asset Id used to identify the DOT token on Token Gateway deployments
        pub dot_asset_id: H256,
    }

    impl TokenGatewayParams {
        pub const fn from_parts(
            protocol_fee_percentage: Percent,
            token_gateway_address: H160,
            dot_asset_id: H256,
        ) -> Self {
            Self { protocol_fee_percentage, token_gateway_address, dot_asset_id }
        }
    }

    #[derive(Clone, Encode, Decode, scale_info::TypeInfo, Eq, PartialEq, RuntimeDebug)]
    pub struct TokenGatewayParamsUpdate {
        pub protocol_fee_percentage: Option<Percent>,
        pub token_gateway_address: Option<H160>,
        pub dot_asset_id: Option<H256>,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        T::AccountId: From<[u8; 32]>,
    {
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[pallet::call_index(0)]
        pub fn set_params(
            origin: OriginFor<T>,
            update: TokenGatewayParamsUpdate,
        ) -> DispatchResult {
            <T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
            let mut current_params = Params::<T>::get().unwrap_or(T::Params::get());

            if let Some(address) = update.token_gateway_address {
                current_params.token_gateway_address = address;
            }

            if let Some(protocol_fee_percentage) = update.protocol_fee_percentage {
                current_params.protocol_fee_percentage = protocol_fee_percentage;
            }

            if let Some(dot_asset_id) = update.dot_asset_id {
                current_params.dot_asset_id = dot_asset_id;
            }

            Params::<T>::put(current_params);
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T>
where
    u128: From<<T::Assets as fungibles::Inspect<T::AccountId>>::Balance>,
    T::AccountId: Into<[u8; 32]>,
{
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

    pub fn protocol_account_id() -> T::AccountId {
        T::ProtocolAccount::get().into_account_truncating()
    }

    pub fn token_gateway_address() -> H160 {
        Params::<T>::get().unwrap_or(<T as Config>::Params::get()).token_gateway_address
    }

    pub fn protocol_fee_percentage() -> Percent {
        Params::<T>::get()
            .unwrap_or(<T as Config>::Params::get())
            .protocol_fee_percentage
    }

    pub fn dot_asset_id() -> H256 {
        Params::<T>::get().unwrap_or(<T as Config>::Params::get()).dot_asset_id
    }

    /// Dispatch ismp request to token gateway on destination chain
    pub fn dispatch_request(
        multi_account: MultiAccount<T::AccountId>,
        amount: <T::Assets as fungibles::Inspect<T::AccountId>>::Balance,
    ) -> Result<(), Error<T>> {
        let dispatcher = Dispatcher::<T>::default();

        let mut to = [0u8; 32];
        to[..20].copy_from_slice(&multi_account.evm_account.0);
        let from: [u8; 32] = multi_account.substrate_account.clone().into();
        let asset_id = Self::dot_asset_id().0.into();
        let body = Body {
            amount: {
                let amount: u128 = amount.into();
                let mut bytes = [0u8; 32];
                U256::from(amount).to_big_endian(&mut bytes);
                alloy_primitives::U256::from_be_bytes(bytes)
            },
            asset_id,
            redeem: false,
            from: from.into(),
            to: to.into(),
        };

        let dispatch_post = DispatchPost {
            dest: multi_account.dest_state_machine,
            from: Self::token_gateway_address().0.to_vec(),
            to: Self::token_gateway_address().0.to_vec(),
            timeout_timestamp: multi_account.timeout,
            data: {
                // Prefix with the handleIncomingAsset enum variant
                let mut encoded = vec![0];
                encoded.extend_from_slice(&alloy_rlp::encode(body));
                encoded
            },
        };

        dispatcher
            .dispatch_request(
                DispatchRequest::Post(dispatch_post),
                multi_account.substrate_account.clone(),
                Default::default(),
            )
            .map_err(|_| Error::<T>::DispatchPostError)?;

        Self::deposit_event(Event::<T>::AssetTeleported {
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
    pub from: alloy_primitives::B256,
    // recipient address
    pub to: alloy_primitives::B256,
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
    <T::Assets as fungibles::Inspect<T::AccountId>>::AssetId: From<MultiLocation>,
    T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
{
    fn on_accept(&self, post: ismp::router::Post) -> Result<(), ismp::error::Error> {
        let request = Request::Post(post.clone());
        // Check that source module is equal to the known token gateway deployment address
        ensure!(
            request.source_module() == Pallet::<T>::token_gateway_address().0.to_vec(),
            ismp::error::Error::ModuleDispatchError {
                msg: "Token Gateway: Unknown source contract address".to_string(),
                meta: Meta {
                    source: request.source_chain(),
                    dest: request.dest_chain(),
                    nonce: request.nonce(),
                },
            }
        );

        let body: Body = alloy_rlp::Decodable::decode(&mut &post.data[1..]).map_err(|_| {
            ismp::error::Error::ModuleDispatchError {
                msg: "Token Gateway: Failed to decode request body".to_string(),
                meta: Meta {
                    source: request.source_chain(),
                    dest: request.dest_chain(),
                    nonce: request.nonce(),
                },
            }
        })?;

        // Check that the asset id is equal to the known asset id
        ensure!(
            body.asset_id.0 == Pallet::<T>::dot_asset_id().0,
            ismp::error::Error::ModuleDispatchError {
                msg: "Token Gateway: AssetId is unknown".to_string(),
                meta: Meta {
                    source: request.source_chain(),
                    dest: request.dest_chain(),
                    nonce: request.nonce(),
                },
            }
        );

        let amount = { U256::from_big_endian(&body.amount.to_be_bytes::<32>()).low_u128() };

        let asset_id = MultiLocation::parent();

        let protocol_account = Pallet::<T>::protocol_account_id();
        let pallet_account = Pallet::<T>::account_id();
        let protocol_percentage = Pallet::<T>::protocol_fee_percentage();

        let protocol_fees = protocol_percentage * amount;
        let amount = amount - protocol_fees;

        T::Assets::transfer(
            asset_id.clone().into(),
            &pallet_account,
            &protocol_account,
            protocol_fees.into(),
            Preservation::Preserve,
        )
        .map_err(|_| ismp::error::Error::ModuleDispatchError {
            msg: "Token Gateway: Error collecting protocol fees".to_string(),
            meta: Meta {
                source: request.source_chain(),
                dest: request.dest_chain(),
                nonce: request.nonce(),
            },
        })?;

        // We don't custody user funds, we send the dot back to the relaychain using xcm
        let xcm_beneficiary: MultiLocation =
            Junction::AccountId32 { network: None, id: body.to.0 }.into();
        let xcm_dest = VersionedMultiLocation::V3(MultiLocation::parent());
        let fee_asset_item = 0;
        let weight_limit = WeightLimit::Unlimited;
        let asset =
            MultiAsset { id: AssetId::Concrete(asset_id), fun: Fungibility::Fungible(amount) };

        let mut assets = MultiAssets::new();
        assets.push(asset);

        // Send xcm back to relaychain
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

        Pallet::<T>::deposit_event(Event::<T>::AssetReceived {
            beneficiary: body.to.0.into(),
            amount: amount.into(),
            source: request.source_chain(),
        });

        Ok(())
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

    fn on_timeout(&self, request: Timeout) -> Result<(), ismp::error::Error> {
        // We don't custody user funds, we send the dot back to the relaychain using xcm

        match request {
            Timeout::Request(Request::Post(post)) => {
                let request = Request::Post(post.clone());
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
                let body: Body =
                    alloy_rlp::Decodable::decode(&mut &post.data[1..]).map_err(|_| {
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
                    Junction::AccountId32 { network: None, id: beneficiary.clone().into() }.into();
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

                Pallet::<T>::deposit_event(Event::<T>::AssetRefunded {
                    beneficiary,
                    amount: amount.into(),
                    source: request.dest_chain(),
                });

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
