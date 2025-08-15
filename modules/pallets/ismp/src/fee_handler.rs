// Copyright (c) 2025 Polytope Labs.
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

use core::marker::PhantomData;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
	traits::{Currency, ExistenceRequirement, Get},
};
use impl_trait_for_tuples::impl_for_tuples;
use ismp::messaging::{Message, MessageWithWeight};
use polkadot_sdk::{
	frame_support::{weights::WeightToFee, PalletId},
	sp_runtime::{traits::AccountIdConversion, Weight},
	*,
};
use sp_runtime::{
	traits::{MaybeDisplay, Member, Zero},
	DispatchError,
};

use crypto_utils::verification::Signature;
use ismp::events::Event;

/// Trait for handling fee calculations and settlements in the ISMP protocol.
///
/// This trait defines the interface for fee handling strategies in cross-chain message processing.
/// Implementations can define various fee models based on the specific requirements of their
/// blockchain ecosystem, economic incentives, and governance preferences.
///
/// The ISMP protocol supports multiple types of messages, including requests, responses,
/// timeouts, and consensus messages, each potentially requiring different fee structures.
/// This trait allows for creating custom fee handling logic that can be tailored to specific
/// use cases.
///
/// ## Fee Handling Strategies
///
/// Implementations of this trait can support various fee models including:
///
/// * **Weight-based fees**: Charging based on computational resources used
/// * **Message-type-based fees**: Different fees for different message types
/// * **Subsidized models**: Where certain operations have reduced or zero fees
/// * **Incentive structures**: Where relayers or validators receive rewards
/// * **Market-based mechanisms**: Where fees adjust based on network congestion
///
/// ## Implementation Notes
///
/// Fee handlers should be designed with the following considerations:
///
/// 1. **Efficiency**: Fee calculation should be computationally inexpensive
/// 2. **Fairness**: Fees should fairly reflect resource usage
/// 3. **Economic security**: Fee models should prevent spam and DoS attacks
/// 4. **Incentive alignment**: Fee structures should encourage proper protocol participation
pub trait FeeHandler {
	/// Process a batch of successfully executed messages and calculate appropriate fees.
	///
	/// This method is invoked once a batch of messages have been successfully processed.
	/// It is the responsibility of implementers to calculate and return the appropriate
	/// `PostDispatchInfo` for fee calculation and settlement based on the messages processed.
	///
	/// ## Parameters
	///
	/// * `messages` - A vector of ISMP protocol messages that have been processed. This includes
	///   various message types such as requests, responses, timeouts, and consensus messages.
	///
	/// ## Returns
	///
	/// Returns a `DispatchResultWithPostInfo` which includes:
	///
	/// * `actual_weight` - The computational weight consumed by processing the messages
	/// * `pays_fee` - Whether the operation should incur fees or not
	///
	/// ## Design Flexibility
	///
	/// This method is deliberately designed to provide flexibility and support a wide range of fee
	/// collection strategies across different blockchain ecosystems. It can accommodate various
	/// economic models including:
	///
	/// * Traditional fee payments where users pay for message processing
	/// * "Negative fees" or incentive structures where relayers receive rewards
	/// * Hybrid models with different fee structures for different message types
	/// * Context-aware pricing based on network conditions or message priority
	///
	/// ## Implementation Considerations
	///
	/// Implementers should consider:
	///
	/// * The computational cost of processing different message types
	/// * Economic incentives for relayers and validators
	/// * Prevention of spam and denial-of-service attacks
	/// * Fairness across different types of network participants
	fn on_executed(
		messages: Vec<MessageWithWeight>,
		events: Vec<Event>,
	) -> DispatchResultWithPostInfo;
}

