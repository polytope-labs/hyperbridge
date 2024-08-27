pub use host_manager::*;
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
					::std::borrow::ToOwned::to_owned("UnauthorizedAccount"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnauthorizedAccount",),
						inputs: ::std::vec![],
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
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
				(
					::std::borrow::ToOwned::to_owned("UnsupportedChain"),
					::std::vec![::ethers::core::abi::ethabi::AbiError {
						name: ::std::borrow::ToOwned::to_owned("UnsupportedChain"),
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qb\0\x15\x9E8\x03\x80b\0\x15\x9E\x839\x81\x01`@\x81\x90Ra\x001\x91a\0\x85V[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xEDV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0\x80W`\0\x80\xFD[\x91\x90PV[`\0`@\x82\x84\x03\x12\x15a\0\x97W`\0\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xC7WcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD3\x83a\0iV[\x81Ra\0\xE1` \x84\x01a\0iV[` \x82\x01R\x93\x92PPPV[a\x14\xA1\x80b\0\0\xFD`\09`\0\xF3\xFE`\x80`@R`\x046\x10a\0\x8AW`\x005`\xE0\x1C\x80cD\xAB \xF8\x11a\0YW\x80cD\xAB \xF8\x14a\x01-W\x80c\xB2\xA0\x1B\xF5\x14a\x01HW\x80c\xBC\r\xD4G\x14a\x01cW\x80c\xCF\xF0\xAB\x96\x14a\x01~W\x80c\xD0\xFF\xF3f\x14a\x01\xD8W`\0\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\x96W\x80c\x0B\xC3{\xAB\x14a\0\xCBW\x80c\x0E\x83$\xA2\x14a\0\xEDW\x80c\x0F\xEE2\xCE\x14a\x01\rW`\0\x80\xFD[6a\0\x91W\0[`\0\x80\xFD[4\x80\x15a\0\xA2W`\0\x80\xFD[Pa\0\xB6a\0\xB16`\x04a\x06\xB0V[a\x01\xF3V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xD7W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\t\xE9V[a\x02*V[\0[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\0\xEBa\x01\x086`\x04a\n<V[a\x02|V[4\x80\x15a\x01\x19W`\0\x80\xFD[Pa\0\xEBa\x01(6`\x04a\nWV[a\x02\xD2V[4\x80\x15a\x019W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0C;V[4\x80\x15a\x01TW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\r\xE3V[4\x80\x15a\x01oW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0ERV[4\x80\x15a\x01\x8AW`\0\x80\xFD[P`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xC2V[4\x80\x15a\x01\xE4W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\x86V[`\0`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02$WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x022a\x05\x9BV[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02cW`@QcT\xBF\xF8E`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x02\xA8W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x02\xFEW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\t\x83\x80a\x0E\xBAV[\x90Pa\x03\xD2`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03cW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\x8B\x91\x90\x81\x01\x90a\x0E\xFEV[a\x03\x95\x83\x80a\x0FtV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x06\x86\x90PV[a\x03\xEFW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0a\x03\xFE`\xC0\x83\x01\x83a\x0FtV[`\0\x81\x81\x10a\x04\x0FWa\x04\x0Fa\x0F\xC1V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04*Wa\x04*a\x0F\xD7V[\x90P`\0\x81`\x01\x81\x11\x15a\x04@Wa\x04@a\x0F\xD7V[\x03a\x04\xEDW`\0a\x04T`\xC0\x84\x01\x84a\x0FtV[a\x04b\x91`\x01\x90\x82\x90a\x0F\xEDV[\x81\x01\x90a\x04o\x91\x90a\x10\x17V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xCFW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xE3W=`\0\x80>=`\0\xFD[PPPPPa\x05\x95V[`\x01\x81`\x01\x81\x11\x15a\x05\x01Wa\x05\x01a\x0F\xD7V[\x03a\x05\x95W`\0a\x05\x15`\xC0\x84\x01\x84a\x0FtV[a\x05#\x91`\x01\x90\x82\x90a\x0F\xEDV[\x81\x01\x90a\x050\x91\x90a\x11?V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x05a\x90\x84\x90`\x04\x01a\x137V[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05{W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x8FW=`\0\x80>=`\0\xFD[PPPPP[PPPPV[`\0Fb\xAA6\xA7\x81\x14a\x05\xD3Wb\x06n\xEE\x81\x14a\x05\xEFWb\xAA7\xDC\x81\x14a\x06\x0BWb\x01J4\x81\x14a\x06'W`a\x81\x14a\x06CWa\x06[V[s\xF1\xC7\xA3\x862[}\"\x02]uB\xB2\x8E\xE8\x81\xCD\xF1\x07\xB3\x91Pa\x06[V[s(n\x1F\xE1\xC3#\xEEbk\xE8\x02\xB1:Q\x84\xB5\x88\xED\x14\xCB\x91Pa\x06[V[sb\\S\x1AV\xDBw,\xC3c\x13\xD0\xA0\x11IV\xAD\x8BV\xC2\x91Pa\x06[V[s\xAE\x9FI\x0E\xE0U\x88\xFD\xD8W\xA0x\xCF\xC1\xF5\xF3\n\xE7\x18_\x91Pa\x06[V[s\xEB\x89w\xED\xCD\xA5\xFA\xBD\xCD\xDE\xB3\x98a\xDF%\xE8\x82\x1A\x9E\x9B\x91P[P`\x01`\x01`\xA0\x1B\x03\x81\x16a\x06\x83W`@Qc\xD2\x1E\xAB7`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x90V[`\0\x81Q\x83Q\x14a\x06\x99WP`\0a\x02$V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[`\0` \x82\x84\x03\x12\x15a\x06\xC2W`\0\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x06\xDAW`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@R\x90V[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xAFWa\x07\xAFa\x06\xE1V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07\xD0Wa\x07\xD0a\x06\xE1V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07\xEFW`\0\x80\xFD[\x815a\x08\x02a\x07\xFD\x82a\x07\xB7V[a\x07\x87V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08\x17W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08KW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08bW`\0\x80\xFD[a\x08ja\x06\xF7V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x83W`\0\x80\xFD[a\x08\x8F\x85\x83\x86\x01a\x07\xDEV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xA5W`\0\x80\xFD[a\x08\xB1\x85\x83\x86\x01a\x07\xDEV[` \x84\x01Ra\x08\xC2`@\x85\x01a\x084V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08\xDBW`\0\x80\xFD[a\x08\xE7\x85\x83\x86\x01a\x07\xDEV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\0W`\0\x80\xFD[a\t\x0C\x85\x83\x86\x01a\x07\xDEV[`\x80\x84\x01Ra\t\x1D`\xA0\x85\x01a\x084V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t6W`\0\x80\xFD[Pa\tC\x84\x82\x85\x01a\x07\xDEV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\taW`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\x84Wa\t\x84a\x06\xE1V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\t\x9CW`\0\x80\xFD[a\t\xA8\x86\x83\x87\x01a\x08PV[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\t\xBEW`\0\x80\xFD[Pa\t\xCB\x85\x82\x86\x01a\x07\xDEV[` \x83\x01RPa\t\xDD`@\x84\x01a\x084V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\xFBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x11W`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\tOV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08KW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\nNW`\0\x80\xFD[a\x06\xDA\x82a\n%V[`\0` \x82\x84\x03\x12\x15a\niW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x7FW`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x06\xDAW`\0\x80\xFD[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n\xAAWa\n\xAAa\x06\xE1V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\n\xC5W`\0\x80\xFD[\x815` a\n\xD5a\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\xF4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\x17W`\0\x80\x81\xFD[a\x0B%\x89\x86\x83\x8B\x01\x01a\x07\xDEV[\x84RP\x91\x83\x01\x91\x83\x01a\n\xF8V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x0BQW`\0\x80\xFD[a\x0BYa\x07\x1FV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0BrW`\0\x80\xFD[a\x0B~\x85\x83\x86\x01a\x07\xDEV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B\x94W`\0\x80\xFD[a\x0B\xA0\x85\x83\x86\x01a\x07\xDEV[` \x84\x01Ra\x0B\xB1`@\x85\x01a\x084V[`@\x84\x01Ra\x0B\xC2``\x85\x01a\n%V[``\x84\x01Ra\x0B\xD3`\x80\x85\x01a\x084V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xECW`\0\x80\xFD[a\x0B\xF8\x85\x83\x86\x01a\n\xB4V[`\xA0\x84\x01Ra\x0C\t`\xC0\x85\x01a\x084V[`\xC0\x84\x01R`\xE0\x84\x015\x91P\x80\x82\x11\x15a\x0C\"W`\0\x80\xFD[Pa\x0C/\x84\x82\x85\x01a\x07\xDEV[`\xE0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0CMW`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0CcW`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0CwW`\0\x80\xFD[a\x0C\x7Fa\x07BV[\x82\x825\x11\x15a\x0C\x8DW`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0C\xA1W`\0\x80\xFD[a\x0C\xA9a\x07BV[\x84\x825\x11\x15a\x0C\xB7W`\0\x80\xFD[a\x0C\xC4\x88\x835\x84\x01a\x0B>V[\x81R\x84` \x83\x015\x11\x15a\x0C\xD7W`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x0C\xEFW`\0\x80\xFD[a\x0C\xFCa\x07\xFD\x835a\n\x91V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\r\x1BW`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\r\xBAW\x88\x815\x11\x15a\r6W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\rNW`\0\x80\xFD[a\rVa\x07BV[\x8A` \x83\x015\x11\x15a\rgW`\0\x80\xFD[a\ry\x8E` \x80\x85\x015\x85\x01\x01a\x07\xDEV[\x81R\x8A`@\x83\x015\x11\x15a\r\x8CW`\0\x80\xFD[a\r\x9F\x8E` `@\x85\x015\x85\x01\x01a\x07\xDEV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\r V[PP\x80` \x84\x01RPP\x80\x83RPPa\r\xD5` \x83\x01a\n%V[` \x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\r\xF5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x0CW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0E W`\0\x80\xFD[a\x0E(a\x07BV[\x825\x82\x81\x11\x15a\x0E7W`\0\x80\xFD[a\x0EC\x87\x82\x86\x01a\tOV[\x82RPa\r\xD5` \x84\x01a\n%V[`\0` \x82\x84\x03\x12\x15a\x0EdW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0EzW`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\x08PV[`\0` \x82\x84\x03\x12\x15a\x0E\x98W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xAEW`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\x0B>V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0E\xD0W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0E\xF5W\x81\x81\x01Q\x83\x82\x01R` \x01a\x0E\xDDV[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0F\x10W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F&W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0F7W`\0\x80\xFD[\x80Qa\x0FEa\x07\xFD\x82a\x07\xB7V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0FZW`\0\x80\xFD[a\x0Fk\x82` \x83\x01` \x86\x01a\x0E\xDAV[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\x8BW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0F\xA5W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0F\xBAW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\xFDW`\0\x80\xFD[\x83\x86\x11\x15a\x10\nW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0``\x82\x84\x03\x12\x15a\x10)W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10KWa\x10Ka\x06\xE1V[`@Ra\x10W\x83a\n%V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x10vW`\0\x80\xFD[`@\x82\x01R\x93\x92PPPV[`\0\x82`\x1F\x83\x01\x12a\x10\x93W`\0\x80\xFD[\x815` a\x10\xA3a\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10\xC2W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3W\x805\x83R\x91\x83\x01\x91\x83\x01a\x10\xC6V[`\0\x82`\x1F\x83\x01\x12a\x10\xEEW`\0\x80\xFD[\x815` a\x10\xFEa\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x1DW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3Wa\x112\x81a\n%V[\x83R\x91\x83\x01\x91\x83\x01a\x11!V[`\0` \x82\x84\x03\x12\x15a\x11QW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11hW`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x11}W`\0\x80\xFD[a\x11\x85a\x07dV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x11\xA9``\x84\x01a\n%V[``\x82\x01Ra\x11\xBA`\x80\x84\x01a\n%V[`\x80\x82\x01Ra\x11\xCB`\xA0\x84\x01a\n%V[`\xA0\x82\x01Ra\x11\xDC`\xC0\x84\x01a\n%V[`\xC0\x82\x01Ra\x11\xED`\xE0\x84\x01a\n%V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x12\x16\x81\x85\x01a\n%V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x12.W`\0\x80\xFD[a\x12:\x88\x82\x87\x01a\x10\x82V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x12TW`\0\x80\xFD[a\x12`\x88\x82\x87\x01a\x10\xDDV[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x12zW`\0\x80\xFD[a\x12\x86\x88\x82\x87\x01a\x07\xDEV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12\xC7W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xABV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12\xC7W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xE6V[`\0\x81Q\x80\x84Ra\x13#\x81` \x86\x01` \x86\x01a\x0E\xDAV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x13r`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x13\xCE\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x14\x04\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x14$a\x01\xE0\x86\x01\x84a\x12\x97V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x14E\x85\x84a\x12\xD2V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x14a\x83\x82a\x13\x0BV[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \x8B\xDB*\xE1\x9E\x8C\x1D\xB8\x88\xA9\xC0\xCBXD\x93\xF4\x81\xA5\x7F\xAA\xB8\x1Cj\n\xD5\x90<b\x9A\xEF:YdsolcC\0\x08\x11\x003";
	/// The bytecode of the contract.
	pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__BYTECODE);
	#[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\x8AW`\x005`\xE0\x1C\x80cD\xAB \xF8\x11a\0YW\x80cD\xAB \xF8\x14a\x01-W\x80c\xB2\xA0\x1B\xF5\x14a\x01HW\x80c\xBC\r\xD4G\x14a\x01cW\x80c\xCF\xF0\xAB\x96\x14a\x01~W\x80c\xD0\xFF\xF3f\x14a\x01\xD8W`\0\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\x96W\x80c\x0B\xC3{\xAB\x14a\0\xCBW\x80c\x0E\x83$\xA2\x14a\0\xEDW\x80c\x0F\xEE2\xCE\x14a\x01\rW`\0\x80\xFD[6a\0\x91W\0[`\0\x80\xFD[4\x80\x15a\0\xA2W`\0\x80\xFD[Pa\0\xB6a\0\xB16`\x04a\x06\xB0V[a\x01\xF3V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xD7W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\t\xE9V[a\x02*V[\0[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\0\xEBa\x01\x086`\x04a\n<V[a\x02|V[4\x80\x15a\x01\x19W`\0\x80\xFD[Pa\0\xEBa\x01(6`\x04a\nWV[a\x02\xD2V[4\x80\x15a\x019W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0C;V[4\x80\x15a\x01TW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\r\xE3V[4\x80\x15a\x01oW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0ERV[4\x80\x15a\x01\x8AW`\0\x80\xFD[P`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xC2V[4\x80\x15a\x01\xE4W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\x86V[`\0`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02$WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x022a\x05\x9BV[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02cW`@QcT\xBF\xF8E`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x02\xA8W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x02\xFEW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\t\x83\x80a\x0E\xBAV[\x90Pa\x03\xD2`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03cW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\x8B\x91\x90\x81\x01\x90a\x0E\xFEV[a\x03\x95\x83\x80a\x0FtV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x06\x86\x90PV[a\x03\xEFW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0a\x03\xFE`\xC0\x83\x01\x83a\x0FtV[`\0\x81\x81\x10a\x04\x0FWa\x04\x0Fa\x0F\xC1V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04*Wa\x04*a\x0F\xD7V[\x90P`\0\x81`\x01\x81\x11\x15a\x04@Wa\x04@a\x0F\xD7V[\x03a\x04\xEDW`\0a\x04T`\xC0\x84\x01\x84a\x0FtV[a\x04b\x91`\x01\x90\x82\x90a\x0F\xEDV[\x81\x01\x90a\x04o\x91\x90a\x10\x17V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xCFW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xE3W=`\0\x80>=`\0\xFD[PPPPPa\x05\x95V[`\x01\x81`\x01\x81\x11\x15a\x05\x01Wa\x05\x01a\x0F\xD7V[\x03a\x05\x95W`\0a\x05\x15`\xC0\x84\x01\x84a\x0FtV[a\x05#\x91`\x01\x90\x82\x90a\x0F\xEDV[\x81\x01\x90a\x050\x91\x90a\x11?V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x05a\x90\x84\x90`\x04\x01a\x137V[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05{W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x8FW=`\0\x80>=`\0\xFD[PPPPP[PPPPV[`\0Fb\xAA6\xA7\x81\x14a\x05\xD3Wb\x06n\xEE\x81\x14a\x05\xEFWb\xAA7\xDC\x81\x14a\x06\x0BWb\x01J4\x81\x14a\x06'W`a\x81\x14a\x06CWa\x06[V[s\xF1\xC7\xA3\x862[}\"\x02]uB\xB2\x8E\xE8\x81\xCD\xF1\x07\xB3\x91Pa\x06[V[s(n\x1F\xE1\xC3#\xEEbk\xE8\x02\xB1:Q\x84\xB5\x88\xED\x14\xCB\x91Pa\x06[V[sb\\S\x1AV\xDBw,\xC3c\x13\xD0\xA0\x11IV\xAD\x8BV\xC2\x91Pa\x06[V[s\xAE\x9FI\x0E\xE0U\x88\xFD\xD8W\xA0x\xCF\xC1\xF5\xF3\n\xE7\x18_\x91Pa\x06[V[s\xEB\x89w\xED\xCD\xA5\xFA\xBD\xCD\xDE\xB3\x98a\xDF%\xE8\x82\x1A\x9E\x9B\x91P[P`\x01`\x01`\xA0\x1B\x03\x81\x16a\x06\x83W`@Qc\xD2\x1E\xAB7`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x90V[`\0\x81Q\x83Q\x14a\x06\x99WP`\0a\x02$V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[`\0` \x82\x84\x03\x12\x15a\x06\xC2W`\0\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x06\xDAW`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@R\x90V[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x19Wa\x07\x19a\x06\xE1V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xAFWa\x07\xAFa\x06\xE1V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07\xD0Wa\x07\xD0a\x06\xE1V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07\xEFW`\0\x80\xFD[\x815a\x08\x02a\x07\xFD\x82a\x07\xB7V[a\x07\x87V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08\x17W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08KW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08bW`\0\x80\xFD[a\x08ja\x06\xF7V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x83W`\0\x80\xFD[a\x08\x8F\x85\x83\x86\x01a\x07\xDEV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xA5W`\0\x80\xFD[a\x08\xB1\x85\x83\x86\x01a\x07\xDEV[` \x84\x01Ra\x08\xC2`@\x85\x01a\x084V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08\xDBW`\0\x80\xFD[a\x08\xE7\x85\x83\x86\x01a\x07\xDEV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\0W`\0\x80\xFD[a\t\x0C\x85\x83\x86\x01a\x07\xDEV[`\x80\x84\x01Ra\t\x1D`\xA0\x85\x01a\x084V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t6W`\0\x80\xFD[Pa\tC\x84\x82\x85\x01a\x07\xDEV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\taW`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\x84Wa\t\x84a\x06\xE1V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\t\x9CW`\0\x80\xFD[a\t\xA8\x86\x83\x87\x01a\x08PV[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\t\xBEW`\0\x80\xFD[Pa\t\xCB\x85\x82\x86\x01a\x07\xDEV[` \x83\x01RPa\t\xDD`@\x84\x01a\x084V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\xFBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x11W`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\tOV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08KW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\nNW`\0\x80\xFD[a\x06\xDA\x82a\n%V[`\0` \x82\x84\x03\x12\x15a\niW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x7FW`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x06\xDAW`\0\x80\xFD[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n\xAAWa\n\xAAa\x06\xE1V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\n\xC5W`\0\x80\xFD[\x815` a\n\xD5a\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\xF4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\x17W`\0\x80\x81\xFD[a\x0B%\x89\x86\x83\x8B\x01\x01a\x07\xDEV[\x84RP\x91\x83\x01\x91\x83\x01a\n\xF8V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x0BQW`\0\x80\xFD[a\x0BYa\x07\x1FV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0BrW`\0\x80\xFD[a\x0B~\x85\x83\x86\x01a\x07\xDEV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B\x94W`\0\x80\xFD[a\x0B\xA0\x85\x83\x86\x01a\x07\xDEV[` \x84\x01Ra\x0B\xB1`@\x85\x01a\x084V[`@\x84\x01Ra\x0B\xC2``\x85\x01a\n%V[``\x84\x01Ra\x0B\xD3`\x80\x85\x01a\x084V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xECW`\0\x80\xFD[a\x0B\xF8\x85\x83\x86\x01a\n\xB4V[`\xA0\x84\x01Ra\x0C\t`\xC0\x85\x01a\x084V[`\xC0\x84\x01R`\xE0\x84\x015\x91P\x80\x82\x11\x15a\x0C\"W`\0\x80\xFD[Pa\x0C/\x84\x82\x85\x01a\x07\xDEV[`\xE0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0CMW`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0CcW`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0CwW`\0\x80\xFD[a\x0C\x7Fa\x07BV[\x82\x825\x11\x15a\x0C\x8DW`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0C\xA1W`\0\x80\xFD[a\x0C\xA9a\x07BV[\x84\x825\x11\x15a\x0C\xB7W`\0\x80\xFD[a\x0C\xC4\x88\x835\x84\x01a\x0B>V[\x81R\x84` \x83\x015\x11\x15a\x0C\xD7W`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x0C\xEFW`\0\x80\xFD[a\x0C\xFCa\x07\xFD\x835a\n\x91V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\r\x1BW`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\r\xBAW\x88\x815\x11\x15a\r6W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\rNW`\0\x80\xFD[a\rVa\x07BV[\x8A` \x83\x015\x11\x15a\rgW`\0\x80\xFD[a\ry\x8E` \x80\x85\x015\x85\x01\x01a\x07\xDEV[\x81R\x8A`@\x83\x015\x11\x15a\r\x8CW`\0\x80\xFD[a\r\x9F\x8E` `@\x85\x015\x85\x01\x01a\x07\xDEV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\r V[PP\x80` \x84\x01RPP\x80\x83RPPa\r\xD5` \x83\x01a\n%V[` \x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\r\xF5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x0CW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0E W`\0\x80\xFD[a\x0E(a\x07BV[\x825\x82\x81\x11\x15a\x0E7W`\0\x80\xFD[a\x0EC\x87\x82\x86\x01a\tOV[\x82RPa\r\xD5` \x84\x01a\n%V[`\0` \x82\x84\x03\x12\x15a\x0EdW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0EzW`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\x08PV[`\0` \x82\x84\x03\x12\x15a\x0E\x98W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xAEW`\0\x80\xFD[a\n\x1D\x84\x82\x85\x01a\x0B>V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0E\xD0W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0E\xF5W\x81\x81\x01Q\x83\x82\x01R` \x01a\x0E\xDDV[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0F\x10W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F&W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0F7W`\0\x80\xFD[\x80Qa\x0FEa\x07\xFD\x82a\x07\xB7V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0FZW`\0\x80\xFD[a\x0Fk\x82` \x83\x01` \x86\x01a\x0E\xDAV[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\x8BW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0F\xA5W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0F\xBAW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\xFDW`\0\x80\xFD[\x83\x86\x11\x15a\x10\nW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0``\x82\x84\x03\x12\x15a\x10)W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10KWa\x10Ka\x06\xE1V[`@Ra\x10W\x83a\n%V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x10vW`\0\x80\xFD[`@\x82\x01R\x93\x92PPPV[`\0\x82`\x1F\x83\x01\x12a\x10\x93W`\0\x80\xFD[\x815` a\x10\xA3a\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10\xC2W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3W\x805\x83R\x91\x83\x01\x91\x83\x01a\x10\xC6V[`\0\x82`\x1F\x83\x01\x12a\x10\xEEW`\0\x80\xFD[\x815` a\x10\xFEa\x07\xFD\x83a\n\x91V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x1DW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0B3Wa\x112\x81a\n%V[\x83R\x91\x83\x01\x91\x83\x01a\x11!V[`\0` \x82\x84\x03\x12\x15a\x11QW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11hW`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x11}W`\0\x80\xFD[a\x11\x85a\x07dV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x11\xA9``\x84\x01a\n%V[``\x82\x01Ra\x11\xBA`\x80\x84\x01a\n%V[`\x80\x82\x01Ra\x11\xCB`\xA0\x84\x01a\n%V[`\xA0\x82\x01Ra\x11\xDC`\xC0\x84\x01a\n%V[`\xC0\x82\x01Ra\x11\xED`\xE0\x84\x01a\n%V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x12\x16\x81\x85\x01a\n%V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x12.W`\0\x80\xFD[a\x12:\x88\x82\x87\x01a\x10\x82V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x12TW`\0\x80\xFD[a\x12`\x88\x82\x87\x01a\x10\xDDV[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x12zW`\0\x80\xFD[a\x12\x86\x88\x82\x87\x01a\x07\xDEV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12\xC7W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xABV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12\xC7W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xE6V[`\0\x81Q\x80\x84Ra\x13#\x81` \x86\x01` \x86\x01a\x0E\xDAV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x13r`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x13\xCE\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x14\x04\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x14$a\x01\xE0\x86\x01\x84a\x12\x97V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x14E\x85\x84a\x12\xD2V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x14a\x83\x82a\x13\x0BV[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \x8B\xDB*\xE1\x9E\x8C\x1D\xB8\x88\xA9\xC0\xCBXD\x93\xF4\x81\xA5\x7F\xAA\xB8\x1Cj\n\xD5\x90<b\x9A\xEF:YdsolcC\0\x08\x11\x003";
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
	///Custom Error type `UnauthorizedAccount` with signature `UnauthorizedAccount()` and selector
	/// `0xa97ff08a`
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
	#[etherror(name = "UnauthorizedAccount", abi = "UnauthorizedAccount()")]
	pub struct UnauthorizedAccount;
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
	///Custom Error type `UnsupportedChain` with signature `UnsupportedChain()` and selector
	/// `0xd21eab37`
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
	#[etherror(name = "UnsupportedChain", abi = "UnsupportedChain()")]
	pub struct UnsupportedChain;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum HostManagerErrors {
		UnauthorizedAccount(UnauthorizedAccount),
		UnauthorizedAction(UnauthorizedAction),
		UnexpectedCall(UnexpectedCall),
		UnsupportedChain(UnsupportedChain),
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
				<UnauthorizedAccount as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnauthorizedAccount(decoded));
			}
			if let Ok(decoded) =
				<UnauthorizedAction as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnauthorizedAction(decoded));
			}
			if let Ok(decoded) = <UnexpectedCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::UnexpectedCall(decoded));
			}
			if let Ok(decoded) = <UnsupportedChain as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnsupportedChain(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for HostManagerErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::UnauthorizedAccount(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnauthorizedAction(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnexpectedCall(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnsupportedChain(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for HostManagerErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<UnauthorizedAccount as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnauthorizedAction as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnexpectedCall as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnsupportedChain as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for HostManagerErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::UnauthorizedAccount(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnauthorizedAction(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnexpectedCall(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnsupportedChain(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for HostManagerErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<UnauthorizedAccount> for HostManagerErrors {
		fn from(value: UnauthorizedAccount) -> Self {
			Self::UnauthorizedAccount(value)
		}
	}
	impl ::core::convert::From<UnauthorizedAction> for HostManagerErrors {
		fn from(value: UnauthorizedAction) -> Self {
			Self::UnauthorizedAction(value)
		}
	}
	impl ::core::convert::From<UnexpectedCall> for HostManagerErrors {
		fn from(value: UnexpectedCall) -> Self {
			Self::UnexpectedCall(value)
		}
	}
	impl ::core::convert::From<UnsupportedChain> for HostManagerErrors {
		fn from(value: UnsupportedChain) -> Self {
			Self::UnsupportedChain(value)
		}
	}
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
		OnAccept(OnAcceptCall),
		OnGetResponse(OnGetResponseCall),
		OnGetTimeout(OnGetTimeoutCall),
		OnPostRequestTimeout(OnPostRequestTimeoutCall),
		OnPostResponse(OnPostResponseCall),
		OnPostResponseTimeout(OnPostResponseTimeoutCall),
		Params(ParamsCall),
		SetIsmpHost(SetIsmpHostCall),
		SupportsInterface(SupportsInterfaceCall),
	}
	impl ::ethers::core::abi::AbiDecode for HostManagerCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
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
				Self::OnAccept(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnGetResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnGetTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostRequestTimeout(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::OnPostResponseTimeout(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Params(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetIsmpHost(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SupportsInterface(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for HostManagerCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostRequestTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::OnPostResponseTimeout(element) => ::core::fmt::Display::fmt(element, f),
				Self::Params(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
				Self::SupportsInterface(element) => ::core::fmt::Display::fmt(element, f),
			}
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
