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

//! Error types for the hyper-fungible-token pallet's ISMP module

extern crate alloc;

use ismp::host::StateMachine;
use polkadot_sdk::*;
use sp_runtime::DispatchError;

/// Errors that can occur during cross-chain message processing
#[derive(thiserror::Error, Debug)]
pub enum HftError {
	#[error("Unknown source contract on {0:?}")]
	UnknownSourceContract(StateMachine),
	#[error("Failed to decode message: {0}")]
	DecodeError(alloy_sol_types::Error),
	#[error("Invalid recipient length: {0}")]
	InvalidRecipientLength(usize),
	#[error("Decimals not configured for {0:?}")]
	DecimalsNotConfigured(StateMachine),
	#[error("Invalid amount conversion: {0}")]
	InvalidAmountConversion(alloc::string::String),
	#[error("Asset transfer failed: {0:?}")]
	TransferFailed(DispatchError),
	#[error("Asset mint failed: {0:?}")]
	MintFailed(DispatchError),
	#[error("Calldata decode error: {0}")]
	CalldataDecodeError(codec::Error),
	#[error("Signature decode error: {0}")]
	SignatureDecodeError(codec::Error),
	#[error("Signature verification failed")]
	SignatureVerificationFailed,
	#[error("ECDSA public key recovery failed")]
	EcdsaRecoveryFailed,
	#[error("Eth signature type is not supported")]
	EthSignatureUnsupported,
	#[error("RuntimeCall decode error: {0}")]
	RuntimeCallDecodeError(codec::Error),
	#[error("Call dispatch error: {0:?}")]
	CallDispatchError(DispatchError),
	#[error("Module does not accept responses")]
	ResponsesNotSupported,
	#[error("Unknown contract on timeout")]
	UnknownContractOnTimeout,
	#[error("Invalid sender length: {0}")]
	InvalidSenderLength(usize),
	#[error("Unsupported timeout type")]
	UnsupportedTimeoutType,
}
