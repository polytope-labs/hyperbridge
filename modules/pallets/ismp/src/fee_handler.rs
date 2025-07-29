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

use polkadot_sdk::*;

use alloc::vec::Vec;
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use ismp::messaging::Message;

use crate::weights::{get_weight, WeightProvider};
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
	fn on_executed(messages: Vec<Message>, events: Vec<Event>) -> DispatchResultWithPostInfo;
}

/// A weight-based fee handler implementation that calculates fees based on message processing
/// weight.
///
/// This implementation computes the actual weight consumed by a batch of messages using the
/// provided `WeightProvider`.
///
/// ## Type Parameters
///
/// * `AccountId` - The account identifier type used to identify fee payers.
/// * `Provider` - A type implementing the `WeightProvider` trait that can calculate weights for
///   different module callbacks based on message types.
///
/// ## Examples
///
/// This handler is typically configured in a runtime to establish a weight-based fee model:
///
/// ```ignore
/// type FeeHandler = WeightFeeHandler<AccountId, WeightInfo>;
/// ```
pub struct WeightFeeHandler<Provider>(PhantomData<Provider>);

impl<Provider> FeeHandler for WeightFeeHandler<Provider>
where
	Provider: WeightProvider,
{
	fn on_executed(messages: Vec<Message>, _events: Vec<Event>) -> DispatchResultWithPostInfo {
		Ok(PostDispatchInfo {
			actual_weight: Some(get_weight::<Provider>(&messages)),
			pays_fee: Pays::Yes,
		})
	}
}


/// A recursive macro to generate FeeHandler implementations for tuples.
#[macro_export]
macro_rules! impl_fee_handler_for_tuple {
    ($head:ident, $tail:ident) => {
        impl<$head, $tail> FeeHandler for ($head, $tail)
        where
            $head: FeeHandler,
            $tail: FeeHandler,
        {
            fn on_executed(messages: Vec<Message>, events: Vec<Event>) -> DispatchResultWithPostInfo {
                $head::on_executed(messages.clone(), events.clone())?;
                $tail::on_executed(messages, events)?;
                Ok(Default::default())
            }
        }
    };

    ($head:ident, $($tail:ident),+) => {
        impl<$head, $($tail),+> FeeHandler for ($head, $($tail),+)
        where
            $head: FeeHandler,
            ($($tail),+): FeeHandler,
        {
            fn on_executed(messages: Vec<Message>, events: Vec<Event>) -> DispatchResultWithPostInfo {
                $head::on_executed(messages.clone(), events.clone())?;
                <($($tail),+)>::on_executed(messages, events)?;
                Ok(Default::default())
            }
        }

        impl_fee_handler_for_tuple!($($tail),+);
    };
}

impl_fee_handler_for_tuple!(A, B);