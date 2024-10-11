pub use host_manager::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types,
)]
pub mod host_manager {
	pub use super::super::shared_types::*;
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
				inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
					name: ::std::borrow::ToOwned::to_owned("managerParams"),
					kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
						::ethers::core::abi::ethabi::ParamType::Address,
						::ethers::core::abi::ethabi::ParamType::Address,
					],),
					internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
						"struct HostManagerParams"
					),),
				},],
			}),
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("host"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("host"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("h"),
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
					::std::borrow::ToOwned::to_owned("onAccept"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onAccept"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("incoming"),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
								],),
								::ethers::core::abi::ethabi::ParamType::Address,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct IncomingPostRequest",),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("onGetResponse"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onGetResponse"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Address,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Array(
											::std::boxed::Box::new(
												::ethers::core::abi::ethabi::ParamType::Bytes,
											),
										),
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
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
								::ethers::core::abi::ethabi::ParamType::Address,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct IncomingGetResponse",),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("onGetTimeout"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onGetTimeout"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Address,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Array(
									::std::boxed::Box::new(
										::ethers::core::abi::ethabi::ParamType::Bytes,
									),
								),
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Bytes,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct GetRequest"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("onPostRequestTimeout"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onPostRequestTimeout",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Bytes,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct PostRequest"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("onPostResponse"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onPostResponse"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
									],),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								],),
								::ethers::core::abi::ethabi::ParamType::Address,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct IncomingPostResponse",),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("onPostResponseTimeout"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("onPostResponseTimeout",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
								],),
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct PostResponse"),
							),
						},],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("params"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("params"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
								::ethers::core::abi::ethabi::ParamType::Address,
								::ethers::core::abi::ethabi::ParamType::Address,
							],),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("struct HostManagerParams"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("quote"),
					::std::vec![
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("quote"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("post"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::Address,
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct DispatchPost"),
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
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("quote"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("get"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Array(
										::std::boxed::Box::new(
											::ethers::core::abi::ethabi::ParamType::Bytes,
										),
									),
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::Bytes,
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct DispatchGet"),
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
						},
						::ethers::core::abi::ethabi::Function {
							name: ::std::borrow::ToOwned::to_owned("quote"),
							inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
								name: ::std::borrow::ToOwned::to_owned("res"),
								kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
									::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Bytes,
									],),
									::ethers::core::abi::ethabi::ParamType::Bytes,
									::ethers::core::abi::ethabi::ParamType::Uint(64usize),
									::ethers::core::abi::ethabi::ParamType::Uint(256usize),
									::ethers::core::abi::ethabi::ParamType::Address,
								],),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("struct DispatchPostResponse",),
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
						},
					],
				),
				(
					::std::borrow::ToOwned::to_owned("setIsmpHost"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setIsmpHost"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("host"),
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
					::std::borrow::ToOwned::to_owned("supportsInterface"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("supportsInterface"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("interfaceId"),
							kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(4usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes4"),
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
			]),
			events: ::std::collections::BTreeMap::new(),
			errors: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnauthorizedCall"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnauthorizedCall"),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnexpectedCall"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnexpectedCall"),
						inputs: ::std::vec![],
					},],
				),
			]),
			receive: true,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static HOSTMANAGER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	#[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x10W_\x80\xFD[P`@Qb\0\x1Ce8\x03\x80b\0\x1Ce\x839\x81\x01`@\x81\x90Rb\0\x003\x91b\0\x02fV[_b\0\0>b\0\x01cV[\x90P`\x01`\x01`\xA0\x1B\x03\x81\x16\x15b\0\x01*W\x80`\x01`\x01`\xA0\x1B\x03\x16cdxF\xA5`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15b\0\0\x8DW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90b\0\0\xB3\x91\x90b\0\x02\xD0V[`@Qc\t^\xA7\xB3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R_\x19`$\x83\x01R\x91\x90\x91\x16\x90c\t^\xA7\xB3\x90`D\x01` `@Q\x80\x83\x03\x81_\x87Z\xF1\x15\x80\x15b\0\x01\x02W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90b\0\x01(\x91\x90b\0\x02\xF3V[P[P\x80Q_\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ub\0\x03\x14V[_Fb\xAA6\xA7\x81\x14b\0\x01\xA8Wb\x06n\xEE\x81\x14b\0\x01\xC3Wb\xAA7\xDC\x81\x14b\0\x01\xDEWb\x01J4\x81\x14b\0\x01\xF9W`a\x81\x14b\0\x02\x14Wa'\xD8\x81\x14b\0\x02/WP\x90V[s'\xB0\xC6\x96\x0By*\x8D\xCB\x01\xF0e+\xDEH\x01\\\xD5\xF2>\x91PP\x90V[s\xFD~+*\xD0\xB2\x9E\xC8\x17\xDC}@h\x81\xB2%\xB8\x1D\xBF\xCF\x91PP\x90V[s0\xE3\xAF\x17G\xB1U\xF3\x7F\x93^\x0E\xC9\x95\xDE^\xA4\xE6u\x86\x91PP\x90V[s\rp7\xBD\x9C\xEA\xEF%\xE5!_\x80\x8D0\x9A\xDD\ne\xCD\xB9\x91PP\x90V[sL\xB0\xF5u\x0Fo\xE1MK\x86\xAC\xA6\xFE\x12iC\xBD\xA3\xC8\xC4\x91PP\x90V[s\x11\xEB\x87\xC7E\xD9zO\xA8\xAE\xC8\x055\x987E\x9D$\r\x1B\x91PP\x90V[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x02aW_\x80\xFD[\x91\x90PV[_`@\x82\x84\x03\x12\x15b\0\x02wW_\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x02\xA6WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Rb\0\x02\xB4\x83b\0\x02JV[\x81Rb\0\x02\xC4` \x84\x01b\0\x02JV[` \x82\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15b\0\x02\xE1W_\x80\xFD[b\0\x02\xEC\x82b\0\x02JV[\x93\x92PPPV[_` \x82\x84\x03\x12\x15b\0\x03\x04W_\x80\xFD[\x81Q\x80\x15\x15\x81\x14b\0\x02\xECW_\x80\xFD[a\x19C\x80b\0\x03\"_9_\xF3\xFE`\x80`@R`\x046\x10a\0\xC2W_5`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0|W\x80c\xCF\xF0\xAB\x96\x11a\0WW\x80c\xCF\xF0\xAB\x96\x14a\x01\xFAW\x80c\xD0\xFF\xF3f\x14a\x02RW\x80c\xDD\x92\xA3\x16\x14a\x02lW\x80c\xF47\xBCY\x14a\x02\x8BW_\x80\xFD[\x80c\xB2\xA0\x1B\xF5\x14a\x01\xA7W\x80c\xBC\r\xD4G\x14a\x01\xC1W\x80c\xBC\xA9l9\x14a\x01\xDBW_\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\xCDW\x80c\x0B\xC3{\xAB\x14a\x01\x01W\x80c\x0E\x83$\xA2\x14a\x01\"W\x80c\x0F\xEE2\xCE\x14a\x01AW\x80c\x10\x8B\xC1\xDD\x14a\x01`W\x80cD\xAB \xF8\x14a\x01\x8DW_\x80\xFD[6a\0\xC9W\0[_\x80\xFD[4\x80\x15a\0\xD8W_\x80\xFD[Pa\0\xECa\0\xE76`\x04a\x08\xD5V[a\x02\xB7V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x0CW_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x0CCV[a\x02\xEDV[\0[4\x80\x15a\x01-W_\x80\xFD[Pa\x01 a\x01<6`\x04a\x0C\x92V[a\x03?V[4\x80\x15a\x01LW_\x80\xFD[Pa\x01 a\x01[6`\x04a\x0C\xABV[a\x03\x93V[4\x80\x15a\x01kW_\x80\xFD[Pa\x01\x7Fa\x01z6`\x04a\x0C\xE1V[a\x06FV[`@Q\x90\x81R` \x01a\0\xF8V[4\x80\x15a\x01\x98W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x0FYV[4\x80\x15a\x01\xB2W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x10\xF3V[4\x80\x15a\x01\xCCW_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x11]V[4\x80\x15a\x01\xE6W_\x80\xFD[Pa\x01\x7Fa\x01\xF56`\x04a\x11\x8EV[a\x06\xCDV[4\x80\x15a\x02\x05W_\x80\xFD[P`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xF8V[4\x80\x15a\x02]W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x12gV[4\x80\x15a\x02wW_\x80\xFD[Pa\x01\x7Fa\x02\x866`\x04a\x12\x98V[a\x07EV[4\x80\x15a\x02\x96W_\x80\xFD[Pa\x02\x9Fa\x07\xCCV[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\xF8V[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02\xE7WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x02\xF5a\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x03&W`@Qc{\xF6\xA1o`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03jW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U_\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03\xBFW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\xCA\x83\x80a\x13NV[\x90Pa\x04\x8C_`\x01\x01_\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01_`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\x1FW=_\x80>=_\xFD[PPPP`@Q=_\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04F\x91\x90\x81\x01\x90a\x13\x8EV[a\x04P\x83\x80a\x13\xFFV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847_\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x08\xAD\x90PV[a\x04\xA9W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04\xB7`\xC0\x83\x01\x83a\x13\xFFV[_\x81\x81\x10a\x04\xC7Wa\x04\xC7a\x14HV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xE2Wa\x04\xE2a\x14\\V[\x90P_\x81`\x01\x81\x11\x15a\x04\xF7Wa\x04\xF7a\x14\\V[\x03a\x05\x9EW_a\x05\n`\xC0\x84\x01\x84a\x13\xFFV[a\x05\x18\x91`\x01\x90\x82\x90a\x14pV[\x81\x01\x90a\x05%\x91\x90a\x14\x97V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05\x82W_\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x94W=_\x80>=_\xFD[PPPPPa\x06@V[`\x01\x81`\x01\x81\x11\x15a\x05\xB2Wa\x05\xB2a\x14\\V[\x03a\x06@W_a\x05\xC5`\xC0\x84\x01\x84a\x13\xFFV[a\x05\xD3\x91`\x01\x90\x82\x90a\x14pV[\x81\x01\x90a\x05\xE0\x91\x90a\x15\x99V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x06\x11\x90\x84\x90`\x04\x01a\x17\x85V[_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x06(W_\x80\xFD[PZ\xF1\x15\x80\x15a\x06:W=_\x80>=_\xFD[PPPPP[PPPPV[_a\x06Oa\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\x8AW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06\xAE\x91\x90a\x18\xB8V[\x82`@\x01QQa\x06\xBE\x91\x90a\x18\xE3V[\x82`\x80\x01Qa\x02\xE7\x91\x90a\x18\xFAV[_a\x06\xD6a\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x07\x11W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x075\x91\x90a\x18\xB8V[\x82`\xA0\x01QQa\x06\xBE\x91\x90a\x18\xE3V[_a\x07Na\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x07\x89W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x07\xAD\x91\x90a\x18\xB8V[\x82` \x01QQa\x07\xBD\x91\x90a\x18\xE3V[\x82``\x01Qa\x02\xE7\x91\x90a\x18\xFAV[_Fb\xAA6\xA7\x81\x14a\x08\x0BWb\x06n\xEE\x81\x14a\x08&Wb\xAA7\xDC\x81\x14a\x08AWb\x01J4\x81\x14a\x08\\W`a\x81\x14a\x08wWa'\xD8\x81\x14a\x08\x92WP\x90V[s'\xB0\xC6\x96\x0By*\x8D\xCB\x01\xF0e+\xDEH\x01\\\xD5\xF2>\x91PP\x90V[s\xFD~+*\xD0\xB2\x9E\xC8\x17\xDC}@h\x81\xB2%\xB8\x1D\xBF\xCF\x91PP\x90V[s0\xE3\xAF\x17G\xB1U\xF3\x7F\x93^\x0E\xC9\x95\xDE^\xA4\xE6u\x86\x91PP\x90V[s\rp7\xBD\x9C\xEA\xEF%\xE5!_\x80\x8D0\x9A\xDD\ne\xCD\xB9\x91PP\x90V[sL\xB0\xF5u\x0Fo\xE1MK\x86\xAC\xA6\xFE\x12iC\xBD\xA3\xC8\xC4\x91PP\x90V[s\x11\xEB\x87\xC7E\xD9zO\xA8\xAE\xC8\x055\x987E\x9D$\r\x1B\x91PP\x90V[_\x81Q\x83Q\x14a\x08\xBEWP_a\x02\xE7V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[_` \x82\x84\x03\x12\x15a\x08\xE5W_\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08\xFCW_\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@R\x90V[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\n5Wa\n5a\t\x03V[`@R\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\nUWa\nUa\t\x03V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[_\x82`\x1F\x83\x01\x12a\nrW_\x80\xFD[\x815a\n\x85a\n\x80\x82a\n=V[a\n\rV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\n\x99W_\x80\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\n\xCBW_\x80\xFD[\x91\x90PV[_`\xE0\x82\x84\x03\x12\x15a\n\xE0W_\x80\xFD[a\n\xE8a\t\x17V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\0W_\x80\xFD[a\x0B\x0C\x85\x83\x86\x01a\ncV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B!W_\x80\xFD[a\x0B-\x85\x83\x86\x01a\ncV[` \x84\x01Ra\x0B>`@\x85\x01a\n\xB5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0BVW_\x80\xFD[a\x0Bb\x85\x83\x86\x01a\ncV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x0BzW_\x80\xFD[a\x0B\x86\x85\x83\x86\x01a\ncV[`\x80\x84\x01Ra\x0B\x97`\xA0\x85\x01a\n\xB5V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xAFW_\x80\xFD[Pa\x0B\xBC\x84\x82\x85\x01a\ncV[`\xC0\x83\x01RP\x92\x91PPV[_``\x82\x84\x03\x12\x15a\x0B\xD8W_\x80\xFD[a\x0B\xE0a\t?V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xF8W_\x80\xFD[a\x0C\x04\x85\x83\x86\x01a\n\xD0V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0C\x19W_\x80\xFD[Pa\x0C&\x84\x82\x85\x01a\ncV[` \x83\x01RPa\x0C8`@\x83\x01a\n\xB5V[`@\x82\x01R\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0CSW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0ChW_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\x0B\xC8V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n\xCBW_\x80\xFD[_` \x82\x84\x03\x12\x15a\x0C\xA2W_\x80\xFD[a\x08\xFC\x82a\x0C|V[_` \x82\x84\x03\x12\x15a\x0C\xBBW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xD0W_\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x08\xFCW_\x80\xFD[_` \x82\x84\x03\x12\x15a\x0C\xF1W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\r\x07W_\x80\xFD[\x90\x83\x01\x90`\xC0\x82\x86\x03\x12\x15a\r\x1AW_\x80\xFD[a\r\"a\taV[\x825\x82\x81\x11\x15a\r0W_\x80\xFD[a\r<\x87\x82\x86\x01a\ncV[\x82RP` \x83\x015\x82\x81\x11\x15a\rPW_\x80\xFD[a\r\\\x87\x82\x86\x01a\ncV[` \x83\x01RP`@\x83\x015\x82\x81\x11\x15a\rsW_\x80\xFD[a\r\x7F\x87\x82\x86\x01a\ncV[`@\x83\x01RPa\r\x91``\x84\x01a\n\xB5V[``\x82\x01R`\x80\x83\x015`\x80\x82\x01Ra\r\xAC`\xA0\x84\x01a\x0C|V[`\xA0\x82\x01R\x95\x94PPPPPV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\xD2Wa\r\xD2a\t\x03V[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\r\xEBW_\x80\xFD[\x815` a\r\xFBa\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0E\x19W_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E;W_\x80\x81\xFD[a\x0EI\x89\x86\x83\x8B\x01\x01a\ncV[\x84RP\x91\x83\x01\x91\x83\x01a\x0E\x1DV[P\x96\x95PPPPPPV[_a\x01\0\x82\x84\x03\x12\x15a\x0EsW_\x80\xFD[a\x0E{a\t\x83V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x93W_\x80\xFD[a\x0E\x9F\x85\x83\x86\x01a\ncV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0E\xB4W_\x80\xFD[a\x0E\xC0\x85\x83\x86\x01a\ncV[` \x84\x01Ra\x0E\xD1`@\x85\x01a\n\xB5V[`@\x84\x01Ra\x0E\xE2``\x85\x01a\x0C|V[``\x84\x01Ra\x0E\xF3`\x80\x85\x01a\n\xB5V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0F\x0BW_\x80\xFD[a\x0F\x17\x85\x83\x86\x01a\r\xDCV[`\xA0\x84\x01Ra\x0F(`\xC0\x85\x01a\n\xB5V[`\xC0\x84\x01R`\xE0\x84\x015\x91P\x80\x82\x11\x15a\x0F@W_\x80\xFD[Pa\x0FM\x84\x82\x85\x01a\ncV[`\xE0\x83\x01RP\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0FiW_\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0F~W_\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0F\x91W_\x80\xFD[a\x0F\x99a\t\xA6V[\x82\x825\x11\x15a\x0F\xA6W_\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0F\xB9W_\x80\xFD[a\x0F\xC1a\t\xA6V[\x84\x825\x11\x15a\x0F\xCEW_\x80\xFD[a\x0F\xDB\x88\x835\x84\x01a\x0EbV[\x81R\x84` \x83\x015\x11\x15a\x0F\xEDW_\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x10\x04W_\x80\xFD[a\x10\x11a\n\x80\x835a\r\xBAV[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x10/W_\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x10\xCAW\x88\x815\x11\x15a\x10IW_\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x10`W_\x80\xFD[a\x10ha\t\xA6V[\x8A` \x83\x015\x11\x15a\x10xW_\x80\xFD[a\x10\x8A\x8E` \x80\x85\x015\x85\x01\x01a\ncV[\x81R\x8A`@\x83\x015\x11\x15a\x10\x9CW_\x80\xFD[a\x10\xAF\x8E` `@\x85\x015\x85\x01\x01a\ncV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x104V[PP\x80` \x84\x01RPP\x80\x83RPPa\x10\xE5` \x83\x01a\x0C|V[` \x82\x01R\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x11\x03W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\x19W_\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x11,W_\x80\xFD[a\x114a\t\xA6V[\x825\x82\x81\x11\x15a\x11BW_\x80\xFD[a\x11N\x87\x82\x86\x01a\x0B\xC8V[\x82RPa\x10\xE5` \x84\x01a\x0C|V[_` \x82\x84\x03\x12\x15a\x11mW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x82W_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\n\xD0V[_` \x82\x84\x03\x12\x15a\x11\x9EW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\xB4W_\x80\xFD[\x90\x83\x01\x90`\xC0\x82\x86\x03\x12\x15a\x11\xC7W_\x80\xFD[a\x11\xCFa\taV[\x825\x82\x81\x11\x15a\x11\xDDW_\x80\xFD[a\x11\xE9\x87\x82\x86\x01a\ncV[\x82RPa\x11\xF8` \x84\x01a\n\xB5V[` \x82\x01R`@\x83\x015\x82\x81\x11\x15a\x12\x0EW_\x80\xFD[a\x12\x1A\x87\x82\x86\x01a\r\xDCV[`@\x83\x01RPa\x12,``\x84\x01a\n\xB5V[``\x82\x01R`\x80\x83\x015`\x80\x82\x01R`\xA0\x83\x015\x82\x81\x11\x15a\x12LW_\x80\xFD[a\x12X\x87\x82\x86\x01a\ncV[`\xA0\x83\x01RP\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x12wW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\x8CW_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\x0EbV[_` \x82\x84\x03\x12\x15a\x12\xA8W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x12\xBEW_\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\x12\xD1W_\x80\xFD[a\x12\xD9a\t\xC8V[\x825\x82\x81\x11\x15a\x12\xE7W_\x80\xFD[a\x12\xF3\x87\x82\x86\x01a\n\xD0V[\x82RP` \x83\x015\x82\x81\x11\x15a\x13\x07W_\x80\xFD[a\x13\x13\x87\x82\x86\x01a\ncV[` \x83\x01RPa\x13%`@\x84\x01a\n\xB5V[`@\x82\x01R``\x83\x015``\x82\x01Ra\x13@`\x80\x84\x01a\x0C|V[`\x80\x82\x01R\x95\x94PPPPPV[_\x825`\xDE\x19\x836\x03\x01\x81\x12a\x13bW_\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[_[\x83\x81\x10\x15a\x13\x86W\x81\x81\x01Q\x83\x82\x01R` \x01a\x13nV[PP_\x91\x01RV[_` \x82\x84\x03\x12\x15a\x13\x9EW_\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x13\xB3W_\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x13\xC3W_\x80\xFD[\x80Qa\x13\xD1a\n\x80\x82a\n=V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x13\xE5W_\x80\xFD[a\x13\xF6\x82` \x83\x01` \x86\x01a\x13lV[\x95\x94PPPPPV[_\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x14\x14W_\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x14-W_\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x14AW_\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[_\x80\x85\x85\x11\x15a\x14~W_\x80\xFD[\x83\x86\x11\x15a\x14\x8AW_\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[_``\x82\x84\x03\x12\x15a\x14\xA7W_\x80\xFD[a\x14\xAFa\t?V[a\x14\xB8\x83a\x0C|V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x14\xD6W_\x80\xFD[`@\x82\x01R\x93\x92PPPV[_\x82`\x1F\x83\x01\x12a\x14\xF1W_\x80\xFD[\x815` a\x15\x01a\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x15\x1FW_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWW\x805\x83R\x91\x83\x01\x91\x83\x01a\x15#V[_\x82`\x1F\x83\x01\x12a\x15IW_\x80\xFD[\x815` a\x15Ya\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x15wW_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWWa\x15\x8C\x81a\x0C|V[\x83R\x91\x83\x01\x91\x83\x01a\x15{V[_` \x82\x84\x03\x12\x15a\x15\xA9W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x15\xBFW_\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x15\xD3W_\x80\xFD[a\x15\xDBa\t\xEAV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x15\xFF``\x84\x01a\x0C|V[``\x82\x01Ra\x16\x10`\x80\x84\x01a\x0C|V[`\x80\x82\x01Ra\x16!`\xA0\x84\x01a\x0C|V[`\xA0\x82\x01Ra\x162`\xC0\x84\x01a\x0C|V[`\xC0\x82\x01Ra\x16C`\xE0\x84\x01a\x0C|V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x16l\x81\x85\x01a\x0C|V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x16\x83W_\x80\xFD[a\x16\x8F\x88\x82\x87\x01a\x14\xE2V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x16\xA8W_\x80\xFD[a\x16\xB4\x88\x82\x87\x01a\x15:V[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x16\xCDW_\x80\xFD[a\x16\xD9\x88\x82\x87\x01a\ncV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[_\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01_[\x83\x81\x10\x15a\x17\x18W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x16\xFCV[P\x94\x95\x94PPPPPV[_\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01_[\x83\x81\x10\x15a\x17\x18W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x175V[_\x81Q\x80\x84Ra\x17q\x81` \x86\x01` \x86\x01a\x13lV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R_``\x83\x01Qa\x17\xBF`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x18\x1B\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x18Q\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x18qa\x01\xE0\x86\x01\x84a\x16\xEAV[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x18\x92\x85\x84a\x17#V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x18\xAE\x83\x82a\x17ZV[\x96\x95PPPPPPV[_` \x82\x84\x03\x12\x15a\x18\xC8W_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x02\xE7Wa\x02\xE7a\x18\xCFV[\x80\x82\x01\x80\x82\x11\x15a\x02\xE7Wa\x02\xE7a\x18\xCFV\xFE\xA2dipfsX\"\x12 Ca\xB2\xC0\xED\x9E\x9E\xAC\xF8/t\xE7U\x93+dM\x02H\xA9\xA3S\x15q\xADq\x11\xB8;4vOdsolcC\0\x08\x14\x003";
	/// The bytecode of the contract.
	pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__BYTECODE);
	#[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\xC2W_5`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0|W\x80c\xCF\xF0\xAB\x96\x11a\0WW\x80c\xCF\xF0\xAB\x96\x14a\x01\xFAW\x80c\xD0\xFF\xF3f\x14a\x02RW\x80c\xDD\x92\xA3\x16\x14a\x02lW\x80c\xF47\xBCY\x14a\x02\x8BW_\x80\xFD[\x80c\xB2\xA0\x1B\xF5\x14a\x01\xA7W\x80c\xBC\r\xD4G\x14a\x01\xC1W\x80c\xBC\xA9l9\x14a\x01\xDBW_\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\xCDW\x80c\x0B\xC3{\xAB\x14a\x01\x01W\x80c\x0E\x83$\xA2\x14a\x01\"W\x80c\x0F\xEE2\xCE\x14a\x01AW\x80c\x10\x8B\xC1\xDD\x14a\x01`W\x80cD\xAB \xF8\x14a\x01\x8DW_\x80\xFD[6a\0\xC9W\0[_\x80\xFD[4\x80\x15a\0\xD8W_\x80\xFD[Pa\0\xECa\0\xE76`\x04a\x08\xD5V[a\x02\xB7V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x0CW_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x0CCV[a\x02\xEDV[\0[4\x80\x15a\x01-W_\x80\xFD[Pa\x01 a\x01<6`\x04a\x0C\x92V[a\x03?V[4\x80\x15a\x01LW_\x80\xFD[Pa\x01 a\x01[6`\x04a\x0C\xABV[a\x03\x93V[4\x80\x15a\x01kW_\x80\xFD[Pa\x01\x7Fa\x01z6`\x04a\x0C\xE1V[a\x06FV[`@Q\x90\x81R` \x01a\0\xF8V[4\x80\x15a\x01\x98W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x0FYV[4\x80\x15a\x01\xB2W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x10\xF3V[4\x80\x15a\x01\xCCW_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x11]V[4\x80\x15a\x01\xE6W_\x80\xFD[Pa\x01\x7Fa\x01\xF56`\x04a\x11\x8EV[a\x06\xCDV[4\x80\x15a\x02\x05W_\x80\xFD[P`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xF8V[4\x80\x15a\x02]W_\x80\xFD[Pa\x01 a\x01\x1B6`\x04a\x12gV[4\x80\x15a\x02wW_\x80\xFD[Pa\x01\x7Fa\x02\x866`\x04a\x12\x98V[a\x07EV[4\x80\x15a\x02\x96W_\x80\xFD[Pa\x02\x9Fa\x07\xCCV[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\xF8V[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02\xE7WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x02\xF5a\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x03&W`@Qc{\xF6\xA1o`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03jW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U_\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03\xBFW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\xCA\x83\x80a\x13NV[\x90Pa\x04\x8C_`\x01\x01_\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01_`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\x1FW=_\x80>=_\xFD[PPPP`@Q=_\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04F\x91\x90\x81\x01\x90a\x13\x8EV[a\x04P\x83\x80a\x13\xFFV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847_\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x08\xAD\x90PV[a\x04\xA9W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04\xB7`\xC0\x83\x01\x83a\x13\xFFV[_\x81\x81\x10a\x04\xC7Wa\x04\xC7a\x14HV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xE2Wa\x04\xE2a\x14\\V[\x90P_\x81`\x01\x81\x11\x15a\x04\xF7Wa\x04\xF7a\x14\\V[\x03a\x05\x9EW_a\x05\n`\xC0\x84\x01\x84a\x13\xFFV[a\x05\x18\x91`\x01\x90\x82\x90a\x14pV[\x81\x01\x90a\x05%\x91\x90a\x14\x97V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05\x82W_\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x94W=_\x80>=_\xFD[PPPPPa\x06@V[`\x01\x81`\x01\x81\x11\x15a\x05\xB2Wa\x05\xB2a\x14\\V[\x03a\x06@W_a\x05\xC5`\xC0\x84\x01\x84a\x13\xFFV[a\x05\xD3\x91`\x01\x90\x82\x90a\x14pV[\x81\x01\x90a\x05\xE0\x91\x90a\x15\x99V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x06\x11\x90\x84\x90`\x04\x01a\x17\x85V[_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x06(W_\x80\xFD[PZ\xF1\x15\x80\x15a\x06:W=_\x80>=_\xFD[PPPPP[PPPPV[_a\x06Oa\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\x8AW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06\xAE\x91\x90a\x18\xB8V[\x82`@\x01QQa\x06\xBE\x91\x90a\x18\xE3V[\x82`\x80\x01Qa\x02\xE7\x91\x90a\x18\xFAV[_a\x06\xD6a\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x07\x11W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x075\x91\x90a\x18\xB8V[\x82`\xA0\x01QQa\x06\xBE\x91\x90a\x18\xE3V[_a\x07Na\x07\xCCV[`\x01`\x01`\xA0\x1B\x03\x16cd\x1Dr\x9D`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x07\x89W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x07\xAD\x91\x90a\x18\xB8V[\x82` \x01QQa\x07\xBD\x91\x90a\x18\xE3V[\x82``\x01Qa\x02\xE7\x91\x90a\x18\xFAV[_Fb\xAA6\xA7\x81\x14a\x08\x0BWb\x06n\xEE\x81\x14a\x08&Wb\xAA7\xDC\x81\x14a\x08AWb\x01J4\x81\x14a\x08\\W`a\x81\x14a\x08wWa'\xD8\x81\x14a\x08\x92WP\x90V[s'\xB0\xC6\x96\x0By*\x8D\xCB\x01\xF0e+\xDEH\x01\\\xD5\xF2>\x91PP\x90V[s\xFD~+*\xD0\xB2\x9E\xC8\x17\xDC}@h\x81\xB2%\xB8\x1D\xBF\xCF\x91PP\x90V[s0\xE3\xAF\x17G\xB1U\xF3\x7F\x93^\x0E\xC9\x95\xDE^\xA4\xE6u\x86\x91PP\x90V[s\rp7\xBD\x9C\xEA\xEF%\xE5!_\x80\x8D0\x9A\xDD\ne\xCD\xB9\x91PP\x90V[sL\xB0\xF5u\x0Fo\xE1MK\x86\xAC\xA6\xFE\x12iC\xBD\xA3\xC8\xC4\x91PP\x90V[s\x11\xEB\x87\xC7E\xD9zO\xA8\xAE\xC8\x055\x987E\x9D$\r\x1B\x91PP\x90V[_\x81Q\x83Q\x14a\x08\xBEWP_a\x02\xE7V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[_` \x82\x84\x03\x12\x15a\x08\xE5W_\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08\xFCW_\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@R\x90V[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t9Wa\t9a\t\x03V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\n5Wa\n5a\t\x03V[`@R\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\nUWa\nUa\t\x03V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[_\x82`\x1F\x83\x01\x12a\nrW_\x80\xFD[\x815a\n\x85a\n\x80\x82a\n=V[a\n\rV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\n\x99W_\x80\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\n\xCBW_\x80\xFD[\x91\x90PV[_`\xE0\x82\x84\x03\x12\x15a\n\xE0W_\x80\xFD[a\n\xE8a\t\x17V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\0W_\x80\xFD[a\x0B\x0C\x85\x83\x86\x01a\ncV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B!W_\x80\xFD[a\x0B-\x85\x83\x86\x01a\ncV[` \x84\x01Ra\x0B>`@\x85\x01a\n\xB5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0BVW_\x80\xFD[a\x0Bb\x85\x83\x86\x01a\ncV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x0BzW_\x80\xFD[a\x0B\x86\x85\x83\x86\x01a\ncV[`\x80\x84\x01Ra\x0B\x97`\xA0\x85\x01a\n\xB5V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xAFW_\x80\xFD[Pa\x0B\xBC\x84\x82\x85\x01a\ncV[`\xC0\x83\x01RP\x92\x91PPV[_``\x82\x84\x03\x12\x15a\x0B\xD8W_\x80\xFD[a\x0B\xE0a\t?V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xF8W_\x80\xFD[a\x0C\x04\x85\x83\x86\x01a\n\xD0V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0C\x19W_\x80\xFD[Pa\x0C&\x84\x82\x85\x01a\ncV[` \x83\x01RPa\x0C8`@\x83\x01a\n\xB5V[`@\x82\x01R\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0CSW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0ChW_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\x0B\xC8V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n\xCBW_\x80\xFD[_` \x82\x84\x03\x12\x15a\x0C\xA2W_\x80\xFD[a\x08\xFC\x82a\x0C|V[_` \x82\x84\x03\x12\x15a\x0C\xBBW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xD0W_\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x08\xFCW_\x80\xFD[_` \x82\x84\x03\x12\x15a\x0C\xF1W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\r\x07W_\x80\xFD[\x90\x83\x01\x90`\xC0\x82\x86\x03\x12\x15a\r\x1AW_\x80\xFD[a\r\"a\taV[\x825\x82\x81\x11\x15a\r0W_\x80\xFD[a\r<\x87\x82\x86\x01a\ncV[\x82RP` \x83\x015\x82\x81\x11\x15a\rPW_\x80\xFD[a\r\\\x87\x82\x86\x01a\ncV[` \x83\x01RP`@\x83\x015\x82\x81\x11\x15a\rsW_\x80\xFD[a\r\x7F\x87\x82\x86\x01a\ncV[`@\x83\x01RPa\r\x91``\x84\x01a\n\xB5V[``\x82\x01R`\x80\x83\x015`\x80\x82\x01Ra\r\xAC`\xA0\x84\x01a\x0C|V[`\xA0\x82\x01R\x95\x94PPPPPV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\xD2Wa\r\xD2a\t\x03V[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\r\xEBW_\x80\xFD[\x815` a\r\xFBa\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0E\x19W_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E;W_\x80\x81\xFD[a\x0EI\x89\x86\x83\x8B\x01\x01a\ncV[\x84RP\x91\x83\x01\x91\x83\x01a\x0E\x1DV[P\x96\x95PPPPPPV[_a\x01\0\x82\x84\x03\x12\x15a\x0EsW_\x80\xFD[a\x0E{a\t\x83V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x93W_\x80\xFD[a\x0E\x9F\x85\x83\x86\x01a\ncV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0E\xB4W_\x80\xFD[a\x0E\xC0\x85\x83\x86\x01a\ncV[` \x84\x01Ra\x0E\xD1`@\x85\x01a\n\xB5V[`@\x84\x01Ra\x0E\xE2``\x85\x01a\x0C|V[``\x84\x01Ra\x0E\xF3`\x80\x85\x01a\n\xB5V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0F\x0BW_\x80\xFD[a\x0F\x17\x85\x83\x86\x01a\r\xDCV[`\xA0\x84\x01Ra\x0F(`\xC0\x85\x01a\n\xB5V[`\xC0\x84\x01R`\xE0\x84\x015\x91P\x80\x82\x11\x15a\x0F@W_\x80\xFD[Pa\x0FM\x84\x82\x85\x01a\ncV[`\xE0\x83\x01RP\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0FiW_\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0F~W_\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0F\x91W_\x80\xFD[a\x0F\x99a\t\xA6V[\x82\x825\x11\x15a\x0F\xA6W_\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0F\xB9W_\x80\xFD[a\x0F\xC1a\t\xA6V[\x84\x825\x11\x15a\x0F\xCEW_\x80\xFD[a\x0F\xDB\x88\x835\x84\x01a\x0EbV[\x81R\x84` \x83\x015\x11\x15a\x0F\xEDW_\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x10\x04W_\x80\xFD[a\x10\x11a\n\x80\x835a\r\xBAV[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x10/W_\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x10\xCAW\x88\x815\x11\x15a\x10IW_\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x10`W_\x80\xFD[a\x10ha\t\xA6V[\x8A` \x83\x015\x11\x15a\x10xW_\x80\xFD[a\x10\x8A\x8E` \x80\x85\x015\x85\x01\x01a\ncV[\x81R\x8A`@\x83\x015\x11\x15a\x10\x9CW_\x80\xFD[a\x10\xAF\x8E` `@\x85\x015\x85\x01\x01a\ncV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x104V[PP\x80` \x84\x01RPP\x80\x83RPPa\x10\xE5` \x83\x01a\x0C|V[` \x82\x01R\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x11\x03W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\x19W_\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x11,W_\x80\xFD[a\x114a\t\xA6V[\x825\x82\x81\x11\x15a\x11BW_\x80\xFD[a\x11N\x87\x82\x86\x01a\x0B\xC8V[\x82RPa\x10\xE5` \x84\x01a\x0C|V[_` \x82\x84\x03\x12\x15a\x11mW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x82W_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\n\xD0V[_` \x82\x84\x03\x12\x15a\x11\x9EW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\xB4W_\x80\xFD[\x90\x83\x01\x90`\xC0\x82\x86\x03\x12\x15a\x11\xC7W_\x80\xFD[a\x11\xCFa\taV[\x825\x82\x81\x11\x15a\x11\xDDW_\x80\xFD[a\x11\xE9\x87\x82\x86\x01a\ncV[\x82RPa\x11\xF8` \x84\x01a\n\xB5V[` \x82\x01R`@\x83\x015\x82\x81\x11\x15a\x12\x0EW_\x80\xFD[a\x12\x1A\x87\x82\x86\x01a\r\xDCV[`@\x83\x01RPa\x12,``\x84\x01a\n\xB5V[``\x82\x01R`\x80\x83\x015`\x80\x82\x01R`\xA0\x83\x015\x82\x81\x11\x15a\x12LW_\x80\xFD[a\x12X\x87\x82\x86\x01a\ncV[`\xA0\x83\x01RP\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x12wW_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\x8CW_\x80\xFD[a\x0Ct\x84\x82\x85\x01a\x0EbV[_` \x82\x84\x03\x12\x15a\x12\xA8W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x12\xBEW_\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\x12\xD1W_\x80\xFD[a\x12\xD9a\t\xC8V[\x825\x82\x81\x11\x15a\x12\xE7W_\x80\xFD[a\x12\xF3\x87\x82\x86\x01a\n\xD0V[\x82RP` \x83\x015\x82\x81\x11\x15a\x13\x07W_\x80\xFD[a\x13\x13\x87\x82\x86\x01a\ncV[` \x83\x01RPa\x13%`@\x84\x01a\n\xB5V[`@\x82\x01R``\x83\x015``\x82\x01Ra\x13@`\x80\x84\x01a\x0C|V[`\x80\x82\x01R\x95\x94PPPPPV[_\x825`\xDE\x19\x836\x03\x01\x81\x12a\x13bW_\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[_[\x83\x81\x10\x15a\x13\x86W\x81\x81\x01Q\x83\x82\x01R` \x01a\x13nV[PP_\x91\x01RV[_` \x82\x84\x03\x12\x15a\x13\x9EW_\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x13\xB3W_\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x13\xC3W_\x80\xFD[\x80Qa\x13\xD1a\n\x80\x82a\n=V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x13\xE5W_\x80\xFD[a\x13\xF6\x82` \x83\x01` \x86\x01a\x13lV[\x95\x94PPPPPV[_\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x14\x14W_\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x14-W_\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x14AW_\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[_\x80\x85\x85\x11\x15a\x14~W_\x80\xFD[\x83\x86\x11\x15a\x14\x8AW_\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[_``\x82\x84\x03\x12\x15a\x14\xA7W_\x80\xFD[a\x14\xAFa\t?V[a\x14\xB8\x83a\x0C|V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x14\xD6W_\x80\xFD[`@\x82\x01R\x93\x92PPPV[_\x82`\x1F\x83\x01\x12a\x14\xF1W_\x80\xFD[\x815` a\x15\x01a\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x15\x1FW_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWW\x805\x83R\x91\x83\x01\x91\x83\x01a\x15#V[_\x82`\x1F\x83\x01\x12a\x15IW_\x80\xFD[\x815` a\x15Ya\n\x80\x83a\r\xBAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x15wW_\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0EWWa\x15\x8C\x81a\x0C|V[\x83R\x91\x83\x01\x91\x83\x01a\x15{V[_` \x82\x84\x03\x12\x15a\x15\xA9W_\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x15\xBFW_\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x15\xD3W_\x80\xFD[a\x15\xDBa\t\xEAV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x15\xFF``\x84\x01a\x0C|V[``\x82\x01Ra\x16\x10`\x80\x84\x01a\x0C|V[`\x80\x82\x01Ra\x16!`\xA0\x84\x01a\x0C|V[`\xA0\x82\x01Ra\x162`\xC0\x84\x01a\x0C|V[`\xC0\x82\x01Ra\x16C`\xE0\x84\x01a\x0C|V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x16l\x81\x85\x01a\x0C|V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x16\x83W_\x80\xFD[a\x16\x8F\x88\x82\x87\x01a\x14\xE2V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x16\xA8W_\x80\xFD[a\x16\xB4\x88\x82\x87\x01a\x15:V[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x16\xCDW_\x80\xFD[a\x16\xD9\x88\x82\x87\x01a\ncV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[_\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01_[\x83\x81\x10\x15a\x17\x18W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x16\xFCV[P\x94\x95\x94PPPPPV[_\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01_[\x83\x81\x10\x15a\x17\x18W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x175V[_\x81Q\x80\x84Ra\x17q\x81` \x86\x01` \x86\x01a\x13lV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R_``\x83\x01Qa\x17\xBF`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x18\x1B\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x18Q\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x18qa\x01\xE0\x86\x01\x84a\x16\xEAV[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x18\x92\x85\x84a\x17#V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x18\xAE\x83\x82a\x17ZV[\x96\x95PPPPPPV[_` \x82\x84\x03\x12\x15a\x18\xC8W_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x02\xE7Wa\x02\xE7a\x18\xCFV[\x80\x82\x01\x80\x82\x11\x15a\x02\xE7Wa\x02\xE7a\x18\xCFV\xFE\xA2dipfsX\"\x12 Ca\xB2\xC0\xED\x9E\x9E\xAC\xF8/t\xE7U\x93+dM\x02H\xA9\xA3S\x15q\xADq\x11\xB8;4vOdsolcC\0\x08\x14\x003";
	/// The deployed bytecode of the contract.
	pub static HOSTMANAGER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
	pub struct HostManager<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for HostManager<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for HostManager<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for HostManager<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for HostManager<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(HostManager)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> HostManager<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), HOSTMANAGER_ABI.clone(), client))
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
				HOSTMANAGER_ABI.clone(),
				HOSTMANAGER_BYTECODE.clone().into(),
				client,
			);
			let deployer = factory.deploy(constructor_args)?;
			let deployer = ::ethers::contract::ContractDeployer::new(deployer);
			Ok(deployer)
		}
		///Calls the contract's `host` (0xf437bc59) function
		pub fn host(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([244, 55, 188, 89], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onAccept` (0x0fee32ce) function
		pub fn on_accept(
			&self,
			incoming: IncomingPostRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([15, 238, 50, 206], (incoming,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onGetResponse` (0x44ab20f8) function
		pub fn on_get_response(
			&self,
			p0: IncomingGetResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([68, 171, 32, 248], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onGetTimeout` (0xd0fff366) function
		pub fn on_get_timeout(
			&self,
			p0: GetRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([208, 255, 243, 102], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onPostRequestTimeout` (0xbc0dd447) function
		pub fn on_post_request_timeout(
			&self,
			p0: PostRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([188, 13, 212, 71], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onPostResponse` (0xb2a01bf5) function
		pub fn on_post_response(
			&self,
			p0: IncomingPostResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([178, 160, 27, 245], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onPostResponseTimeout` (0x0bc37bab) function
		pub fn on_post_response_timeout(
			&self,
			p0: PostResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([11, 195, 123, 171], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `params` (0xcff0ab96) function
		pub fn params(&self) -> ::ethers::contract::builders::ContractCall<M, HostManagerParams> {
			self.0
				.method_hash([207, 240, 171, 150], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `quote` (0x108bc1dd) function
		pub fn quote(
			&self,
			post: DispatchPost,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([16, 139, 193, 221], (post,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `quote` (0xbca96c39) function
		pub fn quote_with_get(
			&self,
			get: DispatchGet,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([188, 169, 108, 57], (get,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `quote` (0xdd92a316) function
		pub fn quote_with_res(
			&self,
			res: DispatchPostResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([221, 146, 163, 22], (res,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setIsmpHost` (0x0e8324a2) function
		pub fn set_ismp_host(
			&self,
			host: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([14, 131, 36, 162], host)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `supportsInterface` (0x01ffc9a7) function
		pub fn supports_interface(
			&self,
			interface_id: [u8; 4],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([1, 255, 201, 167], interface_id)
				.expect("method not found (this should never happen)")
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for HostManager<M> {
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Custom Error type `UnauthorizedAction` with signature `UnauthorizedAction()` and selector
	/// `0x843800fa`
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
	#[etherror(name = "UnauthorizedAction", abi = "UnauthorizedAction()")]
	pub struct UnauthorizedAction;
	///Custom Error type `UnauthorizedCall` with signature `UnauthorizedCall()` and selector
	/// `0x7bf6a16f`
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
	#[etherror(name = "UnauthorizedCall", abi = "UnauthorizedCall()")]
	pub struct UnauthorizedCall;
	///Custom Error type `UnexpectedCall` with signature `UnexpectedCall()` and selector
	/// `0x02cbc79f`
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
	#[etherror(name = "UnexpectedCall", abi = "UnexpectedCall()")]
	pub struct UnexpectedCall;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum HostManagerErrors {
		UnauthorizedAction(UnauthorizedAction),
		UnauthorizedCall(UnauthorizedCall),
		UnexpectedCall(UnexpectedCall),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for HostManagerErrors {
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
				<UnauthorizedAction as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnauthorizedAction(decoded));
			}
			if let Ok(decoded) = <UnauthorizedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnauthorizedCall(decoded));
			}
			if let Ok(decoded) = <UnexpectedCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::UnexpectedCall(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for HostManagerErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::UnauthorizedAction(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnauthorizedCall(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnexpectedCall(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for HostManagerErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<UnauthorizedAction as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnauthorizedCall as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnexpectedCall as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for HostManagerErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::UnauthorizedAction(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnauthorizedCall(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnexpectedCall(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for HostManagerErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<UnauthorizedAction> for HostManagerErrors {
		fn from(value: UnauthorizedAction) -> Self {
			Self::UnauthorizedAction(value)
		}
	}
	impl ::core::convert::From<UnauthorizedCall> for HostManagerErrors {
		fn from(value: UnauthorizedCall) -> Self {
			Self::UnauthorizedCall(value)
		}
	}
	impl ::core::convert::From<UnexpectedCall> for HostManagerErrors {
		fn from(value: UnexpectedCall) -> Self {
			Self::UnexpectedCall(value)
		}
	}
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
	///Container type for all input parameters for the `onAccept` function with signature
	/// `onAccept(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address))` and selector
	/// `0x0fee32ce`
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
		name = "onAccept",
		abi = "onAccept(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address))"
	)]
	pub struct OnAcceptCall {
		pub incoming: IncomingPostRequest,
	}
	///Container type for all input parameters for the `onGetResponse` function with signature
	/// `onGetResponse((((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),
	/// address))` and selector `0x44ab20f8`
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
		name = "onGetResponse",
		abi = "onGetResponse((((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),address))"
	)]
	pub struct OnGetResponseCall(pub IncomingGetResponse);
	///Container type for all input parameters for the `onGetTimeout` function with signature
	/// `onGetTimeout((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes))` and selector
	/// `0xd0fff366`
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
		name = "onGetTimeout",
		abi = "onGetTimeout((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes))"
	)]
	pub struct OnGetTimeoutCall(pub GetRequest);
	///Container type for all input parameters for the `onPostRequestTimeout` function with
	/// signature `onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))` and selector
	/// `0xbc0dd447`
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
		name = "onPostRequestTimeout",
		abi = "onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))"
	)]
	pub struct OnPostRequestTimeoutCall(pub PostRequest);
	///Container type for all input parameters for the `onPostResponse` function with signature
	/// `onPostResponse((((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address))` and
	/// selector `0xb2a01bf5`
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
		name = "onPostResponse",
		abi = "onPostResponse((((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address))"
	)]
	pub struct OnPostResponseCall(pub IncomingPostResponse);
	///Container type for all input parameters for the `onPostResponseTimeout` function with
	/// signature `onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,
	/// uint64))` and selector `0x0bc37bab`
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
		name = "onPostResponseTimeout",
		abi = "onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))"
	)]
	pub struct OnPostResponseTimeoutCall(pub PostResponse);
	///Container type for all input parameters for the `params` function with signature `params()`
	/// and selector `0xcff0ab96`
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
	#[ethcall(name = "params", abi = "params()")]
	pub struct ParamsCall;
	///Container type for all input parameters for the `quote` function with signature
	/// `quote((bytes,bytes,bytes,uint64,uint256,address))` and selector `0x108bc1dd`
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
	#[ethcall(name = "quote", abi = "quote((bytes,bytes,bytes,uint64,uint256,address))")]
	pub struct QuoteCall {
		pub post: DispatchPost,
	}
	///Container type for all input parameters for the `quote` function with signature
	/// `quote((bytes,uint64,bytes[],uint64,uint256,bytes))` and selector `0xbca96c39`
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
	#[ethcall(name = "quote", abi = "quote((bytes,uint64,bytes[],uint64,uint256,bytes))")]
	pub struct QuoteWithGetCall {
		pub get: DispatchGet,
	}
	///Container type for all input parameters for the `quote` function with signature
	/// `quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and
	/// selector `0xdd92a316`
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
		name = "quote",
		abi = "quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))"
	)]
	pub struct QuoteWithResCall {
		pub res: DispatchPostResponse,
	}
	///Container type for all input parameters for the `setIsmpHost` function with signature
	/// `setIsmpHost(address)` and selector `0x0e8324a2`
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
	#[ethcall(name = "setIsmpHost", abi = "setIsmpHost(address)")]
	pub struct SetIsmpHostCall {
		pub host: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `supportsInterface` function with signature
	/// `supportsInterface(bytes4)` and selector `0x01ffc9a7`
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
	#[ethcall(name = "supportsInterface", abi = "supportsInterface(bytes4)")]
	pub struct SupportsInterfaceCall {
		pub interface_id: [u8; 4],
	}
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum HostManagerCalls {
		Host(HostCall),
		OnAccept(OnAcceptCall),
		OnGetResponse(OnGetResponseCall),
		OnGetTimeout(OnGetTimeoutCall),
		OnPostRequestTimeout(OnPostRequestTimeoutCall),
		OnPostResponse(OnPostResponseCall),
		OnPostResponseTimeout(OnPostResponseTimeoutCall),
		Params(ParamsCall),
		Quote(QuoteCall),
		QuoteWithGet(QuoteWithGetCall),
		QuoteWithRes(QuoteWithResCall),
		SetIsmpHost(SetIsmpHostCall),
		SupportsInterface(SupportsInterfaceCall),
	}
	impl ::ethers::core::abi::AbiDecode for HostManagerCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <HostCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Host(decoded));
			}
			if let Ok(decoded) = <OnAcceptCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::OnAccept(decoded));
			}
			if let Ok(decoded) = <OnGetResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OnGetResponse(decoded));
			}
			if let Ok(decoded) = <OnGetTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OnGetTimeout(decoded));
			}
			if let Ok(decoded) =
				<OnPostRequestTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OnPostRequestTimeout(decoded));
			}
			if let Ok(decoded) =
				<OnPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OnPostResponse(decoded));
			}
			if let Ok(decoded) =
				<OnPostResponseTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::OnPostResponseTimeout(decoded));
			}
			if let Ok(decoded) = <ParamsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Params(decoded));
			}
			if let Ok(decoded) = <QuoteCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Quote(decoded));
			}
			if let Ok(decoded) = <QuoteWithGetCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::QuoteWithGet(decoded));
			}
			if let Ok(decoded) = <QuoteWithResCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::QuoteWithRes(decoded));
			}
			if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetIsmpHost(decoded));
			}
			if let Ok(decoded) =
				<SupportsInterfaceCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SupportsInterface(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for HostManagerCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::Host(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnAccept(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnGetResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnGetTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostRequestTimeout(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostResponseTimeout(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Params(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Quote(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::QuoteWithGet(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::QuoteWithRes(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetIsmpHost(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SupportsInterface(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for HostManagerCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::Host(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostRequestTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostResponseTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::Params(element) => ::core::fmt::Display::fmt(element, f),
				Self::Quote(element) => ::core::fmt::Display::fmt(element, f),
				Self::QuoteWithGet(element) => ::core::fmt::Display::fmt(element, f),
				Self::QuoteWithRes(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
				Self::SupportsInterface(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<HostCall> for HostManagerCalls {
		fn from(value: HostCall) -> Self {
			Self::Host(value)
		}
	}
	impl ::core::convert::From<OnAcceptCall> for HostManagerCalls {
		fn from(value: OnAcceptCall) -> Self {
			Self::OnAccept(value)
		}
	}
	impl ::core::convert::From<OnGetResponseCall> for HostManagerCalls {
		fn from(value: OnGetResponseCall) -> Self {
			Self::OnGetResponse(value)
		}
	}
	impl ::core::convert::From<OnGetTimeoutCall> for HostManagerCalls {
		fn from(value: OnGetTimeoutCall) -> Self {
			Self::OnGetTimeout(value)
		}
	}
	impl ::core::convert::From<OnPostRequestTimeoutCall> for HostManagerCalls {
		fn from(value: OnPostRequestTimeoutCall) -> Self {
			Self::OnPostRequestTimeout(value)
		}
	}
	impl ::core::convert::From<OnPostResponseCall> for HostManagerCalls {
		fn from(value: OnPostResponseCall) -> Self {
			Self::OnPostResponse(value)
		}
	}
	impl ::core::convert::From<OnPostResponseTimeoutCall> for HostManagerCalls {
		fn from(value: OnPostResponseTimeoutCall) -> Self {
			Self::OnPostResponseTimeout(value)
		}
	}
	impl ::core::convert::From<ParamsCall> for HostManagerCalls {
		fn from(value: ParamsCall) -> Self {
			Self::Params(value)
		}
	}
	impl ::core::convert::From<QuoteCall> for HostManagerCalls {
		fn from(value: QuoteCall) -> Self {
			Self::Quote(value)
		}
	}
	impl ::core::convert::From<QuoteWithGetCall> for HostManagerCalls {
		fn from(value: QuoteWithGetCall) -> Self {
			Self::QuoteWithGet(value)
		}
	}
	impl ::core::convert::From<QuoteWithResCall> for HostManagerCalls {
		fn from(value: QuoteWithResCall) -> Self {
			Self::QuoteWithRes(value)
		}
	}
	impl ::core::convert::From<SetIsmpHostCall> for HostManagerCalls {
		fn from(value: SetIsmpHostCall) -> Self {
			Self::SetIsmpHost(value)
		}
	}
	impl ::core::convert::From<SupportsInterfaceCall> for HostManagerCalls {
		fn from(value: SupportsInterfaceCall) -> Self {
			Self::SupportsInterface(value)
		}
	}
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
	pub struct HostReturn {
		pub h: ::ethers::core::types::Address,
	}
	///Container type for all return fields from the `params` function with signature `params()`
	/// and selector `0xcff0ab96`
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
	pub struct ParamsReturn(pub HostManagerParams);
	///Container type for all return fields from the `quote` function with signature
	/// `quote((bytes,bytes,bytes,uint64,uint256,address))` and selector `0x108bc1dd`
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
	pub struct QuoteReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `quote` function with signature
	/// `quote((bytes,uint64,bytes[],uint64,uint256,bytes))` and selector `0xbca96c39`
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
	pub struct QuoteWithGetReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `quote` function with signature
	/// `quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and
	/// selector `0xdd92a316`
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
	pub struct QuoteWithResReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `supportsInterface` function with signature
	/// `supportsInterface(bytes4)` and selector `0x01ffc9a7`
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
	pub struct SupportsInterfaceReturn(pub bool);
	///`HostManagerParams(address,address)`
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
	pub struct HostManagerParams {
		pub admin: ::ethers::core::types::Address,
		pub host: ::ethers::core::types::Address,
	}
}
