pub use ovm_gas_price_oracle::*;
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
pub mod ovm_gas_price_oracle {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
				inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
					name: ::std::borrow::ToOwned::to_owned("_owner"),
					kind: ::ethers::core::abi::ethabi::ParamType::Address,
					internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
						"address"
					),),
				},],
			}),
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("decimals"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("decimals"),
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
					::std::borrow::ToOwned::to_owned("gasPrice"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gasPrice"),
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
					::std::borrow::ToOwned::to_owned("getL1Fee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1Fee"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_data"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
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
					::std::borrow::ToOwned::to_owned("getL1GasUsed"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1GasUsed"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_data"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
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
					::std::borrow::ToOwned::to_owned("l1BaseFee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l1BaseFee"),
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
					::std::borrow::ToOwned::to_owned("overhead"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("overhead"),
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
					::std::borrow::ToOwned::to_owned("scalar"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("scalar"),
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
					::std::borrow::ToOwned::to_owned("setDecimals"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setDecimals"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_decimals"),
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
					::std::borrow::ToOwned::to_owned("setGasPrice"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setGasPrice"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_gasPrice"),
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
					::std::borrow::ToOwned::to_owned("setL1BaseFee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setL1BaseFee"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_baseFee"),
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
					::std::borrow::ToOwned::to_owned("setOverhead"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setOverhead"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_overhead"),
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
					::std::borrow::ToOwned::to_owned("setScalar"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setScalar"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_scalar"),
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
			]),
			events: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("DecimalsUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("DecimalsUpdated"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							indexed: false,
						},],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("GasPriceUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("GasPriceUpdated"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							indexed: false,
						},],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("L1BaseFeeUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("L1BaseFeeUpdated"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							indexed: false,
						},],
						anonymous: false,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("OverheadUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("OverheadUpdated"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
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
				(
					::std::borrow::ToOwned::to_owned("ScalarUpdated"),
					::std::vec![::ethers::core::abi::ethabi::Event {
						name: ::std::borrow::ToOwned::to_owned("ScalarUpdated"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							indexed: false,
						},],
						anonymous: false,
					},],
				),
			]),
			errors: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("OwnableInvalidOwner"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("OwnableInvalidOwner",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("owner"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("OwnableUnauthorizedAccount"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("OwnableUnauthorizedAccount",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("account"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
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
	pub static OVM_GASPRICEORACLE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	#[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\t/8\x03\x80a\t/\x839\x81\x01`@\x81\x90Ra\0/\x91a\x015V[\x80`\x01`\x01`\xA0\x1B\x03\x81\x16a\0_W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\0h\x81a\0xV[Pa\0r\x81a\0\xC8V[Pa\x01eV[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[a\0\xD0a\x01\x06V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\0\xFAW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\0VV[a\x01\x03\x81a\0xV[PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x013W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\0VV[V[`\0` \x82\x84\x03\x12\x15a\x01GW`\0\x80\xFD[\x81Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x01^W`\0\x80\xFD[\x93\x92PPPV[a\x07\xBB\x80a\x01t`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xF5W`\x005`\xE0\x1C\x80c\x8C\x88\x85\xC8\x11a\0\x97W\x80c\xDE&\xC4\xA1\x11a\0fW\x80c\xDE&\xC4\xA1\x14a\x01\xBFW\x80c\xF2\xFD\xE3\x8B\x14a\x01\xD2W\x80c\xF4^e\xD8\x14a\x01\xE5W\x80c\xFE\x17;\x97\x14a\x01\xEEW`\0\x80\xFD[\x80c\x8C\x88\x85\xC8\x14a\x01kW\x80c\x8D\xA5\xCB[\x14a\x01~W\x80c\xBE\xDE9\xB5\x14a\x01\x99W\x80c\xBF\x1F\xE4 \x14a\x01\xACW`\0\x80\xFD[\x80cI\x94\x8E\x0E\x11a\0\xD3W\x80cI\x94\x8E\x0E\x14a\x014W\x80cQ\x9BK\xD3\x14a\x01GW\x80cpFU\x97\x14a\x01PW\x80cqP\x18\xA6\x14a\x01cW`\0\x80\xFD[\x80c\x0C\x18\xC1b\x14a\0\xFAW\x80c1<\xE5g\x14a\x01\x16W\x80c5w\xAF\xC5\x14a\x01\x1FW[`\0\x80\xFD[a\x01\x03`\x03T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x01\x03`\x05T\x81V[a\x012a\x01-6`\x04a\x04\xEEV[a\x01\xF7V[\0[a\x01\x03a\x01B6`\x04a\x05\x1DV[a\x02;V[a\x01\x03`\x02T\x81V[a\x012a\x01^6`\x04a\x04\xEEV[a\x02\x97V[a\x012a\x02\xD4V[a\x012a\x01y6`\x04a\x04\xEEV[a\x02\xE8V[`\0T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\rV[a\x012a\x01\xA76`\x04a\x04\xEEV[a\x03%V[a\x012a\x01\xBA6`\x04a\x04\xEEV[a\x03bV[a\x01\x03a\x01\xCD6`\x04a\x05\x1DV[a\x03\x9FV[a\x012a\x01\xE06`\x04a\x05\xCEV[a\x04.V[a\x01\x03`\x04T\x81V[a\x01\x03`\x01T\x81V[a\x01\xFFa\x04qV[`\x03\x81\x90U`@Q\x81\x81R\x7F2t\x0B5\xC0\xEA!6P\xF6\rD6kO\xB2\x11\xC9\x03;PqNJ\x1D4\xE6][\xEB\x9B\xB4\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\0\x80a\x02G\x83a\x03\x9FV[\x90P`\0`\x02T\x82a\x02Y\x91\x90a\x06\x14V[\x90P`\0`\x05T`\na\x02l\x91\x90a\x07\x15V[\x90P`\0`\x04T\x83a\x02~\x91\x90a\x06\x14V[\x90P`\0a\x02\x8C\x83\x83a\x07!V[\x97\x96PPPPPPPV[a\x02\x9Fa\x04qV[`\x04\x81\x90U`@Q\x81\x81R\x7F36\xCD\x97\x08\xEA\xF2v\x9A\x0F\r\xC0g\x9F0\xE8\x0F\x15\xDC\xD8\x8D\x19!\xB5\xA1hX\xE8\xB8\\Y\x1A\x90` \x01a\x020V[a\x02\xDCa\x04qV[a\x02\xE6`\0a\x04\x9EV[V[a\x02\xF0a\x04qV[`\x05\x81\x90U`@Q\x81\x81R\x7F\xD6\x81\x12\xA8p~2m\x08\xBE6V\xB5(\xC1\xBC\xC5\xBB\xBF\xC4\x7FAw\xE2\x17\x9B\x14\xD8d\x088\xC1\x90` \x01a\x020V[a\x03-a\x04qV[`\x02\x81\x90U`@Q\x81\x81R\x7F5\x1F\xB27W\xBB^\xA0Tl\x85\xB7\x99m\xDDqU\xF9k\x93\x9E\xBA\xA5\xFF{\xC4\x9Cu\xF2\x7F,D\x90` \x01a\x020V[a\x03ja\x04qV[`\x01\x81\x90U`@Q\x81\x81R\x7F\xFC\xDC\xCC`t\xC6\xC4.K\xD5x\xAA\x98p\xC6\x97\xDC\x97j'\thE-+\x8C\x8D\xC3i\xFA\xE3\x96\x90` \x01a\x020V[`\0\x80\x80[\x83Q\x81\x10\x15a\x04\x07W\x83\x81\x81Q\x81\x10a\x03\xBFWa\x03\xBFa\x07CV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16`\0\x03a\x03\xE7Wa\x03\xE0`\x04\x83a\x07YV[\x91Pa\x03\xF5V[a\x03\xF2`\x10\x83a\x07YV[\x91P[\x80a\x03\xFF\x81a\x07lV[\x91PPa\x03\xA4V[P`\0`\x03T\x82a\x04\x18\x91\x90a\x07YV[\x90Pa\x04&\x81a\x04@a\x07YV[\x94\x93PPPPV[a\x046a\x04qV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x04eW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x04n\x81a\x04\x9EV[PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xE6W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x04\\V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\0` \x82\x84\x03\x12\x15a\x05\0W`\0\x80\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0` \x82\x84\x03\x12\x15a\x05/W`\0\x80\xFD[\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x05GW`\0\x80\xFD[\x81\x84\x01\x91P\x84`\x1F\x83\x01\x12a\x05[W`\0\x80\xFD[\x815\x81\x81\x11\x15a\x05mWa\x05ma\x05\x07V[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01\x90\x83\x82\x11\x81\x83\x10\x17\x15a\x05\x95Wa\x05\x95a\x05\x07V[\x81`@R\x82\x81R\x87` \x84\x87\x01\x01\x11\x15a\x05\xAEW`\0\x80\xFD[\x82` \x86\x01` \x83\x017`\0\x92\x81\x01` \x01\x92\x90\x92RP\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\x05\xE0W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05\xF7W`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x06+Wa\x06+a\x05\xFEV[\x92\x91PPV[`\x01\x81\x81[\x80\x85\x11\x15a\x06lW\x81`\0\x19\x04\x82\x11\x15a\x06RWa\x06Ra\x05\xFEV[\x80\x85\x16\x15a\x06_W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x066V[P\x92P\x92\x90PV[`\0\x82a\x06\x83WP`\x01a\x06+V[\x81a\x06\x90WP`\0a\x06+V[\x81`\x01\x81\x14a\x06\xA6W`\x02\x81\x14a\x06\xB0Wa\x06\xCCV[`\x01\x91PPa\x06+V[`\xFF\x84\x11\x15a\x06\xC1Wa\x06\xC1a\x05\xFEV[PP`\x01\x82\x1Ba\x06+V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x06\xEFWP\x81\x81\na\x06+V[a\x06\xF9\x83\x83a\x061V[\x80`\0\x19\x04\x82\x11\x15a\x07\rWa\x07\ra\x05\xFEV[\x02\x93\x92PPPV[`\0a\x05\xF7\x83\x83a\x06tV[`\0\x82a\x07>WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[\x80\x82\x01\x80\x82\x11\x15a\x06+Wa\x06+a\x05\xFEV[`\0`\x01\x82\x01a\x07~Wa\x07~a\x05\xFEV[P`\x01\x01\x90V\xFE\xA2dipfsX\"\x12 \xCB\xC1\x1A\x04\xC2c\xD8\x10J\r\xA19\xF3\x11\r\x85\x14\x17\xBD\xC1\xBCG\xC4\xF5\xD9\x89\xFD$m\x04\xBA\xF9dsolcC\0\x08\x15\x003";
	/// The bytecode of the contract.
	pub static OVM_GASPRICEORACLE_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__BYTECODE);
	#[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xF5W`\x005`\xE0\x1C\x80c\x8C\x88\x85\xC8\x11a\0\x97W\x80c\xDE&\xC4\xA1\x11a\0fW\x80c\xDE&\xC4\xA1\x14a\x01\xBFW\x80c\xF2\xFD\xE3\x8B\x14a\x01\xD2W\x80c\xF4^e\xD8\x14a\x01\xE5W\x80c\xFE\x17;\x97\x14a\x01\xEEW`\0\x80\xFD[\x80c\x8C\x88\x85\xC8\x14a\x01kW\x80c\x8D\xA5\xCB[\x14a\x01~W\x80c\xBE\xDE9\xB5\x14a\x01\x99W\x80c\xBF\x1F\xE4 \x14a\x01\xACW`\0\x80\xFD[\x80cI\x94\x8E\x0E\x11a\0\xD3W\x80cI\x94\x8E\x0E\x14a\x014W\x80cQ\x9BK\xD3\x14a\x01GW\x80cpFU\x97\x14a\x01PW\x80cqP\x18\xA6\x14a\x01cW`\0\x80\xFD[\x80c\x0C\x18\xC1b\x14a\0\xFAW\x80c1<\xE5g\x14a\x01\x16W\x80c5w\xAF\xC5\x14a\x01\x1FW[`\0\x80\xFD[a\x01\x03`\x03T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x01\x03`\x05T\x81V[a\x012a\x01-6`\x04a\x04\xEEV[a\x01\xF7V[\0[a\x01\x03a\x01B6`\x04a\x05\x1DV[a\x02;V[a\x01\x03`\x02T\x81V[a\x012a\x01^6`\x04a\x04\xEEV[a\x02\x97V[a\x012a\x02\xD4V[a\x012a\x01y6`\x04a\x04\xEEV[a\x02\xE8V[`\0T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\rV[a\x012a\x01\xA76`\x04a\x04\xEEV[a\x03%V[a\x012a\x01\xBA6`\x04a\x04\xEEV[a\x03bV[a\x01\x03a\x01\xCD6`\x04a\x05\x1DV[a\x03\x9FV[a\x012a\x01\xE06`\x04a\x05\xCEV[a\x04.V[a\x01\x03`\x04T\x81V[a\x01\x03`\x01T\x81V[a\x01\xFFa\x04qV[`\x03\x81\x90U`@Q\x81\x81R\x7F2t\x0B5\xC0\xEA!6P\xF6\rD6kO\xB2\x11\xC9\x03;PqNJ\x1D4\xE6][\xEB\x9B\xB4\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\0\x80a\x02G\x83a\x03\x9FV[\x90P`\0`\x02T\x82a\x02Y\x91\x90a\x06\x14V[\x90P`\0`\x05T`\na\x02l\x91\x90a\x07\x15V[\x90P`\0`\x04T\x83a\x02~\x91\x90a\x06\x14V[\x90P`\0a\x02\x8C\x83\x83a\x07!V[\x97\x96PPPPPPPV[a\x02\x9Fa\x04qV[`\x04\x81\x90U`@Q\x81\x81R\x7F36\xCD\x97\x08\xEA\xF2v\x9A\x0F\r\xC0g\x9F0\xE8\x0F\x15\xDC\xD8\x8D\x19!\xB5\xA1hX\xE8\xB8\\Y\x1A\x90` \x01a\x020V[a\x02\xDCa\x04qV[a\x02\xE6`\0a\x04\x9EV[V[a\x02\xF0a\x04qV[`\x05\x81\x90U`@Q\x81\x81R\x7F\xD6\x81\x12\xA8p~2m\x08\xBE6V\xB5(\xC1\xBC\xC5\xBB\xBF\xC4\x7FAw\xE2\x17\x9B\x14\xD8d\x088\xC1\x90` \x01a\x020V[a\x03-a\x04qV[`\x02\x81\x90U`@Q\x81\x81R\x7F5\x1F\xB27W\xBB^\xA0Tl\x85\xB7\x99m\xDDqU\xF9k\x93\x9E\xBA\xA5\xFF{\xC4\x9Cu\xF2\x7F,D\x90` \x01a\x020V[a\x03ja\x04qV[`\x01\x81\x90U`@Q\x81\x81R\x7F\xFC\xDC\xCC`t\xC6\xC4.K\xD5x\xAA\x98p\xC6\x97\xDC\x97j'\thE-+\x8C\x8D\xC3i\xFA\xE3\x96\x90` \x01a\x020V[`\0\x80\x80[\x83Q\x81\x10\x15a\x04\x07W\x83\x81\x81Q\x81\x10a\x03\xBFWa\x03\xBFa\x07CV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16`\0\x03a\x03\xE7Wa\x03\xE0`\x04\x83a\x07YV[\x91Pa\x03\xF5V[a\x03\xF2`\x10\x83a\x07YV[\x91P[\x80a\x03\xFF\x81a\x07lV[\x91PPa\x03\xA4V[P`\0`\x03T\x82a\x04\x18\x91\x90a\x07YV[\x90Pa\x04&\x81a\x04@a\x07YV[\x94\x93PPPPV[a\x046a\x04qV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x04eW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x04n\x81a\x04\x9EV[PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xE6W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x04\\V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\0` \x82\x84\x03\x12\x15a\x05\0W`\0\x80\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0` \x82\x84\x03\x12\x15a\x05/W`\0\x80\xFD[\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x05GW`\0\x80\xFD[\x81\x84\x01\x91P\x84`\x1F\x83\x01\x12a\x05[W`\0\x80\xFD[\x815\x81\x81\x11\x15a\x05mWa\x05ma\x05\x07V[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01\x90\x83\x82\x11\x81\x83\x10\x17\x15a\x05\x95Wa\x05\x95a\x05\x07V[\x81`@R\x82\x81R\x87` \x84\x87\x01\x01\x11\x15a\x05\xAEW`\0\x80\xFD[\x82` \x86\x01` \x83\x017`\0\x92\x81\x01` \x01\x92\x90\x92RP\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\x05\xE0W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05\xF7W`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x06+Wa\x06+a\x05\xFEV[\x92\x91PPV[`\x01\x81\x81[\x80\x85\x11\x15a\x06lW\x81`\0\x19\x04\x82\x11\x15a\x06RWa\x06Ra\x05\xFEV[\x80\x85\x16\x15a\x06_W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x066V[P\x92P\x92\x90PV[`\0\x82a\x06\x83WP`\x01a\x06+V[\x81a\x06\x90WP`\0a\x06+V[\x81`\x01\x81\x14a\x06\xA6W`\x02\x81\x14a\x06\xB0Wa\x06\xCCV[`\x01\x91PPa\x06+V[`\xFF\x84\x11\x15a\x06\xC1Wa\x06\xC1a\x05\xFEV[PP`\x01\x82\x1Ba\x06+V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x06\xEFWP\x81\x81\na\x06+V[a\x06\xF9\x83\x83a\x061V[\x80`\0\x19\x04\x82\x11\x15a\x07\rWa\x07\ra\x05\xFEV[\x02\x93\x92PPPV[`\0a\x05\xF7\x83\x83a\x06tV[`\0\x82a\x07>WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[\x80\x82\x01\x80\x82\x11\x15a\x06+Wa\x06+a\x05\xFEV[`\0`\x01\x82\x01a\x07~Wa\x07~a\x05\xFEV[P`\x01\x01\x90V\xFE\xA2dipfsX\"\x12 \xCB\xC1\x1A\x04\xC2c\xD8\x10J\r\xA19\xF3\x11\r\x85\x14\x17\xBD\xC1\xBCG\xC4\xF5\xD9\x89\xFD$m\x04\xBA\xF9dsolcC\0\x08\x15\x003";
	/// The deployed bytecode of the contract.
	pub static OVM_GASPRICEORACLE_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
	pub struct OVM_gasPriceOracle<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for OVM_gasPriceOracle<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for OVM_gasPriceOracle<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for OVM_gasPriceOracle<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for OVM_gasPriceOracle<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(OVM_gasPriceOracle))
				.field(&self.address())
				.finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> OVM_gasPriceOracle<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(
				address.into(),
				OVM_GASPRICEORACLE_ABI.clone(),
				client,
			))
		}
		/// Constructs the general purpose `Deployer` instance based on the provided constructor
		/// arguments and sends it. Returns a new instance of a deployer that returns an instance of
		/// this contract after sending the transaction
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
				OVM_GASPRICEORACLE_ABI.clone(),
				OVM_GASPRICEORACLE_BYTECODE.clone().into(),
				client,
			);
			let deployer = factory.deploy(constructor_args)?;
			let deployer = ::ethers::contract::ContractDeployer::new(deployer);
			Ok(deployer)
		}
		///Calls the contract's `decimals` (0x313ce567) function
		pub fn decimals(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([49, 60, 229, 103], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gasPrice` (0xfe173b97) function
		pub fn gas_price(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([254, 23, 59, 151], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1Fee` (0x49948e0e) function
		pub fn get_l1_fee(
			&self,
			data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([73, 148, 142, 14], data)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1GasUsed` (0xde26c4a1) function
		pub fn get_l1_gas_used(
			&self,
			data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([222, 38, 196, 161], data)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l1BaseFee` (0x519b4bd3) function
		pub fn l_1_base_fee(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([81, 155, 75, 211], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `overhead` (0x0c18c162) function
		pub fn overhead(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([12, 24, 193, 98], ())
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
		///Calls the contract's `scalar` (0xf45e65d8) function
		pub fn scalar(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([244, 94, 101, 216], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setDecimals` (0x8c8885c8) function
		pub fn set_decimals(
			&self,
			decimals: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([140, 136, 133, 200], decimals)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setGasPrice` (0xbf1fe420) function
		pub fn set_gas_price(
			&self,
			gas_price: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([191, 31, 228, 32], gas_price)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setL1BaseFee` (0xbede39b5) function
		pub fn set_l1_base_fee(
			&self,
			base_fee: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([190, 222, 57, 181], base_fee)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setOverhead` (0x3577afc5) function
		pub fn set_overhead(
			&self,
			overhead: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([53, 119, 175, 197], overhead)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setScalar` (0x70465597) function
		pub fn set_scalar(
			&self,
			scalar: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([112, 70, 85, 151], scalar)
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
		///Gets the contract's `DecimalsUpdated` event
		pub fn decimals_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, DecimalsUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `GasPriceUpdated` event
		pub fn gas_price_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GasPriceUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `L1BaseFeeUpdated` event
		pub fn l1_base_fee_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, L1BaseFeeUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `OverheadUpdated` event
		pub fn overhead_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OverheadUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `OwnershipTransferred` event
		pub fn ownership_transferred_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OwnershipTransferredFilter>
		{
			self.0.event()
		}
		///Gets the contract's `ScalarUpdated` event
		pub fn scalar_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ScalarUpdatedFilter> {
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OVM_gasPriceOracleEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
		for OVM_gasPriceOracle<M>
	{
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Custom Error type `OwnableInvalidOwner` with signature `OwnableInvalidOwner(address)` and
	/// selector `0x1e4fbdf7`
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
	#[etherror(name = "OwnableInvalidOwner", abi = "OwnableInvalidOwner(address)")]
	pub struct OwnableInvalidOwner {
		pub owner: ::ethers::core::types::Address,
	}
	///Custom Error type `OwnableUnauthorizedAccount` with signature
	/// `OwnableUnauthorizedAccount(address)` and selector `0x118cdaa7`
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
	#[etherror(name = "OwnableUnauthorizedAccount", abi = "OwnableUnauthorizedAccount(address)")]
	pub struct OwnableUnauthorizedAccount {
		pub account: ::ethers::core::types::Address,
	}
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum OVM_gasPriceOracleErrors {
		OwnableInvalidOwner(OwnableInvalidOwner),
		OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for OVM_gasPriceOracleErrors {
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
				<OwnableInvalidOwner as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OwnableInvalidOwner(decoded));
			}
			if let Ok(decoded) =
				<OwnableUnauthorizedAccount as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OwnableUnauthorizedAccount(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for OVM_gasPriceOracleErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::OwnableInvalidOwner(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::OwnableUnauthorizedAccount(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for OVM_gasPriceOracleErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<OwnableInvalidOwner as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<OwnableUnauthorizedAccount as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for OVM_gasPriceOracleErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::OwnableInvalidOwner(element) => ::core::fmt::Display::fmt(element, f),
				Self::OwnableUnauthorizedAccount(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for OVM_gasPriceOracleErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<OwnableInvalidOwner> for OVM_gasPriceOracleErrors {
		fn from(value: OwnableInvalidOwner) -> Self {
			Self::OwnableInvalidOwner(value)
		}
	}
	impl ::core::convert::From<OwnableUnauthorizedAccount> for OVM_gasPriceOracleErrors {
		fn from(value: OwnableUnauthorizedAccount) -> Self {
			Self::OwnableUnauthorizedAccount(value)
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
	#[ethevent(name = "DecimalsUpdated", abi = "DecimalsUpdated(uint256)")]
	pub struct DecimalsUpdatedFilter(pub ::ethers::core::types::U256);
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
	#[ethevent(name = "GasPriceUpdated", abi = "GasPriceUpdated(uint256)")]
	pub struct GasPriceUpdatedFilter(pub ::ethers::core::types::U256);
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
	#[ethevent(name = "L1BaseFeeUpdated", abi = "L1BaseFeeUpdated(uint256)")]
	pub struct L1BaseFeeUpdatedFilter(pub ::ethers::core::types::U256);
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
	#[ethevent(name = "OverheadUpdated", abi = "OverheadUpdated(uint256)")]
	pub struct OverheadUpdatedFilter(pub ::ethers::core::types::U256);
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
	#[ethevent(name = "ScalarUpdated", abi = "ScalarUpdated(uint256)")]
	pub struct ScalarUpdatedFilter(pub ::ethers::core::types::U256);
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum OVM_gasPriceOracleEvents {
		DecimalsUpdatedFilter(DecimalsUpdatedFilter),
		GasPriceUpdatedFilter(GasPriceUpdatedFilter),
		L1BaseFeeUpdatedFilter(L1BaseFeeUpdatedFilter),
		OverheadUpdatedFilter(OverheadUpdatedFilter),
		OwnershipTransferredFilter(OwnershipTransferredFilter),
		ScalarUpdatedFilter(ScalarUpdatedFilter),
	}
	impl ::ethers::contract::EthLogDecode for OVM_gasPriceOracleEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = DecimalsUpdatedFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::DecimalsUpdatedFilter(decoded));
			}
			if let Ok(decoded) = GasPriceUpdatedFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::GasPriceUpdatedFilter(decoded));
			}
			if let Ok(decoded) = L1BaseFeeUpdatedFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::L1BaseFeeUpdatedFilter(decoded));
			}
			if let Ok(decoded) = OverheadUpdatedFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::OverheadUpdatedFilter(decoded));
			}
			if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::OwnershipTransferredFilter(decoded));
			}
			if let Ok(decoded) = ScalarUpdatedFilter::decode_log(log) {
				return Ok(OVM_gasPriceOracleEvents::ScalarUpdatedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for OVM_gasPriceOracleEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::DecimalsUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::GasPriceUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::L1BaseFeeUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::OverheadUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::OwnershipTransferredFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::ScalarUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<DecimalsUpdatedFilter> for OVM_gasPriceOracleEvents {
		fn from(value: DecimalsUpdatedFilter) -> Self {
			Self::DecimalsUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<GasPriceUpdatedFilter> for OVM_gasPriceOracleEvents {
		fn from(value: GasPriceUpdatedFilter) -> Self {
			Self::GasPriceUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<L1BaseFeeUpdatedFilter> for OVM_gasPriceOracleEvents {
		fn from(value: L1BaseFeeUpdatedFilter) -> Self {
			Self::L1BaseFeeUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<OverheadUpdatedFilter> for OVM_gasPriceOracleEvents {
		fn from(value: OverheadUpdatedFilter) -> Self {
			Self::OverheadUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<OwnershipTransferredFilter> for OVM_gasPriceOracleEvents {
		fn from(value: OwnershipTransferredFilter) -> Self {
			Self::OwnershipTransferredFilter(value)
		}
	}
	impl ::core::convert::From<ScalarUpdatedFilter> for OVM_gasPriceOracleEvents {
		fn from(value: ScalarUpdatedFilter) -> Self {
			Self::ScalarUpdatedFilter(value)
		}
	}
	///Container type for all input parameters for the `decimals` function with signature
	/// `decimals()` and selector `0x313ce567`
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
	#[ethcall(name = "decimals", abi = "decimals()")]
	pub struct DecimalsCall;
	///Container type for all input parameters for the `gasPrice` function with signature
	/// `gasPrice()` and selector `0xfe173b97`
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
	#[ethcall(name = "gasPrice", abi = "gasPrice()")]
	pub struct GasPriceCall;
	///Container type for all input parameters for the `getL1Fee` function with signature
	/// `getL1Fee(bytes)` and selector `0x49948e0e`
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
	#[ethcall(name = "getL1Fee", abi = "getL1Fee(bytes)")]
	pub struct GetL1FeeCall {
		pub data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `getL1GasUsed` function with signature
	/// `getL1GasUsed(bytes)` and selector `0xde26c4a1`
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
	#[ethcall(name = "getL1GasUsed", abi = "getL1GasUsed(bytes)")]
	pub struct GetL1GasUsedCall {
		pub data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `l1BaseFee` function with signature
	/// `l1BaseFee()` and selector `0x519b4bd3`
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
	#[ethcall(name = "l1BaseFee", abi = "l1BaseFee()")]
	pub struct L1BaseFeeCall;
	///Container type for all input parameters for the `overhead` function with signature
	/// `overhead()` and selector `0x0c18c162`
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
	#[ethcall(name = "overhead", abi = "overhead()")]
	pub struct OverheadCall;
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
	///Container type for all input parameters for the `scalar` function with signature `scalar()`
	/// and selector `0xf45e65d8`
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
	#[ethcall(name = "scalar", abi = "scalar()")]
	pub struct ScalarCall;
	///Container type for all input parameters for the `setDecimals` function with signature
	/// `setDecimals(uint256)` and selector `0x8c8885c8`
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
	#[ethcall(name = "setDecimals", abi = "setDecimals(uint256)")]
	pub struct SetDecimalsCall {
		pub decimals: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `setGasPrice` function with signature
	/// `setGasPrice(uint256)` and selector `0xbf1fe420`
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
	#[ethcall(name = "setGasPrice", abi = "setGasPrice(uint256)")]
	pub struct SetGasPriceCall {
		pub gas_price: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `setL1BaseFee` function with signature
	/// `setL1BaseFee(uint256)` and selector `0xbede39b5`
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
	#[ethcall(name = "setL1BaseFee", abi = "setL1BaseFee(uint256)")]
	pub struct SetL1BaseFeeCall {
		pub base_fee: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `setOverhead` function with signature
	/// `setOverhead(uint256)` and selector `0x3577afc5`
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
	#[ethcall(name = "setOverhead", abi = "setOverhead(uint256)")]
	pub struct SetOverheadCall {
		pub overhead: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `setScalar` function with signature
	/// `setScalar(uint256)` and selector `0x70465597`
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
	#[ethcall(name = "setScalar", abi = "setScalar(uint256)")]
	pub struct SetScalarCall {
		pub scalar: ::ethers::core::types::U256,
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
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum OVM_gasPriceOracleCalls {
		Decimals(DecimalsCall),
		GasPrice(GasPriceCall),
		GetL1Fee(GetL1FeeCall),
		GetL1GasUsed(GetL1GasUsedCall),
		L1BaseFee(L1BaseFeeCall),
		Overhead(OverheadCall),
		Owner(OwnerCall),
		RenounceOwnership(RenounceOwnershipCall),
		Scalar(ScalarCall),
		SetDecimals(SetDecimalsCall),
		SetGasPrice(SetGasPriceCall),
		SetL1BaseFee(SetL1BaseFeeCall),
		SetOverhead(SetOverheadCall),
		SetScalar(SetScalarCall),
		TransferOwnership(TransferOwnershipCall),
	}
	impl ::ethers::core::abi::AbiDecode for OVM_gasPriceOracleCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <DecimalsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Decimals(decoded));
			}
			if let Ok(decoded) = <GasPriceCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GasPrice(decoded));
			}
			if let Ok(decoded) = <GetL1FeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetL1Fee(decoded));
			}
			if let Ok(decoded) = <GetL1GasUsedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1GasUsed(decoded));
			}
			if let Ok(decoded) = <L1BaseFeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::L1BaseFee(decoded));
			}
			if let Ok(decoded) = <OverheadCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Overhead(decoded));
			}
			if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Owner(decoded));
			}
			if let Ok(decoded) =
				<RenounceOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RenounceOwnership(decoded));
			}
			if let Ok(decoded) = <ScalarCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Scalar(decoded));
			}
			if let Ok(decoded) = <SetDecimalsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetDecimals(decoded));
			}
			if let Ok(decoded) = <SetGasPriceCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetGasPrice(decoded));
			}
			if let Ok(decoded) = <SetL1BaseFeeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetL1BaseFee(decoded));
			}
			if let Ok(decoded) = <SetOverheadCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetOverhead(decoded));
			}
			if let Ok(decoded) = <SetScalarCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetScalar(decoded));
			}
			if let Ok(decoded) =
				<TransferOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::TransferOwnership(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for OVM_gasPriceOracleCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::Decimals(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GasPrice(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1Fee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1GasUsed(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L1BaseFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Overhead(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RenounceOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Scalar(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetDecimals(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetGasPrice(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetL1BaseFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetOverhead(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetScalar(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::TransferOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for OVM_gasPriceOracleCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Decimals(element) => ::core::fmt::Display::fmt(element, f),
				Self::GasPrice(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1Fee(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1GasUsed(element) => ::core::fmt::Display::fmt(element, f),
				Self::L1BaseFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::Overhead(element) => ::core::fmt::Display::fmt(element, f),
				Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
				Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
				Self::Scalar(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetDecimals(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetGasPrice(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetL1BaseFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetOverhead(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetScalar(element) => ::core::fmt::Display::fmt(element, f),
				Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<DecimalsCall> for OVM_gasPriceOracleCalls {
		fn from(value: DecimalsCall) -> Self {
			Self::Decimals(value)
		}
	}
	impl ::core::convert::From<GasPriceCall> for OVM_gasPriceOracleCalls {
		fn from(value: GasPriceCall) -> Self {
			Self::GasPrice(value)
		}
	}
	impl ::core::convert::From<GetL1FeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: GetL1FeeCall) -> Self {
			Self::GetL1Fee(value)
		}
	}
	impl ::core::convert::From<GetL1GasUsedCall> for OVM_gasPriceOracleCalls {
		fn from(value: GetL1GasUsedCall) -> Self {
			Self::GetL1GasUsed(value)
		}
	}
	impl ::core::convert::From<L1BaseFeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: L1BaseFeeCall) -> Self {
			Self::L1BaseFee(value)
		}
	}
	impl ::core::convert::From<OverheadCall> for OVM_gasPriceOracleCalls {
		fn from(value: OverheadCall) -> Self {
			Self::Overhead(value)
		}
	}
	impl ::core::convert::From<OwnerCall> for OVM_gasPriceOracleCalls {
		fn from(value: OwnerCall) -> Self {
			Self::Owner(value)
		}
	}
	impl ::core::convert::From<RenounceOwnershipCall> for OVM_gasPriceOracleCalls {
		fn from(value: RenounceOwnershipCall) -> Self {
			Self::RenounceOwnership(value)
		}
	}
	impl ::core::convert::From<ScalarCall> for OVM_gasPriceOracleCalls {
		fn from(value: ScalarCall) -> Self {
			Self::Scalar(value)
		}
	}
	impl ::core::convert::From<SetDecimalsCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetDecimalsCall) -> Self {
			Self::SetDecimals(value)
		}
	}
	impl ::core::convert::From<SetGasPriceCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetGasPriceCall) -> Self {
			Self::SetGasPrice(value)
		}
	}
	impl ::core::convert::From<SetL1BaseFeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetL1BaseFeeCall) -> Self {
			Self::SetL1BaseFee(value)
		}
	}
	impl ::core::convert::From<SetOverheadCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetOverheadCall) -> Self {
			Self::SetOverhead(value)
		}
	}
	impl ::core::convert::From<SetScalarCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetScalarCall) -> Self {
			Self::SetScalar(value)
		}
	}
	impl ::core::convert::From<TransferOwnershipCall> for OVM_gasPriceOracleCalls {
		fn from(value: TransferOwnershipCall) -> Self {
			Self::TransferOwnership(value)
		}
	}
	///Container type for all return fields from the `decimals` function with signature
	/// `decimals()` and selector `0x313ce567`
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
	pub struct DecimalsReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `gasPrice` function with signature
	/// `gasPrice()` and selector `0xfe173b97`
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
	pub struct GasPriceReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1Fee` function with signature
	/// `getL1Fee(bytes)` and selector `0x49948e0e`
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
	pub struct GetL1FeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1GasUsed` function with signature
	/// `getL1GasUsed(bytes)` and selector `0xde26c4a1`
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
	pub struct GetL1GasUsedReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `l1BaseFee` function with signature
	/// `l1BaseFee()` and selector `0x519b4bd3`
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
	pub struct L1BaseFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `overhead` function with signature
	/// `overhead()` and selector `0x0c18c162`
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
	pub struct OverheadReturn(pub ::ethers::core::types::U256);
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
	///Container type for all return fields from the `scalar` function with signature `scalar()`
	/// and selector `0xf45e65d8`
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
	pub struct ScalarReturn(pub ::ethers::core::types::U256);
}
