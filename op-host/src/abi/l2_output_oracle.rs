pub use l2_output_oracle::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
	clippy::enum_variant_names,
	clippy::too_many_arguments,
	clippy::upper_case_acronyms,
	clippy::type_complexity,
	dead_code,
	non_camel_case_types
)]
pub mod l2_output_oracle {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
				inputs: ::std::vec![
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_submissionInterval"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_l2BlockTime"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_startingBlockNumber"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_startingTimestamp"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_proposer"),
						kind: ::ethers::core::abi::ethabi::ParamType::Address,
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("address"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_challenger"),
						kind: ::ethers::core::abi::ethabi::ParamType::Address,
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("address"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_finalizationPeriodSeconds",),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
				],
			}),
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("CHALLENGER"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("CHALLENGER"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("FINALIZATION_PERIOD_SECONDS"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("FINALIZATION_PERIOD_SECONDS",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("L2_BLOCK_TIME"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("L2_BLOCK_TIME"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("PROPOSER"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("PROPOSER"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("SUBMISSION_INTERVAL"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("SUBMISSION_INTERVAL",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("computeL2Timestamp"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("computeL2Timestamp"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_l2BlockNumber"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("deleteL2Outputs"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("deleteL2Outputs"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_l2OutputIndex"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL2Output"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL2Output"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_l2OutputIndex"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
								::ethers::core::abi::ethabi::ParamType::Uint(128usize),
								::ethers::core::abi::ethabi::ParamType::Uint(128usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct Types.OutputProposal",),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL2OutputAfter"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL2OutputAfter"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_l2BlockNumber"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
								::ethers::core::abi::ethabi::ParamType::Uint(128usize),
								::ethers::core::abi::ethabi::ParamType::Uint(128usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct Types.OutputProposal",),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL2OutputIndexAfter"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL2OutputIndexAfter",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_l2BlockNumber"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("initialize"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("initialize"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_startingBlockNumber",),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_startingTimestamp",),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("latestBlockNumber"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("latestBlockNumber"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("latestOutputIndex"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("latestOutputIndex"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("nextBlockNumber"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("nextBlockNumber"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("nextOutputIndex"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("nextOutputIndex"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("proposeL2Output"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("proposeL2Output"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_outputRoot"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes32"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_l2BlockNumber"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_l1BlockHash"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes32"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_l1BlockNumber"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("startingBlockNumber"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("startingBlockNumber",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("startingTimestamp"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("startingTimestamp"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("version"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("version"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::String,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("string"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
			]),
			events: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("Initialized"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("Initialized"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::borrow::ToOwned::to_owned("version"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
							indexed: false,
						},],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("OutputProposed"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("OutputProposed"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("outputRoot"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("l2OutputIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("l2BlockNumber"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("l1Timestamp"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("OutputsDeleted"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("OutputsDeleted"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("prevNextOutputIndex",),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("newNextOutputIndex",),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
			]),
			errors: ::std::collections::BTreeMap::new(),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static L2OUTPUTORACLE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct L2OutputOracle<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for L2OutputOracle<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for L2OutputOracle<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for L2OutputOracle<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for L2OutputOracle<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(L2OutputOracle))
				.field(&self.address())
				.finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> L2OutputOracle<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(
				address.into(),
				L2OUTPUTORACLE_ABI.clone(),
				client,
			))
		}
		///Calls the contract's `CHALLENGER` (0x6b4d98dd) function
		pub fn challenger(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([107, 77, 152, 221], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `FINALIZATION_PERIOD_SECONDS` (0xf4daa291) function
		pub fn finalization_period_seconds(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([244, 218, 162, 145], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `L2_BLOCK_TIME` (0x002134cc) function
		pub fn l2_block_time(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([0, 33, 52, 204], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `PROPOSER` (0xbffa7f0f) function
		pub fn proposer(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([191, 250, 127, 15], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `SUBMISSION_INTERVAL` (0x529933df) function
		pub fn submission_interval(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([82, 153, 51, 223], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `computeL2Timestamp` (0xd1de856c) function
		pub fn compute_l2_timestamp(
			&self,
			l_2_block_number: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([209, 222, 133, 108], l_2_block_number)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `deleteL2Outputs` (0x89c44cbb) function
		pub fn delete_l2_outputs(
			&self,
			l_2_output_index: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([137, 196, 76, 187], l_2_output_index)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL2Output` (0xa25ae557) function
		pub fn get_l2_output(
			&self,
			l_2_output_index: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, OutputProposal> {
			self.0
				.method_hash([162, 90, 229, 87], l_2_output_index)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL2OutputAfter` (0xcf8e5cf0) function
		pub fn get_l2_output_after(
			&self,
			l_2_block_number: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, OutputProposal> {
			self.0
				.method_hash([207, 142, 92, 240], l_2_block_number)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL2OutputIndexAfter` (0x7f006420) function
		pub fn get_l2_output_index_after(
			&self,
			l_2_block_number: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([127, 0, 100, 32], l_2_block_number)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `initialize` (0xe4a30116) function
		pub fn initialize(
			&self,
			starting_block_number: ::ethers::core::types::U256,
			starting_timestamp: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([228, 163, 1, 22], (starting_block_number, starting_timestamp))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestBlockNumber` (0x4599c788) function
		pub fn latest_block_number(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([69, 153, 199, 136], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestOutputIndex` (0x69f16eec) function
		pub fn latest_output_index(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([105, 241, 110, 236], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `nextBlockNumber` (0xdcec3348) function
		pub fn next_block_number(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([220, 236, 51, 72], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `nextOutputIndex` (0x6abcf563) function
		pub fn next_output_index(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([106, 188, 245, 99], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `proposeL2Output` (0x9aaab648) function
		pub fn propose_l2_output(
			&self,
			output_root: [u8; 32],
			l_2_block_number: ::ethers::core::types::U256,
			l_1_block_hash: [u8; 32],
			l_1_block_number: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash(
					[154, 170, 182, 72],
					(output_root, l_2_block_number, l_1_block_hash, l_1_block_number),
				)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `startingBlockNumber` (0x70872aa5) function
		pub fn starting_block_number(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([112, 135, 42, 165], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `startingTimestamp` (0x88786272) function
		pub fn starting_timestamp(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([136, 120, 98, 114], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `version` (0x54fd4d50) function
		pub fn version(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
			self.0
				.method_hash([84, 253, 77, 80], ())
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `Initialized` event
		pub fn initialized_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, InitializedFilter> {
			self.0.event()
		}
		///Gets the contract's `OutputProposed` event
		pub fn output_proposed_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OutputProposedFilter> {
			self.0.event()
		}
		///Gets the contract's `OutputsDeleted` event
		pub fn outputs_deleted_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OutputsDeletedFilter> {
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, L2OutputOracleEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
		for L2OutputOracle<M>
	{
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "Initialized", abi = "Initialized(uint8)")]
	pub struct InitializedFilter {
		pub version: u8,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "OutputProposed", abi = "OutputProposed(bytes32,uint256,uint256,uint256)")]
	pub struct OutputProposedFilter {
		#[ethevent(indexed)]
		pub output_root: [u8; 32],
		#[ethevent(indexed)]
		pub l_2_output_index: ::ethers::core::types::U256,
		#[ethevent(indexed)]
		pub l_2_block_number: ::ethers::core::types::U256,
		pub l_1_timestamp: ::ethers::core::types::U256,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "OutputsDeleted", abi = "OutputsDeleted(uint256,uint256)")]
	pub struct OutputsDeletedFilter {
		#[ethevent(indexed)]
		pub prev_next_output_index: ::ethers::core::types::U256,
		#[ethevent(indexed)]
		pub new_next_output_index: ::ethers::core::types::U256,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum L2OutputOracleEvents {
		InitializedFilter(InitializedFilter),
		OutputProposedFilter(OutputProposedFilter),
		OutputsDeletedFilter(OutputsDeletedFilter),
	}
	impl ::ethers::contract::EthLogDecode for L2OutputOracleEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = InitializedFilter::decode_log(log) {
				return Ok(L2OutputOracleEvents::InitializedFilter(decoded));
			}
			if let Ok(decoded) = OutputProposedFilter::decode_log(log) {
				return Ok(L2OutputOracleEvents::OutputProposedFilter(decoded));
			}
			if let Ok(decoded) = OutputsDeletedFilter::decode_log(log) {
				return Ok(L2OutputOracleEvents::OutputsDeletedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for L2OutputOracleEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::InitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::OutputProposedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::OutputsDeletedFilter(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<InitializedFilter> for L2OutputOracleEvents {
		fn from(value: InitializedFilter) -> Self {
			Self::InitializedFilter(value)
		}
	}
	impl ::core::convert::From<OutputProposedFilter> for L2OutputOracleEvents {
		fn from(value: OutputProposedFilter) -> Self {
			Self::OutputProposedFilter(value)
		}
	}
	impl ::core::convert::From<OutputsDeletedFilter> for L2OutputOracleEvents {
		fn from(value: OutputsDeletedFilter) -> Self {
			Self::OutputsDeletedFilter(value)
		}
	}
	///Container type for all input parameters for the `CHALLENGER` function with signature
	/// `CHALLENGER()` and selector `0x6b4d98dd`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "CHALLENGER", abi = "CHALLENGER()")]
	pub struct ChallengerCall;
	///Container type for all input parameters for the `FINALIZATION_PERIOD_SECONDS` function with
	/// signature `FINALIZATION_PERIOD_SECONDS()` and selector `0xf4daa291`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "FINALIZATION_PERIOD_SECONDS", abi = "FINALIZATION_PERIOD_SECONDS()")]
	pub struct FinalizationPeriodSecondsCall;
	///Container type for all input parameters for the `L2_BLOCK_TIME` function with signature
	/// `L2_BLOCK_TIME()` and selector `0x002134cc`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "L2_BLOCK_TIME", abi = "L2_BLOCK_TIME()")]
	pub struct L2BlockTimeCall;
	///Container type for all input parameters for the `PROPOSER` function with signature
	/// `PROPOSER()` and selector `0xbffa7f0f`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "PROPOSER", abi = "PROPOSER()")]
	pub struct ProposerCall;
	///Container type for all input parameters for the `SUBMISSION_INTERVAL` function with
	/// signature `SUBMISSION_INTERVAL()` and selector `0x529933df`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "SUBMISSION_INTERVAL", abi = "SUBMISSION_INTERVAL()")]
	pub struct SubmissionIntervalCall;
	///Container type for all input parameters for the `computeL2Timestamp` function with signature
	/// `computeL2Timestamp(uint256)` and selector `0xd1de856c`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "computeL2Timestamp", abi = "computeL2Timestamp(uint256)")]
	pub struct ComputeL2TimestampCall {
		pub l_2_block_number: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `deleteL2Outputs` function with signature
	/// `deleteL2Outputs(uint256)` and selector `0x89c44cbb`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "deleteL2Outputs", abi = "deleteL2Outputs(uint256)")]
	pub struct DeleteL2OutputsCall {
		pub l_2_output_index: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `getL2Output` function with signature
	/// `getL2Output(uint256)` and selector `0xa25ae557`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "getL2Output", abi = "getL2Output(uint256)")]
	pub struct GetL2OutputCall {
		pub l_2_output_index: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `getL2OutputAfter` function with signature
	/// `getL2OutputAfter(uint256)` and selector `0xcf8e5cf0`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "getL2OutputAfter", abi = "getL2OutputAfter(uint256)")]
	pub struct GetL2OutputAfterCall {
		pub l_2_block_number: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `getL2OutputIndexAfter` function with
	/// signature `getL2OutputIndexAfter(uint256)` and selector `0x7f006420`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "getL2OutputIndexAfter", abi = "getL2OutputIndexAfter(uint256)")]
	pub struct GetL2OutputIndexAfterCall {
		pub l_2_block_number: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `initialize` function with signature
	/// `initialize(uint256,uint256)` and selector `0xe4a30116`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "initialize", abi = "initialize(uint256,uint256)")]
	pub struct InitializeCall {
		pub starting_block_number: ::ethers::core::types::U256,
		pub starting_timestamp: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `latestBlockNumber` function with signature
	/// `latestBlockNumber()` and selector `0x4599c788`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "latestBlockNumber", abi = "latestBlockNumber()")]
	pub struct LatestBlockNumberCall;
	///Container type for all input parameters for the `latestOutputIndex` function with signature
	/// `latestOutputIndex()` and selector `0x69f16eec`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "latestOutputIndex", abi = "latestOutputIndex()")]
	pub struct LatestOutputIndexCall;
	///Container type for all input parameters for the `nextBlockNumber` function with signature
	/// `nextBlockNumber()` and selector `0xdcec3348`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "nextBlockNumber", abi = "nextBlockNumber()")]
	pub struct NextBlockNumberCall;
	///Container type for all input parameters for the `nextOutputIndex` function with signature
	/// `nextOutputIndex()` and selector `0x6abcf563`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "nextOutputIndex", abi = "nextOutputIndex()")]
	pub struct NextOutputIndexCall;
	///Container type for all input parameters for the `proposeL2Output` function with signature
	/// `proposeL2Output(bytes32,uint256,bytes32,uint256)` and selector `0x9aaab648`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "proposeL2Output", abi = "proposeL2Output(bytes32,uint256,bytes32,uint256)")]
	pub struct ProposeL2OutputCall {
		pub output_root: [u8; 32],
		pub l_2_block_number: ::ethers::core::types::U256,
		pub l_1_block_hash: [u8; 32],
		pub l_1_block_number: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `startingBlockNumber` function with
	/// signature `startingBlockNumber()` and selector `0x70872aa5`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "startingBlockNumber", abi = "startingBlockNumber()")]
	pub struct StartingBlockNumberCall;
	///Container type for all input parameters for the `startingTimestamp` function with signature
	/// `startingTimestamp()` and selector `0x88786272`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "startingTimestamp", abi = "startingTimestamp()")]
	pub struct StartingTimestampCall;
	///Container type for all input parameters for the `version` function with signature
	/// `version()` and selector `0x54fd4d50`
	#[derive(
		Clone,
		::ethers::contract::EthCall,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethcall(name = "version", abi = "version()")]
	pub struct VersionCall;
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum L2OutputOracleCalls {
		Challenger(ChallengerCall),
		FinalizationPeriodSeconds(FinalizationPeriodSecondsCall),
		L2BlockTime(L2BlockTimeCall),
		Proposer(ProposerCall),
		SubmissionInterval(SubmissionIntervalCall),
		ComputeL2Timestamp(ComputeL2TimestampCall),
		DeleteL2Outputs(DeleteL2OutputsCall),
		GetL2Output(GetL2OutputCall),
		GetL2OutputAfter(GetL2OutputAfterCall),
		GetL2OutputIndexAfter(GetL2OutputIndexAfterCall),
		Initialize(InitializeCall),
		LatestBlockNumber(LatestBlockNumberCall),
		LatestOutputIndex(LatestOutputIndexCall),
		NextBlockNumber(NextBlockNumberCall),
		NextOutputIndex(NextOutputIndexCall),
		ProposeL2Output(ProposeL2OutputCall),
		StartingBlockNumber(StartingBlockNumberCall),
		StartingTimestamp(StartingTimestampCall),
		Version(VersionCall),
	}
	impl ::ethers::core::abi::AbiDecode for L2OutputOracleCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <ChallengerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Challenger(decoded));
			}
			if let Ok(decoded) =
				<FinalizationPeriodSecondsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::FinalizationPeriodSeconds(decoded));
			}
			if let Ok(decoded) = <L2BlockTimeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::L2BlockTime(decoded));
			}
			if let Ok(decoded) = <ProposerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Proposer(decoded));
			}
			if let Ok(decoded) =
				<SubmissionIntervalCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SubmissionInterval(decoded));
			}
			if let Ok(decoded) =
				<ComputeL2TimestampCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ComputeL2Timestamp(decoded));
			}
			if let Ok(decoded) =
				<DeleteL2OutputsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DeleteL2Outputs(decoded));
			}
			if let Ok(decoded) = <GetL2OutputCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetL2Output(decoded));
			}
			if let Ok(decoded) =
				<GetL2OutputAfterCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL2OutputAfter(decoded));
			}
			if let Ok(decoded) =
				<GetL2OutputIndexAfterCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL2OutputIndexAfter(decoded));
			}
			if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Initialize(decoded));
			}
			if let Ok(decoded) =
				<LatestBlockNumberCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestBlockNumber(decoded));
			}
			if let Ok(decoded) =
				<LatestOutputIndexCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestOutputIndex(decoded));
			}
			if let Ok(decoded) =
				<NextBlockNumberCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::NextBlockNumber(decoded));
			}
			if let Ok(decoded) =
				<NextOutputIndexCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::NextOutputIndex(decoded));
			}
			if let Ok(decoded) =
				<ProposeL2OutputCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ProposeL2Output(decoded));
			}
			if let Ok(decoded) =
				<StartingBlockNumberCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StartingBlockNumber(decoded));
			}
			if let Ok(decoded) =
				<StartingTimestampCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StartingTimestamp(decoded));
			}
			if let Ok(decoded) = <VersionCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Version(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for L2OutputOracleCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::Challenger(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FinalizationPeriodSeconds(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::L2BlockTime(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Proposer(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SubmissionInterval(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ComputeL2Timestamp(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DeleteL2Outputs(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL2Output(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL2OutputAfter(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL2OutputIndexAfter(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestBlockNumber(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestOutputIndex(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::NextBlockNumber(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::NextOutputIndex(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ProposeL2Output(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StartingBlockNumber(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StartingTimestamp(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Version(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for L2OutputOracleCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Challenger(element) => ::core::fmt::Display::fmt(element, f),
				Self::FinalizationPeriodSeconds(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2BlockTime(element) => ::core::fmt::Display::fmt(element, f),
				Self::Proposer(element) => ::core::fmt::Display::fmt(element, f),
				Self::SubmissionInterval(element) => ::core::fmt::Display::fmt(element, f),
				Self::ComputeL2Timestamp(element) => ::core::fmt::Display::fmt(element, f),
				Self::DeleteL2Outputs(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL2Output(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL2OutputAfter(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL2OutputIndexAfter(element) => ::core::fmt::Display::fmt(element, f),
				Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestBlockNumber(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestOutputIndex(element) => ::core::fmt::Display::fmt(element, f),
				Self::NextBlockNumber(element) => ::core::fmt::Display::fmt(element, f),
				Self::NextOutputIndex(element) => ::core::fmt::Display::fmt(element, f),
				Self::ProposeL2Output(element) => ::core::fmt::Display::fmt(element, f),
				Self::StartingBlockNumber(element) => ::core::fmt::Display::fmt(element, f),
				Self::StartingTimestamp(element) => ::core::fmt::Display::fmt(element, f),
				Self::Version(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<ChallengerCall> for L2OutputOracleCalls {
		fn from(value: ChallengerCall) -> Self {
			Self::Challenger(value)
		}
	}
	impl ::core::convert::From<FinalizationPeriodSecondsCall> for L2OutputOracleCalls {
		fn from(value: FinalizationPeriodSecondsCall) -> Self {
			Self::FinalizationPeriodSeconds(value)
		}
	}
	impl ::core::convert::From<L2BlockTimeCall> for L2OutputOracleCalls {
		fn from(value: L2BlockTimeCall) -> Self {
			Self::L2BlockTime(value)
		}
	}
	impl ::core::convert::From<ProposerCall> for L2OutputOracleCalls {
		fn from(value: ProposerCall) -> Self {
			Self::Proposer(value)
		}
	}
	impl ::core::convert::From<SubmissionIntervalCall> for L2OutputOracleCalls {
		fn from(value: SubmissionIntervalCall) -> Self {
			Self::SubmissionInterval(value)
		}
	}
	impl ::core::convert::From<ComputeL2TimestampCall> for L2OutputOracleCalls {
		fn from(value: ComputeL2TimestampCall) -> Self {
			Self::ComputeL2Timestamp(value)
		}
	}
	impl ::core::convert::From<DeleteL2OutputsCall> for L2OutputOracleCalls {
		fn from(value: DeleteL2OutputsCall) -> Self {
			Self::DeleteL2Outputs(value)
		}
	}
	impl ::core::convert::From<GetL2OutputCall> for L2OutputOracleCalls {
		fn from(value: GetL2OutputCall) -> Self {
			Self::GetL2Output(value)
		}
	}
	impl ::core::convert::From<GetL2OutputAfterCall> for L2OutputOracleCalls {
		fn from(value: GetL2OutputAfterCall) -> Self {
			Self::GetL2OutputAfter(value)
		}
	}
	impl ::core::convert::From<GetL2OutputIndexAfterCall> for L2OutputOracleCalls {
		fn from(value: GetL2OutputIndexAfterCall) -> Self {
			Self::GetL2OutputIndexAfter(value)
		}
	}
	impl ::core::convert::From<InitializeCall> for L2OutputOracleCalls {
		fn from(value: InitializeCall) -> Self {
			Self::Initialize(value)
		}
	}
	impl ::core::convert::From<LatestBlockNumberCall> for L2OutputOracleCalls {
		fn from(value: LatestBlockNumberCall) -> Self {
			Self::LatestBlockNumber(value)
		}
	}
	impl ::core::convert::From<LatestOutputIndexCall> for L2OutputOracleCalls {
		fn from(value: LatestOutputIndexCall) -> Self {
			Self::LatestOutputIndex(value)
		}
	}
	impl ::core::convert::From<NextBlockNumberCall> for L2OutputOracleCalls {
		fn from(value: NextBlockNumberCall) -> Self {
			Self::NextBlockNumber(value)
		}
	}
	impl ::core::convert::From<NextOutputIndexCall> for L2OutputOracleCalls {
		fn from(value: NextOutputIndexCall) -> Self {
			Self::NextOutputIndex(value)
		}
	}
	impl ::core::convert::From<ProposeL2OutputCall> for L2OutputOracleCalls {
		fn from(value: ProposeL2OutputCall) -> Self {
			Self::ProposeL2Output(value)
		}
	}
	impl ::core::convert::From<StartingBlockNumberCall> for L2OutputOracleCalls {
		fn from(value: StartingBlockNumberCall) -> Self {
			Self::StartingBlockNumber(value)
		}
	}
	impl ::core::convert::From<StartingTimestampCall> for L2OutputOracleCalls {
		fn from(value: StartingTimestampCall) -> Self {
			Self::StartingTimestamp(value)
		}
	}
	impl ::core::convert::From<VersionCall> for L2OutputOracleCalls {
		fn from(value: VersionCall) -> Self {
			Self::Version(value)
		}
	}
	///Container type for all return fields from the `CHALLENGER` function with signature
	/// `CHALLENGER()` and selector `0x6b4d98dd`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct ChallengerReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `FINALIZATION_PERIOD_SECONDS` function with
	/// signature `FINALIZATION_PERIOD_SECONDS()` and selector `0xf4daa291`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct FinalizationPeriodSecondsReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `L2_BLOCK_TIME` function with signature
	/// `L2_BLOCK_TIME()` and selector `0x002134cc`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct L2BlockTimeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `PROPOSER` function with signature
	/// `PROPOSER()` and selector `0xbffa7f0f`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct ProposerReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `SUBMISSION_INTERVAL` function with signature
	/// `SUBMISSION_INTERVAL()` and selector `0x529933df`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct SubmissionIntervalReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `computeL2Timestamp` function with signature
	/// `computeL2Timestamp(uint256)` and selector `0xd1de856c`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct ComputeL2TimestampReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL2Output` function with signature
	/// `getL2Output(uint256)` and selector `0xa25ae557`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct GetL2OutputReturn(pub OutputProposal);
	///Container type for all return fields from the `getL2OutputAfter` function with signature
	/// `getL2OutputAfter(uint256)` and selector `0xcf8e5cf0`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct GetL2OutputAfterReturn(pub OutputProposal);
	///Container type for all return fields from the `getL2OutputIndexAfter` function with
	/// signature `getL2OutputIndexAfter(uint256)` and selector `0x7f006420`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct GetL2OutputIndexAfterReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `latestBlockNumber` function with signature
	/// `latestBlockNumber()` and selector `0x4599c788`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct LatestBlockNumberReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `latestOutputIndex` function with signature
	/// `latestOutputIndex()` and selector `0x69f16eec`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct LatestOutputIndexReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `nextBlockNumber` function with signature
	/// `nextBlockNumber()` and selector `0xdcec3348`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct NextBlockNumberReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `nextOutputIndex` function with signature
	/// `nextOutputIndex()` and selector `0x6abcf563`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct NextOutputIndexReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `startingBlockNumber` function with signature
	/// `startingBlockNumber()` and selector `0x70872aa5`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct StartingBlockNumberReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `startingTimestamp` function with signature
	/// `startingTimestamp()` and selector `0x88786272`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct StartingTimestampReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `version` function with signature `version()`
	/// and selector `0x54fd4d50`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct VersionReturn(pub ::std::string::String);
	///`OutputProposal(bytes32,uint128,uint128)`
	#[derive(
		Clone,
		::ethers::contract::EthAbiType,
		::ethers::contract::EthAbiCodec,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	pub struct OutputProposal {
		pub output_root: [u8; 32],
		pub timestamp: u128,
		pub l_2_block_number: u128,
	}
}
