pub use fault_dispute_game::*;
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
pub mod fault_dispute_game {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
				inputs: ::std::vec![
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_gameType"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("GameType"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_absolutePrestate"),
						kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("Claim"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_maxGameDepth"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_splitDepth"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_clockExtension"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("Duration"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_maxClockDuration"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("Duration"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_vm"),
						kind: ::ethers::core::abi::ethabi::ParamType::Address,
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("contract IBigStepper"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_weth"),
						kind: ::ethers::core::abi::ethabi::ParamType::Address,
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("contract IDelayedWETH"),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_anchorStateRegistry"),
						kind: ::ethers::core::abi::ethabi::ParamType::Address,
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("contract IAnchorStateRegistry",),
						),
					},
					::ethers::core::abi::ethabi::Param {
						name: ::std::borrow::ToOwned::to_owned("_l2ChainId"),
						kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
						internal_type: ::core::option::Option::Some(
							::std::borrow::ToOwned::to_owned("uint256"),
						),
					},
				],
			}),
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("absolutePrestate"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("absolutePrestate"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("absolutePrestate_"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Claim"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("addLocalData"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("addLocalData"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_ident"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_execLeafIdx"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_partOffset"),
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
					::std::borrow::ToOwned::to_owned("anchorStateRegistry"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("anchorStateRegistry",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("registry_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IAnchorStateRegistry",),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("attack"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("attack"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_disputed"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_parentIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_claim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("challengeRootL2Block"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("challengeRootL2Block",),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_outputRootProof"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
									::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned(
										"struct Types.OutputRootProof",
									),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_headerRLP"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("claimCredit"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("claimCredit"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_recipient"),
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
					::std::borrow::ToOwned::to_owned("claimData"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("claimData"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("parentIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint32"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("counteredBy"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("address"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("claimant"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("address"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("bond"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(128usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint128"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("claim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("position"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(128usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Position"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("clock"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(128usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Clock"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("claimDataLen"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("claimDataLen"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("len_"),
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
					::std::borrow::ToOwned::to_owned("claims"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("claims"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Hash"),
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
					::std::borrow::ToOwned::to_owned("clockExtension"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("clockExtension"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("clockExtension_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Duration"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("createdAt"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("createdAt"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Timestamp"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("credit"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("credit"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
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
					::std::borrow::ToOwned::to_owned("defend"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("defend"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_disputed"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_parentIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_claim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("extraData"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("extraData"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("extraData_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gameCreator"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameCreator"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("creator_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gameData"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameData"),
						inputs: ::std::vec![],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("gameType_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("GameType"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("rootClaim_"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("extraData_"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gameType"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gameType"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("gameType_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("GameType"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getChallengerDuration"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getChallengerDuration",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_claimIndex"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("duration_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Duration"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getNumToResolve"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getNumToResolve"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_claimIndex"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("numRemainingChildren_",),
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
					::std::borrow::ToOwned::to_owned("getRequiredBond"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getRequiredBond"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_position"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(128usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Position"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("requiredBond_"),
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
						inputs: ::std::vec![],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("l1Head"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l1Head"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("l1Head_"),
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
					::std::borrow::ToOwned::to_owned("l2BlockNumber"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l2BlockNumber"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("l2BlockNumber_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("l2BlockNumberChallenged"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l2BlockNumberChallenged",),
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
					::std::borrow::ToOwned::to_owned("l2BlockNumberChallenger"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l2BlockNumberChallenger",),
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
					::std::borrow::ToOwned::to_owned("l2ChainId"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l2ChainId"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("l2ChainId_"),
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
					::std::borrow::ToOwned::to_owned("maxClockDuration"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("maxClockDuration"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("maxClockDuration_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Duration"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("maxGameDepth"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("maxGameDepth"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("maxGameDepth_"),
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
					::std::borrow::ToOwned::to_owned("move"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("move"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_disputed"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_challengeIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_claim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Claim"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_isAttack"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bool,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bool"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("resolutionCheckpoints"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("resolutionCheckpoints",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("initialCheckpointComplete",),
								kind: ::ethers::core::abi::ethabi::ParamType::Bool,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bool"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("subgameIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint32"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("leftmostPosition"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(128usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Position"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("counteredBy"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("address"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("resolve"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("resolve"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("status_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("enum GameStatus"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("resolveClaim"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("resolveClaim"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_claimIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_numToResolve"),
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
					::std::borrow::ToOwned::to_owned("resolvedAt"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("resolvedAt"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Timestamp"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("resolvedSubgames"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("resolvedSubgames"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
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
					::std::borrow::ToOwned::to_owned("rootClaim"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("rootClaim"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("rootClaim_"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Claim"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("splitDepth"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("splitDepth"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("splitDepth_"),
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
					::std::borrow::ToOwned::to_owned("startingBlockNumber"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("startingBlockNumber",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("startingBlockNumber_",),
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
					::std::borrow::ToOwned::to_owned("startingOutputRoot"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("startingOutputRoot"),
						inputs: ::std::vec![],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("root"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("Hash"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("l2BlockNumber"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("startingRootHash"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("startingRootHash"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("startingRootHash_"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Hash"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("status"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("status"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("enum GameStatus"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("step"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("step"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_claimIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_isAttack"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bool,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bool"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_stateData"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("_proof"),
								kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("bytes"),
								),
							},
						],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("subgames"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("subgames"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
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
				(
					::std::borrow::ToOwned::to_owned("vm"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("vm"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("vm_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IBigStepper"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("weth"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("weth"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("weth_"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("contract IDelayedWETH"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
			]),
			events: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("Move"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("Move"),
						inputs: ::std::vec![
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("parentIndex"),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("claim"),
								kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
								indexed: true,
							},
							::ethers::core::abi::ethabi::EventParam {
								name: ::std::borrow::ToOwned::to_owned("claimant"),
								kind: ::ethers::core::abi::ethabi::ParamType::Address,
								indexed: true,
							},
						],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("Resolved"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("Resolved"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::borrow::ToOwned::to_owned("status"),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
							indexed: true,
						},],
						anonymous: false,
					},],
				),
			]),
			errors: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("AlreadyInitialized"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("AlreadyInitialized"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("AnchorRootNotFound"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("AnchorRootNotFound"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("BlockNumberMatches"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("BlockNumberMatches"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("BondTransferFailed"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("BondTransferFailed"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("CannotDefendRootClaim"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("CannotDefendRootClaim",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ClaimAboveSplit"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ClaimAboveSplit"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ClaimAlreadyExists"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ClaimAlreadyExists"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ClaimAlreadyResolved"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ClaimAlreadyResolved",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ClockNotExpired"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ClockNotExpired"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ClockTimeExceeded"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ClockTimeExceeded"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ContentLengthMismatch"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ContentLengthMismatch",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("DuplicateStep"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("DuplicateStep"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("EmptyItem"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("EmptyItem"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("GameDepthExceeded"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("GameDepthExceeded"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("GameNotInProgress"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("GameNotInProgress"),
						inputs: ::std::vec![],
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
					::std::borrow::ToOwned::to_owned("InvalidChallengePeriod"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidChallengePeriod",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidClockExtension"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidClockExtension",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidDataRemainder"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidDataRemainder",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidDisputedClaimIndex"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidDisputedClaimIndex",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidHeader"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidHeader"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidHeaderRLP"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidHeaderRLP"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidLocalIdent"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidLocalIdent"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidOutputRootProof"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidOutputRootProof",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidParent"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidParent"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidPrestate"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidPrestate"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("InvalidSplitDepth"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InvalidSplitDepth"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("L2BlockNumberChallenged"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("L2BlockNumberChallenged",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("MaxDepthTooLarge"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("MaxDepthTooLarge"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("NoCreditToClaim"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("NoCreditToClaim"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("OutOfOrderResolution"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("OutOfOrderResolution",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnexpectedList"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnexpectedList"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnexpectedRootClaim"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnexpectedRootClaim",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("rootClaim"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("Claim"),
							),
						},],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnexpectedString"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnexpectedString"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("ValidStep"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("ValidStep"),
						inputs: ::std::vec![],
					},],
				),
			]),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static FAULTDISPUTEGAME_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct FaultDisputeGame<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for FaultDisputeGame<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for FaultDisputeGame<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for FaultDisputeGame<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for FaultDisputeGame<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(FaultDisputeGame))
				.field(&self.address())
				.finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> FaultDisputeGame<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(
				address.into(),
				FAULTDISPUTEGAME_ABI.clone(),
				client,
			))
		}
		///Calls the contract's `absolutePrestate` (0x8d450a95) function
		pub fn absolute_prestate(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([141, 69, 10, 149], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `addLocalData` (0xf8f43ff6) function
		pub fn add_local_data(
			&self,
			ident: ::ethers::core::types::U256,
			exec_leaf_idx: ::ethers::core::types::U256,
			part_offset: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([248, 244, 63, 246], (ident, exec_leaf_idx, part_offset))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `anchorStateRegistry` (0x5c0cba33) function
		pub fn anchor_state_registry(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([92, 12, 186, 51], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `attack` (0x472777c6) function
		pub fn attack(
			&self,
			disputed: [u8; 32],
			parent_index: ::ethers::core::types::U256,
			claim: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([71, 39, 119, 198], (disputed, parent_index, claim))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `challengeRootL2Block` (0x01935130) function
		pub fn challenge_root_l2_block(
			&self,
			output_root_proof: OutputRootProof,
			header_rlp: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([1, 147, 81, 48], (output_root_proof, header_rlp))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `claimCredit` (0x60e27464) function
		pub fn claim_credit(
			&self,
			recipient: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([96, 226, 116, 100], recipient)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `claimData` (0xc6f0308c) function
		pub fn claim_data(
			&self,
			p0: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(
				u32,
				::ethers::core::types::Address,
				::ethers::core::types::Address,
				u128,
				[u8; 32],
				u128,
				u128,
			),
		> {
			self.0
				.method_hash([198, 240, 48, 140], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `claimDataLen` (0x8980e0cc) function
		pub fn claim_data_len(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([137, 128, 224, 204], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `claims` (0xeff0f592) function
		pub fn claims(&self, p0: [u8; 32]) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([239, 240, 245, 146], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `clockExtension` (0x6b6716c0) function
		pub fn clock_extension(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([107, 103, 22, 192], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `createdAt` (0xcf09e0d0) function
		pub fn created_at(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([207, 9, 224, 208], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `credit` (0xd5d44d80) function
		pub fn credit(
			&self,
			p0: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([213, 212, 77, 128], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `defend` (0x7b0f0adc) function
		pub fn defend(
			&self,
			disputed: [u8; 32],
			parent_index: ::ethers::core::types::U256,
			claim: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([123, 15, 10, 220], (disputed, parent_index, claim))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `extraData` (0x609d3334) function
		pub fn extra_data(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([96, 157, 51, 52], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameCreator` (0x37b1b229) function
		pub fn game_creator(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([55, 177, 178, 41], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameData` (0xfa24f743) function
		pub fn game_data(
			&self,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(u32, [u8; 32], ::ethers::core::types::Bytes),
		> {
			self.0
				.method_hash([250, 36, 247, 67], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gameType` (0xbbdc02db) function
		pub fn game_type(&self) -> ::ethers::contract::builders::ContractCall<M, u32> {
			self.0
				.method_hash([187, 220, 2, 219], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getChallengerDuration` (0xbd8da956) function
		pub fn get_challenger_duration(
			&self,
			claim_index: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([189, 141, 169, 86], claim_index)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getNumToResolve` (0x5a5fa2d9) function
		pub fn get_num_to_resolve(
			&self,
			claim_index: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([90, 95, 162, 217], claim_index)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getRequiredBond` (0xc395e1ca) function
		pub fn get_required_bond(
			&self,
			position: u128,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([195, 149, 225, 202], position)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `initialize` (0x8129fc1c) function
		pub fn initialize(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([129, 41, 252, 28], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l1Head` (0x6361506d) function
		pub fn l_1_head(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([99, 97, 80, 109], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l2BlockNumber` (0x8b85902b) function
		pub fn l_2_block_number(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([139, 133, 144, 43], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l2BlockNumberChallenged` (0x3e3ac912) function
		pub fn l_2_block_number_challenged(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([62, 58, 201, 18], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l2BlockNumberChallenger` (0x30dbe570) function
		pub fn l_2_block_number_challenger(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([48, 219, 229, 112], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l2ChainId` (0xd6ae3cd5) function
		pub fn l_2_chain_id(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([214, 174, 60, 213], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `maxClockDuration` (0xdabd396d) function
		pub fn max_clock_duration(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([218, 189, 57, 109], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `maxGameDepth` (0xfa315aa9) function
		pub fn max_game_depth(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([250, 49, 90, 169], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `move` (0x6f034409) function
		pub fn move_(
			&self,
			disputed: [u8; 32],
			challenge_index: ::ethers::core::types::U256,
			claim: [u8; 32],
			is_attack: bool,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([111, 3, 68, 9], (disputed, challenge_index, claim, is_attack))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `resolutionCheckpoints` (0xa445ece6) function
		pub fn resolution_checkpoints(
			&self,
			p0: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(bool, u32, u128, ::ethers::core::types::Address),
		> {
			self.0
				.method_hash([164, 69, 236, 230], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `resolve` (0x2810e1d6) function
		pub fn resolve(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
			self.0
				.method_hash([40, 16, 225, 214], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `resolveClaim` (0x03c2924d) function
		pub fn resolve_claim(
			&self,
			claim_index: ::ethers::core::types::U256,
			num_to_resolve: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([3, 194, 146, 77], (claim_index, num_to_resolve))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `resolvedAt` (0x19effeb4) function
		pub fn resolved_at(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([25, 239, 254, 180], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `resolvedSubgames` (0xfe2bbeb2) function
		pub fn resolved_subgames(
			&self,
			p0: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([254, 43, 190, 178], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `rootClaim` (0xbcef3b55) function
		pub fn root_claim(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([188, 239, 59, 85], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `splitDepth` (0xec5e6308) function
		pub fn split_depth(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([236, 94, 99, 8], ())
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
		///Calls the contract's `startingOutputRoot` (0x57da950e) function
		pub fn starting_output_root(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ([u8; 32], ::ethers::core::types::U256)>
		{
			self.0
				.method_hash([87, 218, 149, 14], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `startingRootHash` (0x25fc2ace) function
		pub fn starting_root_hash(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([37, 252, 42, 206], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `status` (0x200d2ed2) function
		pub fn status(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
			self.0
				.method_hash([32, 13, 46, 210], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `step` (0xd8cc1a3c) function
		pub fn step(
			&self,
			claim_index: ::ethers::core::types::U256,
			is_attack: bool,
			state_data: ::ethers::core::types::Bytes,
			proof: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([216, 204, 26, 60], (claim_index, is_attack, state_data, proof))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `subgames` (0x2ad69aeb) function
		pub fn subgames(
			&self,
			p0: ::ethers::core::types::U256,
			p1: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([42, 214, 154, 235], (p0, p1))
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
		///Calls the contract's `vm` (0x3a768463) function
		pub fn vm(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([58, 118, 132, 99], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `weth` (0x3fc8cef3) function
		pub fn weth(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([63, 200, 206, 243], ())
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `Move` event
		pub fn move_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, MoveFilter> {
			self.0.event()
		}
		///Gets the contract's `Resolved` event
		pub fn resolved_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ResolvedFilter> {
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, FaultDisputeGameEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
		for FaultDisputeGame<M>
	{
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Custom Error type `AlreadyInitialized` with signature `AlreadyInitialized()` and selector
	/// `0x0dc149f0`
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
	#[etherror(name = "AlreadyInitialized", abi = "AlreadyInitialized()")]
	pub struct AlreadyInitialized;
	///Custom Error type `AnchorRootNotFound` with signature `AnchorRootNotFound()` and selector
	/// `0x6a6bc3b2`
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
	#[etherror(name = "AnchorRootNotFound", abi = "AnchorRootNotFound()")]
	pub struct AnchorRootNotFound;
	///Custom Error type `BlockNumberMatches` with signature `BlockNumberMatches()` and selector
	/// `0xb8ed8830`
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
	#[etherror(name = "BlockNumberMatches", abi = "BlockNumberMatches()")]
	pub struct BlockNumberMatches;
	///Custom Error type `BondTransferFailed` with signature `BondTransferFailed()` and selector
	/// `0x83e6cc6b`
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
	#[etherror(name = "BondTransferFailed", abi = "BondTransferFailed()")]
	pub struct BondTransferFailed;
	///Custom Error type `CannotDefendRootClaim` with signature `CannotDefendRootClaim()` and
	/// selector `0xa42637bc`
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
	#[etherror(name = "CannotDefendRootClaim", abi = "CannotDefendRootClaim()")]
	pub struct CannotDefendRootClaim;
	///Custom Error type `ClaimAboveSplit` with signature `ClaimAboveSplit()` and selector
	/// `0xb34b5c22`
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
	#[etherror(name = "ClaimAboveSplit", abi = "ClaimAboveSplit()")]
	pub struct ClaimAboveSplit;
	///Custom Error type `ClaimAlreadyExists` with signature `ClaimAlreadyExists()` and selector
	/// `0x80497e3b`
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
	#[etherror(name = "ClaimAlreadyExists", abi = "ClaimAlreadyExists()")]
	pub struct ClaimAlreadyExists;
	///Custom Error type `ClaimAlreadyResolved` with signature `ClaimAlreadyResolved()` and
	/// selector `0xf1a94581`
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
	#[etherror(name = "ClaimAlreadyResolved", abi = "ClaimAlreadyResolved()")]
	pub struct ClaimAlreadyResolved;
	///Custom Error type `ClockNotExpired` with signature `ClockNotExpired()` and selector
	/// `0xf2440b53`
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
	#[etherror(name = "ClockNotExpired", abi = "ClockNotExpired()")]
	pub struct ClockNotExpired;
	///Custom Error type `ClockTimeExceeded` with signature `ClockTimeExceeded()` and selector
	/// `0x3381d114`
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
	#[etherror(name = "ClockTimeExceeded", abi = "ClockTimeExceeded()")]
	pub struct ClockTimeExceeded;
	///Custom Error type `ContentLengthMismatch` with signature `ContentLengthMismatch()` and
	/// selector `0x66c94485`
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
	#[etherror(name = "ContentLengthMismatch", abi = "ContentLengthMismatch()")]
	pub struct ContentLengthMismatch;
	///Custom Error type `DuplicateStep` with signature `DuplicateStep()` and selector `0x9071e6af`
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
	#[etherror(name = "DuplicateStep", abi = "DuplicateStep()")]
	pub struct DuplicateStep;
	///Custom Error type `EmptyItem` with signature `EmptyItem()` and selector `0x5ab458fb`
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
	#[etherror(name = "EmptyItem", abi = "EmptyItem()")]
	pub struct EmptyItem;
	///Custom Error type `GameDepthExceeded` with signature `GameDepthExceeded()` and selector
	/// `0x56f57b2b`
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
	#[etherror(name = "GameDepthExceeded", abi = "GameDepthExceeded()")]
	pub struct GameDepthExceeded;
	///Custom Error type `GameNotInProgress` with signature `GameNotInProgress()` and selector
	/// `0x67fe1950`
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
	#[etherror(name = "GameNotInProgress", abi = "GameNotInProgress()")]
	pub struct GameNotInProgress;
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
	///Custom Error type `InvalidChallengePeriod` with signature `InvalidChallengePeriod()` and
	/// selector `0xb4e12433`
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
	#[etherror(name = "InvalidChallengePeriod", abi = "InvalidChallengePeriod()")]
	pub struct InvalidChallengePeriod;
	///Custom Error type `InvalidClockExtension` with signature `InvalidClockExtension()` and
	/// selector `0x8d77ecac`
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
	#[etherror(name = "InvalidClockExtension", abi = "InvalidClockExtension()")]
	pub struct InvalidClockExtension;
	///Custom Error type `InvalidDataRemainder` with signature `InvalidDataRemainder()` and
	/// selector `0x5c5537b8`
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
	#[etherror(name = "InvalidDataRemainder", abi = "InvalidDataRemainder()")]
	pub struct InvalidDataRemainder;
	///Custom Error type `InvalidDisputedClaimIndex` with signature `InvalidDisputedClaimIndex()`
	/// and selector `0x30140332`
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
	#[etherror(name = "InvalidDisputedClaimIndex", abi = "InvalidDisputedClaimIndex()")]
	pub struct InvalidDisputedClaimIndex;
	///Custom Error type `InvalidHeader` with signature `InvalidHeader()` and selector `0xbabb01dd`
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
	#[etherror(name = "InvalidHeader", abi = "InvalidHeader()")]
	pub struct InvalidHeader;
	///Custom Error type `InvalidHeaderRLP` with signature `InvalidHeaderRLP()` and selector
	/// `0xd81d583b`
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
	#[etherror(name = "InvalidHeaderRLP", abi = "InvalidHeaderRLP()")]
	pub struct InvalidHeaderRLP;
	///Custom Error type `InvalidLocalIdent` with signature `InvalidLocalIdent()` and selector
	/// `0xff137e65`
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
	#[etherror(name = "InvalidLocalIdent", abi = "InvalidLocalIdent()")]
	pub struct InvalidLocalIdent;
	///Custom Error type `InvalidOutputRootProof` with signature `InvalidOutputRootProof()` and
	/// selector `0x9cc00b5b`
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
	#[etherror(name = "InvalidOutputRootProof", abi = "InvalidOutputRootProof()")]
	pub struct InvalidOutputRootProof;
	///Custom Error type `InvalidParent` with signature `InvalidParent()` and selector `0x5f53dd98`
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
	#[etherror(name = "InvalidParent", abi = "InvalidParent()")]
	pub struct InvalidParent;
	///Custom Error type `InvalidPrestate` with signature `InvalidPrestate()` and selector
	/// `0x696550ff`
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
	#[etherror(name = "InvalidPrestate", abi = "InvalidPrestate()")]
	pub struct InvalidPrestate;
	///Custom Error type `InvalidSplitDepth` with signature `InvalidSplitDepth()` and selector
	/// `0xe62ccf39`
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
	#[etherror(name = "InvalidSplitDepth", abi = "InvalidSplitDepth()")]
	pub struct InvalidSplitDepth;
	///Custom Error type `L2BlockNumberChallenged` with signature `L2BlockNumberChallenged()` and
	/// selector `0x0ea2e752`
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
	#[etherror(name = "L2BlockNumberChallenged", abi = "L2BlockNumberChallenged()")]
	pub struct L2BlockNumberChallenged;
	///Custom Error type `MaxDepthTooLarge` with signature `MaxDepthTooLarge()` and selector
	/// `0x77dfe332`
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
	#[etherror(name = "MaxDepthTooLarge", abi = "MaxDepthTooLarge()")]
	pub struct MaxDepthTooLarge;
	///Custom Error type `NoCreditToClaim` with signature `NoCreditToClaim()` and selector
	/// `0x17bfe5f7`
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
	#[etherror(name = "NoCreditToClaim", abi = "NoCreditToClaim()")]
	pub struct NoCreditToClaim;
	///Custom Error type `OutOfOrderResolution` with signature `OutOfOrderResolution()` and
	/// selector `0x9a076646`
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
	#[etherror(name = "OutOfOrderResolution", abi = "OutOfOrderResolution()")]
	pub struct OutOfOrderResolution;
	///Custom Error type `UnexpectedList` with signature `UnexpectedList()` and selector
	/// `0x1ff9b2e4`
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
	#[etherror(name = "UnexpectedList", abi = "UnexpectedList()")]
	pub struct UnexpectedList;
	///Custom Error type `UnexpectedRootClaim` with signature `UnexpectedRootClaim(bytes32)` and
	/// selector `0xf40239db`
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
	#[etherror(name = "UnexpectedRootClaim", abi = "UnexpectedRootClaim(bytes32)")]
	pub struct UnexpectedRootClaim {
		pub root_claim: [u8; 32],
	}
	///Custom Error type `UnexpectedString` with signature `UnexpectedString()` and selector
	/// `0x4b9c6abe`
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
	#[etherror(name = "UnexpectedString", abi = "UnexpectedString()")]
	pub struct UnexpectedString;
	///Custom Error type `ValidStep` with signature `ValidStep()` and selector `0xfb4e40dd`
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
	#[etherror(name = "ValidStep", abi = "ValidStep()")]
	pub struct ValidStep;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum FaultDisputeGameErrors {
		AlreadyInitialized(AlreadyInitialized),
		AnchorRootNotFound(AnchorRootNotFound),
		BlockNumberMatches(BlockNumberMatches),
		BondTransferFailed(BondTransferFailed),
		CannotDefendRootClaim(CannotDefendRootClaim),
		ClaimAboveSplit(ClaimAboveSplit),
		ClaimAlreadyExists(ClaimAlreadyExists),
		ClaimAlreadyResolved(ClaimAlreadyResolved),
		ClockNotExpired(ClockNotExpired),
		ClockTimeExceeded(ClockTimeExceeded),
		ContentLengthMismatch(ContentLengthMismatch),
		DuplicateStep(DuplicateStep),
		EmptyItem(EmptyItem),
		GameDepthExceeded(GameDepthExceeded),
		GameNotInProgress(GameNotInProgress),
		IncorrectBondAmount(IncorrectBondAmount),
		InvalidChallengePeriod(InvalidChallengePeriod),
		InvalidClockExtension(InvalidClockExtension),
		InvalidDataRemainder(InvalidDataRemainder),
		InvalidDisputedClaimIndex(InvalidDisputedClaimIndex),
		InvalidHeader(InvalidHeader),
		InvalidHeaderRLP(InvalidHeaderRLP),
		InvalidLocalIdent(InvalidLocalIdent),
		InvalidOutputRootProof(InvalidOutputRootProof),
		InvalidParent(InvalidParent),
		InvalidPrestate(InvalidPrestate),
		InvalidSplitDepth(InvalidSplitDepth),
		L2BlockNumberChallenged(L2BlockNumberChallenged),
		MaxDepthTooLarge(MaxDepthTooLarge),
		NoCreditToClaim(NoCreditToClaim),
		OutOfOrderResolution(OutOfOrderResolution),
		UnexpectedList(UnexpectedList),
		UnexpectedRootClaim(UnexpectedRootClaim),
		UnexpectedString(UnexpectedString),
		ValidStep(ValidStep),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for FaultDisputeGameErrors {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) =
				<::std::string::String as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RevertString(decoded));
			}
			if let Ok(decoded) =
				<AlreadyInitialized as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AlreadyInitialized(decoded));
			}
			if let Ok(decoded) =
				<AnchorRootNotFound as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AnchorRootNotFound(decoded));
			}
			if let Ok(decoded) =
				<BlockNumberMatches as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::BlockNumberMatches(decoded));
			}
			if let Ok(decoded) =
				<BondTransferFailed as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::BondTransferFailed(decoded));
			}
			if let Ok(decoded) =
				<CannotDefendRootClaim as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::CannotDefendRootClaim(decoded));
			}
			if let Ok(decoded) = <ClaimAboveSplit as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ClaimAboveSplit(decoded));
			}
			if let Ok(decoded) =
				<ClaimAlreadyExists as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ClaimAlreadyExists(decoded));
			}
			if let Ok(decoded) =
				<ClaimAlreadyResolved as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ClaimAlreadyResolved(decoded));
			}
			if let Ok(decoded) = <ClockNotExpired as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ClockNotExpired(decoded));
			}
			if let Ok(decoded) = <ClockTimeExceeded as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ClockTimeExceeded(decoded));
			}
			if let Ok(decoded) =
				<ContentLengthMismatch as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ContentLengthMismatch(decoded));
			}
			if let Ok(decoded) = <DuplicateStep as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::DuplicateStep(decoded));
			}
			if let Ok(decoded) = <EmptyItem as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::EmptyItem(decoded));
			}
			if let Ok(decoded) = <GameDepthExceeded as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GameDepthExceeded(decoded));
			}
			if let Ok(decoded) = <GameNotInProgress as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GameNotInProgress(decoded));
			}
			if let Ok(decoded) =
				<IncorrectBondAmount as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::IncorrectBondAmount(decoded));
			}
			if let Ok(decoded) =
				<InvalidChallengePeriod as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidChallengePeriod(decoded));
			}
			if let Ok(decoded) =
				<InvalidClockExtension as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidClockExtension(decoded));
			}
			if let Ok(decoded) =
				<InvalidDataRemainder as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidDataRemainder(decoded));
			}
			if let Ok(decoded) =
				<InvalidDisputedClaimIndex as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidDisputedClaimIndex(decoded));
			}
			if let Ok(decoded) = <InvalidHeader as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InvalidHeader(decoded));
			}
			if let Ok(decoded) = <InvalidHeaderRLP as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidHeaderRLP(decoded));
			}
			if let Ok(decoded) = <InvalidLocalIdent as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidLocalIdent(decoded));
			}
			if let Ok(decoded) =
				<InvalidOutputRootProof as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidOutputRootProof(decoded));
			}
			if let Ok(decoded) = <InvalidParent as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InvalidParent(decoded));
			}
			if let Ok(decoded) = <InvalidPrestate as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InvalidPrestate(decoded));
			}
			if let Ok(decoded) = <InvalidSplitDepth as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidSplitDepth(decoded));
			}
			if let Ok(decoded) =
				<L2BlockNumberChallenged as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::L2BlockNumberChallenged(decoded));
			}
			if let Ok(decoded) = <MaxDepthTooLarge as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MaxDepthTooLarge(decoded));
			}
			if let Ok(decoded) = <NoCreditToClaim as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::NoCreditToClaim(decoded));
			}
			if let Ok(decoded) =
				<OutOfOrderResolution as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OutOfOrderResolution(decoded));
			}
			if let Ok(decoded) = <UnexpectedList as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::UnexpectedList(decoded));
			}
			if let Ok(decoded) =
				<UnexpectedRootClaim as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnexpectedRootClaim(decoded));
			}
			if let Ok(decoded) = <UnexpectedString as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnexpectedString(decoded));
			}
			if let Ok(decoded) = <ValidStep as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ValidStep(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for FaultDisputeGameErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::AlreadyInitialized(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::AnchorRootNotFound(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::BlockNumberMatches(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::BondTransferFailed(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::CannotDefendRootClaim(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimAboveSplit(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimAlreadyExists(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimAlreadyResolved(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ClockNotExpired(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ClockTimeExceeded(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ContentLengthMismatch(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DuplicateStep(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::EmptyItem(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameDepthExceeded(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameNotInProgress(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IncorrectBondAmount(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidChallengePeriod(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidClockExtension(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidDataRemainder(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidDisputedClaimIndex(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidHeader(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidHeaderRLP(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidLocalIdent(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidOutputRootProof(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidParent(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidPrestate(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidSplitDepth(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L2BlockNumberChallenged(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::MaxDepthTooLarge(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::NoCreditToClaim(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OutOfOrderResolution(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnexpectedList(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnexpectedRootClaim(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnexpectedString(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidStep(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for FaultDisputeGameErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<AlreadyInitialized as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<AnchorRootNotFound as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<BlockNumberMatches as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<BondTransferFailed as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<CannotDefendRootClaim as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <ClaimAboveSplit as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<ClaimAlreadyExists as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<ClaimAlreadyResolved as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <ClockNotExpired as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<ClockTimeExceeded as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<ContentLengthMismatch as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <DuplicateStep as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <EmptyItem as ::ethers::contract::EthError>::selector() => true,
				_ if selector ==
					<GameDepthExceeded as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<GameNotInProgress as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<IncorrectBondAmount as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidChallengePeriod as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidClockExtension as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidDataRemainder as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidDisputedClaimIndex as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidHeader as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidHeaderRLP as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidLocalIdent as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidOutputRootProof as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidParent as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidPrestate as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidSplitDepth as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<L2BlockNumberChallenged as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <MaxDepthTooLarge as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <NoCreditToClaim as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<OutOfOrderResolution as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnexpectedList as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnexpectedRootClaim as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnexpectedString as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <ValidStep as ::ethers::contract::EthError>::selector() => true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for FaultDisputeGameErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::AlreadyInitialized(element) => ::core::fmt::Display::fmt(element, f),
				Self::AnchorRootNotFound(element) => ::core::fmt::Display::fmt(element, f),
				Self::BlockNumberMatches(element) => ::core::fmt::Display::fmt(element, f),
				Self::BondTransferFailed(element) => ::core::fmt::Display::fmt(element, f),
				Self::CannotDefendRootClaim(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimAboveSplit(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimAlreadyExists(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimAlreadyResolved(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClockNotExpired(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClockTimeExceeded(element) => ::core::fmt::Display::fmt(element, f),
				Self::ContentLengthMismatch(element) => ::core::fmt::Display::fmt(element, f),
				Self::DuplicateStep(element) => ::core::fmt::Display::fmt(element, f),
				Self::EmptyItem(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameDepthExceeded(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameNotInProgress(element) => ::core::fmt::Display::fmt(element, f),
				Self::IncorrectBondAmount(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidChallengePeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidClockExtension(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidDataRemainder(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidDisputedClaimIndex(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidHeader(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidHeaderRLP(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidLocalIdent(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidOutputRootProof(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidParent(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidPrestate(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidSplitDepth(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2BlockNumberChallenged(element) => ::core::fmt::Display::fmt(element, f),
				Self::MaxDepthTooLarge(element) => ::core::fmt::Display::fmt(element, f),
				Self::NoCreditToClaim(element) => ::core::fmt::Display::fmt(element, f),
				Self::OutOfOrderResolution(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnexpectedList(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnexpectedRootClaim(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnexpectedString(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidStep(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for FaultDisputeGameErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<AlreadyInitialized> for FaultDisputeGameErrors {
		fn from(value: AlreadyInitialized) -> Self {
			Self::AlreadyInitialized(value)
		}
	}
	impl ::core::convert::From<AnchorRootNotFound> for FaultDisputeGameErrors {
		fn from(value: AnchorRootNotFound) -> Self {
			Self::AnchorRootNotFound(value)
		}
	}
	impl ::core::convert::From<BlockNumberMatches> for FaultDisputeGameErrors {
		fn from(value: BlockNumberMatches) -> Self {
			Self::BlockNumberMatches(value)
		}
	}
	impl ::core::convert::From<BondTransferFailed> for FaultDisputeGameErrors {
		fn from(value: BondTransferFailed) -> Self {
			Self::BondTransferFailed(value)
		}
	}
	impl ::core::convert::From<CannotDefendRootClaim> for FaultDisputeGameErrors {
		fn from(value: CannotDefendRootClaim) -> Self {
			Self::CannotDefendRootClaim(value)
		}
	}
	impl ::core::convert::From<ClaimAboveSplit> for FaultDisputeGameErrors {
		fn from(value: ClaimAboveSplit) -> Self {
			Self::ClaimAboveSplit(value)
		}
	}
	impl ::core::convert::From<ClaimAlreadyExists> for FaultDisputeGameErrors {
		fn from(value: ClaimAlreadyExists) -> Self {
			Self::ClaimAlreadyExists(value)
		}
	}
	impl ::core::convert::From<ClaimAlreadyResolved> for FaultDisputeGameErrors {
		fn from(value: ClaimAlreadyResolved) -> Self {
			Self::ClaimAlreadyResolved(value)
		}
	}
	impl ::core::convert::From<ClockNotExpired> for FaultDisputeGameErrors {
		fn from(value: ClockNotExpired) -> Self {
			Self::ClockNotExpired(value)
		}
	}
	impl ::core::convert::From<ClockTimeExceeded> for FaultDisputeGameErrors {
		fn from(value: ClockTimeExceeded) -> Self {
			Self::ClockTimeExceeded(value)
		}
	}
	impl ::core::convert::From<ContentLengthMismatch> for FaultDisputeGameErrors {
		fn from(value: ContentLengthMismatch) -> Self {
			Self::ContentLengthMismatch(value)
		}
	}
	impl ::core::convert::From<DuplicateStep> for FaultDisputeGameErrors {
		fn from(value: DuplicateStep) -> Self {
			Self::DuplicateStep(value)
		}
	}
	impl ::core::convert::From<EmptyItem> for FaultDisputeGameErrors {
		fn from(value: EmptyItem) -> Self {
			Self::EmptyItem(value)
		}
	}
	impl ::core::convert::From<GameDepthExceeded> for FaultDisputeGameErrors {
		fn from(value: GameDepthExceeded) -> Self {
			Self::GameDepthExceeded(value)
		}
	}
	impl ::core::convert::From<GameNotInProgress> for FaultDisputeGameErrors {
		fn from(value: GameNotInProgress) -> Self {
			Self::GameNotInProgress(value)
		}
	}
	impl ::core::convert::From<IncorrectBondAmount> for FaultDisputeGameErrors {
		fn from(value: IncorrectBondAmount) -> Self {
			Self::IncorrectBondAmount(value)
		}
	}
	impl ::core::convert::From<InvalidChallengePeriod> for FaultDisputeGameErrors {
		fn from(value: InvalidChallengePeriod) -> Self {
			Self::InvalidChallengePeriod(value)
		}
	}
	impl ::core::convert::From<InvalidClockExtension> for FaultDisputeGameErrors {
		fn from(value: InvalidClockExtension) -> Self {
			Self::InvalidClockExtension(value)
		}
	}
	impl ::core::convert::From<InvalidDataRemainder> for FaultDisputeGameErrors {
		fn from(value: InvalidDataRemainder) -> Self {
			Self::InvalidDataRemainder(value)
		}
	}
	impl ::core::convert::From<InvalidDisputedClaimIndex> for FaultDisputeGameErrors {
		fn from(value: InvalidDisputedClaimIndex) -> Self {
			Self::InvalidDisputedClaimIndex(value)
		}
	}
	impl ::core::convert::From<InvalidHeader> for FaultDisputeGameErrors {
		fn from(value: InvalidHeader) -> Self {
			Self::InvalidHeader(value)
		}
	}
	impl ::core::convert::From<InvalidHeaderRLP> for FaultDisputeGameErrors {
		fn from(value: InvalidHeaderRLP) -> Self {
			Self::InvalidHeaderRLP(value)
		}
	}
	impl ::core::convert::From<InvalidLocalIdent> for FaultDisputeGameErrors {
		fn from(value: InvalidLocalIdent) -> Self {
			Self::InvalidLocalIdent(value)
		}
	}
	impl ::core::convert::From<InvalidOutputRootProof> for FaultDisputeGameErrors {
		fn from(value: InvalidOutputRootProof) -> Self {
			Self::InvalidOutputRootProof(value)
		}
	}
	impl ::core::convert::From<InvalidParent> for FaultDisputeGameErrors {
		fn from(value: InvalidParent) -> Self {
			Self::InvalidParent(value)
		}
	}
	impl ::core::convert::From<InvalidPrestate> for FaultDisputeGameErrors {
		fn from(value: InvalidPrestate) -> Self {
			Self::InvalidPrestate(value)
		}
	}
	impl ::core::convert::From<InvalidSplitDepth> for FaultDisputeGameErrors {
		fn from(value: InvalidSplitDepth) -> Self {
			Self::InvalidSplitDepth(value)
		}
	}
	impl ::core::convert::From<L2BlockNumberChallenged> for FaultDisputeGameErrors {
		fn from(value: L2BlockNumberChallenged) -> Self {
			Self::L2BlockNumberChallenged(value)
		}
	}
	impl ::core::convert::From<MaxDepthTooLarge> for FaultDisputeGameErrors {
		fn from(value: MaxDepthTooLarge) -> Self {
			Self::MaxDepthTooLarge(value)
		}
	}
	impl ::core::convert::From<NoCreditToClaim> for FaultDisputeGameErrors {
		fn from(value: NoCreditToClaim) -> Self {
			Self::NoCreditToClaim(value)
		}
	}
	impl ::core::convert::From<OutOfOrderResolution> for FaultDisputeGameErrors {
		fn from(value: OutOfOrderResolution) -> Self {
			Self::OutOfOrderResolution(value)
		}
	}
	impl ::core::convert::From<UnexpectedList> for FaultDisputeGameErrors {
		fn from(value: UnexpectedList) -> Self {
			Self::UnexpectedList(value)
		}
	}
	impl ::core::convert::From<UnexpectedRootClaim> for FaultDisputeGameErrors {
		fn from(value: UnexpectedRootClaim) -> Self {
			Self::UnexpectedRootClaim(value)
		}
	}
	impl ::core::convert::From<UnexpectedString> for FaultDisputeGameErrors {
		fn from(value: UnexpectedString) -> Self {
			Self::UnexpectedString(value)
		}
	}
	impl ::core::convert::From<ValidStep> for FaultDisputeGameErrors {
		fn from(value: ValidStep) -> Self {
			Self::ValidStep(value)
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
	#[ethevent(name = "Move", abi = "Move(uint256,bytes32,address)")]
	pub struct MoveFilter {
		#[ethevent(indexed)]
		pub parent_index: ::ethers::core::types::U256,
		#[ethevent(indexed)]
		pub claim: [u8; 32],
		#[ethevent(indexed)]
		pub claimant: ::ethers::core::types::Address,
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
	#[ethevent(name = "Resolved", abi = "Resolved(uint8)")]
	pub struct ResolvedFilter {
		#[ethevent(indexed)]
		pub status: u8,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum FaultDisputeGameEvents {
		MoveFilter(MoveFilter),
		ResolvedFilter(ResolvedFilter),
	}
	impl ::ethers::contract::EthLogDecode for FaultDisputeGameEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = MoveFilter::decode_log(log) {
				return Ok(FaultDisputeGameEvents::MoveFilter(decoded));
			}
			if let Ok(decoded) = ResolvedFilter::decode_log(log) {
				return Ok(FaultDisputeGameEvents::ResolvedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for FaultDisputeGameEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::MoveFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResolvedFilter(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<MoveFilter> for FaultDisputeGameEvents {
		fn from(value: MoveFilter) -> Self {
			Self::MoveFilter(value)
		}
	}
	impl ::core::convert::From<ResolvedFilter> for FaultDisputeGameEvents {
		fn from(value: ResolvedFilter) -> Self {
			Self::ResolvedFilter(value)
		}
	}
	///Container type for all input parameters for the `absolutePrestate` function with signature
	/// `absolutePrestate()` and selector `0x8d450a95`
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
	#[ethcall(name = "absolutePrestate", abi = "absolutePrestate()")]
	pub struct AbsolutePrestateCall;
	///Container type for all input parameters for the `addLocalData` function with signature
	/// `addLocalData(uint256,uint256,uint256)` and selector `0xf8f43ff6`
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
	#[ethcall(name = "addLocalData", abi = "addLocalData(uint256,uint256,uint256)")]
	pub struct AddLocalDataCall {
		pub ident: ::ethers::core::types::U256,
		pub exec_leaf_idx: ::ethers::core::types::U256,
		pub part_offset: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `anchorStateRegistry` function with
	/// signature `anchorStateRegistry()` and selector `0x5c0cba33`
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
	#[ethcall(name = "anchorStateRegistry", abi = "anchorStateRegistry()")]
	pub struct AnchorStateRegistryCall;
	///Container type for all input parameters for the `attack` function with signature
	/// `attack(bytes32,uint256,bytes32)` and selector `0x472777c6`
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
	#[ethcall(name = "attack", abi = "attack(bytes32,uint256,bytes32)")]
	pub struct AttackCall {
		pub disputed: [u8; 32],
		pub parent_index: ::ethers::core::types::U256,
		pub claim: [u8; 32],
	}
	///Container type for all input parameters for the `challengeRootL2Block` function with
	/// signature `challengeRootL2Block((bytes32,bytes32,bytes32,bytes32),bytes)` and selector
	/// `0x01935130`
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
		name = "challengeRootL2Block",
		abi = "challengeRootL2Block((bytes32,bytes32,bytes32,bytes32),bytes)"
	)]
	pub struct ChallengeRootL2BlockCall {
		pub output_root_proof: OutputRootProof,
		pub header_rlp: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `claimCredit` function with signature
	/// `claimCredit(address)` and selector `0x60e27464`
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
	#[ethcall(name = "claimCredit", abi = "claimCredit(address)")]
	pub struct ClaimCreditCall {
		pub recipient: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `claimData` function with signature
	/// `claimData(uint256)` and selector `0xc6f0308c`
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
	#[ethcall(name = "claimData", abi = "claimData(uint256)")]
	pub struct ClaimDataCall(pub ::ethers::core::types::U256);
	///Container type for all input parameters for the `claimDataLen` function with signature
	/// `claimDataLen()` and selector `0x8980e0cc`
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
	#[ethcall(name = "claimDataLen", abi = "claimDataLen()")]
	pub struct ClaimDataLenCall;
	///Container type for all input parameters for the `claims` function with signature
	/// `claims(bytes32)` and selector `0xeff0f592`
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
	#[ethcall(name = "claims", abi = "claims(bytes32)")]
	pub struct ClaimsCall(pub [u8; 32]);
	///Container type for all input parameters for the `clockExtension` function with signature
	/// `clockExtension()` and selector `0x6b6716c0`
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
	#[ethcall(name = "clockExtension", abi = "clockExtension()")]
	pub struct ClockExtensionCall;
	///Container type for all input parameters for the `createdAt` function with signature
	/// `createdAt()` and selector `0xcf09e0d0`
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
	#[ethcall(name = "createdAt", abi = "createdAt()")]
	pub struct CreatedAtCall;
	///Container type for all input parameters for the `credit` function with signature
	/// `credit(address)` and selector `0xd5d44d80`
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
	#[ethcall(name = "credit", abi = "credit(address)")]
	pub struct CreditCall(pub ::ethers::core::types::Address);
	///Container type for all input parameters for the `defend` function with signature
	/// `defend(bytes32,uint256,bytes32)` and selector `0x7b0f0adc`
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
	#[ethcall(name = "defend", abi = "defend(bytes32,uint256,bytes32)")]
	pub struct DefendCall {
		pub disputed: [u8; 32],
		pub parent_index: ::ethers::core::types::U256,
		pub claim: [u8; 32],
	}
	///Container type for all input parameters for the `extraData` function with signature
	/// `extraData()` and selector `0x609d3334`
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
	#[ethcall(name = "extraData", abi = "extraData()")]
	pub struct ExtraDataCall;
	///Container type for all input parameters for the `gameCreator` function with signature
	/// `gameCreator()` and selector `0x37b1b229`
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
	#[ethcall(name = "gameCreator", abi = "gameCreator()")]
	pub struct GameCreatorCall;
	///Container type for all input parameters for the `gameData` function with signature
	/// `gameData()` and selector `0xfa24f743`
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
	#[ethcall(name = "gameData", abi = "gameData()")]
	pub struct GameDataCall;
	///Container type for all input parameters for the `gameType` function with signature
	/// `gameType()` and selector `0xbbdc02db`
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
	#[ethcall(name = "gameType", abi = "gameType()")]
	pub struct GameTypeCall;
	///Container type for all input parameters for the `getChallengerDuration` function with
	/// signature `getChallengerDuration(uint256)` and selector `0xbd8da956`
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
	#[ethcall(name = "getChallengerDuration", abi = "getChallengerDuration(uint256)")]
	pub struct GetChallengerDurationCall {
		pub claim_index: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `getNumToResolve` function with signature
	/// `getNumToResolve(uint256)` and selector `0x5a5fa2d9`
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
	#[ethcall(name = "getNumToResolve", abi = "getNumToResolve(uint256)")]
	pub struct GetNumToResolveCall {
		pub claim_index: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `getRequiredBond` function with signature
	/// `getRequiredBond(uint128)` and selector `0xc395e1ca`
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
	#[ethcall(name = "getRequiredBond", abi = "getRequiredBond(uint128)")]
	pub struct GetRequiredBondCall {
		pub position: u128,
	}
	///Container type for all input parameters for the `initialize` function with signature
	/// `initialize()` and selector `0x8129fc1c`
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
	#[ethcall(name = "initialize", abi = "initialize()")]
	pub struct InitializeCall;
	///Container type for all input parameters for the `l1Head` function with signature `l1Head()`
	/// and selector `0x6361506d`
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
	#[ethcall(name = "l1Head", abi = "l1Head()")]
	pub struct L1HeadCall;
	///Container type for all input parameters for the `l2BlockNumber` function with signature
	/// `l2BlockNumber()` and selector `0x8b85902b`
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
	#[ethcall(name = "l2BlockNumber", abi = "l2BlockNumber()")]
	pub struct L2BlockNumberCall;
	///Container type for all input parameters for the `l2BlockNumberChallenged` function with
	/// signature `l2BlockNumberChallenged()` and selector `0x3e3ac912`
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
	#[ethcall(name = "l2BlockNumberChallenged", abi = "l2BlockNumberChallenged()")]
	pub struct L2BlockNumberChallengedCall;
	///Container type for all input parameters for the `l2BlockNumberChallenger` function with
	/// signature `l2BlockNumberChallenger()` and selector `0x30dbe570`
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
	#[ethcall(name = "l2BlockNumberChallenger", abi = "l2BlockNumberChallenger()")]
	pub struct L2BlockNumberChallengerCall;
	///Container type for all input parameters for the `l2ChainId` function with signature
	/// `l2ChainId()` and selector `0xd6ae3cd5`
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
	#[ethcall(name = "l2ChainId", abi = "l2ChainId()")]
	pub struct L2ChainIdCall;
	///Container type for all input parameters for the `maxClockDuration` function with signature
	/// `maxClockDuration()` and selector `0xdabd396d`
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
	#[ethcall(name = "maxClockDuration", abi = "maxClockDuration()")]
	pub struct MaxClockDurationCall;
	///Container type for all input parameters for the `maxGameDepth` function with signature
	/// `maxGameDepth()` and selector `0xfa315aa9`
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
	#[ethcall(name = "maxGameDepth", abi = "maxGameDepth()")]
	pub struct MaxGameDepthCall;
	///Container type for all input parameters for the `move` function with signature
	/// `move(bytes32,uint256,bytes32,bool)` and selector `0x6f034409`
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
	#[ethcall(name = "move", abi = "move(bytes32,uint256,bytes32,bool)")]
	pub struct MoveCall {
		pub disputed: [u8; 32],
		pub challenge_index: ::ethers::core::types::U256,
		pub claim: [u8; 32],
		pub is_attack: bool,
	}
	///Container type for all input parameters for the `resolutionCheckpoints` function with
	/// signature `resolutionCheckpoints(uint256)` and selector `0xa445ece6`
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
	#[ethcall(name = "resolutionCheckpoints", abi = "resolutionCheckpoints(uint256)")]
	pub struct ResolutionCheckpointsCall(pub ::ethers::core::types::U256);
	///Container type for all input parameters for the `resolve` function with signature
	/// `resolve()` and selector `0x2810e1d6`
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
	#[ethcall(name = "resolve", abi = "resolve()")]
	pub struct ResolveCall;
	///Container type for all input parameters for the `resolveClaim` function with signature
	/// `resolveClaim(uint256,uint256)` and selector `0x03c2924d`
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
	#[ethcall(name = "resolveClaim", abi = "resolveClaim(uint256,uint256)")]
	pub struct ResolveClaimCall {
		pub claim_index: ::ethers::core::types::U256,
		pub num_to_resolve: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `resolvedAt` function with signature
	/// `resolvedAt()` and selector `0x19effeb4`
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
	#[ethcall(name = "resolvedAt", abi = "resolvedAt()")]
	pub struct ResolvedAtCall;
	///Container type for all input parameters for the `resolvedSubgames` function with signature
	/// `resolvedSubgames(uint256)` and selector `0xfe2bbeb2`
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
	#[ethcall(name = "resolvedSubgames", abi = "resolvedSubgames(uint256)")]
	pub struct ResolvedSubgamesCall(pub ::ethers::core::types::U256);
	///Container type for all input parameters for the `rootClaim` function with signature
	/// `rootClaim()` and selector `0xbcef3b55`
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
	#[ethcall(name = "rootClaim", abi = "rootClaim()")]
	pub struct RootClaimCall;
	///Container type for all input parameters for the `splitDepth` function with signature
	/// `splitDepth()` and selector `0xec5e6308`
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
	#[ethcall(name = "splitDepth", abi = "splitDepth()")]
	pub struct SplitDepthCall;
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
	///Container type for all input parameters for the `startingOutputRoot` function with signature
	/// `startingOutputRoot()` and selector `0x57da950e`
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
	#[ethcall(name = "startingOutputRoot", abi = "startingOutputRoot()")]
	pub struct StartingOutputRootCall;
	///Container type for all input parameters for the `startingRootHash` function with signature
	/// `startingRootHash()` and selector `0x25fc2ace`
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
	#[ethcall(name = "startingRootHash", abi = "startingRootHash()")]
	pub struct StartingRootHashCall;
	///Container type for all input parameters for the `status` function with signature `status()`
	/// and selector `0x200d2ed2`
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
	#[ethcall(name = "status", abi = "status()")]
	pub struct StatusCall;
	///Container type for all input parameters for the `step` function with signature
	/// `step(uint256,bool,bytes,bytes)` and selector `0xd8cc1a3c`
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
	#[ethcall(name = "step", abi = "step(uint256,bool,bytes,bytes)")]
	pub struct StepCall {
		pub claim_index: ::ethers::core::types::U256,
		pub is_attack: bool,
		pub state_data: ::ethers::core::types::Bytes,
		pub proof: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `subgames` function with signature
	/// `subgames(uint256,uint256)` and selector `0x2ad69aeb`
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
	#[ethcall(name = "subgames", abi = "subgames(uint256,uint256)")]
	pub struct SubgamesCall(pub ::ethers::core::types::U256, pub ::ethers::core::types::U256);
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
	///Container type for all input parameters for the `vm` function with signature `vm()` and
	/// selector `0x3a768463`
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
	#[ethcall(name = "vm", abi = "vm()")]
	pub struct VmCall;
	///Container type for all input parameters for the `weth` function with signature `weth()` and
	/// selector `0x3fc8cef3`
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
	#[ethcall(name = "weth", abi = "weth()")]
	pub struct WethCall;
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum FaultDisputeGameCalls {
		AbsolutePrestate(AbsolutePrestateCall),
		AddLocalData(AddLocalDataCall),
		AnchorStateRegistry(AnchorStateRegistryCall),
		Attack(AttackCall),
		ChallengeRootL2Block(ChallengeRootL2BlockCall),
		ClaimCredit(ClaimCreditCall),
		ClaimData(ClaimDataCall),
		ClaimDataLen(ClaimDataLenCall),
		Claims(ClaimsCall),
		ClockExtension(ClockExtensionCall),
		CreatedAt(CreatedAtCall),
		Credit(CreditCall),
		Defend(DefendCall),
		ExtraData(ExtraDataCall),
		GameCreator(GameCreatorCall),
		GameData(GameDataCall),
		GameType(GameTypeCall),
		GetChallengerDuration(GetChallengerDurationCall),
		GetNumToResolve(GetNumToResolveCall),
		GetRequiredBond(GetRequiredBondCall),
		Initialize(InitializeCall),
		L1Head(L1HeadCall),
		L2BlockNumber(L2BlockNumberCall),
		L2BlockNumberChallenged(L2BlockNumberChallengedCall),
		L2BlockNumberChallenger(L2BlockNumberChallengerCall),
		L2ChainId(L2ChainIdCall),
		MaxClockDuration(MaxClockDurationCall),
		MaxGameDepth(MaxGameDepthCall),
		Move(MoveCall),
		ResolutionCheckpoints(ResolutionCheckpointsCall),
		Resolve(ResolveCall),
		ResolveClaim(ResolveClaimCall),
		ResolvedAt(ResolvedAtCall),
		ResolvedSubgames(ResolvedSubgamesCall),
		RootClaim(RootClaimCall),
		SplitDepth(SplitDepthCall),
		StartingBlockNumber(StartingBlockNumberCall),
		StartingOutputRoot(StartingOutputRootCall),
		StartingRootHash(StartingRootHashCall),
		Status(StatusCall),
		Step(StepCall),
		Subgames(SubgamesCall),
		Version(VersionCall),
		Vm(VmCall),
		Weth(WethCall),
	}
	impl ::ethers::core::abi::AbiDecode for FaultDisputeGameCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) =
				<AbsolutePrestateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AbsolutePrestate(decoded));
			}
			if let Ok(decoded) = <AddLocalDataCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AddLocalData(decoded));
			}
			if let Ok(decoded) =
				<AnchorStateRegistryCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AnchorStateRegistry(decoded));
			}
			if let Ok(decoded) = <AttackCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Attack(decoded));
			}
			if let Ok(decoded) =
				<ChallengeRootL2BlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ChallengeRootL2Block(decoded));
			}
			if let Ok(decoded) = <ClaimCreditCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ClaimCredit(decoded));
			}
			if let Ok(decoded) = <ClaimDataCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ClaimData(decoded));
			}
			if let Ok(decoded) = <ClaimDataLenCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ClaimDataLen(decoded));
			}
			if let Ok(decoded) = <ClaimsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Claims(decoded));
			}
			if let Ok(decoded) =
				<ClockExtensionCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ClockExtension(decoded));
			}
			if let Ok(decoded) = <CreatedAtCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::CreatedAt(decoded));
			}
			if let Ok(decoded) = <CreditCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Credit(decoded));
			}
			if let Ok(decoded) = <DefendCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Defend(decoded));
			}
			if let Ok(decoded) = <ExtraDataCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ExtraData(decoded));
			}
			if let Ok(decoded) = <GameCreatorCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameCreator(decoded));
			}
			if let Ok(decoded) = <GameDataCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameData(decoded));
			}
			if let Ok(decoded) = <GameTypeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GameType(decoded));
			}
			if let Ok(decoded) =
				<GetChallengerDurationCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetChallengerDuration(decoded));
			}
			if let Ok(decoded) =
				<GetNumToResolveCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetNumToResolve(decoded));
			}
			if let Ok(decoded) =
				<GetRequiredBondCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetRequiredBond(decoded));
			}
			if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Initialize(decoded));
			}
			if let Ok(decoded) = <L1HeadCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::L1Head(decoded));
			}
			if let Ok(decoded) = <L2BlockNumberCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::L2BlockNumber(decoded));
			}
			if let Ok(decoded) =
				<L2BlockNumberChallengedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::L2BlockNumberChallenged(decoded));
			}
			if let Ok(decoded) =
				<L2BlockNumberChallengerCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::L2BlockNumberChallenger(decoded));
			}
			if let Ok(decoded) = <L2ChainIdCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::L2ChainId(decoded));
			}
			if let Ok(decoded) =
				<MaxClockDurationCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MaxClockDuration(decoded));
			}
			if let Ok(decoded) = <MaxGameDepthCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MaxGameDepth(decoded));
			}
			if let Ok(decoded) = <MoveCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Move(decoded));
			}
			if let Ok(decoded) =
				<ResolutionCheckpointsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResolutionCheckpoints(decoded));
			}
			if let Ok(decoded) = <ResolveCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Resolve(decoded));
			}
			if let Ok(decoded) = <ResolveClaimCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResolveClaim(decoded));
			}
			if let Ok(decoded) = <ResolvedAtCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ResolvedAt(decoded));
			}
			if let Ok(decoded) =
				<ResolvedSubgamesCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResolvedSubgames(decoded));
			}
			if let Ok(decoded) = <RootClaimCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::RootClaim(decoded));
			}
			if let Ok(decoded) = <SplitDepthCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SplitDepth(decoded));
			}
			if let Ok(decoded) =
				<StartingBlockNumberCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StartingBlockNumber(decoded));
			}
			if let Ok(decoded) =
				<StartingOutputRootCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StartingOutputRoot(decoded));
			}
			if let Ok(decoded) =
				<StartingRootHashCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StartingRootHash(decoded));
			}
			if let Ok(decoded) = <StatusCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Status(decoded));
			}
			if let Ok(decoded) = <StepCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Step(decoded));
			}
			if let Ok(decoded) = <SubgamesCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Subgames(decoded));
			}
			if let Ok(decoded) = <VersionCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Version(decoded));
			}
			if let Ok(decoded) = <VmCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Vm(decoded));
			}
			if let Ok(decoded) = <WethCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Weth(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for FaultDisputeGameCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::AbsolutePrestate(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::AddLocalData(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::AnchorStateRegistry(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Attack(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChallengeRootL2Block(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimCredit(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimData(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ClaimDataLen(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Claims(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ClockExtension(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::CreatedAt(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Credit(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Defend(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ExtraData(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameCreator(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameData(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameType(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetChallengerDuration(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetNumToResolve(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetRequiredBond(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L1Head(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L2BlockNumber(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L2BlockNumberChallenged(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::L2BlockNumberChallenger(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::L2ChainId(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::MaxClockDuration(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::MaxGameDepth(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Move(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResolutionCheckpoints(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Resolve(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResolveClaim(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResolvedAt(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResolvedSubgames(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RootClaim(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SplitDepth(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StartingBlockNumber(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StartingOutputRoot(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StartingRootHash(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Status(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Step(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Subgames(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Version(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Vm(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Weth(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for FaultDisputeGameCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::AbsolutePrestate(element) => ::core::fmt::Display::fmt(element, f),
				Self::AddLocalData(element) => ::core::fmt::Display::fmt(element, f),
				Self::AnchorStateRegistry(element) => ::core::fmt::Display::fmt(element, f),
				Self::Attack(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChallengeRootL2Block(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimCredit(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimData(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClaimDataLen(element) => ::core::fmt::Display::fmt(element, f),
				Self::Claims(element) => ::core::fmt::Display::fmt(element, f),
				Self::ClockExtension(element) => ::core::fmt::Display::fmt(element, f),
				Self::CreatedAt(element) => ::core::fmt::Display::fmt(element, f),
				Self::Credit(element) => ::core::fmt::Display::fmt(element, f),
				Self::Defend(element) => ::core::fmt::Display::fmt(element, f),
				Self::ExtraData(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameCreator(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameData(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameType(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetChallengerDuration(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetNumToResolve(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetRequiredBond(element) => ::core::fmt::Display::fmt(element, f),
				Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
				Self::L1Head(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2BlockNumber(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2BlockNumberChallenged(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2BlockNumberChallenger(element) => ::core::fmt::Display::fmt(element, f),
				Self::L2ChainId(element) => ::core::fmt::Display::fmt(element, f),
				Self::MaxClockDuration(element) => ::core::fmt::Display::fmt(element, f),
				Self::MaxGameDepth(element) => ::core::fmt::Display::fmt(element, f),
				Self::Move(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResolutionCheckpoints(element) => ::core::fmt::Display::fmt(element, f),
				Self::Resolve(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResolveClaim(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResolvedAt(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResolvedSubgames(element) => ::core::fmt::Display::fmt(element, f),
				Self::RootClaim(element) => ::core::fmt::Display::fmt(element, f),
				Self::SplitDepth(element) => ::core::fmt::Display::fmt(element, f),
				Self::StartingBlockNumber(element) => ::core::fmt::Display::fmt(element, f),
				Self::StartingOutputRoot(element) => ::core::fmt::Display::fmt(element, f),
				Self::StartingRootHash(element) => ::core::fmt::Display::fmt(element, f),
				Self::Status(element) => ::core::fmt::Display::fmt(element, f),
				Self::Step(element) => ::core::fmt::Display::fmt(element, f),
				Self::Subgames(element) => ::core::fmt::Display::fmt(element, f),
				Self::Version(element) => ::core::fmt::Display::fmt(element, f),
				Self::Vm(element) => ::core::fmt::Display::fmt(element, f),
				Self::Weth(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<AbsolutePrestateCall> for FaultDisputeGameCalls {
		fn from(value: AbsolutePrestateCall) -> Self {
			Self::AbsolutePrestate(value)
		}
	}
	impl ::core::convert::From<AddLocalDataCall> for FaultDisputeGameCalls {
		fn from(value: AddLocalDataCall) -> Self {
			Self::AddLocalData(value)
		}
	}
	impl ::core::convert::From<AnchorStateRegistryCall> for FaultDisputeGameCalls {
		fn from(value: AnchorStateRegistryCall) -> Self {
			Self::AnchorStateRegistry(value)
		}
	}
	impl ::core::convert::From<AttackCall> for FaultDisputeGameCalls {
		fn from(value: AttackCall) -> Self {
			Self::Attack(value)
		}
	}
	impl ::core::convert::From<ChallengeRootL2BlockCall> for FaultDisputeGameCalls {
		fn from(value: ChallengeRootL2BlockCall) -> Self {
			Self::ChallengeRootL2Block(value)
		}
	}
	impl ::core::convert::From<ClaimCreditCall> for FaultDisputeGameCalls {
		fn from(value: ClaimCreditCall) -> Self {
			Self::ClaimCredit(value)
		}
	}
	impl ::core::convert::From<ClaimDataCall> for FaultDisputeGameCalls {
		fn from(value: ClaimDataCall) -> Self {
			Self::ClaimData(value)
		}
	}
	impl ::core::convert::From<ClaimDataLenCall> for FaultDisputeGameCalls {
		fn from(value: ClaimDataLenCall) -> Self {
			Self::ClaimDataLen(value)
		}
	}
	impl ::core::convert::From<ClaimsCall> for FaultDisputeGameCalls {
		fn from(value: ClaimsCall) -> Self {
			Self::Claims(value)
		}
	}
	impl ::core::convert::From<ClockExtensionCall> for FaultDisputeGameCalls {
		fn from(value: ClockExtensionCall) -> Self {
			Self::ClockExtension(value)
		}
	}
	impl ::core::convert::From<CreatedAtCall> for FaultDisputeGameCalls {
		fn from(value: CreatedAtCall) -> Self {
			Self::CreatedAt(value)
		}
	}
	impl ::core::convert::From<CreditCall> for FaultDisputeGameCalls {
		fn from(value: CreditCall) -> Self {
			Self::Credit(value)
		}
	}
	impl ::core::convert::From<DefendCall> for FaultDisputeGameCalls {
		fn from(value: DefendCall) -> Self {
			Self::Defend(value)
		}
	}
	impl ::core::convert::From<ExtraDataCall> for FaultDisputeGameCalls {
		fn from(value: ExtraDataCall) -> Self {
			Self::ExtraData(value)
		}
	}
	impl ::core::convert::From<GameCreatorCall> for FaultDisputeGameCalls {
		fn from(value: GameCreatorCall) -> Self {
			Self::GameCreator(value)
		}
	}
	impl ::core::convert::From<GameDataCall> for FaultDisputeGameCalls {
		fn from(value: GameDataCall) -> Self {
			Self::GameData(value)
		}
	}
	impl ::core::convert::From<GameTypeCall> for FaultDisputeGameCalls {
		fn from(value: GameTypeCall) -> Self {
			Self::GameType(value)
		}
	}
	impl ::core::convert::From<GetChallengerDurationCall> for FaultDisputeGameCalls {
		fn from(value: GetChallengerDurationCall) -> Self {
			Self::GetChallengerDuration(value)
		}
	}
	impl ::core::convert::From<GetNumToResolveCall> for FaultDisputeGameCalls {
		fn from(value: GetNumToResolveCall) -> Self {
			Self::GetNumToResolve(value)
		}
	}
	impl ::core::convert::From<GetRequiredBondCall> for FaultDisputeGameCalls {
		fn from(value: GetRequiredBondCall) -> Self {
			Self::GetRequiredBond(value)
		}
	}
	impl ::core::convert::From<InitializeCall> for FaultDisputeGameCalls {
		fn from(value: InitializeCall) -> Self {
			Self::Initialize(value)
		}
	}
	impl ::core::convert::From<L1HeadCall> for FaultDisputeGameCalls {
		fn from(value: L1HeadCall) -> Self {
			Self::L1Head(value)
		}
	}
	impl ::core::convert::From<L2BlockNumberCall> for FaultDisputeGameCalls {
		fn from(value: L2BlockNumberCall) -> Self {
			Self::L2BlockNumber(value)
		}
	}
	impl ::core::convert::From<L2BlockNumberChallengedCall> for FaultDisputeGameCalls {
		fn from(value: L2BlockNumberChallengedCall) -> Self {
			Self::L2BlockNumberChallenged(value)
		}
	}
	impl ::core::convert::From<L2BlockNumberChallengerCall> for FaultDisputeGameCalls {
		fn from(value: L2BlockNumberChallengerCall) -> Self {
			Self::L2BlockNumberChallenger(value)
		}
	}
	impl ::core::convert::From<L2ChainIdCall> for FaultDisputeGameCalls {
		fn from(value: L2ChainIdCall) -> Self {
			Self::L2ChainId(value)
		}
	}
	impl ::core::convert::From<MaxClockDurationCall> for FaultDisputeGameCalls {
		fn from(value: MaxClockDurationCall) -> Self {
			Self::MaxClockDuration(value)
		}
	}
	impl ::core::convert::From<MaxGameDepthCall> for FaultDisputeGameCalls {
		fn from(value: MaxGameDepthCall) -> Self {
			Self::MaxGameDepth(value)
		}
	}
	impl ::core::convert::From<MoveCall> for FaultDisputeGameCalls {
		fn from(value: MoveCall) -> Self {
			Self::Move(value)
		}
	}
	impl ::core::convert::From<ResolutionCheckpointsCall> for FaultDisputeGameCalls {
		fn from(value: ResolutionCheckpointsCall) -> Self {
			Self::ResolutionCheckpoints(value)
		}
	}
	impl ::core::convert::From<ResolveCall> for FaultDisputeGameCalls {
		fn from(value: ResolveCall) -> Self {
			Self::Resolve(value)
		}
	}
	impl ::core::convert::From<ResolveClaimCall> for FaultDisputeGameCalls {
		fn from(value: ResolveClaimCall) -> Self {
			Self::ResolveClaim(value)
		}
	}
	impl ::core::convert::From<ResolvedAtCall> for FaultDisputeGameCalls {
		fn from(value: ResolvedAtCall) -> Self {
			Self::ResolvedAt(value)
		}
	}
	impl ::core::convert::From<ResolvedSubgamesCall> for FaultDisputeGameCalls {
		fn from(value: ResolvedSubgamesCall) -> Self {
			Self::ResolvedSubgames(value)
		}
	}
	impl ::core::convert::From<RootClaimCall> for FaultDisputeGameCalls {
		fn from(value: RootClaimCall) -> Self {
			Self::RootClaim(value)
		}
	}
	impl ::core::convert::From<SplitDepthCall> for FaultDisputeGameCalls {
		fn from(value: SplitDepthCall) -> Self {
			Self::SplitDepth(value)
		}
	}
	impl ::core::convert::From<StartingBlockNumberCall> for FaultDisputeGameCalls {
		fn from(value: StartingBlockNumberCall) -> Self {
			Self::StartingBlockNumber(value)
		}
	}
	impl ::core::convert::From<StartingOutputRootCall> for FaultDisputeGameCalls {
		fn from(value: StartingOutputRootCall) -> Self {
			Self::StartingOutputRoot(value)
		}
	}
	impl ::core::convert::From<StartingRootHashCall> for FaultDisputeGameCalls {
		fn from(value: StartingRootHashCall) -> Self {
			Self::StartingRootHash(value)
		}
	}
	impl ::core::convert::From<StatusCall> for FaultDisputeGameCalls {
		fn from(value: StatusCall) -> Self {
			Self::Status(value)
		}
	}
	impl ::core::convert::From<StepCall> for FaultDisputeGameCalls {
		fn from(value: StepCall) -> Self {
			Self::Step(value)
		}
	}
	impl ::core::convert::From<SubgamesCall> for FaultDisputeGameCalls {
		fn from(value: SubgamesCall) -> Self {
			Self::Subgames(value)
		}
	}
	impl ::core::convert::From<VersionCall> for FaultDisputeGameCalls {
		fn from(value: VersionCall) -> Self {
			Self::Version(value)
		}
	}
	impl ::core::convert::From<VmCall> for FaultDisputeGameCalls {
		fn from(value: VmCall) -> Self {
			Self::Vm(value)
		}
	}
	impl ::core::convert::From<WethCall> for FaultDisputeGameCalls {
		fn from(value: WethCall) -> Self {
			Self::Weth(value)
		}
	}
	///Container type for all return fields from the `absolutePrestate` function with signature
	/// `absolutePrestate()` and selector `0x8d450a95`
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
	pub struct AbsolutePrestateReturn {
		pub absolute_prestate: [u8; 32],
	}
	///Container type for all return fields from the `anchorStateRegistry` function with signature
	/// `anchorStateRegistry()` and selector `0x5c0cba33`
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
	pub struct AnchorStateRegistryReturn {
		pub registry: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `claimData` function with signature
	/// `claimData(uint256)` and selector `0xc6f0308c`
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
	pub struct ClaimDataReturn {
		pub parent_index: u32,
		pub countered_by: ::ethers::core::types::Address,
		pub claimant: ::ethers::core::types::Address,
		pub bond: u128,
		pub claim: [u8; 32],
		pub position: u128,
		pub clock: u128,
	}
	///Container type for all return fields from the `claimDataLen` function with signature
	/// `claimDataLen()` and selector `0x8980e0cc`
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
	pub struct ClaimDataLenReturn {
		pub len: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `claims` function with signature
	/// `claims(bytes32)` and selector `0xeff0f592`
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
	pub struct ClaimsReturn(pub bool);
	///Container type for all return fields from the `clockExtension` function with signature
	/// `clockExtension()` and selector `0x6b6716c0`
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
	pub struct ClockExtensionReturn {
		pub clock_extension: u64,
	}
	///Container type for all return fields from the `createdAt` function with signature
	/// `createdAt()` and selector `0xcf09e0d0`
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
	pub struct CreatedAtReturn(pub u64);
	///Container type for all return fields from the `credit` function with signature
	/// `credit(address)` and selector `0xd5d44d80`
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
	pub struct CreditReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `extraData` function with signature
	/// `extraData()` and selector `0x609d3334`
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
	pub struct ExtraDataReturn {
		pub extra_data: ::ethers::core::types::Bytes,
	}
	///Container type for all return fields from the `gameCreator` function with signature
	/// `gameCreator()` and selector `0x37b1b229`
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
	pub struct GameCreatorReturn {
		pub creator: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `gameData` function with signature
	/// `gameData()` and selector `0xfa24f743`
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
	pub struct GameDataReturn {
		pub game_type: u32,
		pub root_claim: [u8; 32],
		pub extra_data: ::ethers::core::types::Bytes,
	}
	///Container type for all return fields from the `gameType` function with signature
	/// `gameType()` and selector `0xbbdc02db`
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
	pub struct GameTypeReturn {
		pub game_type: u32,
	}
	///Container type for all return fields from the `getChallengerDuration` function with
	/// signature `getChallengerDuration(uint256)` and selector `0xbd8da956`
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
	pub struct GetChallengerDurationReturn {
		pub duration: u64,
	}
	///Container type for all return fields from the `getNumToResolve` function with signature
	/// `getNumToResolve(uint256)` and selector `0x5a5fa2d9`
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
	pub struct GetNumToResolveReturn {
		pub num_remaining_children: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `getRequiredBond` function with signature
	/// `getRequiredBond(uint128)` and selector `0xc395e1ca`
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
	pub struct GetRequiredBondReturn {
		pub required_bond: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `l1Head` function with signature `l1Head()`
	/// and selector `0x6361506d`
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
	pub struct L1HeadReturn {
		pub l_1_head: [u8; 32],
	}
	///Container type for all return fields from the `l2BlockNumber` function with signature
	/// `l2BlockNumber()` and selector `0x8b85902b`
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
	pub struct L2BlockNumberReturn {
		pub l_2_block_number: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `l2BlockNumberChallenged` function with
	/// signature `l2BlockNumberChallenged()` and selector `0x3e3ac912`
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
	pub struct L2BlockNumberChallengedReturn(pub bool);
	///Container type for all return fields from the `l2BlockNumberChallenger` function with
	/// signature `l2BlockNumberChallenger()` and selector `0x30dbe570`
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
	pub struct L2BlockNumberChallengerReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `l2ChainId` function with signature
	/// `l2ChainId()` and selector `0xd6ae3cd5`
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
	pub struct L2ChainIdReturn {
		pub l_2_chain_id: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `maxClockDuration` function with signature
	/// `maxClockDuration()` and selector `0xdabd396d`
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
	pub struct MaxClockDurationReturn {
		pub max_clock_duration: u64,
	}
	///Container type for all return fields from the `maxGameDepth` function with signature
	/// `maxGameDepth()` and selector `0xfa315aa9`
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
	pub struct MaxGameDepthReturn {
		pub max_game_depth: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `resolutionCheckpoints` function with
	/// signature `resolutionCheckpoints(uint256)` and selector `0xa445ece6`
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
	pub struct ResolutionCheckpointsReturn {
		pub initial_checkpoint_complete: bool,
		pub subgame_index: u32,
		pub leftmost_position: u128,
		pub countered_by: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `resolve` function with signature `resolve()`
	/// and selector `0x2810e1d6`
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
	pub struct ResolveReturn {
		pub status: u8,
	}
	///Container type for all return fields from the `resolvedAt` function with signature
	/// `resolvedAt()` and selector `0x19effeb4`
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
	pub struct ResolvedAtReturn(pub u64);
	///Container type for all return fields from the `resolvedSubgames` function with signature
	/// `resolvedSubgames(uint256)` and selector `0xfe2bbeb2`
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
	pub struct ResolvedSubgamesReturn(pub bool);
	///Container type for all return fields from the `rootClaim` function with signature
	/// `rootClaim()` and selector `0xbcef3b55`
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
	pub struct RootClaimReturn {
		pub root_claim: [u8; 32],
	}
	///Container type for all return fields from the `splitDepth` function with signature
	/// `splitDepth()` and selector `0xec5e6308`
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
	pub struct SplitDepthReturn {
		pub split_depth: ::ethers::core::types::U256,
	}
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
	pub struct StartingBlockNumberReturn {
		pub starting_block_number: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `startingOutputRoot` function with signature
	/// `startingOutputRoot()` and selector `0x57da950e`
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
	pub struct StartingOutputRootReturn {
		pub root: [u8; 32],
		pub l_2_block_number: ::ethers::core::types::U256,
	}
	///Container type for all return fields from the `startingRootHash` function with signature
	/// `startingRootHash()` and selector `0x25fc2ace`
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
	pub struct StartingRootHashReturn {
		pub starting_root_hash: [u8; 32],
	}
	///Container type for all return fields from the `status` function with signature `status()`
	/// and selector `0x200d2ed2`
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
	pub struct StatusReturn(pub u8);
	///Container type for all return fields from the `subgames` function with signature
	/// `subgames(uint256,uint256)` and selector `0x2ad69aeb`
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
	pub struct SubgamesReturn(pub ::ethers::core::types::U256);
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
	///Container type for all return fields from the `vm` function with signature `vm()` and
	/// selector `0x3a768463`
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
	pub struct VmReturn {
		pub vm: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `weth` function with signature `weth()` and
	/// selector `0x3fc8cef3`
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
	pub struct WethReturn {
		pub weth: ::ethers::core::types::Address,
	}
	///`OutputRootProof(bytes32,bytes32,bytes32,bytes32)`
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
	pub struct OutputRootProof {
		pub version: [u8; 32],
		pub state_root: [u8; 32],
		pub message_passer_storage_root: [u8; 32],
		pub latest_blockhash: [u8; 32],
	}
}
