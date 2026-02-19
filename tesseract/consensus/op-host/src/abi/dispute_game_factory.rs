pub use dispute_game_factory::*;
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
pub mod dispute_game_factory {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::None,
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("__constructor__"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("__constructor__"),
						inputs: ::std::vec![],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("create"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("create"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_rootClaim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_extraData"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("proxy_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("findLatestGames"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("findLatestGames"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_start"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_n"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("games_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Array(
								::std::boxed::Box::new(
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Uint(256usize),
										::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
									],),
								),
							),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned(
									"struct IDisputeGameFactory.GameSearchResult[]",
								),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gameArgs"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameArgs"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("GameType"),
							),
						},],
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
					::std::borrow::ToOwned::to_owned("gameAtIndex"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameAtIndex"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_index"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("gameType_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("timestamp_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Timestamp"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("proxy_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gameCount"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameCount"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("gameCount_"),
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
					::std::borrow::ToOwned::to_owned("gameImpls"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameImpls"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("GameType"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("games"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("games"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_rootClaim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_extraData"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("proxy_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("timestamp_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Timestamp"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getGameUUID"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getGameUUID"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_rootClaim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_extraData"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("uuid_"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Hash"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("initBonds"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("initBonds"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("GameType"),
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
					::std::borrow::ToOwned::to_owned("initVersion"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("initVersion"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint8"),
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
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_owner"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("owner"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("owner"),
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
					::std::borrow::ToOwned::to_owned("proxyAdmin"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("proxyAdmin"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IProxyAdmin"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("proxyAdminOwner"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("proxyAdminOwner"),
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
					::std::borrow::ToOwned::to_owned("renounceOwnership"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("renounceOwnership"),
						inputs: ::std::vec![],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("setImplementation"),
					::std::vec![
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("setImplementation"),
							inputs: ::std::vec![
								::ethers::core::abi::ethabi::Param {
									name: ::std::borrow::ToOwned::to_owned("_gameType"),
									kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
									internal_type: ::core::option::Option::Some(
										::std::borrow::ToOwned::to_owned("GameType"),
									),
								},
								::ethers::core::abi::ethabi::Param {
									name: ::std::borrow::ToOwned::to_owned("_impl"),
									kind: ::ethers::core::abi::ethabi::ParamType::Address,
									internal_type: ::core::option::Option::Some(
										::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
									),
								},
							],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("setImplementation"),
							inputs: ::std::vec![
								::ethers::core::abi::ethabi::Param {
									name: ::std::borrow::ToOwned::to_owned("_gameType"),
									kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
									internal_type: ::core::option::Option::Some(
										::std::borrow::ToOwned::to_owned("GameType"),
									),
								},
								::ethers::core::abi::ethabi::Param {
									name: ::std::borrow::ToOwned::to_owned("_impl"),
									kind: ::ethers::core::abi::ethabi::ParamType::Address,
									internal_type: ::core::option::Option::Some(
										::std::borrow::ToOwned::to_owned("contract IDisputeGame"),
									),
								},
								::ethers::core::abi::ethabi::Param {
									name: ::std::borrow::ToOwned::to_owned("_args"),
									kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
									internal_type: ::core::option::Option::Some(
										::std::borrow::ToOwned::to_owned("bytes"),
									),
								},
							],
							outputs: ::std::vec![],
							constant: ::core::option::Option::None,
							state_mutability:
								::ethers::core::abi::ethabi::StateMutability::NonPayable,
						},
					],
				),
				(
					::std::borrow::ToOwned::to_owned("setInitBond"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setInitBond"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_initBond"),
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
					::std::borrow::ToOwned::to_owned("transferOwnership"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("transferOwnership"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("newOwner"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
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
					::std::borrow::ToOwned::to_owned("DisputeGameCreated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("DisputeGameCreated"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("disputeProxy"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("rootClaim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ImplementationArgsSet"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("ImplementationArgsSet",),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("args"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								indexed: false,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ImplementationSet"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("ImplementationSet"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("impl"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InitBondUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("InitBondUpdated"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("gameType"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("newBond"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
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
					::std::borrow::ToOwned::to_owned("OwnershipTransferred"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("OwnershipTransferred",),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("previousOwner"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("newOwner"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
			]),
			errors: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("GameAlreadyExists"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("GameAlreadyExists"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("uuid"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Hash"),
							),
						},],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("IncorrectBondAmount"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("IncorrectBondAmount",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("NoImplementation"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("NoImplementation"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("gameType"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("GameType"),
							),
						},],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ProxyAdminOwnedBase_NotProxyAdmin"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ProxyAdminOwnedBase_NotProxyAdmin",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned(
						"ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner",
					),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner",
						),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ProxyAdminOwnedBase_NotProxyAdminOwner"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ProxyAdminOwnedBase_NotProxyAdminOwner",
						),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned(
						"ProxyAdminOwnedBase_NotResolvedDelegateProxy",
					),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ProxyAdminOwnedBase_NotResolvedDelegateProxy",
						),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned(
						"ProxyAdminOwnedBase_NotSharedProxyAdminOwner",
					),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ProxyAdminOwnedBase_NotSharedProxyAdminOwner",
						),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ProxyAdminOwnedBase_ProxyAdminNotFound"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ProxyAdminOwnedBase_ProxyAdminNotFound",
						),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ReinitializableBase_ZeroInitVersion"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned(
							"ReinitializableBase_ZeroInitVersion",
						),
						inputs: ::std::vec![],
					},],
				),
			]),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static DISPUTEGAMEFACTORY_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct DisputeGameFactory<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for DisputeGameFactory<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for DisputeGameFactory<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for DisputeGameFactory<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for DisputeGameFactory<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(DisputeGameFactory))
				.field(&self.address())
				.finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> DisputeGameFactory<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(
				address.into(),
				DISPUTEGAMEFACTORY_ABI.clone(),
				client,
			))
		}
		///Calls the contract's `__constructor__` (0x1c0082a3) function
		pub fn constructor(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([28, 0, 130, 163], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `create` (0x82ecf2f6) function
		pub fn create(
			&self,
			game_type: u32,
			root_claim: [u8; 32],
			extra_data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([130, 236, 242, 246], (game_type, root_claim, extra_data))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `findLatestGames` (0x254bd683) function
		pub fn find_latest_games(
			&self,
			game_type: u32,
			start: ::ethers::core::types::U256,
			n: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::std::vec::Vec<GameSearchResult>> {
			self.0
				.method_hash([37, 75, 214, 131], (game_type, start, n))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameArgs` (0x74cc86ac) function
		pub fn game_args(
			&self,
			p0: u32,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([116, 204, 134, 172], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameAtIndex` (0xbb8aa1fc) function
		pub fn game_at_index(
			&self,
			index: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, (u32, u64, ::ethers::core::types::Address)>
		{
			self.0
				.method_hash([187, 138, 161, 252], index)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameCount` (0x4d1975b4) function
		pub fn game_count(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([77, 25, 117, 180], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameImpls` (0x1b685b9e) function
		pub fn game_impls(
			&self,
			p0: u32,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([27, 104, 91, 158], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `games` (0x5f0150cb) function
		pub fn games(
			&self,
			game_type: u32,
			root_claim: [u8; 32],
			extra_data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, (::ethers::core::types::Address, u64)>
		{
			self.0
				.method_hash([95, 1, 80, 203], (game_type, root_claim, extra_data))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getGameUUID` (0x96cd9720) function
		pub fn get_game_uuid(
			&self,
			game_type: u32,
			root_claim: [u8; 32],
			extra_data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([150, 205, 151, 32], (game_type, root_claim, extra_data))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `initBonds` (0x6593dc6e) function
		pub fn init_bonds(
			&self,
			p0: u32,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([101, 147, 220, 110], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `initVersion` (0x38d38c97) function
		pub fn init_version(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
			self.0
				.method_hash([56, 211, 140, 151], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `initialize` (0xc4d66de8) function
		pub fn initialize(
			&self,
			owner: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([196, 214, 109, 232], owner)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `owner` (0x8da5cb5b) function
		pub fn owner(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([141, 165, 203, 91], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `proxyAdmin` (0x3e47158c) function
		pub fn proxy_admin(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([62, 71, 21, 140], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `proxyAdminOwner` (0xdad544e0) function
		pub fn proxy_admin_owner(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([218, 213, 68, 224], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `renounceOwnership` (0x715018a6) function
		pub fn renounce_ownership(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([113, 80, 24, 166], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setImplementation` (0x14f6b1a3) function
		pub fn set_implementation(
			&self,
			game_type: u32,
			impl_: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([20, 246, 177, 163], (game_type, impl_))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setImplementation` (0xb1070957) function
		pub fn set_implementation_with_game_type_and_impl(
			&self,
			game_type: u32,
			impl_: ::ethers::core::types::Address,
			args: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([177, 7, 9, 87], (game_type, impl_, args))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setInitBond` (0x1e334240) function
		pub fn set_init_bond(
			&self,
			game_type: u32,
			init_bond: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([30, 51, 66, 64], (game_type, init_bond))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `transferOwnership` (0xf2fde38b) function
		pub fn transfer_ownership(
			&self,
			new_owner: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([242, 253, 227, 139], new_owner)
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
		///Gets the contract's `DisputeGameCreated` event
		pub fn dispute_game_created_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, DisputeGameCreatedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `ImplementationArgsSet` event
		pub fn implementation_args_set_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ImplementationArgsSetFilter>
		{
			self.0.event()
		}
		///Gets the contract's `ImplementationSet` event
		pub fn implementation_set_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ImplementationSetFilter>
		{
			self.0.event()
		}
		///Gets the contract's `InitBondUpdated` event
		pub fn init_bond_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, InitBondUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `Initialized` event
		pub fn initialized_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, InitializedFilter> {
			self.0.event()
		}
		///Gets the contract's `OwnershipTransferred` event
		pub fn ownership_transferred_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OwnershipTransferredFilter>
		{
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, DisputeGameFactoryEvents>
		{
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
		for DisputeGameFactory<M>
	{
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Custom Error type `GameAlreadyExists` with signature `GameAlreadyExists(bytes32)` and
	/// selector `0x014f6fe5`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(name = "GameAlreadyExists", abi = "GameAlreadyExists(bytes32)")]
	pub struct GameAlreadyExists {
		pub uuid: [u8; 32],
	}
	///Custom Error type `IncorrectBondAmount` with signature `IncorrectBondAmount()` and selector
	/// `0x8620aa19`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(name = "IncorrectBondAmount", abi = "IncorrectBondAmount()")]
	pub struct IncorrectBondAmount;
	///Custom Error type `NoImplementation` with signature `NoImplementation(uint32)` and selector
	/// `0x031c6de4`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(name = "NoImplementation", abi = "NoImplementation(uint32)")]
	pub struct NoImplementation {
		pub game_type: u32,
	}
	///Custom Error type `ProxyAdminOwnedBase_NotProxyAdmin` with signature
	/// `ProxyAdminOwnedBase_NotProxyAdmin()` and selector `0xe818dcc3`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_NotProxyAdmin",
		abi = "ProxyAdminOwnedBase_NotProxyAdmin()"
	)]
	pub struct ProxyAdminOwnedBase_NotProxyAdmin;
	///Custom Error type `ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner` with signature
	/// `ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner()` and selector `0xc4050a26`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner",
		abi = "ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner()"
	)]
	pub struct ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner;
	///Custom Error type `ProxyAdminOwnedBase_NotProxyAdminOwner` with signature
	/// `ProxyAdminOwnedBase_NotProxyAdminOwner()` and selector `0x7f12c64b`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_NotProxyAdminOwner",
		abi = "ProxyAdminOwnedBase_NotProxyAdminOwner()"
	)]
	pub struct ProxyAdminOwnedBase_NotProxyAdminOwner;
	///Custom Error type `ProxyAdminOwnedBase_NotResolvedDelegateProxy` with signature
	/// `ProxyAdminOwnedBase_NotResolvedDelegateProxy()` and selector `0x54e433cd`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_NotResolvedDelegateProxy",
		abi = "ProxyAdminOwnedBase_NotResolvedDelegateProxy()"
	)]
	pub struct ProxyAdminOwnedBase_NotResolvedDelegateProxy;
	///Custom Error type `ProxyAdminOwnedBase_NotSharedProxyAdminOwner` with signature
	/// `ProxyAdminOwnedBase_NotSharedProxyAdminOwner()` and selector `0x075c4314`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_NotSharedProxyAdminOwner",
		abi = "ProxyAdminOwnedBase_NotSharedProxyAdminOwner()"
	)]
	pub struct ProxyAdminOwnedBase_NotSharedProxyAdminOwner;
	///Custom Error type `ProxyAdminOwnedBase_ProxyAdminNotFound` with signature
	/// `ProxyAdminOwnedBase_ProxyAdminNotFound()` and selector `0x332144db`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ProxyAdminOwnedBase_ProxyAdminNotFound",
		abi = "ProxyAdminOwnedBase_ProxyAdminNotFound()"
	)]
	pub struct ProxyAdminOwnedBase_ProxyAdminNotFound;
	///Custom Error type `ReinitializableBase_ZeroInitVersion` with signature
	/// `ReinitializableBase_ZeroInitVersion()` and selector `0x9b01afed`
	#[derive(
		Clone,
		::ethers::contract::EthError,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[etherror(
		name = "ReinitializableBase_ZeroInitVersion",
		abi = "ReinitializableBase_ZeroInitVersion()"
	)]
	pub struct ReinitializableBase_ZeroInitVersion;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum DisputeGameFactoryErrors {
		GameAlreadyExists(GameAlreadyExists),
		IncorrectBondAmount(IncorrectBondAmount),
		NoImplementation(NoImplementation),
		ProxyAdminOwnedBase_NotProxyAdmin(ProxyAdminOwnedBase_NotProxyAdmin),
		ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner(
			ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner,
		),
		ProxyAdminOwnedBase_NotProxyAdminOwner(ProxyAdminOwnedBase_NotProxyAdminOwner),
		ProxyAdminOwnedBase_NotResolvedDelegateProxy(ProxyAdminOwnedBase_NotResolvedDelegateProxy),
		ProxyAdminOwnedBase_NotSharedProxyAdminOwner(ProxyAdminOwnedBase_NotSharedProxyAdminOwner),
		ProxyAdminOwnedBase_ProxyAdminNotFound(ProxyAdminOwnedBase_ProxyAdminNotFound),
		ReinitializableBase_ZeroInitVersion(ReinitializableBase_ZeroInitVersion),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for DisputeGameFactoryErrors {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) =
				<::std::string::String as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RevertString(decoded));
			}
			if let Ok(decoded) = <GameAlreadyExists as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GameAlreadyExists(decoded));
			}
			if let Ok(decoded) =
				<IncorrectBondAmount as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::IncorrectBondAmount(decoded));
			}
			if let Ok(decoded) = <NoImplementation as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::NoImplementation(decoded));
			}
			if let Ok(decoded) =
				<ProxyAdminOwnedBase_NotProxyAdmin as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ProxyAdminOwnedBase_NotProxyAdmin(decoded));
			}
			if let Ok(decoded) = <ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(
                    Self::ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner(decoded),
                );
            }
			if let Ok(decoded) =
				<ProxyAdminOwnedBase_NotProxyAdminOwner as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::ProxyAdminOwnedBase_NotProxyAdminOwner(decoded));
			}
			if let Ok(decoded) = <ProxyAdminOwnedBase_NotResolvedDelegateProxy as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ProxyAdminOwnedBase_NotResolvedDelegateProxy(decoded));
            }
			if let Ok(decoded) = <ProxyAdminOwnedBase_NotSharedProxyAdminOwner as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ProxyAdminOwnedBase_NotSharedProxyAdminOwner(decoded));
            }
			if let Ok(decoded) =
				<ProxyAdminOwnedBase_ProxyAdminNotFound as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::ProxyAdminOwnedBase_ProxyAdminNotFound(decoded));
			}
			if let Ok(decoded) =
				<ReinitializableBase_ZeroInitVersion as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::ReinitializableBase_ZeroInitVersion(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for DisputeGameFactoryErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::GameAlreadyExists(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IncorrectBondAmount(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::NoImplementation(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_NotProxyAdmin(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_NotProxyAdminOwner(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_NotResolvedDelegateProxy(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_NotSharedProxyAdminOwner(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwnedBase_ProxyAdminNotFound(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ReinitializableBase_ZeroInitVersion(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for DisputeGameFactoryErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <GameAlreadyExists as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <IncorrectBondAmount as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NoImplementation as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_NotProxyAdmin as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_NotProxyAdminOwner as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_NotResolvedDelegateProxy as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_NotSharedProxyAdminOwner as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ProxyAdminOwnedBase_ProxyAdminNotFound as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ReinitializableBase_ZeroInitVersion as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
		}
	}
	impl ::core::fmt::Display for DisputeGameFactoryErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::GameAlreadyExists(element) => ::core::fmt::Display::fmt(element, f),
				Self::IncorrectBondAmount(element) => ::core::fmt::Display::fmt(element, f),
				Self::NoImplementation(element) => ::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_NotProxyAdmin(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_NotProxyAdminOwner(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_NotResolvedDelegateProxy(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_NotSharedProxyAdminOwner(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwnedBase_ProxyAdminNotFound(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::ReinitializableBase_ZeroInitVersion(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for DisputeGameFactoryErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<GameAlreadyExists> for DisputeGameFactoryErrors {
		fn from(value: GameAlreadyExists) -> Self {
			Self::GameAlreadyExists(value)
		}
	}
	impl ::core::convert::From<IncorrectBondAmount> for DisputeGameFactoryErrors {
		fn from(value: IncorrectBondAmount) -> Self {
			Self::IncorrectBondAmount(value)
		}
	}
	impl ::core::convert::From<NoImplementation> for DisputeGameFactoryErrors {
		fn from(value: NoImplementation) -> Self {
			Self::NoImplementation(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_NotProxyAdmin> for DisputeGameFactoryErrors {
		fn from(value: ProxyAdminOwnedBase_NotProxyAdmin) -> Self {
			Self::ProxyAdminOwnedBase_NotProxyAdmin(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner>
		for DisputeGameFactoryErrors
	{
		fn from(value: ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner) -> Self {
			Self::ProxyAdminOwnedBase_NotProxyAdminOrProxyAdminOwner(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_NotProxyAdminOwner> for DisputeGameFactoryErrors {
		fn from(value: ProxyAdminOwnedBase_NotProxyAdminOwner) -> Self {
			Self::ProxyAdminOwnedBase_NotProxyAdminOwner(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_NotResolvedDelegateProxy>
		for DisputeGameFactoryErrors
	{
		fn from(value: ProxyAdminOwnedBase_NotResolvedDelegateProxy) -> Self {
			Self::ProxyAdminOwnedBase_NotResolvedDelegateProxy(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_NotSharedProxyAdminOwner>
		for DisputeGameFactoryErrors
	{
		fn from(value: ProxyAdminOwnedBase_NotSharedProxyAdminOwner) -> Self {
			Self::ProxyAdminOwnedBase_NotSharedProxyAdminOwner(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnedBase_ProxyAdminNotFound> for DisputeGameFactoryErrors {
		fn from(value: ProxyAdminOwnedBase_ProxyAdminNotFound) -> Self {
			Self::ProxyAdminOwnedBase_ProxyAdminNotFound(value)
		}
	}
	impl ::core::convert::From<ReinitializableBase_ZeroInitVersion> for DisputeGameFactoryErrors {
		fn from(value: ReinitializableBase_ZeroInitVersion) -> Self {
			Self::ReinitializableBase_ZeroInitVersion(value)
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
	#[ethevent(name = "DisputeGameCreated", abi = "DisputeGameCreated(address,uint32,bytes32)")]
	pub struct DisputeGameCreatedFilter {
		#[ethevent(indexed)]
		pub dispute_proxy: ::ethers::core::types::Address,
		#[ethevent(indexed)]
		pub game_type: u32,
		#[ethevent(indexed)]
		pub root_claim: [u8; 32],
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
	#[ethevent(name = "ImplementationArgsSet", abi = "ImplementationArgsSet(uint32,bytes)")]
	pub struct ImplementationArgsSetFilter {
		#[ethevent(indexed)]
		pub game_type: u32,
		pub args: ::ethers::core::types::Bytes,
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
	#[ethevent(name = "ImplementationSet", abi = "ImplementationSet(address,uint32)")]
	pub struct ImplementationSetFilter {
		#[ethevent(indexed)]
		pub impl_: ::ethers::core::types::Address,
		#[ethevent(indexed)]
		pub game_type: u32,
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
	#[ethevent(name = "InitBondUpdated", abi = "InitBondUpdated(uint32,uint256)")]
	pub struct InitBondUpdatedFilter {
		#[ethevent(indexed)]
		pub game_type: u32,
		#[ethevent(indexed)]
		pub new_bond: ::ethers::core::types::U256,
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
	#[ethevent(name = "OwnershipTransferred", abi = "OwnershipTransferred(address,address)")]
	pub struct OwnershipTransferredFilter {
		#[ethevent(indexed)]
		pub previous_owner: ::ethers::core::types::Address,
		#[ethevent(indexed)]
		pub new_owner: ::ethers::core::types::Address,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum DisputeGameFactoryEvents {
		DisputeGameCreatedFilter(DisputeGameCreatedFilter),
		ImplementationArgsSetFilter(ImplementationArgsSetFilter),
		ImplementationSetFilter(ImplementationSetFilter),
		InitBondUpdatedFilter(InitBondUpdatedFilter),
		InitializedFilter(InitializedFilter),
		OwnershipTransferredFilter(OwnershipTransferredFilter),
	}
	impl ::ethers::contract::EthLogDecode for DisputeGameFactoryEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = DisputeGameCreatedFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::DisputeGameCreatedFilter(decoded));
			}
			if let Ok(decoded) = ImplementationArgsSetFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::ImplementationArgsSetFilter(decoded));
			}
			if let Ok(decoded) = ImplementationSetFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::ImplementationSetFilter(decoded));
			}
			if let Ok(decoded) = InitBondUpdatedFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::InitBondUpdatedFilter(decoded));
			}
			if let Ok(decoded) = InitializedFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::InitializedFilter(decoded));
			}
			if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
				return Ok(DisputeGameFactoryEvents::OwnershipTransferredFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for DisputeGameFactoryEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::DisputeGameCreatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::ImplementationArgsSetFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::ImplementationSetFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::InitBondUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::InitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::OwnershipTransferredFilter(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<DisputeGameCreatedFilter> for DisputeGameFactoryEvents {
		fn from(value: DisputeGameCreatedFilter) -> Self {
			Self::DisputeGameCreatedFilter(value)
		}
	}
	impl ::core::convert::From<ImplementationArgsSetFilter> for DisputeGameFactoryEvents {
		fn from(value: ImplementationArgsSetFilter) -> Self {
			Self::ImplementationArgsSetFilter(value)
		}
	}
	impl ::core::convert::From<ImplementationSetFilter> for DisputeGameFactoryEvents {
		fn from(value: ImplementationSetFilter) -> Self {
			Self::ImplementationSetFilter(value)
		}
	}
	impl ::core::convert::From<InitBondUpdatedFilter> for DisputeGameFactoryEvents {
		fn from(value: InitBondUpdatedFilter) -> Self {
			Self::InitBondUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<InitializedFilter> for DisputeGameFactoryEvents {
		fn from(value: InitializedFilter) -> Self {
			Self::InitializedFilter(value)
		}
	}
	impl ::core::convert::From<OwnershipTransferredFilter> for DisputeGameFactoryEvents {
		fn from(value: OwnershipTransferredFilter) -> Self {
			Self::OwnershipTransferredFilter(value)
		}
	}
	///Container type for all input parameters for the `__constructor__` function with signature
	/// `__constructor__()` and selector `0x1c0082a3`
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
	#[ethcall(name = "__constructor__", abi = "__constructor__()")]
	pub struct ConstructorCall;
	///Container type for all input parameters for the `create` function with signature
	/// `create(uint32,bytes32,bytes)` and selector `0x82ecf2f6`
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
	#[ethcall(name = "create", abi = "create(uint32,bytes32,bytes)")]
	pub struct CreateCall {
		pub game_type: u32,
		pub root_claim: [u8; 32],
		pub extra_data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `findLatestGames` function with signature
	/// `findLatestGames(uint32,uint256,uint256)` and selector `0x254bd683`
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
	#[ethcall(name = "findLatestGames", abi = "findLatestGames(uint32,uint256,uint256)")]
	pub struct FindLatestGamesCall {
		pub game_type: u32,
		pub start: ::ethers::core::types::U256,
		pub n: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `gameArgs` function with signature
	/// `gameArgs(uint32)` and selector `0x74cc86ac`
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
	#[ethcall(name = "gameArgs", abi = "gameArgs(uint32)")]
	pub struct GameArgsCall(pub u32);
	///Container type for all input parameters for the `gameAtIndex` function with signature
	/// `gameAtIndex(uint256)` and selector `0xbb8aa1fc`
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
	#[ethcall(name = "gameAtIndex", abi = "gameAtIndex(uint256)")]
	pub struct GameAtIndexCall {
		pub index: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `gameCount` function with signature
	/// `gameCount()` and selector `0x4d1975b4`
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
	#[ethcall(name = "gameCount", abi = "gameCount()")]
	pub struct GameCountCall;
	///Container type for all input parameters for the `gameImpls` function with signature
	/// `gameImpls(uint32)` and selector `0x1b685b9e`
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
	#[ethcall(name = "gameImpls", abi = "gameImpls(uint32)")]
	pub struct GameImplsCall(pub u32);
	///Container type for all input parameters for the `games` function with signature
	/// `games(uint32,bytes32,bytes)` and selector `0x5f0150cb`
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
	#[ethcall(name = "games", abi = "games(uint32,bytes32,bytes)")]
	pub struct GamesCall {
		pub game_type: u32,
		pub root_claim: [u8; 32],
		pub extra_data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `getGameUUID` function with signature
	/// `getGameUUID(uint32,bytes32,bytes)` and selector `0x96cd9720`
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
	#[ethcall(name = "getGameUUID", abi = "getGameUUID(uint32,bytes32,bytes)")]
	pub struct GetGameUUIDCall {
		pub game_type: u32,
		pub root_claim: [u8; 32],
		pub extra_data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `initBonds` function with signature
	/// `initBonds(uint32)` and selector `0x6593dc6e`
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
	#[ethcall(name = "initBonds", abi = "initBonds(uint32)")]
	pub struct InitBondsCall(pub u32);
	///Container type for all input parameters for the `initVersion` function with signature
	/// `initVersion()` and selector `0x38d38c97`
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
	#[ethcall(name = "initVersion", abi = "initVersion()")]
	pub struct InitVersionCall;
	///Container type for all input parameters for the `initialize` function with signature
	/// `initialize(address)` and selector `0xc4d66de8`
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
	#[ethcall(name = "initialize", abi = "initialize(address)")]
	pub struct InitializeCall {
		pub owner: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `owner` function with signature `owner()`
	/// and selector `0x8da5cb5b`
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
	#[ethcall(name = "owner", abi = "owner()")]
	pub struct OwnerCall;
	///Container type for all input parameters for the `proxyAdmin` function with signature
	/// `proxyAdmin()` and selector `0x3e47158c`
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
	#[ethcall(name = "proxyAdmin", abi = "proxyAdmin()")]
	pub struct ProxyAdminCall;
	///Container type for all input parameters for the `proxyAdminOwner` function with signature
	/// `proxyAdminOwner()` and selector `0xdad544e0`
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
	#[ethcall(name = "proxyAdminOwner", abi = "proxyAdminOwner()")]
	pub struct ProxyAdminOwnerCall;
	///Container type for all input parameters for the `renounceOwnership` function with signature
	/// `renounceOwnership()` and selector `0x715018a6`
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
	#[ethcall(name = "renounceOwnership", abi = "renounceOwnership()")]
	pub struct RenounceOwnershipCall;
	///Container type for all input parameters for the `setImplementation` function with signature
	/// `setImplementation(uint32,address)` and selector `0x14f6b1a3`
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
	#[ethcall(name = "setImplementation", abi = "setImplementation(uint32,address)")]
	pub struct SetImplementationCall {
		pub game_type: u32,
		pub impl_: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `setImplementation` function with signature
	/// `setImplementation(uint32,address,bytes)` and selector `0xb1070957`
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
	#[ethcall(name = "setImplementation", abi = "setImplementation(uint32,address,bytes)")]
	pub struct SetImplementationWithGameTypeAndImplCall {
		pub game_type: u32,
		pub impl_: ::ethers::core::types::Address,
		pub args: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `setInitBond` function with signature
	/// `setInitBond(uint32,uint256)` and selector `0x1e334240`
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
	#[ethcall(name = "setInitBond", abi = "setInitBond(uint32,uint256)")]
	pub struct SetInitBondCall {
		pub game_type: u32,
		pub init_bond: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `transferOwnership` function with signature
	/// `transferOwnership(address)` and selector `0xf2fde38b`
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
	#[ethcall(name = "transferOwnership", abi = "transferOwnership(address)")]
	pub struct TransferOwnershipCall {
		pub new_owner: ::ethers::core::types::Address,
	}
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
	pub enum DisputeGameFactoryCalls {
		Constructor(ConstructorCall),
		Create(CreateCall),
		FindLatestGames(FindLatestGamesCall),
		GameArgs(GameArgsCall),
		GameAtIndex(GameAtIndexCall),
		GameCount(GameCountCall),
		GameImpls(GameImplsCall),
		Games(GamesCall),
		GetGameUUID(GetGameUUIDCall),
		InitBonds(InitBondsCall),
		InitVersion(InitVersionCall),
		Initialize(InitializeCall),
		Owner(OwnerCall),
		ProxyAdmin(ProxyAdminCall),
		ProxyAdminOwner(ProxyAdminOwnerCall),
		RenounceOwnership(RenounceOwnershipCall),
		SetImplementation(SetImplementationCall),
		SetImplementationWithGameTypeAndImpl(SetImplementationWithGameTypeAndImplCall),
		SetInitBond(SetInitBondCall),
		TransferOwnership(TransferOwnershipCall),
		Version(VersionCall),
	}
	impl ::ethers::core::abi::AbiDecode for DisputeGameFactoryCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <ConstructorCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Constructor(decoded));
			}
			if let Ok(decoded) = <CreateCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Create(decoded));
			}
			if let Ok(decoded) =
				<FindLatestGamesCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::FindLatestGames(decoded));
			}
			if let Ok(decoded) = <GameArgsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameArgs(decoded));
			}
			if let Ok(decoded) = <GameAtIndexCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameAtIndex(decoded));
			}
			if let Ok(decoded) = <GameCountCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameCount(decoded));
			}
			if let Ok(decoded) = <GameImplsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameImpls(decoded));
			}
			if let Ok(decoded) = <GamesCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Games(decoded));
			}
			if let Ok(decoded) = <GetGameUUIDCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetGameUUID(decoded));
			}
			if let Ok(decoded) = <InitBondsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InitBonds(decoded));
			}
			if let Ok(decoded) = <InitVersionCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InitVersion(decoded));
			}
			if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Initialize(decoded));
			}
			if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Owner(decoded));
			}
			if let Ok(decoded) = <ProxyAdminCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ProxyAdmin(decoded));
			}
			if let Ok(decoded) =
				<ProxyAdminOwnerCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ProxyAdminOwner(decoded));
			}
			if let Ok(decoded) =
				<RenounceOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RenounceOwnership(decoded));
			}
			if let Ok(decoded) =
				<SetImplementationCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetImplementation(decoded));
			}
			if let Ok(decoded) =
				<SetImplementationWithGameTypeAndImplCall as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::SetImplementationWithGameTypeAndImpl(decoded));
			}
			if let Ok(decoded) = <SetInitBondCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetInitBond(decoded));
			}
			if let Ok(decoded) =
				<TransferOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::TransferOwnership(decoded));
			}
			if let Ok(decoded) = <VersionCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Version(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for DisputeGameFactoryCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::Constructor(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Create(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FindLatestGames(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameArgs(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameAtIndex(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameCount(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameImpls(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Games(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetGameUUID(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InitBonds(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InitVersion(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdmin(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ProxyAdminOwner(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RenounceOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetImplementation(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetImplementationWithGameTypeAndImpl(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::SetInitBond(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::TransferOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Version(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for DisputeGameFactoryCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Constructor(element) => ::core::fmt::Display::fmt(element, f),
				Self::Create(element) => ::core::fmt::Display::fmt(element, f),
				Self::FindLatestGames(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameArgs(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameAtIndex(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameCount(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameImpls(element) => ::core::fmt::Display::fmt(element, f),
				Self::Games(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetGameUUID(element) => ::core::fmt::Display::fmt(element, f),
				Self::InitBonds(element) => ::core::fmt::Display::fmt(element, f),
				Self::InitVersion(element) => ::core::fmt::Display::fmt(element, f),
				Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
				Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
				Self::ProxyAdmin(element) => ::core::fmt::Display::fmt(element, f),
				Self::ProxyAdminOwner(element) => ::core::fmt::Display::fmt(element, f),
				Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetImplementation(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetImplementationWithGameTypeAndImpl(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::SetInitBond(element) => ::core::fmt::Display::fmt(element, f),
				Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
				Self::Version(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<ConstructorCall> for DisputeGameFactoryCalls {
		fn from(value: ConstructorCall) -> Self {
			Self::Constructor(value)
		}
	}
	impl ::core::convert::From<CreateCall> for DisputeGameFactoryCalls {
		fn from(value: CreateCall) -> Self {
			Self::Create(value)
		}
	}
	impl ::core::convert::From<FindLatestGamesCall> for DisputeGameFactoryCalls {
		fn from(value: FindLatestGamesCall) -> Self {
			Self::FindLatestGames(value)
		}
	}
	impl ::core::convert::From<GameArgsCall> for DisputeGameFactoryCalls {
		fn from(value: GameArgsCall) -> Self {
			Self::GameArgs(value)
		}
	}
	impl ::core::convert::From<GameAtIndexCall> for DisputeGameFactoryCalls {
		fn from(value: GameAtIndexCall) -> Self {
			Self::GameAtIndex(value)
		}
	}
	impl ::core::convert::From<GameCountCall> for DisputeGameFactoryCalls {
		fn from(value: GameCountCall) -> Self {
			Self::GameCount(value)
		}
	}
	impl ::core::convert::From<GameImplsCall> for DisputeGameFactoryCalls {
		fn from(value: GameImplsCall) -> Self {
			Self::GameImpls(value)
		}
	}
	impl ::core::convert::From<GamesCall> for DisputeGameFactoryCalls {
		fn from(value: GamesCall) -> Self {
			Self::Games(value)
		}
	}
	impl ::core::convert::From<GetGameUUIDCall> for DisputeGameFactoryCalls {
		fn from(value: GetGameUUIDCall) -> Self {
			Self::GetGameUUID(value)
		}
	}
	impl ::core::convert::From<InitBondsCall> for DisputeGameFactoryCalls {
		fn from(value: InitBondsCall) -> Self {
			Self::InitBonds(value)
		}
	}
	impl ::core::convert::From<InitVersionCall> for DisputeGameFactoryCalls {
		fn from(value: InitVersionCall) -> Self {
			Self::InitVersion(value)
		}
	}
	impl ::core::convert::From<InitializeCall> for DisputeGameFactoryCalls {
		fn from(value: InitializeCall) -> Self {
			Self::Initialize(value)
		}
	}
	impl ::core::convert::From<OwnerCall> for DisputeGameFactoryCalls {
		fn from(value: OwnerCall) -> Self {
			Self::Owner(value)
		}
	}
	impl ::core::convert::From<ProxyAdminCall> for DisputeGameFactoryCalls {
		fn from(value: ProxyAdminCall) -> Self {
			Self::ProxyAdmin(value)
		}
	}
	impl ::core::convert::From<ProxyAdminOwnerCall> for DisputeGameFactoryCalls {
		fn from(value: ProxyAdminOwnerCall) -> Self {
			Self::ProxyAdminOwner(value)
		}
	}
	impl ::core::convert::From<RenounceOwnershipCall> for DisputeGameFactoryCalls {
		fn from(value: RenounceOwnershipCall) -> Self {
			Self::RenounceOwnership(value)
		}
	}
	impl ::core::convert::From<SetImplementationCall> for DisputeGameFactoryCalls {
		fn from(value: SetImplementationCall) -> Self {
			Self::SetImplementation(value)
		}
	}
	impl ::core::convert::From<SetImplementationWithGameTypeAndImplCall> for DisputeGameFactoryCalls {
		fn from(value: SetImplementationWithGameTypeAndImplCall) -> Self {
			Self::SetImplementationWithGameTypeAndImpl(value)
		}
	}
	impl ::core::convert::From<SetInitBondCall> for DisputeGameFactoryCalls {
		fn from(value: SetInitBondCall) -> Self {
			Self::SetInitBond(value)
		}
	}
	impl ::core::convert::From<TransferOwnershipCall> for DisputeGameFactoryCalls {
		fn from(value: TransferOwnershipCall) -> Self {
			Self::TransferOwnership(value)
		}
	}
	impl ::core::convert::From<VersionCall> for DisputeGameFactoryCalls {
		fn from(value: VersionCall) -> Self {
			Self::Version(value)
		}
	}
	///Container type for all return fields from the `create` function with signature
	/// `create(uint32,bytes32,bytes)` and selector `0x82ecf2f6`
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
	pub struct CreateReturn {
		pub proxy: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `findLatestGames` function with signature
	/// `findLatestGames(uint32,uint256,uint256)` and selector `0x254bd683`
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
	pub struct FindLatestGamesReturn {
		pub games: ::std::vec::Vec<GameSearchResult>,
	}
	///Container type for all return fields from the `gameArgs` function with signature
	/// `gameArgs(uint32)` and selector `0x74cc86ac`
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
	pub struct GameArgsReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `gameAtIndex` function with signature
	/// `gameAtIndex(uint256)` and selector `0xbb8aa1fc`
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
	pub struct GameAtIndexReturn {
		pub game_type: u32,
		pub timestamp: u64,
		pub proxy: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `gameCount` function with signature
	/// `gameCount()` and selector `0x4d1975b4`
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
	pub struct GameCountReturn {
		pub game_count: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `gameImpls` function with signature
	/// `gameImpls(uint32)` and selector `0x1b685b9e`
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
	pub struct GameImplsReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `games` function with signature
	/// `games(uint32,bytes32,bytes)` and selector `0x5f0150cb`
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
	pub struct GamesReturn {
		pub proxy: ::ethers::core::types::Address,
		pub timestamp: u64,
	}
	///Container type for all return fields from the `getGameUUID` function with signature
	/// `getGameUUID(uint32,bytes32,bytes)` and selector `0x96cd9720`
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
	pub struct GetGameUUIDReturn {
		pub uuid: [u8; 32],
	}
	///Container type for all return fields from the `initBonds` function with signature
	/// `initBonds(uint32)` and selector `0x6593dc6e`
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
	pub struct InitBondsReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `initVersion` function with signature
	/// `initVersion()` and selector `0x38d38c97`
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
	pub struct InitVersionReturn(pub u8);
	///Container type for all return fields from the `owner` function with signature `owner()` and
	/// selector `0x8da5cb5b`
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
	pub struct OwnerReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `proxyAdmin` function with signature
	/// `proxyAdmin()` and selector `0x3e47158c`
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
	pub struct ProxyAdminReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `proxyAdminOwner` function with signature
	/// `proxyAdminOwner()` and selector `0xdad544e0`
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
	pub struct ProxyAdminOwnerReturn(pub ::ethers::core::types::Address);
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
	///`GameSearchResult(uint256,bytes32,uint64,bytes32,bytes)`
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
	pub struct GameSearchResult {
		pub index: ::ethers::core::types::U256,
		pub metadata: [u8; 32],
		pub timestamp: u64,
		pub root_claim: [u8; 32],
		pub extra_data: ::ethers::core::types::Bytes,
	}
}
