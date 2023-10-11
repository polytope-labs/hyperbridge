pub use i_ismp_host::*;
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
pub mod i_ismp_host {
	pub use super::super::shared_types::*;
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::None,
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("admin"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("admin"),
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
					::std::borrow::ToOwned::to_owned("challengePeriod"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("challengePeriod"),
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
					::std::borrow::ToOwned::to_owned("consensusClient"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("consensusClient"),
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
					::std::borrow::ToOwned::to_owned("consensusState"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("consensusState"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("consensusUpdateTime"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("consensusUpdateTime",),
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
					::std::borrow::ToOwned::to_owned("dispatch"),
					::std::vec![
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatch"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("request"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Array(
										::std::boxed::Box::new(
											::ethers::core::abi::ethabi::ParamType::Bytes,
										),
									),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct DispatchGet"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatch"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("response"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									],),
									::ethers::core::abi::ethabi::ParamType::Bytes,
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct PostResponse"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatch"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("request"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct DispatchPost"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
					],
				),
				(
					::std::borrow::ToOwned::to_owned("dispatchIncoming"),
					::std::vec![
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("timeout"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									],),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct PostTimeout"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("request"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct PostRequest"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("request"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Array(
										::std::boxed::Box::new(
											::ethers::core::abi::ethabi::ParamType::Bytes,
										),
									),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct GetRequest"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("response"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									],),
									::ethers::core::abi::ethabi::ParamType::Bytes,
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct PostResponse"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("response"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Array(
											::std::boxed::Box::new(
												::ethers::core::abi::ethabi::ParamType::Bytes,
											),
										),
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									],),
									::ethers::core::abi::ethabi::ParamType::Array(
										::std::boxed::Box::new(
											::ethers::core::abi::ethabi::ParamType::Tuple(
												::std::vec![
													::ethers::core::abi::ethabi::ParamType::Bytes,
													::ethers::core::abi::ethabi::ParamType::Bytes,
												],
											),
										),
									),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct GetResponse"),
								),
							},],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
					],
				),
				(
					::std::borrow::ToOwned::to_owned("frozen"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("frozen"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("host"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("host"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("latestStateMachineHeight"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("latestStateMachineHeight",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("requestCommitments"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("requestCommitments"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("commitment"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes32"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("requestReceipts"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("requestReceipts"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("commitment"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes32"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("responseCommitments"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("responseCommitments",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("commitment"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes32"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("responseReceipts"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("responseReceipts"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("commitment"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes32"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("setBridgeParams"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setBridgeParams"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("params"),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Address,
								::ethers::core::abi::ethabi::ParamType::Address,
								::ethers::core::abi::ethabi::ParamType::Address,
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct BridgeParams"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("setConsensusState"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setConsensusState"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("consensusState"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("setFrozenState"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setFrozenState"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("newState"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("stateMachineCommitment"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("stateMachineCommitment",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("height"),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct StateMachineHeight",),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
								::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct StateCommitment"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("stateMachineCommitmentUpdateTime"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("stateMachineCommitmentUpdateTime",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("height"),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								::ethers::core::abi::ethabi::ParamType::Uint(256usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct StateMachineHeight",),
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
					::std::borrow::ToOwned::to_owned("storeConsensusState"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("storeConsensusState",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("state"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("storeConsensusUpdateTime"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("storeConsensusUpdateTime",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("time"),
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
					::std::borrow::ToOwned::to_owned("storeLatestStateMachineHeight"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("storeLatestStateMachineHeight",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("height"),
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
					::std::borrow::ToOwned::to_owned("storeStateMachineCommitment"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("storeStateMachineCommitment",),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("height"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct StateMachineHeight",),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("commitment"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct StateCommitment"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("storeStateMachineCommitmentUpdateTime"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned(
							"storeStateMachineCommitmentUpdateTime",
						),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("height"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct StateMachineHeight",),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("time"),
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
					::std::borrow::ToOwned::to_owned("timestamp"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("timestamp"),
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
					::std::borrow::ToOwned::to_owned("unStakingPeriod"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("unStakingPeriod"),
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
			]),
			events: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("GetRequestEvent"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("GetRequestEvent"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("source"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("dest"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("from"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("keys"),
								kind: ::ethers::core::abi::ethabi::ParamType::Array(
									::std::boxed::Box::new(
										::ethers::core::abi::ethabi::ParamType::Bytes,
									),
								),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("nonce"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("height"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gaslimit"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("PostRequestEvent"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("PostRequestEvent"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("source"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("dest"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("from"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("to"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("nonce"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("data"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gaslimit"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("PostResponseEvent"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("PostResponseEvent"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("source"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("dest"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("from"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("to"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("nonce"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("data"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gaslimit"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: false,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("response"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
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
	pub static IISMPHOST_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct IIsmpHost<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for IIsmpHost<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for IIsmpHost<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for IIsmpHost<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for IIsmpHost<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(IIsmpHost)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> IIsmpHost<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), IISMPHOST_ABI.clone(), client))
		}
		///Calls the contract's `admin` (0xf851a440) function
		pub fn admin(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([248, 81, 164, 64], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `challengePeriod` (0xf3f480d9) function
		pub fn challenge_period(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([243, 244, 128, 217], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `consensusClient` (0x2476132b) function
		pub fn consensus_client(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([36, 118, 19, 43], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `consensusState` (0xbbad99d4) function
		pub fn consensus_state(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([187, 173, 153, 212], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `consensusUpdateTime` (0x9a8425bc) function
		pub fn consensus_update_time(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([154, 132, 37, 188], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0x67bd911f) function
		pub fn dispatch(
			&self,
			request: DispatchPost,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([103, 189, 145, 31], (request,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0xccbaa9ea) function
		pub fn dispatch_with_response(
			&self,
			response: PostResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([204, 186, 169, 234], (response,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0xd25bcd3d) function
		pub fn dispatch_with_request(
			&self,
			request: DispatchPost,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([210, 91, 205, 61], (request,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0x25bbc406) function
		pub fn dispatch_incoming_0(
			&self,
			timeout: PostTimeout,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([37, 187, 196, 6], (timeout,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0x3b8c2bf7) function
		pub fn dispatch_incoming_1(
			&self,
			request: GetRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([59, 140, 43, 247], (request,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0x84566a5d) function
		pub fn dispatch_incoming_2(
			&self,
			request: GetRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([132, 86, 106, 93], (request,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0x8cf66b92) function
		pub fn dispatch_incoming_3(
			&self,
			response: GetResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([140, 246, 107, 146], (response,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0xf0736091) function
		pub fn dispatch_incoming_4(
			&self,
			response: GetResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([240, 115, 96, 145], (response,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `frozen` (0x054f7d9c) function
		pub fn frozen(&self) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([5, 79, 125, 156], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `host` (0xf437bc59) function
		pub fn host(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([244, 55, 188, 89], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestStateMachineHeight` (0x56b65597) function
		pub fn latest_state_machine_height(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([86, 182, 85, 151], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `requestCommitments` (0x368bf464) function
		pub fn request_commitments(
			&self,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([54, 139, 244, 100], commitment)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `requestReceipts` (0x19667a3e) function
		pub fn request_receipts(
			&self,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([25, 102, 122, 62], commitment)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `responseCommitments` (0x2211f1dd) function
		pub fn response_commitments(
			&self,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([34, 17, 241, 221], commitment)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `responseReceipts` (0x8856337e) function
		pub fn response_receipts(
			&self,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([136, 86, 51, 126], commitment)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setBridgeParams` (0xc5ea977a) function
		pub fn set_bridge_params(
			&self,
			params: BridgeParams,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([197, 234, 151, 122], (params,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setConsensusState` (0xa15f7431) function
		pub fn set_consensus_state(
			&self,
			consensus_state: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([161, 95, 116, 49], consensus_state)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setFrozenState` (0x19e8faf1) function
		pub fn set_frozen_state(
			&self,
			new_state: bool,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([25, 232, 250, 241], new_state)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `stateMachineCommitment` (0xa70a8c47) function
		pub fn state_machine_commitment(
			&self,
			height: StateMachineHeight,
		) -> ::ethers::contract::builders::ContractCall<M, StateCommitment> {
			self.0
				.method_hash([167, 10, 140, 71], (height,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `stateMachineCommitmentUpdateTime` (0x1a880a93) function
		pub fn state_machine_commitment_update_time(
			&self,
			height: StateMachineHeight,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([26, 136, 10, 147], (height,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `storeConsensusState` (0xb4974cf0) function
		pub fn store_consensus_state(
			&self,
			state: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([180, 151, 76, 240], state)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `storeConsensusUpdateTime` (0xd860cb47) function
		pub fn store_consensus_update_time(
			&self,
			time: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([216, 96, 203, 71], time)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `storeLatestStateMachineHeight` (0xa0756ecd) function
		pub fn store_latest_state_machine_height(
			&self,
			height: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([160, 117, 110, 205], height)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `storeStateMachineCommitment` (0x559efe9e) function
		pub fn store_state_machine_commitment(
			&self,
			height: StateMachineHeight,
			commitment: StateCommitment,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([85, 158, 254, 158], (height, commitment))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `storeStateMachineCommitmentUpdateTime` (0x14863dcb) function
		pub fn store_state_machine_commitment_update_time(
			&self,
			height: StateMachineHeight,
			time: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([20, 134, 61, 203], (height, time))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `timestamp` (0xb80777ea) function
		pub fn timestamp(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([184, 7, 119, 234], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `unStakingPeriod` (0xd40784c7) function
		pub fn un_staking_period(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([212, 7, 132, 199], ())
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `GetRequestEvent` event
		pub fn get_request_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GetRequestEventFilter> {
			self.0.event()
		}
		///Gets the contract's `PostRequestEvent` event
		pub fn post_request_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostRequestEventFilter> {
			self.0.event()
		}
		///Gets the contract's `PostResponseEvent` event
		pub fn post_response_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostResponseEventFilter> {
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, IIsmpHostEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for IIsmpHost<M> {
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
	#[ethevent(
		name = "GetRequestEvent",
		abi = "GetRequestEvent(bytes,bytes,bytes,bytes[],uint256,uint256,uint256,uint256)"
	)]
	pub struct GetRequestEventFilter {
		pub source: ::ethers::core::types::Bytes,
		pub dest: ::ethers::core::types::Bytes,
		pub from: ::ethers::core::types::Bytes,
		pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
		#[ethevent(indexed)]
		pub nonce: ::ethers::core::types::U256,
		pub height: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub gaslimit: ::ethers::core::types::U256,
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
	#[ethevent(
		name = "PostRequestEvent",
		abi = "PostRequestEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,uint256)"
	)]
	pub struct PostRequestEventFilter {
		pub source: ::ethers::core::types::Bytes,
		pub dest: ::ethers::core::types::Bytes,
		pub from: ::ethers::core::types::Bytes,
		pub to: ::ethers::core::types::Bytes,
		#[ethevent(indexed)]
		pub nonce: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub data: ::ethers::core::types::Bytes,
		pub gaslimit: ::ethers::core::types::U256,
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
	#[ethevent(
		name = "PostResponseEvent",
		abi = "PostResponseEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,uint256,bytes)"
	)]
	pub struct PostResponseEventFilter {
		pub source: ::ethers::core::types::Bytes,
		pub dest: ::ethers::core::types::Bytes,
		pub from: ::ethers::core::types::Bytes,
		pub to: ::ethers::core::types::Bytes,
		#[ethevent(indexed)]
		pub nonce: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub data: ::ethers::core::types::Bytes,
		pub gaslimit: ::ethers::core::types::U256,
		pub response: ::ethers::core::types::Bytes,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum IIsmpHostEvents {
		GetRequestEventFilter(GetRequestEventFilter),
		PostRequestEventFilter(PostRequestEventFilter),
		PostResponseEventFilter(PostResponseEventFilter),
	}
	impl ::ethers::contract::EthLogDecode for IIsmpHostEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = GetRequestEventFilter::decode_log(log) {
				return Ok(IIsmpHostEvents::GetRequestEventFilter(decoded))
			}
			if let Ok(decoded) = PostRequestEventFilter::decode_log(log) {
				return Ok(IIsmpHostEvents::PostRequestEventFilter(decoded))
			}
			if let Ok(decoded) = PostResponseEventFilter::decode_log(log) {
				return Ok(IIsmpHostEvents::PostResponseEventFilter(decoded))
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for IIsmpHostEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::GetRequestEventFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostRequestEventFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostResponseEventFilter(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<GetRequestEventFilter> for IIsmpHostEvents {
		fn from(value: GetRequestEventFilter) -> Self {
			Self::GetRequestEventFilter(value)
		}
	}
	impl ::core::convert::From<PostRequestEventFilter> for IIsmpHostEvents {
		fn from(value: PostRequestEventFilter) -> Self {
			Self::PostRequestEventFilter(value)
		}
	}
	impl ::core::convert::From<PostResponseEventFilter> for IIsmpHostEvents {
		fn from(value: PostResponseEventFilter) -> Self {
			Self::PostResponseEventFilter(value)
		}
	}
	///Container type for all input parameters for the `admin` function with signature `admin()`
	/// and selector `0xf851a440`
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
	#[ethcall(name = "admin", abi = "admin()")]
	pub struct AdminCall;
	///Container type for all input parameters for the `challengePeriod` function with signature
	/// `challengePeriod()` and selector `0xf3f480d9`
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
	#[ethcall(name = "challengePeriod", abi = "challengePeriod()")]
	pub struct ChallengePeriodCall;
	///Container type for all input parameters for the `consensusClient` function with signature
	/// `consensusClient()` and selector `0x2476132b`
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
	#[ethcall(name = "consensusClient", abi = "consensusClient()")]
	pub struct ConsensusClientCall;
	///Container type for all input parameters for the `consensusState` function with signature
	/// `consensusState()` and selector `0xbbad99d4`
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
	#[ethcall(name = "consensusState", abi = "consensusState()")]
	pub struct ConsensusStateCall;
	///Container type for all input parameters for the `consensusUpdateTime` function with
	/// signature `consensusUpdateTime()` and selector `0x9a8425bc`
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
	#[ethcall(name = "consensusUpdateTime", abi = "consensusUpdateTime()")]
	pub struct ConsensusUpdateTimeCall;
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch((bytes,uint64,bytes[],uint64,uint64))` and selector `0x67bd911f`
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
	#[ethcall(name = "dispatch", abi = "dispatch((bytes,uint64,bytes[],uint64,uint64))")]
	pub struct DispatchCall {
		pub request: DispatchPost,
	}
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))` and selector
	/// `0xccbaa9ea`
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
	#[ethcall(
		name = "dispatch",
		abi = "dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))"
	)]
	pub struct DispatchWithResponseCall {
		pub response: PostResponse,
	}
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch((bytes,bytes,bytes,uint64,uint64))` and selector `0xd25bcd3d`
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
	#[ethcall(name = "dispatch", abi = "dispatch((bytes,bytes,bytes,uint64,uint64))")]
	pub struct DispatchWithRequestCall {
		pub request: DispatchPost,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64)))` and selector
	/// `0x25bbc406`
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
	#[ethcall(
		name = "dispatchIncoming",
		abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64)))"
	)]
	pub struct DispatchIncoming0Call {
		pub timeout: PostTimeout,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector
	/// `0x3b8c2bf7`
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
	#[ethcall(
		name = "dispatchIncoming",
		abi = "dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
	)]
	pub struct DispatchIncoming1Call {
		pub request: GetRequest,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector
	/// `0x84566a5d`
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
	#[ethcall(
		name = "dispatchIncoming",
		abi = "dispatchIncoming((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
	)]
	pub struct DispatchIncoming2Call {
		pub request: GetRequest,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))` and
	/// selector `0x8cf66b92`
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
	#[ethcall(
		name = "dispatchIncoming",
		abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))"
	)]
	pub struct DispatchIncoming3Call {
		pub response: GetResponse,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,
	/// bytes)[]))` and selector `0xf0736091`
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
	#[ethcall(
		name = "dispatchIncoming",
		abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))"
	)]
	pub struct DispatchIncoming4Call {
		pub response: GetResponse,
	}
	///Container type for all input parameters for the `frozen` function with signature `frozen()`
	/// and selector `0x054f7d9c`
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
	#[ethcall(name = "frozen", abi = "frozen()")]
	pub struct FrozenCall;
	///Container type for all input parameters for the `host` function with signature `host()` and
	/// selector `0xf437bc59`
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
	#[ethcall(name = "host", abi = "host()")]
	pub struct HostCall;
	///Container type for all input parameters for the `latestStateMachineHeight` function with
	/// signature `latestStateMachineHeight()` and selector `0x56b65597`
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
	#[ethcall(name = "latestStateMachineHeight", abi = "latestStateMachineHeight()")]
	pub struct LatestStateMachineHeightCall;
	///Container type for all input parameters for the `requestCommitments` function with signature
	/// `requestCommitments(bytes32)` and selector `0x368bf464`
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
	#[ethcall(name = "requestCommitments", abi = "requestCommitments(bytes32)")]
	pub struct RequestCommitmentsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `requestReceipts` function with signature
	/// `requestReceipts(bytes32)` and selector `0x19667a3e`
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
	#[ethcall(name = "requestReceipts", abi = "requestReceipts(bytes32)")]
	pub struct RequestReceiptsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `responseCommitments` function with
	/// signature `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
	#[ethcall(name = "responseCommitments", abi = "responseCommitments(bytes32)")]
	pub struct ResponseCommitmentsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `responseReceipts` function with signature
	/// `responseReceipts(bytes32)` and selector `0x8856337e`
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
	#[ethcall(name = "responseReceipts", abi = "responseReceipts(bytes32)")]
	pub struct ResponseReceiptsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `setBridgeParams` function with signature
	/// `setBridgeParams((address,address,address,uint256,uint256,uint256))` and selector
	/// `0xc5ea977a`
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
	#[ethcall(
		name = "setBridgeParams",
		abi = "setBridgeParams((address,address,address,uint256,uint256,uint256))"
	)]
	pub struct SetBridgeParamsCall {
		pub params: BridgeParams,
	}
	///Container type for all input parameters for the `setConsensusState` function with signature
	/// `setConsensusState(bytes)` and selector `0xa15f7431`
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
	#[ethcall(name = "setConsensusState", abi = "setConsensusState(bytes)")]
	pub struct SetConsensusStateCall {
		pub consensus_state: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `setFrozenState` function with signature
	/// `setFrozenState(bool)` and selector `0x19e8faf1`
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
	#[ethcall(name = "setFrozenState", abi = "setFrozenState(bool)")]
	pub struct SetFrozenStateCall {
		pub new_state: bool,
	}
	///Container type for all input parameters for the `stateMachineCommitment` function with
	/// signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
	#[ethcall(name = "stateMachineCommitment", abi = "stateMachineCommitment((uint256,uint256))")]
	pub struct StateMachineCommitmentCall {
		pub height: StateMachineHeight,
	}
	///Container type for all input parameters for the `stateMachineCommitmentUpdateTime` function
	/// with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector
	/// `0x1a880a93`
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
	#[ethcall(
		name = "stateMachineCommitmentUpdateTime",
		abi = "stateMachineCommitmentUpdateTime((uint256,uint256))"
	)]
	pub struct StateMachineCommitmentUpdateTimeCall {
		pub height: StateMachineHeight,
	}
	///Container type for all input parameters for the `storeConsensusState` function with
	/// signature `storeConsensusState(bytes)` and selector `0xb4974cf0`
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
	#[ethcall(name = "storeConsensusState", abi = "storeConsensusState(bytes)")]
	pub struct StoreConsensusStateCall {
		pub state: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `storeConsensusUpdateTime` function with
	/// signature `storeConsensusUpdateTime(uint256)` and selector `0xd860cb47`
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
	#[ethcall(name = "storeConsensusUpdateTime", abi = "storeConsensusUpdateTime(uint256)")]
	pub struct StoreConsensusUpdateTimeCall {
		pub time: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `storeLatestStateMachineHeight` function
	/// with signature `storeLatestStateMachineHeight(uint256)` and selector `0xa0756ecd`
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
	#[ethcall(
		name = "storeLatestStateMachineHeight",
		abi = "storeLatestStateMachineHeight(uint256)"
	)]
	pub struct StoreLatestStateMachineHeightCall {
		pub height: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `storeStateMachineCommitment` function with
	/// signature `storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))` and
	/// selector `0x559efe9e`
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
	#[ethcall(
		name = "storeStateMachineCommitment",
		abi = "storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))"
	)]
	pub struct StoreStateMachineCommitmentCall {
		pub height: StateMachineHeight,
		pub commitment: StateCommitment,
	}
	///Container type for all input parameters for the `storeStateMachineCommitmentUpdateTime`
	/// function with signature `storeStateMachineCommitmentUpdateTime((uint256,uint256),uint256)`
	/// and selector `0x14863dcb`
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
	#[ethcall(
		name = "storeStateMachineCommitmentUpdateTime",
		abi = "storeStateMachineCommitmentUpdateTime((uint256,uint256),uint256)"
	)]
	pub struct StoreStateMachineCommitmentUpdateTimeCall {
		pub height: StateMachineHeight,
		pub time: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `timestamp` function with signature
	/// `timestamp()` and selector `0xb80777ea`
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
	#[ethcall(name = "timestamp", abi = "timestamp()")]
	pub struct TimestampCall;
	///Container type for all input parameters for the `unStakingPeriod` function with signature
	/// `unStakingPeriod()` and selector `0xd40784c7`
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
	#[ethcall(name = "unStakingPeriod", abi = "unStakingPeriod()")]
	pub struct UnStakingPeriodCall;
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum IIsmpHostCalls {
		Admin(AdminCall),
		ChallengePeriod(ChallengePeriodCall),
		ConsensusClient(ConsensusClientCall),
		ConsensusState(ConsensusStateCall),
		ConsensusUpdateTime(ConsensusUpdateTimeCall),
		Dispatch(DispatchCall),
		DispatchWithResponse(DispatchWithResponseCall),
		DispatchWithRequest(DispatchWithRequestCall),
		DispatchIncoming0(DispatchIncoming0Call),
		DispatchIncoming1(DispatchIncoming1Call),
		DispatchIncoming2(DispatchIncoming2Call),
		DispatchIncoming3(DispatchIncoming3Call),
		DispatchIncoming4(DispatchIncoming4Call),
		Frozen(FrozenCall),
		Host(HostCall),
		LatestStateMachineHeight(LatestStateMachineHeightCall),
		RequestCommitments(RequestCommitmentsCall),
		RequestReceipts(RequestReceiptsCall),
		ResponseCommitments(ResponseCommitmentsCall),
		ResponseReceipts(ResponseReceiptsCall),
		SetBridgeParams(SetBridgeParamsCall),
		SetConsensusState(SetConsensusStateCall),
		SetFrozenState(SetFrozenStateCall),
		StateMachineCommitment(StateMachineCommitmentCall),
		StateMachineCommitmentUpdateTime(StateMachineCommitmentUpdateTimeCall),
		StoreConsensusState(StoreConsensusStateCall),
		StoreConsensusUpdateTime(StoreConsensusUpdateTimeCall),
		StoreLatestStateMachineHeight(StoreLatestStateMachineHeightCall),
		StoreStateMachineCommitment(StoreStateMachineCommitmentCall),
		StoreStateMachineCommitmentUpdateTime(StoreStateMachineCommitmentUpdateTimeCall),
		Timestamp(TimestampCall),
		UnStakingPeriod(UnStakingPeriodCall),
	}
	impl ::ethers::core::abi::AbiDecode for IIsmpHostCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <AdminCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Admin(decoded))
			}
			if let Ok(decoded) =
				<ChallengePeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ChallengePeriod(decoded))
			}
			if let Ok(decoded) =
				<ConsensusClientCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusClient(decoded))
			}
			if let Ok(decoded) =
				<ConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusState(decoded))
			}
			if let Ok(decoded) =
				<ConsensusUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusUpdateTime(decoded))
			}
			if let Ok(decoded) = <DispatchCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Dispatch(decoded))
			}
			if let Ok(decoded) =
				<DispatchWithResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchWithResponse(decoded))
			}
			if let Ok(decoded) =
				<DispatchWithRequestCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchWithRequest(decoded))
			}
			if let Ok(decoded) =
				<DispatchIncoming0Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming0(decoded))
			}
			if let Ok(decoded) =
				<DispatchIncoming1Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming1(decoded))
			}
			if let Ok(decoded) =
				<DispatchIncoming2Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming2(decoded))
			}
			if let Ok(decoded) =
				<DispatchIncoming3Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming3(decoded))
			}
			if let Ok(decoded) =
				<DispatchIncoming4Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming4(decoded))
			}
			if let Ok(decoded) = <FrozenCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Frozen(decoded))
			}
			if let Ok(decoded) = <HostCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Host(decoded))
			}
			if let Ok(decoded) =
				<LatestStateMachineHeightCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestStateMachineHeight(decoded))
			}
			if let Ok(decoded) =
				<RequestCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RequestCommitments(decoded))
			}
			if let Ok(decoded) =
				<RequestReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RequestReceipts(decoded))
			}
			if let Ok(decoded) =
				<ResponseCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResponseCommitments(decoded))
			}
			if let Ok(decoded) =
				<ResponseReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResponseReceipts(decoded))
			}
			if let Ok(decoded) =
				<SetBridgeParamsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetBridgeParams(decoded))
			}
			if let Ok(decoded) =
				<SetConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetConsensusState(decoded))
			}
			if let Ok(decoded) =
				<SetFrozenStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetFrozenState(decoded))
			}
			if let Ok(decoded) =
				<StateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StateMachineCommitment(decoded))
			}
			if let Ok(decoded) =
				<StateMachineCommitmentUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::StateMachineCommitmentUpdateTime(decoded))
			}
			if let Ok(decoded) =
				<StoreConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreConsensusState(decoded))
			}
			if let Ok(decoded) =
				<StoreConsensusUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreConsensusUpdateTime(decoded))
			}
			if let Ok(decoded) =
				<StoreLatestStateMachineHeightCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreLatestStateMachineHeight(decoded))
			}
			if let Ok(decoded) =
				<StoreStateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreStateMachineCommitment(decoded))
			}
			if let Ok(decoded) = <StoreStateMachineCommitmentUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreStateMachineCommitmentUpdateTime(decoded));
            }
			if let Ok(decoded) = <TimestampCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Timestamp(decoded))
			}
			if let Ok(decoded) =
				<UnStakingPeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnStakingPeriod(decoded))
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for IIsmpHostCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::Admin(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChallengePeriod(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusClient(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Dispatch(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchWithResponse(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchWithRequest(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming0(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming1(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming2(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming3(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming4(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Frozen(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Host(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestStateMachineHeight(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RequestCommitments(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RequestReceipts(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResponseCommitments(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ResponseReceipts(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetBridgeParams(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetConsensusState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetFrozenState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StateMachineCommitment(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StateMachineCommitmentUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreConsensusState(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreConsensusUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreLatestStateMachineHeight(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreStateMachineCommitment(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreStateMachineCommitmentUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Timestamp(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnStakingPeriod(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for IIsmpHostCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Admin(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChallengePeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::ConsensusClient(element) => ::core::fmt::Display::fmt(element, f),
				Self::ConsensusState(element) => ::core::fmt::Display::fmt(element, f),
				Self::ConsensusUpdateTime(element) => ::core::fmt::Display::fmt(element, f),
				Self::Dispatch(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchWithResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchWithRequest(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming0(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming1(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming2(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming3(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming4(element) => ::core::fmt::Display::fmt(element, f),
				Self::Frozen(element) => ::core::fmt::Display::fmt(element, f),
				Self::Host(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestStateMachineHeight(element) => ::core::fmt::Display::fmt(element, f),
				Self::RequestCommitments(element) => ::core::fmt::Display::fmt(element, f),
				Self::RequestReceipts(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResponseCommitments(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResponseReceipts(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetBridgeParams(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetConsensusState(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetFrozenState(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateMachineCommitment(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateMachineCommitmentUpdateTime(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::StoreConsensusState(element) => ::core::fmt::Display::fmt(element, f),
				Self::StoreConsensusUpdateTime(element) => ::core::fmt::Display::fmt(element, f),
				Self::StoreLatestStateMachineHeight(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::StoreStateMachineCommitment(element) => ::core::fmt::Display::fmt(element, f),
				Self::StoreStateMachineCommitmentUpdateTime(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::Timestamp(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnStakingPeriod(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<AdminCall> for IIsmpHostCalls {
		fn from(value: AdminCall) -> Self {
			Self::Admin(value)
		}
	}
	impl ::core::convert::From<ChallengePeriodCall> for IIsmpHostCalls {
		fn from(value: ChallengePeriodCall) -> Self {
			Self::ChallengePeriod(value)
		}
	}
	impl ::core::convert::From<ConsensusClientCall> for IIsmpHostCalls {
		fn from(value: ConsensusClientCall) -> Self {
			Self::ConsensusClient(value)
		}
	}
	impl ::core::convert::From<ConsensusStateCall> for IIsmpHostCalls {
		fn from(value: ConsensusStateCall) -> Self {
			Self::ConsensusState(value)
		}
	}
	impl ::core::convert::From<ConsensusUpdateTimeCall> for IIsmpHostCalls {
		fn from(value: ConsensusUpdateTimeCall) -> Self {
			Self::ConsensusUpdateTime(value)
		}
	}
	impl ::core::convert::From<DispatchCall> for IIsmpHostCalls {
		fn from(value: DispatchCall) -> Self {
			Self::Dispatch(value)
		}
	}
	impl ::core::convert::From<DispatchWithResponseCall> for IIsmpHostCalls {
		fn from(value: DispatchWithResponseCall) -> Self {
			Self::DispatchWithResponse(value)
		}
	}
	impl ::core::convert::From<DispatchWithRequestCall> for IIsmpHostCalls {
		fn from(value: DispatchWithRequestCall) -> Self {
			Self::DispatchWithRequest(value)
		}
	}
	impl ::core::convert::From<DispatchIncoming0Call> for IIsmpHostCalls {
		fn from(value: DispatchIncoming0Call) -> Self {
			Self::DispatchIncoming0(value)
		}
	}
	impl ::core::convert::From<DispatchIncoming1Call> for IIsmpHostCalls {
		fn from(value: DispatchIncoming1Call) -> Self {
			Self::DispatchIncoming1(value)
		}
	}
	impl ::core::convert::From<DispatchIncoming2Call> for IIsmpHostCalls {
		fn from(value: DispatchIncoming2Call) -> Self {
			Self::DispatchIncoming2(value)
		}
	}
	impl ::core::convert::From<DispatchIncoming3Call> for IIsmpHostCalls {
		fn from(value: DispatchIncoming3Call) -> Self {
			Self::DispatchIncoming3(value)
		}
	}
	impl ::core::convert::From<DispatchIncoming4Call> for IIsmpHostCalls {
		fn from(value: DispatchIncoming4Call) -> Self {
			Self::DispatchIncoming4(value)
		}
	}
	impl ::core::convert::From<FrozenCall> for IIsmpHostCalls {
		fn from(value: FrozenCall) -> Self {
			Self::Frozen(value)
		}
	}
	impl ::core::convert::From<HostCall> for IIsmpHostCalls {
		fn from(value: HostCall) -> Self {
			Self::Host(value)
		}
	}
	impl ::core::convert::From<LatestStateMachineHeightCall> for IIsmpHostCalls {
		fn from(value: LatestStateMachineHeightCall) -> Self {
			Self::LatestStateMachineHeight(value)
		}
	}
	impl ::core::convert::From<RequestCommitmentsCall> for IIsmpHostCalls {
		fn from(value: RequestCommitmentsCall) -> Self {
			Self::RequestCommitments(value)
		}
	}
	impl ::core::convert::From<RequestReceiptsCall> for IIsmpHostCalls {
		fn from(value: RequestReceiptsCall) -> Self {
			Self::RequestReceipts(value)
		}
	}
	impl ::core::convert::From<ResponseCommitmentsCall> for IIsmpHostCalls {
		fn from(value: ResponseCommitmentsCall) -> Self {
			Self::ResponseCommitments(value)
		}
	}
	impl ::core::convert::From<ResponseReceiptsCall> for IIsmpHostCalls {
		fn from(value: ResponseReceiptsCall) -> Self {
			Self::ResponseReceipts(value)
		}
	}
	impl ::core::convert::From<SetBridgeParamsCall> for IIsmpHostCalls {
		fn from(value: SetBridgeParamsCall) -> Self {
			Self::SetBridgeParams(value)
		}
	}
	impl ::core::convert::From<SetConsensusStateCall> for IIsmpHostCalls {
		fn from(value: SetConsensusStateCall) -> Self {
			Self::SetConsensusState(value)
		}
	}
	impl ::core::convert::From<SetFrozenStateCall> for IIsmpHostCalls {
		fn from(value: SetFrozenStateCall) -> Self {
			Self::SetFrozenState(value)
		}
	}
	impl ::core::convert::From<StateMachineCommitmentCall> for IIsmpHostCalls {
		fn from(value: StateMachineCommitmentCall) -> Self {
			Self::StateMachineCommitment(value)
		}
	}
	impl ::core::convert::From<StateMachineCommitmentUpdateTimeCall> for IIsmpHostCalls {
		fn from(value: StateMachineCommitmentUpdateTimeCall) -> Self {
			Self::StateMachineCommitmentUpdateTime(value)
		}
	}
	impl ::core::convert::From<StoreConsensusStateCall> for IIsmpHostCalls {
		fn from(value: StoreConsensusStateCall) -> Self {
			Self::StoreConsensusState(value)
		}
	}
	impl ::core::convert::From<StoreConsensusUpdateTimeCall> for IIsmpHostCalls {
		fn from(value: StoreConsensusUpdateTimeCall) -> Self {
			Self::StoreConsensusUpdateTime(value)
		}
	}
	impl ::core::convert::From<StoreLatestStateMachineHeightCall> for IIsmpHostCalls {
		fn from(value: StoreLatestStateMachineHeightCall) -> Self {
			Self::StoreLatestStateMachineHeight(value)
		}
	}
	impl ::core::convert::From<StoreStateMachineCommitmentCall> for IIsmpHostCalls {
		fn from(value: StoreStateMachineCommitmentCall) -> Self {
			Self::StoreStateMachineCommitment(value)
		}
	}
	impl ::core::convert::From<StoreStateMachineCommitmentUpdateTimeCall> for IIsmpHostCalls {
		fn from(value: StoreStateMachineCommitmentUpdateTimeCall) -> Self {
			Self::StoreStateMachineCommitmentUpdateTime(value)
		}
	}
	impl ::core::convert::From<TimestampCall> for IIsmpHostCalls {
		fn from(value: TimestampCall) -> Self {
			Self::Timestamp(value)
		}
	}
	impl ::core::convert::From<UnStakingPeriodCall> for IIsmpHostCalls {
		fn from(value: UnStakingPeriodCall) -> Self {
			Self::UnStakingPeriod(value)
		}
	}
	///Container type for all return fields from the `admin` function with signature `admin()` and
	/// selector `0xf851a440`
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
	pub struct AdminReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `challengePeriod` function with signature
	/// `challengePeriod()` and selector `0xf3f480d9`
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
	pub struct ChallengePeriodReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `consensusClient` function with signature
	/// `consensusClient()` and selector `0x2476132b`
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
	pub struct ConsensusClientReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `consensusState` function with signature
	/// `consensusState()` and selector `0xbbad99d4`
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
	pub struct ConsensusStateReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `consensusUpdateTime` function with signature
	/// `consensusUpdateTime()` and selector `0x9a8425bc`
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
	pub struct ConsensusUpdateTimeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `frozen` function with signature `frozen()`
	/// and selector `0x054f7d9c`
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
	pub struct FrozenReturn(pub bool);
	///Container type for all return fields from the `host` function with signature `host()` and
	/// selector `0xf437bc59`
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
	pub struct HostReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `latestStateMachineHeight` function with
	/// signature `latestStateMachineHeight()` and selector `0x56b65597`
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
	pub struct LatestStateMachineHeightReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `requestCommitments` function with signature
	/// `requestCommitments(bytes32)` and selector `0x368bf464`
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
	pub struct RequestCommitmentsReturn(pub bool);
	///Container type for all return fields from the `requestReceipts` function with signature
	/// `requestReceipts(bytes32)` and selector `0x19667a3e`
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
	pub struct RequestReceiptsReturn(pub bool);
	///Container type for all return fields from the `responseCommitments` function with signature
	/// `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
	pub struct ResponseCommitmentsReturn(pub bool);
	///Container type for all return fields from the `responseReceipts` function with signature
	/// `responseReceipts(bytes32)` and selector `0x8856337e`
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
	pub struct ResponseReceiptsReturn(pub bool);
	///Container type for all return fields from the `stateMachineCommitment` function with
	/// signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
	pub struct StateMachineCommitmentReturn(pub StateCommitment);
	///Container type for all return fields from the `stateMachineCommitmentUpdateTime` function
	/// with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector
	/// `0x1a880a93`
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
	pub struct StateMachineCommitmentUpdateTimeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `timestamp` function with signature
	/// `timestamp()` and selector `0xb80777ea`
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
	pub struct TimestampReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `unStakingPeriod` function with signature
	/// `unStakingPeriod()` and selector `0xd40784c7`
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
	pub struct UnStakingPeriodReturn(pub ::ethers::core::types::U256);
	///`BridgeParams(address,address,address,uint256,uint256,uint256)`
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
	pub struct BridgeParams {
		pub admin: ::ethers::core::types::Address,
		pub consensus: ::ethers::core::types::Address,
		pub handler: ::ethers::core::types::Address,
		pub challenge_period: ::ethers::core::types::U256,
		pub unstaking_period: ::ethers::core::types::U256,
		pub default_timeout: ::ethers::core::types::U256,
	}
	///`DispatchGet(bytes,uint64,bytes[],uint64,uint64)`
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
	pub struct DispatchGet {
		pub dest: ::ethers::core::types::Bytes,
		pub height: u64,
		pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
		pub timeout: u64,
		pub gaslimit: u64,
	}
	///`DispatchPost(bytes,bytes,bytes,uint64,uint64)`
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
	pub struct DispatchPost {
		pub dest: ::ethers::core::types::Bytes,
		pub to: ::ethers::core::types::Bytes,
		pub body: ::ethers::core::types::Bytes,
		pub timeout: u64,
		pub gaslimit: u64,
	}
	///`PostTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))`
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
	pub struct PostTimeout {
		pub request: PostRequest,
	}
}
