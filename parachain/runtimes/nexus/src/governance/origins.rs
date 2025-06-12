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

pub use custom_origins::*;

#[polkadot_sdk::frame_support::pallet]
pub mod custom_origins {
	use crate::{Balance, UNIT};
	use polkadot_sdk::frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(
		PartialEq,
		Eq,
		Clone,
		MaxEncodedLen,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		RuntimeDebug,
	)]
	#[pallet::origin]
	pub enum Origin {
		/// Origin able to dispatch a whitelisted call.
		WhitelistedCaller,
		/// Origin for managing the composition of the fellowship.
		FellowshipAdmin,
		/// Origin commanded by any members of the Fellowship (no Dan grade needed).
		FellowshipInitiates,
		/// Origin commanded by Fellows (3rd Dan fellows or greater).
		Fellows,
		/// Origin commanded by rank 1 of the Polkadot Fellowship and with a success of 1.
		Fellowship1Dan,
		/// Origin commanded by rank 2 of the Polkadot Fellowship and with a success of 2.
		Fellowship2Dan,
		/// Origin commanded by rank 3 of the Polkadot Fellowship and with a success of 3.
		Fellowship3Dan,
		/// Origin commanded by rank 4 of the Polkadot Fellowship and with a success of 4.
		Fellowship4Dan,
		/// Origin able to cancel referenda.
		ReferendumCanceller,
		/// Origin able to kill referenda.
		ReferendumKiller,
		/// Origin able to execute treasury.spend.
		TreasurySpend,
	}

	macro_rules! decl_unit_ensures {
			( $name:ident: $success_type:ty = $success:expr ) => {
				pub struct $name;
				impl<O: OriginTrait + From<Origin>> EnsureOrigin<O> for $name
				where
					for <'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
				{
					type Success = $success_type;
					fn try_origin(o: O) -> Result<Self::Success, O> {
						match o.caller().try_into() {
							Ok(Origin::$name) => return Ok($success),
							_ => (),
						}

						Err(o)
					}
					#[cfg(feature = "runtime-benchmarks")]
					fn try_successful_origin() -> Result<O, ()> {
						Ok(O::from(Origin::$name))
					}
				}
			};
			( $name:ident ) => { decl_unit_ensures! { $name : () = () } };
			( $name:ident: $success_type:ty = $success:expr, $( $rest:tt )* ) => {
				decl_unit_ensures! { $name: $success_type = $success }
				decl_unit_ensures! { $( $rest )* }
			};
			( $name:ident, $( $rest:tt )* ) => {
				decl_unit_ensures! { $name }
				decl_unit_ensures! { $( $rest )* }
			};
			() => {}
		}
	decl_unit_ensures!(
		ReferendumCanceller,
		ReferendumKiller,
		WhitelistedCaller,
		FellowshipAdmin,
		FellowshipInitiates: u16 = 0,
		Fellows: u16 = 3,
		TreasurySpend,
	);

	macro_rules! decl_ensure {
			(
				$vis:vis type $name:ident: EnsureOrigin<Success = $success_type:ty> {
					$( $item:ident = $success:expr, )*
				}
			) => {
				$vis struct $name;
				impl<O: OriginTrait + From<Origin> + TryFrom<Origin>> EnsureOrigin<O> for $name
				where
					for <'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
				{
					type Success = $success_type;
					fn try_origin(o: O) -> Result<Self::Success, O> {
						match o.caller().try_into() {
							$(
								Ok(Origin::$item) => return Ok($success),
							)*
							_ => (),
						}

						Err(o)
					}
					#[cfg(feature = "runtime-benchmarks")]
					fn try_successful_origin() -> Result<O, ()> {
						// By convention the more privileged origins go later, so for greatest chance
						// of success, we want the last one.
						let _result: Result<O, ()> = Err(());
						$(
							let _result: Result<O, ()> = Ok(O::from(Origin::$item));
						)*
						_result
					}
				}
			}
		}

	decl_ensure! {
		pub type Spender: EnsureOrigin<Success = Balance> {
			TreasurySpend = 100_000 * UNIT,
		}
	}

	decl_ensure! {
		pub type EnsureFellowship: EnsureOrigin<Success = u16> {
			Fellowship1Dan = 1,
			Fellowship2Dan = 2,
			Fellowship3Dan = 3,
			Fellowship4Dan = 4,
		}
	}
}
