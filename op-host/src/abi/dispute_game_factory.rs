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
			constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
				inputs: ::std::vec![],
			}),
			functions: ::core::convert::From::from([
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
					::std::vec![::ethers::core::abi::ethabi::Function {
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
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
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
					::std::borrow::ToOwned::to_owned("InsufficientBond"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("InsufficientBond"),
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
			]),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static DISPUTEGAMEFACTORY_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	#[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x11W`\0\x80\xFD[Pb\0\0\x1E`\0b\0\0$V[b\0\x02\x92V[`\0Ta\x01\0\x90\x04`\xFF\x16\x15\x80\x80\x15b\0\0EWP`\0T`\x01`\xFF\x90\x91\x16\x10[\x80b\0\0uWPb\0\0b0b\0\x01b` \x1Bb\0\x0C\xDC\x17` \x1CV[\x15\x80\x15b\0\0uWP`\0T`\xFF\x16`\x01\x14[b\0\0\xDEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`.`$\x82\x01R\x7FInitializable: contract is alrea`D\x82\x01Rm\x19\x1EH\x1A[\x9A]\x1AX[\x1A^\x99Y`\x92\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0\x80T`\xFF\x19\x16`\x01\x17\x90U\x80\x15b\0\x01\x02W`\0\x80Ta\xFF\0\x19\x16a\x01\0\x17\x90U[b\0\x01\x0Cb\0\x01qV[b\0\x01\x17\x82b\0\x01\xD9V[\x80\x15b\0\x01^W`\0\x80Ta\xFF\0\x19\x16\x90U`@Q`\x01\x81R\x7F\x7F&\xB8?\xF9n\x1F+jh/\x138R\xF6y\x8A\t\xC4e\xDA\x95\x92\x14`\xCE\xFB8G@$\x98\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPV[`\x01`\x01`\xA0\x1B\x03\x16;\x15\x15\x90V[`\0Ta\x01\0\x90\x04`\xFF\x16b\0\x01\xCDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R`\0\x80Q` b\0\x18\0\x839\x81Q\x91R`D\x82\x01Rjnitializing`\xA8\x1B`d\x82\x01R`\x84\x01b\0\0\xD5V[b\0\x01\xD7b\0\x02+V[V[`3\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x90\x93U`@Q\x91\x16\x91\x90\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90`\0\x90\xA3PPV[`\0Ta\x01\0\x90\x04`\xFF\x16b\0\x02\x87W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R`\0\x80Q` b\0\x18\0\x839\x81Q\x91R`D\x82\x01Rjnitializing`\xA8\x1B`d\x82\x01R`\x84\x01b\0\0\xD5V[b\0\x01\xD73b\0\x01\xD9V[a\x15^\x80b\0\x02\xA2`\09`\0\xF3\xFE`\x80`@R`\x046\x10a\0\xE8W`\x005`\xE0\x1C\x80ce\x93\xDCn\x11a\0\x8AW\x80c\x96\xCD\x97 \x11a\0YW\x80c\x96\xCD\x97 \x14a\x03\x13W\x80c\xBB\x8A\xA1\xFC\x14a\x033W\x80c\xC4\xD6m\xE8\x14a\x03\x94W\x80c\xF2\xFD\xE3\x8B\x14a\x03\xB4W`\0\x80\xFD[\x80ce\x93\xDCn\x14a\x02\x93W\x80cqP\x18\xA6\x14a\x02\xC0W\x80c\x82\xEC\xF2\xF6\x14a\x02\xD5W\x80c\x8D\xA5\xCB[\x14a\x02\xE8W`\0\x80\xFD[\x80c%K\xD6\x83\x11a\0\xC6W\x80c%K\xD6\x83\x14a\x01\x9CW\x80cM\x19u\xB4\x14a\x01\xC9W\x80cT\xFDMP\x14a\x01\xE8W\x80c_\x01P\xCB\x14a\x02>W`\0\x80\xFD[\x80c\x14\xF6\xB1\xA3\x14a\0\xEDW\x80c\x1Bh[\x9E\x14a\x01\x0FW\x80c\x1E3B@\x14a\x01|W[`\0\x80\xFD[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\x01\ra\x01\x086`\x04a\x10\x99V[a\x03\xD4V[\0[4\x80\x15a\x01\x1BW`\0\x80\xFD[Pa\x01Ra\x01*6`\x04a\x10\xD0V[`e` R`\0\x90\x81R`@\x90 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81V[`@Qs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x88W`\0\x80\xFD[Pa\x01\ra\x01\x976`\x04a\x10\xEBV[a\x04^V[4\x80\x15a\x01\xA8W`\0\x80\xFD[Pa\x01\xBCa\x01\xB76`\x04a\x11\x15V[a\x04\xAAV[`@Qa\x01s\x91\x90a\x11\xC2V[4\x80\x15a\x01\xD5W`\0\x80\xFD[P`hT[`@Q\x90\x81R` \x01a\x01sV[4\x80\x15a\x01\xF4W`\0\x80\xFD[Pa\x021`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01\x7F0.2.0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81RP\x81V[`@Qa\x01s\x91\x90a\x12\x7FV[4\x80\x15a\x02JW`\0\x80\xFD[Pa\x02^a\x02Y6`\x04a\x12\x92V[a\x06\xEEV[`@\x80Qs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x93\x16\x83Rg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16` \x83\x01R\x01a\x01sV[4\x80\x15a\x02\x9FW`\0\x80\xFD[Pa\x01\xDAa\x02\xAE6`\x04a\x10\xD0V[`f` R`\0\x90\x81R`@\x90 T\x81V[4\x80\x15a\x02\xCCW`\0\x80\xFD[Pa\x01\ra\x07AV[a\x01Ra\x02\xE36`\x04a\x12\x92V[a\x07UV[4\x80\x15a\x02\xF4W`\0\x80\xFD[P`3Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16a\x01RV[4\x80\x15a\x03\x1FW`\0\x80\xFD[Pa\x01\xDAa\x03.6`\x04a\x12\x92V[a\t\xEEV[4\x80\x15a\x03?W`\0\x80\xFD[Pa\x03Sa\x03N6`\x04a\x13\x19V[a\n'V[`@\x80Qc\xFF\xFF\xFF\xFF\x90\x94\x16\x84Rg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x16` \x84\x01Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x90\x82\x01R``\x01a\x01sV[4\x80\x15a\x03\xA0W`\0\x80\xFD[Pa\x01\ra\x03\xAF6`\x04a\x132V[a\n\x89V[4\x80\x15a\x03\xC0W`\0\x80\xFD[Pa\x01\ra\x03\xCF6`\x04a\x132V[a\x0C%V[a\x03\xDCa\x0C\xF8V[c\xFF\xFF\xFF\xFF\x82\x16`\0\x81\x81R`e` R`@\x80\x82 \x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x86\x16\x90\x81\x17\x90\x91U\x90Q\x90\x91\x7F\xFFQ=\x80\xE2\xC7\xFAHv\x08\xF7\na\x8D\xFB\xC0\xCFAV\x99\xDCiX\x8Ct~\x8CqVl\x88\xDE\x91\xA3PPV[a\x04fa\x0C\xF8V[c\xFF\xFF\xFF\xFF\x82\x16`\0\x81\x81R`f` R`@\x80\x82 \x84\x90UQ\x83\x92\x91\x7Ft\xD6f\\K&\xD5YjZ\xA1=0\x14\xE0\xC0j\xF4\xD3\"\x07Zy\x7F\x87\xB0<\xD4\xC5\xBC\x91\xCA\x91\xA3PPV[`hT``\x90\x83\x10\x15\x80a\x04\xBCWP\x81\x15[a\x06\xE7WP`@\x80Q`\x05\x83\x90\x1B\x81\x01` \x01\x90\x91R\x82[\x83\x81\x11a\x06\xE5W`\0`h\x82\x81T\x81\x10a\x04\xF0Wa\x04\xF0a\x13OV[`\0\x91\x82R` \x90\x91 \x01T\x90P`\xE0\x81\x90\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\xA0\x83\x90\x1C\x16s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x83\x16`\xFF\x80\x8A\x16\x90\x84\x16\x03a\x06\xB6W`\x01\x86Q\x01\x86R`\0\x81s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c`\x9D34`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x05\x8AW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x82\x01`@Ra\x05\xD0\x91\x90\x81\x01\x90a\x13\xADV[\x90P`\0\x82s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c\xBC\xEF;U`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\x1FW=`\0\x80>=`\0\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06C\x91\x90a\x14xV[\x90P`@Q\x80`\xA0\x01`@R\x80\x88\x81R` \x01\x87\x81R` \x01\x85g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81R` \x01\x82\x81R` \x01\x83\x81RP\x88`\x01\x8AQa\x06\x85\x91\x90a\x14\x91V[\x81Q\x81\x10a\x06\x95Wa\x06\x95a\x13OV[` \x02` \x01\x01\x81\x90RP\x88\x88Q\x10a\x06\xB3WPPPPPPa\x06\xE5V[PP[PP\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x01\x91Pa\x04\xD4\x90PV[P[\x93\x92PPPV[`\0\x80`\0a\x06\xFF\x87\x87\x87\x87a\t\xEEV[`\0\x90\x81R`g` R`@\x90 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x98`\xA0\x91\x90\x91\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x97P\x95PPPPPPV[a\x07Ia\x0C\xF8V[a\x07S`\0a\ryV[V[c\xFF\xFF\xFF\xFF\x84\x16`\0\x90\x81R`e` R`@\x81 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x80a\x07\xC5W`@Q\x7F\x03\x1Cm\xE4\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81Rc\xFF\xFF\xFF\xFF\x87\x16`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x86\x16`\0\x90\x81R`f` R`@\x90 T4\x10\x15a\x08\x14W`@Q\x7F\xE9,F\x9F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0a\x08!`\x01Ca\x14\x91V[@\x90Pa\x08\x89\x86\x82\x87\x87`@Q` \x01a\x08>\x94\x93\x92\x91\x90a\x14\xCFV[`@\x80Q\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x81\x84\x03\x01\x81R\x91\x90Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x84\x16\x90a\r\xF0V[\x92P\x82s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c\x81)\xFC\x1C4`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x85\x88\x80;\x15\x80\x15a\x08\xD3W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x08\xE7W=`\0\x80>=`\0\xFD[PPPPP`\0a\x08\xFA\x88\x88\x88\x88a\t\xEEV[`\0\x81\x81R`g` R`@\x90 T\x90\x91P\x15a\tFW`@Q\x7F\x01Oo\xE5\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R`\x04\x81\x01\x82\x90R`$\x01a\x07\xBCV[`\0B`\xA0\x1B`\xE0\x8A\x90\x1B\x17\x85\x17`\0\x83\x81R`g` R`@\x80\x82 \x83\x90U`h\x80T`\x01\x81\x01\x82U\x90\x83R\x7F\xA2\x154 \xD8D\x92\x8BD!e\x02\x03\xC7{\xAB\xC8\xB3=\x7F.{E\x0E)f\xDB\x0C\"\twS\x01\x83\x90UQ\x91\x92P\x89\x91c\xFF\xFF\xFF\xFF\x8C\x16\x91s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x89\x16\x91\x7F[V^\xFE\x82A\x1D\xA9\x88\x14\xF3V\xD0\xE7\xBC\xB8\xF0!\x9B\x8D\x97\x03\x07\xC5\xAF\xB4\xA6\x90:\x8B.5\x91\x90\xA4PPPP\x94\x93PPPPV[`\0\x84\x84\x84\x84`@Q` \x01a\n\x07\x94\x93\x92\x91\x90a\x14\xF0V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x94\x93PPPPV[`\0\x80`\0a\n|`h\x85\x81T\x81\x10a\nBWa\nBa\x13OV[\x90`\0R` `\0 \x01T`\xE0\x81\x90\x1C\x91`\xA0\x82\x90\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x91s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x90V[\x91\x96\x90\x95P\x90\x93P\x91PPV[`\0Ta\x01\0\x90\x04`\xFF\x16\x15\x80\x80\x15a\n\xA9WP`\0T`\x01`\xFF\x90\x91\x16\x10[\x80a\n\xC3WP0;\x15\x80\x15a\n\xC3WP`\0T`\xFF\x16`\x01\x14[a\x0BOW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`.`$\x82\x01R\x7FInitializable: contract is alrea`D\x82\x01R\x7Fdy initialized\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\x16`\x01\x17\x90U\x80\x15a\x0B\xADW`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\xFF\x16a\x01\0\x17\x90U[a\x0B\xB5a\x0F$V[a\x0B\xBE\x82a\ryV[\x80\x15a\x0C!W`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\xFF\x16\x90U`@Q`\x01\x81R\x7F\x7F&\xB8?\xF9n\x1F+jh/\x138R\xF6y\x8A\t\xC4e\xDA\x95\x92\x14`\xCE\xFB8G@$\x98\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPV[a\x0C-a\x0C\xF8V[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16a\x0C\xD0W`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FOwnable: new owner is the zero a`D\x82\x01R\x7Fddress\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x0C\xD9\x81a\ryV[PV[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16;\x15\x15\x90V[`3Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x163\x14a\x07SW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FOwnable: caller is not the owner`D\x82\x01R`d\x01a\x07\xBCV[`3\x80Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x83\x81\x16\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83\x16\x81\x17\x90\x93U`@Q\x91\x16\x91\x90\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90`\0\x90\xA3PPV[`\0`\x02\x82Q\x01`?\x81\x01`\n\x81\x03`@Q\x83`X\x1B\x82`\xE8\x1B\x17\x7Fa\0\0=\x81`\n=9\xF36==7====a\0\0\x80`5696\x01=s\0\0\x17\x81R\x86``\x1B`\x1E\x82\x01R\x7FZ\xF4==\x93\x80>`3W\xFD[\xF3\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`2\x82\x01R\x85Q\x91P`?\x81\x01` \x87\x01[` \x84\x10a\x0E\xA8W\x80Q\x82R\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x90\x93\x01\x92` \x91\x82\x01\x91\x01a\x0EkV[Q\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF` \x85\x90\x03`\x03\x1B\x1B\x16\x81R`\xF0\x85\x90\x1B\x90\x83\x01R\x82\x81`\0\xF0\x94P\x84a\x0F\x15W\x7F\xEB\xFE\xF1\x88\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R` `\0\xFD[\x90\x91\x01`@RP\x90\x93\x92PPPV[`\0Ta\x01\0\x90\x04`\xFF\x16a\x0F\xBBW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FInitializable: contract is not i`D\x82\x01R\x7Fnitializing\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x07S`\0Ta\x01\0\x90\x04`\xFF\x16a\x10UW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FInitializable: contract is not i`D\x82\x01R\x7Fnitializing\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x07S3a\ryV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x10rW`\0\x80\xFD[\x91\x90PV[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x0C\xD9W`\0\x80\xFD[`\0\x80`@\x83\x85\x03\x12\x15a\x10\xACW`\0\x80\xFD[a\x10\xB5\x83a\x10^V[\x91P` \x83\x015a\x10\xC5\x81a\x10wV[\x80\x91PP\x92P\x92\x90PV[`\0` \x82\x84\x03\x12\x15a\x10\xE2W`\0\x80\xFD[a\x06\xE7\x82a\x10^V[`\0\x80`@\x83\x85\x03\x12\x15a\x10\xFEW`\0\x80\xFD[a\x11\x07\x83a\x10^V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\x11*W`\0\x80\xFD[a\x113\x84a\x10^V[\x95` \x85\x015\x95P`@\x90\x94\x015\x93\x92PPPV[`\0[\x83\x81\x10\x15a\x11cW\x81\x81\x01Q\x83\x82\x01R` \x01a\x11KV[\x83\x81\x11\x15a\x11rW`\0\x84\x84\x01R[PPPPV[`\0\x81Q\x80\x84Ra\x11\x90\x81` \x86\x01` \x86\x01a\x11HV[`\x1F\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0` \x80\x83\x01\x81\x84R\x80\x85Q\x80\x83R`@\x92P\x82\x86\x01\x91P\x82\x81`\x05\x1B\x87\x01\x01\x84\x88\x01`\0[\x83\x81\x10\x15a\x12qW\x88\x83\x03\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xC0\x01\x85R\x81Q\x80Q\x84R\x87\x81\x01Q\x88\x85\x01R\x86\x81\x01Qg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x87\x85\x01R``\x80\x82\x01Q\x90\x85\x01R`\x80\x90\x81\x01Q`\xA0\x91\x85\x01\x82\x90R\x90a\x12]\x81\x86\x01\x83a\x11xV[\x96\x89\x01\x96\x94PPP\x90\x86\x01\x90`\x01\x01a\x11\xE9V[P\x90\x98\x97PPPPPPPPV[` \x81R`\0a\x06\xE7` \x83\x01\x84a\x11xV[`\0\x80`\0\x80``\x85\x87\x03\x12\x15a\x12\xA8W`\0\x80\xFD[a\x12\xB1\x85a\x10^V[\x93P` \x85\x015\x92P`@\x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x12\xD5W`\0\x80\xFD[\x81\x87\x01\x91P\x87`\x1F\x83\x01\x12a\x12\xE9W`\0\x80\xFD[\x815\x81\x81\x11\x15a\x12\xF8W`\0\x80\xFD[\x88` \x82\x85\x01\x01\x11\x15a\x13\nW`\0\x80\xFD[\x95\x98\x94\x97PP` \x01\x94PPPV[`\0` \x82\x84\x03\x12\x15a\x13+W`\0\x80\xFD[P5\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x13DW`\0\x80\xFD[\x815a\x06\xE7\x81a\x10wV[\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`2`\x04R`$`\0\xFD[\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`A`\x04R`$`\0\xFD[`\0` \x82\x84\x03\x12\x15a\x13\xBFW`\0\x80\xFD[\x81Qg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x13\xD7W`\0\x80\xFD[\x81\x84\x01\x91P\x84`\x1F\x83\x01\x12a\x13\xEBW`\0\x80\xFD[\x81Q\x81\x81\x11\x15a\x13\xFDWa\x13\xFDa\x13~V[`@Q`\x1F\x82\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x90\x81\x16`?\x01\x16\x81\x01\x90\x83\x82\x11\x81\x83\x10\x17\x15a\x14CWa\x14Ca\x13~V[\x81`@R\x82\x81R\x87` \x84\x87\x01\x01\x11\x15a\x14\\W`\0\x80\xFD[a\x14m\x83` \x83\x01` \x88\x01a\x11HV[\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x14\x8AW`\0\x80\xFD[PQ\x91\x90PV[`\0\x82\x82\x10\x15a\x14\xCAW\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`\x11`\x04R`$`\0\xFD[P\x03\x90V[\x84\x81R\x83` \x82\x01R\x81\x83`@\x83\x017`\0\x91\x01`@\x01\x90\x81R\x93\x92PPPV[c\xFF\xFF\xFF\xFF\x85\x16\x81R\x83` \x82\x01R```@\x82\x01R\x81``\x82\x01R\x81\x83`\x80\x83\x017`\0\x81\x83\x01`\x80\x90\x81\x01\x91\x90\x91R`\x1F\x90\x92\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x01\x01\x93\x92PPPV\xFE\xA1dsolcC\0\x08\x0F\0\nInitializable: contract is not i";
	/// The bytecode of the contract.
	pub static DISPUTEGAMEFACTORY_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__BYTECODE);
	#[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\xE8W`\x005`\xE0\x1C\x80ce\x93\xDCn\x11a\0\x8AW\x80c\x96\xCD\x97 \x11a\0YW\x80c\x96\xCD\x97 \x14a\x03\x13W\x80c\xBB\x8A\xA1\xFC\x14a\x033W\x80c\xC4\xD6m\xE8\x14a\x03\x94W\x80c\xF2\xFD\xE3\x8B\x14a\x03\xB4W`\0\x80\xFD[\x80ce\x93\xDCn\x14a\x02\x93W\x80cqP\x18\xA6\x14a\x02\xC0W\x80c\x82\xEC\xF2\xF6\x14a\x02\xD5W\x80c\x8D\xA5\xCB[\x14a\x02\xE8W`\0\x80\xFD[\x80c%K\xD6\x83\x11a\0\xC6W\x80c%K\xD6\x83\x14a\x01\x9CW\x80cM\x19u\xB4\x14a\x01\xC9W\x80cT\xFDMP\x14a\x01\xE8W\x80c_\x01P\xCB\x14a\x02>W`\0\x80\xFD[\x80c\x14\xF6\xB1\xA3\x14a\0\xEDW\x80c\x1Bh[\x9E\x14a\x01\x0FW\x80c\x1E3B@\x14a\x01|W[`\0\x80\xFD[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\x01\ra\x01\x086`\x04a\x10\x99V[a\x03\xD4V[\0[4\x80\x15a\x01\x1BW`\0\x80\xFD[Pa\x01Ra\x01*6`\x04a\x10\xD0V[`e` R`\0\x90\x81R`@\x90 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81V[`@Qs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x88W`\0\x80\xFD[Pa\x01\ra\x01\x976`\x04a\x10\xEBV[a\x04^V[4\x80\x15a\x01\xA8W`\0\x80\xFD[Pa\x01\xBCa\x01\xB76`\x04a\x11\x15V[a\x04\xAAV[`@Qa\x01s\x91\x90a\x11\xC2V[4\x80\x15a\x01\xD5W`\0\x80\xFD[P`hT[`@Q\x90\x81R` \x01a\x01sV[4\x80\x15a\x01\xF4W`\0\x80\xFD[Pa\x021`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01\x7F0.2.0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81RP\x81V[`@Qa\x01s\x91\x90a\x12\x7FV[4\x80\x15a\x02JW`\0\x80\xFD[Pa\x02^a\x02Y6`\x04a\x12\x92V[a\x06\xEEV[`@\x80Qs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x93\x16\x83Rg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16` \x83\x01R\x01a\x01sV[4\x80\x15a\x02\x9FW`\0\x80\xFD[Pa\x01\xDAa\x02\xAE6`\x04a\x10\xD0V[`f` R`\0\x90\x81R`@\x90 T\x81V[4\x80\x15a\x02\xCCW`\0\x80\xFD[Pa\x01\ra\x07AV[a\x01Ra\x02\xE36`\x04a\x12\x92V[a\x07UV[4\x80\x15a\x02\xF4W`\0\x80\xFD[P`3Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16a\x01RV[4\x80\x15a\x03\x1FW`\0\x80\xFD[Pa\x01\xDAa\x03.6`\x04a\x12\x92V[a\t\xEEV[4\x80\x15a\x03?W`\0\x80\xFD[Pa\x03Sa\x03N6`\x04a\x13\x19V[a\n'V[`@\x80Qc\xFF\xFF\xFF\xFF\x90\x94\x16\x84Rg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x16` \x84\x01Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x90\x82\x01R``\x01a\x01sV[4\x80\x15a\x03\xA0W`\0\x80\xFD[Pa\x01\ra\x03\xAF6`\x04a\x132V[a\n\x89V[4\x80\x15a\x03\xC0W`\0\x80\xFD[Pa\x01\ra\x03\xCF6`\x04a\x132V[a\x0C%V[a\x03\xDCa\x0C\xF8V[c\xFF\xFF\xFF\xFF\x82\x16`\0\x81\x81R`e` R`@\x80\x82 \x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x86\x16\x90\x81\x17\x90\x91U\x90Q\x90\x91\x7F\xFFQ=\x80\xE2\xC7\xFAHv\x08\xF7\na\x8D\xFB\xC0\xCFAV\x99\xDCiX\x8Ct~\x8CqVl\x88\xDE\x91\xA3PPV[a\x04fa\x0C\xF8V[c\xFF\xFF\xFF\xFF\x82\x16`\0\x81\x81R`f` R`@\x80\x82 \x84\x90UQ\x83\x92\x91\x7Ft\xD6f\\K&\xD5YjZ\xA1=0\x14\xE0\xC0j\xF4\xD3\"\x07Zy\x7F\x87\xB0<\xD4\xC5\xBC\x91\xCA\x91\xA3PPV[`hT``\x90\x83\x10\x15\x80a\x04\xBCWP\x81\x15[a\x06\xE7WP`@\x80Q`\x05\x83\x90\x1B\x81\x01` \x01\x90\x91R\x82[\x83\x81\x11a\x06\xE5W`\0`h\x82\x81T\x81\x10a\x04\xF0Wa\x04\xF0a\x13OV[`\0\x91\x82R` \x90\x91 \x01T\x90P`\xE0\x81\x90\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\xA0\x83\x90\x1C\x16s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x83\x16`\xFF\x80\x8A\x16\x90\x84\x16\x03a\x06\xB6W`\x01\x86Q\x01\x86R`\0\x81s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c`\x9D34`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x05\x8AW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x82\x01`@Ra\x05\xD0\x91\x90\x81\x01\x90a\x13\xADV[\x90P`\0\x82s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c\xBC\xEF;U`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\x1FW=`\0\x80>=`\0\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06C\x91\x90a\x14xV[\x90P`@Q\x80`\xA0\x01`@R\x80\x88\x81R` \x01\x87\x81R` \x01\x85g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81R` \x01\x82\x81R` \x01\x83\x81RP\x88`\x01\x8AQa\x06\x85\x91\x90a\x14\x91V[\x81Q\x81\x10a\x06\x95Wa\x06\x95a\x13OV[` \x02` \x01\x01\x81\x90RP\x88\x88Q\x10a\x06\xB3WPPPPPPa\x06\xE5V[PP[PP\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x01\x91Pa\x04\xD4\x90PV[P[\x93\x92PPPV[`\0\x80`\0a\x06\xFF\x87\x87\x87\x87a\t\xEEV[`\0\x90\x81R`g` R`@\x90 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x98`\xA0\x91\x90\x91\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x97P\x95PPPPPPV[a\x07Ia\x0C\xF8V[a\x07S`\0a\ryV[V[c\xFF\xFF\xFF\xFF\x84\x16`\0\x90\x81R`e` R`@\x81 Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x80a\x07\xC5W`@Q\x7F\x03\x1Cm\xE4\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81Rc\xFF\xFF\xFF\xFF\x87\x16`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x86\x16`\0\x90\x81R`f` R`@\x90 T4\x10\x15a\x08\x14W`@Q\x7F\xE9,F\x9F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0a\x08!`\x01Ca\x14\x91V[@\x90Pa\x08\x89\x86\x82\x87\x87`@Q` \x01a\x08>\x94\x93\x92\x91\x90a\x14\xCFV[`@\x80Q\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x81\x84\x03\x01\x81R\x91\x90Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x84\x16\x90a\r\xF0V[\x92P\x82s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16c\x81)\xFC\x1C4`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x85\x88\x80;\x15\x80\x15a\x08\xD3W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x08\xE7W=`\0\x80>=`\0\xFD[PPPPP`\0a\x08\xFA\x88\x88\x88\x88a\t\xEEV[`\0\x81\x81R`g` R`@\x90 T\x90\x91P\x15a\tFW`@Q\x7F\x01Oo\xE5\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R`\x04\x81\x01\x82\x90R`$\x01a\x07\xBCV[`\0B`\xA0\x1B`\xE0\x8A\x90\x1B\x17\x85\x17`\0\x83\x81R`g` R`@\x80\x82 \x83\x90U`h\x80T`\x01\x81\x01\x82U\x90\x83R\x7F\xA2\x154 \xD8D\x92\x8BD!e\x02\x03\xC7{\xAB\xC8\xB3=\x7F.{E\x0E)f\xDB\x0C\"\twS\x01\x83\x90UQ\x91\x92P\x89\x91c\xFF\xFF\xFF\xFF\x8C\x16\x91s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x89\x16\x91\x7F[V^\xFE\x82A\x1D\xA9\x88\x14\xF3V\xD0\xE7\xBC\xB8\xF0!\x9B\x8D\x97\x03\x07\xC5\xAF\xB4\xA6\x90:\x8B.5\x91\x90\xA4PPPP\x94\x93PPPPV[`\0\x84\x84\x84\x84`@Q` \x01a\n\x07\x94\x93\x92\x91\x90a\x14\xF0V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x94\x93PPPPV[`\0\x80`\0a\n|`h\x85\x81T\x81\x10a\nBWa\nBa\x13OV[\x90`\0R` `\0 \x01T`\xE0\x81\x90\x1C\x91`\xA0\x82\x90\x1Cg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x91s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x90V[\x91\x96\x90\x95P\x90\x93P\x91PPV[`\0Ta\x01\0\x90\x04`\xFF\x16\x15\x80\x80\x15a\n\xA9WP`\0T`\x01`\xFF\x90\x91\x16\x10[\x80a\n\xC3WP0;\x15\x80\x15a\n\xC3WP`\0T`\xFF\x16`\x01\x14[a\x0BOW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`.`$\x82\x01R\x7FInitializable: contract is alrea`D\x82\x01R\x7Fdy initialized\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\x16`\x01\x17\x90U\x80\x15a\x0B\xADW`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\xFF\x16a\x01\0\x17\x90U[a\x0B\xB5a\x0F$V[a\x0B\xBE\x82a\ryV[\x80\x15a\x0C!W`\0\x80T\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\xFF\x16\x90U`@Q`\x01\x81R\x7F\x7F&\xB8?\xF9n\x1F+jh/\x138R\xF6y\x8A\t\xC4e\xDA\x95\x92\x14`\xCE\xFB8G@$\x98\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPV[a\x0C-a\x0C\xF8V[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16a\x0C\xD0W`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FOwnable: new owner is the zero a`D\x82\x01R\x7Fddress\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x0C\xD9\x81a\ryV[PV[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16;\x15\x15\x90V[`3Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x163\x14a\x07SW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FOwnable: caller is not the owner`D\x82\x01R`d\x01a\x07\xBCV[`3\x80Ts\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x83\x81\x16\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83\x16\x81\x17\x90\x93U`@Q\x91\x16\x91\x90\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90`\0\x90\xA3PPV[`\0`\x02\x82Q\x01`?\x81\x01`\n\x81\x03`@Q\x83`X\x1B\x82`\xE8\x1B\x17\x7Fa\0\0=\x81`\n=9\xF36==7====a\0\0\x80`5696\x01=s\0\0\x17\x81R\x86``\x1B`\x1E\x82\x01R\x7FZ\xF4==\x93\x80>`3W\xFD[\xF3\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`2\x82\x01R\x85Q\x91P`?\x81\x01` \x87\x01[` \x84\x10a\x0E\xA8W\x80Q\x82R\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x90\x93\x01\x92` \x91\x82\x01\x91\x01a\x0EkV[Q\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF` \x85\x90\x03`\x03\x1B\x1B\x16\x81R`\xF0\x85\x90\x1B\x90\x83\x01R\x82\x81`\0\xF0\x94P\x84a\x0F\x15W\x7F\xEB\xFE\xF1\x88\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R` `\0\xFD[\x90\x91\x01`@RP\x90\x93\x92PPPV[`\0Ta\x01\0\x90\x04`\xFF\x16a\x0F\xBBW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FInitializable: contract is not i`D\x82\x01R\x7Fnitializing\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x07S`\0Ta\x01\0\x90\x04`\xFF\x16a\x10UW`@Q\x7F\x08\xC3y\xA0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FInitializable: contract is not i`D\x82\x01R\x7Fnitializing\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`d\x82\x01R`\x84\x01a\x07\xBCV[a\x07S3a\ryV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x10rW`\0\x80\xFD[\x91\x90PV[s\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x0C\xD9W`\0\x80\xFD[`\0\x80`@\x83\x85\x03\x12\x15a\x10\xACW`\0\x80\xFD[a\x10\xB5\x83a\x10^V[\x91P` \x83\x015a\x10\xC5\x81a\x10wV[\x80\x91PP\x92P\x92\x90PV[`\0` \x82\x84\x03\x12\x15a\x10\xE2W`\0\x80\xFD[a\x06\xE7\x82a\x10^V[`\0\x80`@\x83\x85\x03\x12\x15a\x10\xFEW`\0\x80\xFD[a\x11\x07\x83a\x10^V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\x11*W`\0\x80\xFD[a\x113\x84a\x10^V[\x95` \x85\x015\x95P`@\x90\x94\x015\x93\x92PPPV[`\0[\x83\x81\x10\x15a\x11cW\x81\x81\x01Q\x83\x82\x01R` \x01a\x11KV[\x83\x81\x11\x15a\x11rW`\0\x84\x84\x01R[PPPPV[`\0\x81Q\x80\x84Ra\x11\x90\x81` \x86\x01` \x86\x01a\x11HV[`\x1F\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0` \x80\x83\x01\x81\x84R\x80\x85Q\x80\x83R`@\x92P\x82\x86\x01\x91P\x82\x81`\x05\x1B\x87\x01\x01\x84\x88\x01`\0[\x83\x81\x10\x15a\x12qW\x88\x83\x03\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xC0\x01\x85R\x81Q\x80Q\x84R\x87\x81\x01Q\x88\x85\x01R\x86\x81\x01Qg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16\x87\x85\x01R``\x80\x82\x01Q\x90\x85\x01R`\x80\x90\x81\x01Q`\xA0\x91\x85\x01\x82\x90R\x90a\x12]\x81\x86\x01\x83a\x11xV[\x96\x89\x01\x96\x94PPP\x90\x86\x01\x90`\x01\x01a\x11\xE9V[P\x90\x98\x97PPPPPPPPV[` \x81R`\0a\x06\xE7` \x83\x01\x84a\x11xV[`\0\x80`\0\x80``\x85\x87\x03\x12\x15a\x12\xA8W`\0\x80\xFD[a\x12\xB1\x85a\x10^V[\x93P` \x85\x015\x92P`@\x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x12\xD5W`\0\x80\xFD[\x81\x87\x01\x91P\x87`\x1F\x83\x01\x12a\x12\xE9W`\0\x80\xFD[\x815\x81\x81\x11\x15a\x12\xF8W`\0\x80\xFD[\x88` \x82\x85\x01\x01\x11\x15a\x13\nW`\0\x80\xFD[\x95\x98\x94\x97PP` \x01\x94PPPV[`\0` \x82\x84\x03\x12\x15a\x13+W`\0\x80\xFD[P5\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x13DW`\0\x80\xFD[\x815a\x06\xE7\x81a\x10wV[\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`2`\x04R`$`\0\xFD[\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`A`\x04R`$`\0\xFD[`\0` \x82\x84\x03\x12\x15a\x13\xBFW`\0\x80\xFD[\x81Qg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x13\xD7W`\0\x80\xFD[\x81\x84\x01\x91P\x84`\x1F\x83\x01\x12a\x13\xEBW`\0\x80\xFD[\x81Q\x81\x81\x11\x15a\x13\xFDWa\x13\xFDa\x13~V[`@Q`\x1F\x82\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x90\x81\x16`?\x01\x16\x81\x01\x90\x83\x82\x11\x81\x83\x10\x17\x15a\x14CWa\x14Ca\x13~V[\x81`@R\x82\x81R\x87` \x84\x87\x01\x01\x11\x15a\x14\\W`\0\x80\xFD[a\x14m\x83` \x83\x01` \x88\x01a\x11HV[\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x14\x8AW`\0\x80\xFD[PQ\x91\x90PV[`\0\x82\x82\x10\x15a\x14\xCAW\x7FNH{q\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\0R`\x11`\x04R`$`\0\xFD[P\x03\x90V[\x84\x81R\x83` \x82\x01R\x81\x83`@\x83\x017`\0\x91\x01`@\x01\x90\x81R\x93\x92PPPV[c\xFF\xFF\xFF\xFF\x85\x16\x81R\x83` \x82\x01R```@\x82\x01R\x81``\x82\x01R\x81\x83`\x80\x83\x017`\0\x81\x83\x01`\x80\x90\x81\x01\x91\x90\x91R`\x1F\x90\x92\x01\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xE0\x16\x01\x01\x93\x92PPPV\xFE\xA1dsolcC\0\x08\x0F\0\n";
	/// The deployed bytecode of the contract.
	pub static DISPUTEGAMEFACTORY_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
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
		/// Constructs the general purpose `Deployer` instance based on the provided constructor
		/// arguments and sends it. Returns a new instance of a deployer that returns an instance
		/// of this contract after sending the transaction
		///
		/// Notes:
		/// - If there are no constructor arguments, you should pass `()` as the argument.
		/// - The default poll duration is 7 seconds.
		/// - The default number of confirmations is 1 block.
		///
		///
		/// # Example
		///
		/// Generate contract bindings with `abigen!` and deploy a new contract instance.
		///
		/// *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact.
		///
		/// ```ignore
		/// # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {
		///     abigen!(Greeter, "../greeter.json");
		///
		///    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();
		///    let msg = greeter_contract.greet().call().await.unwrap();
		/// # }
		/// ```
		pub fn deploy<T: ::ethers::core::abi::Tokenize>(
			client: ::std::sync::Arc<M>,
			constructor_args: T,
		) -> ::core::result::Result<
			::ethers::contract::builders::ContractDeployer<M, Self>,
			::ethers::contract::ContractError<M>,
		> {
			let factory = ::ethers::contract::ContractFactory::new(
				DISPUTEGAMEFACTORY_ABI.clone(),
				DISPUTEGAMEFACTORY_BYTECODE.clone().into(),
				client,
			);
			let deployer = factory.deploy(constructor_args)?;
			let deployer = ::ethers::contract::ContractDeployer::new(deployer);
			Ok(deployer)
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
	///Custom Error type `InsufficientBond` with signature `InsufficientBond()` and selector
	/// `0xe92c469f`
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
	#[etherror(name = "InsufficientBond", abi = "InsufficientBond()")]
	pub struct InsufficientBond;
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
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum DisputeGameFactoryErrors {
		GameAlreadyExists(GameAlreadyExists),
		InsufficientBond(InsufficientBond),
		NoImplementation(NoImplementation),
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
			if let Ok(decoded) = <InsufficientBond as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InsufficientBond(decoded));
			}
			if let Ok(decoded) = <NoImplementation as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::NoImplementation(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for DisputeGameFactoryErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::GameAlreadyExists(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InsufficientBond(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::NoImplementation(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for DisputeGameFactoryErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<GameAlreadyExists as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InsufficientBond as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <NoImplementation as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for DisputeGameFactoryErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::GameAlreadyExists(element) => ::core::fmt::Display::fmt(element, f),
				Self::InsufficientBond(element) => ::core::fmt::Display::fmt(element, f),
				Self::NoImplementation(element) => ::core::fmt::Display::fmt(element, f),
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
	impl ::core::convert::From<InsufficientBond> for DisputeGameFactoryErrors {
		fn from(value: InsufficientBond) -> Self {
			Self::InsufficientBond(value)
		}
	}
	impl ::core::convert::From<NoImplementation> for DisputeGameFactoryErrors {
		fn from(value: NoImplementation) -> Self {
			Self::NoImplementation(value)
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
		Create(CreateCall),
		FindLatestGames(FindLatestGamesCall),
		GameAtIndex(GameAtIndexCall),
		GameCount(GameCountCall),
		GameImpls(GameImplsCall),
		Games(GamesCall),
		GetGameUUID(GetGameUUIDCall),
		InitBonds(InitBondsCall),
		Initialize(InitializeCall),
		Owner(OwnerCall),
		RenounceOwnership(RenounceOwnershipCall),
		SetImplementation(SetImplementationCall),
		SetInitBond(SetInitBondCall),
		TransferOwnership(TransferOwnershipCall),
		Version(VersionCall),
	}
	impl ::ethers::core::abi::AbiDecode for DisputeGameFactoryCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <CreateCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Create(decoded));
			}
			if let Ok(decoded) =
				<FindLatestGamesCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::FindLatestGames(decoded));
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
			if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Initialize(decoded));
			}
			if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Owner(decoded));
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
				Self::Create(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FindLatestGames(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameAtIndex(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameCount(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GameImpls(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Games(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetGameUUID(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InitBonds(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RenounceOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetImplementation(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetInitBond(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::TransferOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Version(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for DisputeGameFactoryCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Create(element) => ::core::fmt::Display::fmt(element, f),
				Self::FindLatestGames(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameAtIndex(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameCount(element) => ::core::fmt::Display::fmt(element, f),
				Self::GameImpls(element) => ::core::fmt::Display::fmt(element, f),
				Self::Games(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetGameUUID(element) => ::core::fmt::Display::fmt(element, f),
				Self::InitBonds(element) => ::core::fmt::Display::fmt(element, f),
				Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
				Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
				Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetImplementation(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetInitBond(element) => ::core::fmt::Display::fmt(element, f),
				Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
				Self::Version(element) => ::core::fmt::Display::fmt(element, f),
			}
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