/// A weight-based fee handler implementation that calculates and charges fees based on message
/// processing weight.
///
/// This implementation computes the weight consumed by a batch of messages, converts this weight
/// into a fee, and charges the fee to the message originator's account. The behavior is
/// configurable through a `ChargePolicy`.
///
/// ## Type Parameters
///
/// * `AccountId` - The account identifier type used to identify fee payers.
/// * `C` - The `Currency` trait implementation for handling balances.
/// * `W` - A type implementing `WeightToFee` to convert computational `Weight` into a `Balance`.
/// * `T` - A `Get<AccountId>` implementation that returns the Treasury's account ID.
/// * `Policy` - A type implementing `ChargePolicy` to determine if fees should be charged.
///
/// ## Examples
///
/// This handler is typically configured in a runtime to establish a weight-based fee model.
///
/// ```ignore
/// // In the runtime configuration
/// use pallet_ismp::fee_handler::{self, WeightToFee};
/// use frame_support::weights::WeightToFee as SubstrateWeightToFee;
///
/// // An adapter to use the runtime's default WeightToFee implementation
/// pub struct IsmpWeightToFee;
/// impl WeightToFee for IsmpWeightToFee {
///     type Balance = Balance;
///     fn convert(weight: Weight) -> Self::Balance {
///         <Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(&weight)
///     }
/// }
///
/// // Define the fee handler type
/// type FeeHandler = fee_handler::WeightFeeHandler<
///     AccountId,
///     Balances,
///     IsmpWeightToFee,
/// 	TreasuryPalletId.
/// 	true
/// >;
/// ```
pub struct WeightFeeHandler<AccountId, C, W, T, const POLICY: bool>(
	PhantomData<(AccountId, C, W, T)>,
);

type BalanceOf<C, AccountId> = <C as Currency<AccountId>>::Balance;

impl<AccountId, C, W, T, const POLICY: bool> FeeHandler
	for WeightFeeHandler<AccountId, C, W, T, POLICY>
where
	AccountId: Member + MaybeDisplay + Decode + Encode,
	C: Currency<AccountId>,
	W: WeightToFee<Balance = BalanceOf<C, AccountId>>,
	T: Get<PalletId>,
{
	fn on_executed(
		messages: Vec<MessageWithWeight>,
		_events: Vec<Event>,
	) -> DispatchResultWithPostInfo {
		if !POLICY {
			return Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		}
		let mut total_weight = Weight::zero();
		let treasury_account: AccountId = T::get().into_account_truncating();

		for message in &messages {
			let weight = message.weight;
			total_weight.saturating_accrue(weight);
			let fee = W::weight_to_fee(&weight);

			if fee.is_zero() {
				continue
			}

			let originator = match message.message.clone() {
				Message::Request(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.requests.encode());
					Signature::decode(&mut &msg.signer[..])
						.ok()
						.and_then(|sig| sig.verify_and_get_sr25519_pubkey(&data, None).ok())
				},
				Message::Response(msg) => {
					let data = sp_io::hashing::keccak_256(&msg.datagram.encode());
					Signature::decode(&mut &msg.signer[..])
						.ok()
						.and_then(|sig| sig.verify_and_get_sr25519_pubkey(&data, None).ok())
				},
				_ => None,
			};

			if let Some(originator_bytes) = originator {
				if let Ok(account) = AccountId::decode(&mut &originator_bytes[..]) {
					C::transfer(&account, &treasury_account, fee, ExistenceRequirement::KeepAlive)
						.map_err(|_| DispatchError::Other("Failed to transfer fee to treasury"))?;
				}
			}
		}

		Ok(PostDispatchInfo { actual_weight: Some(total_weight), pays_fee: Pays::Yes })
	}
}

#[impl_for_tuples(5)]
impl FeeHandler for TupleIdentifier {
	fn on_executed(
		messages: Vec<MessageWithWeight>,
		events: Vec<Event>,
	) -> DispatchResultWithPostInfo {
		for_tuples!( #(
            <TupleIdentifier as FeeHandler>::on_executed(messages.clone(), events.clone())?;
        )* );

		Ok(Default::default())
	}
}
