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
										::ethers::core::abi::ethabi::ParamType::Bytes,
										::ethers::core::abi::ethabi::ParamType::Uint(64usize),
										::ethers::core::abi::ethabi::ParamType::Array(
											::std::boxed::Box::new(
												::ethers::core::abi::ethabi::ParamType::Bytes,
											),
										),
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
								::ethers::core::abi::ethabi::ParamType::Bytes,
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
								::ethers::core::abi::ethabi::ParamType::Array(
									::std::boxed::Box::new(
										::ethers::core::abi::ethabi::ParamType::Bytes,
									),
								),
								::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
			]),
			events: ::std::collections::BTreeMap::new(),
			errors: ::std::collections::BTreeMap::new(),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static HOSTMANAGER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	#[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x14X8\x03\x80a\x14X\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\x83V[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xEBV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0~W`\0\x80\xFD[\x91\x90PV[`\0`@\x82\x84\x03\x12\x15a\0\x95W`\0\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xC5WcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD1\x83a\0gV[\x81Ra\0\xDF` \x84\x01a\0gV[` \x82\x01R\x93\x92PPPV[a\x13^\x80a\0\xFA`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xB5\xA9\x82K\x11a\0[W\x80c\xB5\xA9\x82K\x14a\0\xD6W\x80c\xBC\r\xD4G\x14a\0\xE9W\x80c\xC4\x92\xE4&\x14a\0\xFCW\x80c\xCF\xF0\xAB\x96\x14a\x01\nW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\x0F\xEE2\xCE\x14a\0\xB5W\x80c\xB2\xA0\x1B\xF5\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\t\nV[a\x01]V[\0[a\0\xA0a\0\xB06`\x04a\t]V[a\x01\xB9V[a\0\xA0a\0\xC36`\x04a\t\x7FV[a\x02<V[a\0\xA0a\0\x9B6`\x04a\t\xB9V[a\0\xA0a\0\xE46`\x04a\x0B\xCEV[a\x05NV[a\0\xA0a\0\xF76`\x04a\rhV[a\x05\xA2V[a\0\xA0a\0\xE46`\x04a\r\x9CV[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x13W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xB0V[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x96W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xB0V[6a\x02\xA1\x82\x80a\r\xD0V[\x90Pa\x03j`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x02\xFBW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03#\x91\x90\x81\x01\x90a\x0E\x14V[a\x03-\x83\x80a\x0E\x8AV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xF8\x90PV[a\x03\xADW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xB0V[`\0a\x03\xBC`\xC0\x83\x01\x83a\x0E\x8AV[`\0\x81\x81\x10a\x03\xCDWa\x03\xCDa\x0E\xD7V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x03\xE8Wa\x03\xE8a\x0E\xEDV[\x90P`\0\x81`\x01\x81\x11\x15a\x03\xFEWa\x03\xFEa\x0E\xEDV[\x03a\x04\xA1W`\0a\x04\x12`\xC0\x84\x01\x84a\x0E\x8AV[a\x04 \x91`\x01\x90\x82\x90a\x0F\x03V[\x81\x01\x90a\x04-\x91\x90a\x0F-V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\x83W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\x97W=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xB5Wa\x04\xB5a\x0E\xEDV[\x03a\x05\x15W`\0a\x04\xC9`\xC0\x84\x01\x84a\x0E\x8AV[a\x04\xD7\x91`\x01\x90\x82\x90a\x0F\x03V[\x81\x01\x90a\x04\xE4\x91\x90a\x10@V[`\x01T`@Qc\x03\xCB\x07\xF5`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\x03\xCB\x07\xF5\x90a\x04i\x90\x84\x90`\x04\x01a\x12\x1BV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xB0V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xB0V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xB0V[`\0\x81Q\x83Q\x14a\x06\x0BWP`\0a\x06\x1FV[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@Qa\x01\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\xD0Wa\x06\xD0a\x06%V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x06\xF1Wa\x06\xF1a\x06%V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07\x10W`\0\x80\xFD[\x815a\x07#a\x07\x1E\x82a\x06\xD8V[a\x06\xA8V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x078W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07lW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x07\x83W`\0\x80\xFD[a\x07\x8Ba\x06;V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xA4W`\0\x80\xFD[a\x07\xB0\x85\x83\x86\x01a\x06\xFFV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\xC6W`\0\x80\xFD[a\x07\xD2\x85\x83\x86\x01a\x06\xFFV[` \x84\x01Ra\x07\xE3`@\x85\x01a\x07UV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xFCW`\0\x80\xFD[a\x08\x08\x85\x83\x86\x01a\x06\xFFV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x08!W`\0\x80\xFD[a\x08-\x85\x83\x86\x01a\x06\xFFV[`\x80\x84\x01Ra\x08>`\xA0\x85\x01a\x07UV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08WW`\0\x80\xFD[Pa\x08d\x84\x82\x85\x01a\x06\xFFV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\x08\x82W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\x08\xA5Wa\x08\xA5a\x06%V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\x08\xBDW`\0\x80\xFD[a\x08\xC9\x86\x83\x87\x01a\x07qV[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\x08\xDFW`\0\x80\xFD[Pa\x08\xEC\x85\x82\x86\x01a\x06\xFFV[` \x83\x01RPa\x08\xFE`@\x84\x01a\x07UV[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\x1CW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t2W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\x08pV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x07lW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\toW`\0\x80\xFD[a\tx\x82a\tFV[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\t\x91W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xA7W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\txW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\t\xCBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xE2W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\t\xF6W`\0\x80\xFD[a\t\xFEa\x06cV[\x825\x82\x81\x11\x15a\n\rW`\0\x80\xFD[a\n\x19\x87\x82\x86\x01a\x08pV[\x82RPa\n(` \x84\x01a\tFV[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\nOWa\nOa\x06%V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\njW`\0\x80\xFD[\x815` a\nza\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\x99W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xBCW`\0\x80\x81\xFD[a\n\xCA\x89\x86\x83\x8B\x01\x01a\x06\xFFV[\x84RP\x91\x83\x01\x91\x83\x01a\n\x9DV[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\n\xF5W`\0\x80\xFD[a\n\xFDa\x06;V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x16W`\0\x80\xFD[a\x0B\"\x85\x83\x86\x01a\x06\xFFV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B8W`\0\x80\xFD[a\x0BD\x85\x83\x86\x01a\x06\xFFV[` \x84\x01Ra\x0BU`@\x85\x01a\x07UV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0BnW`\0\x80\xFD[a\x0Bz\x85\x83\x86\x01a\x06\xFFV[``\x84\x01Ra\x0B\x8B`\x80\x85\x01a\x07UV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xA4W`\0\x80\xFD[Pa\x0B\xB1\x84\x82\x85\x01a\nYV[`\xA0\x83\x01RPa\x0B\xC3`\xC0\x83\x01a\x07UV[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0B\xE0W`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0B\xF6W`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0C\nW`\0\x80\xFD[a\x0C\x12a\x06cV[\x82\x825\x11\x15a\x0C W`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0C4W`\0\x80\xFD[a\x0C<a\x06cV[\x84\x825\x11\x15a\x0CJW`\0\x80\xFD[a\x0CW\x88\x835\x84\x01a\n\xE3V[\x81R\x84` \x83\x015\x11\x15a\x0CjW`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x0C\x82W`\0\x80\xFD[a\x0C\x8Fa\x07\x1E\x835a\n6V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x0C\xAEW`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\rMW\x88\x815\x11\x15a\x0C\xC9W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0C\xE1W`\0\x80\xFD[a\x0C\xE9a\x06cV[\x8A` \x83\x015\x11\x15a\x0C\xFAW`\0\x80\xFD[a\r\x0C\x8E` \x80\x85\x015\x85\x01\x01a\x06\xFFV[\x81R\x8A`@\x83\x015\x11\x15a\r\x1FW`\0\x80\xFD[a\r2\x8E` `@\x85\x015\x85\x01\x01a\x06\xFFV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x0C\xB3V[PP\x80` \x84\x01RPP\x80\x83RPPa\n(` \x83\x01a\tFV[`\0` \x82\x84\x03\x12\x15a\rzW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\x90W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\x07qV[`\0` \x82\x84\x03\x12\x15a\r\xAEW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\xC4W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\n\xE3V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\r\xE6W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0E\x0BW\x81\x81\x01Q\x83\x82\x01R` \x01a\r\xF3V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0E&W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E<W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0EMW`\0\x80\xFD[\x80Qa\x0E[a\x07\x1E\x82a\x06\xD8V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0EpW`\0\x80\xFD[a\x0E\x81\x82` \x83\x01` \x86\x01a\r\xF0V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0E\xA1W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0E\xBBW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E\xD0W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\x13W`\0\x80\xFD[\x83\x86\x11\x15a\x0F W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F?W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0FaWa\x0Faa\x06%V[`@Ra\x0Fm\x83a\tFV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x0F\x94W`\0\x80\xFD[\x815` a\x0F\xA4a\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0F\xC3W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8W\x805\x83R\x91\x83\x01\x91\x83\x01a\x0F\xC7V[`\0\x82`\x1F\x83\x01\x12a\x0F\xEFW`\0\x80\xFD[\x815` a\x0F\xFFa\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10\x1EW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8Wa\x103\x81a\tFV[\x83R\x91\x83\x01\x91\x83\x01a\x10\"V[`\0` \x82\x84\x03\x12\x15a\x10RW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x10iW`\0\x80\xFD[\x90\x83\x01\x90a\x01\x80\x82\x86\x03\x12\x15a\x10~W`\0\x80\xFD[a\x10\x86a\x06\x85V[\x825\x81R` \x83\x015` \x82\x01Ra\x10\xA0`@\x84\x01a\tFV[`@\x82\x01Ra\x10\xB1``\x84\x01a\tFV[``\x82\x01Ra\x10\xC2`\x80\x84\x01a\tFV[`\x80\x82\x01Ra\x10\xD3`\xA0\x84\x01a\tFV[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x10\xFA\x81\x85\x01a\tFV[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x11\x12W`\0\x80\xFD[a\x11\x1E\x88\x82\x87\x01a\x0F\x83V[\x82\x84\x01RPPa\x01@\x80\x84\x015\x83\x81\x11\x15a\x118W`\0\x80\xFD[a\x11D\x88\x82\x87\x01a\x0F\xDEV[\x82\x84\x01RPPa\x01`\x80\x84\x015\x83\x81\x11\x15a\x11^W`\0\x80\xFD[a\x11j\x88\x82\x87\x01a\x06\xFFV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x11\xABW\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\x8FV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x11\xABW\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\xCAV[`\0\x81Q\x80\x84Ra\x12\x07\x81` \x86\x01` \x86\x01a\r\xF0V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x12L``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x12\xC1\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\x80a\x01@\x81\x81\x86\x01Ra\x12\xE1a\x01\xA0\x86\x01\x84a\x11{V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01`\x81\x87\x86\x03\x01\x81\x88\x01Ra\x13\x02\x85\x84a\x11\xB6V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x13\x1E\x83\x82a\x11\xEFV[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xF6\x81\t\x1B)<\xC7\x06\x97\xCD<z\xA6\xCA\xB2\xF2\x98\x82\xBF\xA0\x0Flf\xC6\n\x9B\x8D\xE8d\xF5\xB8\x1EdsolcC\0\x08\x11\x003";
	/// The bytecode of the contract.
	pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
		::ethers::core::types::Bytes::from_static(__BYTECODE);
	#[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xB5\xA9\x82K\x11a\0[W\x80c\xB5\xA9\x82K\x14a\0\xD6W\x80c\xBC\r\xD4G\x14a\0\xE9W\x80c\xC4\x92\xE4&\x14a\0\xFCW\x80c\xCF\xF0\xAB\x96\x14a\x01\nW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\x0F\xEE2\xCE\x14a\0\xB5W\x80c\xB2\xA0\x1B\xF5\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\t\nV[a\x01]V[\0[a\0\xA0a\0\xB06`\x04a\t]V[a\x01\xB9V[a\0\xA0a\0\xC36`\x04a\t\x7FV[a\x02<V[a\0\xA0a\0\x9B6`\x04a\t\xB9V[a\0\xA0a\0\xE46`\x04a\x0B\xCEV[a\x05NV[a\0\xA0a\0\xF76`\x04a\rhV[a\x05\xA2V[a\0\xA0a\0\xE46`\x04a\r\x9CV[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x13W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xB0V[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x96W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xB0V[6a\x02\xA1\x82\x80a\r\xD0V[\x90Pa\x03j`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x02\xFBW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03#\x91\x90\x81\x01\x90a\x0E\x14V[a\x03-\x83\x80a\x0E\x8AV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xF8\x90PV[a\x03\xADW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xB0V[`\0a\x03\xBC`\xC0\x83\x01\x83a\x0E\x8AV[`\0\x81\x81\x10a\x03\xCDWa\x03\xCDa\x0E\xD7V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x03\xE8Wa\x03\xE8a\x0E\xEDV[\x90P`\0\x81`\x01\x81\x11\x15a\x03\xFEWa\x03\xFEa\x0E\xEDV[\x03a\x04\xA1W`\0a\x04\x12`\xC0\x84\x01\x84a\x0E\x8AV[a\x04 \x91`\x01\x90\x82\x90a\x0F\x03V[\x81\x01\x90a\x04-\x91\x90a\x0F-V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\x83W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\x97W=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xB5Wa\x04\xB5a\x0E\xEDV[\x03a\x05\x15W`\0a\x04\xC9`\xC0\x84\x01\x84a\x0E\x8AV[a\x04\xD7\x91`\x01\x90\x82\x90a\x0F\x03V[\x81\x01\x90a\x04\xE4\x91\x90a\x10@V[`\x01T`@Qc\x03\xCB\x07\xF5`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\x03\xCB\x07\xF5\x90a\x04i\x90\x84\x90`\x04\x01a\x12\x1BV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xB0V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xB0V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xB0V[`\0\x81Q\x83Q\x14a\x06\x0BWP`\0a\x06\x1FV[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@Qa\x01\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06]Wa\x06]a\x06%V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\xD0Wa\x06\xD0a\x06%V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x06\xF1Wa\x06\xF1a\x06%V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07\x10W`\0\x80\xFD[\x815a\x07#a\x07\x1E\x82a\x06\xD8V[a\x06\xA8V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x078W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07lW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x07\x83W`\0\x80\xFD[a\x07\x8Ba\x06;V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xA4W`\0\x80\xFD[a\x07\xB0\x85\x83\x86\x01a\x06\xFFV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\xC6W`\0\x80\xFD[a\x07\xD2\x85\x83\x86\x01a\x06\xFFV[` \x84\x01Ra\x07\xE3`@\x85\x01a\x07UV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xFCW`\0\x80\xFD[a\x08\x08\x85\x83\x86\x01a\x06\xFFV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x08!W`\0\x80\xFD[a\x08-\x85\x83\x86\x01a\x06\xFFV[`\x80\x84\x01Ra\x08>`\xA0\x85\x01a\x07UV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08WW`\0\x80\xFD[Pa\x08d\x84\x82\x85\x01a\x06\xFFV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\x08\x82W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\x08\xA5Wa\x08\xA5a\x06%V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\x08\xBDW`\0\x80\xFD[a\x08\xC9\x86\x83\x87\x01a\x07qV[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\x08\xDFW`\0\x80\xFD[Pa\x08\xEC\x85\x82\x86\x01a\x06\xFFV[` \x83\x01RPa\x08\xFE`@\x84\x01a\x07UV[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\x1CW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t2W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\x08pV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x07lW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\toW`\0\x80\xFD[a\tx\x82a\tFV[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\t\x91W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xA7W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\txW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\t\xCBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xE2W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\t\xF6W`\0\x80\xFD[a\t\xFEa\x06cV[\x825\x82\x81\x11\x15a\n\rW`\0\x80\xFD[a\n\x19\x87\x82\x86\x01a\x08pV[\x82RPa\n(` \x84\x01a\tFV[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\nOWa\nOa\x06%V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\njW`\0\x80\xFD[\x815` a\nza\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\x99W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xBCW`\0\x80\x81\xFD[a\n\xCA\x89\x86\x83\x8B\x01\x01a\x06\xFFV[\x84RP\x91\x83\x01\x91\x83\x01a\n\x9DV[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\n\xF5W`\0\x80\xFD[a\n\xFDa\x06;V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x16W`\0\x80\xFD[a\x0B\"\x85\x83\x86\x01a\x06\xFFV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B8W`\0\x80\xFD[a\x0BD\x85\x83\x86\x01a\x06\xFFV[` \x84\x01Ra\x0BU`@\x85\x01a\x07UV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0BnW`\0\x80\xFD[a\x0Bz\x85\x83\x86\x01a\x06\xFFV[``\x84\x01Ra\x0B\x8B`\x80\x85\x01a\x07UV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\xA4W`\0\x80\xFD[Pa\x0B\xB1\x84\x82\x85\x01a\nYV[`\xA0\x83\x01RPa\x0B\xC3`\xC0\x83\x01a\x07UV[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0B\xE0W`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\x0B\xF6W`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\x0C\nW`\0\x80\xFD[a\x0C\x12a\x06cV[\x82\x825\x11\x15a\x0C W`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\x0C4W`\0\x80\xFD[a\x0C<a\x06cV[\x84\x825\x11\x15a\x0CJW`\0\x80\xFD[a\x0CW\x88\x835\x84\x01a\n\xE3V[\x81R\x84` \x83\x015\x11\x15a\x0CjW`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\x0C\x82W`\0\x80\xFD[a\x0C\x8Fa\x07\x1E\x835a\n6V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x0C\xAEW`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\rMW\x88\x815\x11\x15a\x0C\xC9W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0C\xE1W`\0\x80\xFD[a\x0C\xE9a\x06cV[\x8A` \x83\x015\x11\x15a\x0C\xFAW`\0\x80\xFD[a\r\x0C\x8E` \x80\x85\x015\x85\x01\x01a\x06\xFFV[\x81R\x8A`@\x83\x015\x11\x15a\r\x1FW`\0\x80\xFD[a\r2\x8E` `@\x85\x015\x85\x01\x01a\x06\xFFV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x0C\xB3V[PP\x80` \x84\x01RPP\x80\x83RPPa\n(` \x83\x01a\tFV[`\0` \x82\x84\x03\x12\x15a\rzW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\x90W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\x07qV[`\0` \x82\x84\x03\x12\x15a\r\xAEW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\xC4W`\0\x80\xFD[a\t>\x84\x82\x85\x01a\n\xE3V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\r\xE6W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0E\x0BW\x81\x81\x01Q\x83\x82\x01R` \x01a\r\xF3V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0E&W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E<W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0EMW`\0\x80\xFD[\x80Qa\x0E[a\x07\x1E\x82a\x06\xD8V[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0EpW`\0\x80\xFD[a\x0E\x81\x82` \x83\x01` \x86\x01a\r\xF0V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0E\xA1W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0E\xBBW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E\xD0W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\x13W`\0\x80\xFD[\x83\x86\x11\x15a\x0F W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F?W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0FaWa\x0Faa\x06%V[`@Ra\x0Fm\x83a\tFV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x0F\x94W`\0\x80\xFD[\x815` a\x0F\xA4a\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0F\xC3W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8W\x805\x83R\x91\x83\x01\x91\x83\x01a\x0F\xC7V[`\0\x82`\x1F\x83\x01\x12a\x0F\xEFW`\0\x80\xFD[\x815` a\x0F\xFFa\x07\x1E\x83a\n6V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10\x1EW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xD8Wa\x103\x81a\tFV[\x83R\x91\x83\x01\x91\x83\x01a\x10\"V[`\0` \x82\x84\x03\x12\x15a\x10RW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x10iW`\0\x80\xFD[\x90\x83\x01\x90a\x01\x80\x82\x86\x03\x12\x15a\x10~W`\0\x80\xFD[a\x10\x86a\x06\x85V[\x825\x81R` \x83\x015` \x82\x01Ra\x10\xA0`@\x84\x01a\tFV[`@\x82\x01Ra\x10\xB1``\x84\x01a\tFV[``\x82\x01Ra\x10\xC2`\x80\x84\x01a\tFV[`\x80\x82\x01Ra\x10\xD3`\xA0\x84\x01a\tFV[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x10\xFA\x81\x85\x01a\tFV[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x11\x12W`\0\x80\xFD[a\x11\x1E\x88\x82\x87\x01a\x0F\x83V[\x82\x84\x01RPPa\x01@\x80\x84\x015\x83\x81\x11\x15a\x118W`\0\x80\xFD[a\x11D\x88\x82\x87\x01a\x0F\xDEV[\x82\x84\x01RPPa\x01`\x80\x84\x015\x83\x81\x11\x15a\x11^W`\0\x80\xFD[a\x11j\x88\x82\x87\x01a\x06\xFFV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x11\xABW\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\x8FV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x11\xABW\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\xCAV[`\0\x81Q\x80\x84Ra\x12\x07\x81` \x86\x01` \x86\x01a\r\xF0V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x12L``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x12\xC1\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\x80a\x01@\x81\x81\x86\x01Ra\x12\xE1a\x01\xA0\x86\x01\x84a\x11{V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01`\x81\x87\x86\x03\x01\x81\x88\x01Ra\x13\x02\x85\x84a\x11\xB6V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x13\x1E\x83\x82a\x11\xEFV[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xF6\x81\t\x1B)<\xC7\x06\x97\xCD<z\xA6\xCA\xB2\xF2\x98\x82\xBF\xA0\x0Flf\xC6\n\x9B\x8D\xE8d\xF5\xB8\x1EdsolcC\0\x08\x11\x003";
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
		///Calls the contract's `onGetResponse` (0xb5a9824b) function
		pub fn on_get_response(
			&self,
			p0: IncomingGetResponse,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([181, 169, 130, 75], (p0,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `onGetTimeout` (0xc492e426) function
		pub fn on_get_timeout(
			&self,
			p0: GetRequest,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([196, 146, 228, 38], (p0,))
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
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for HostManager<M> {
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
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
	/// `onGetResponse((((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]),
	/// address))` and selector `0xb5a9824b`
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
		abi = "onGetResponse((((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]),address))"
	)]
	pub struct OnGetResponseCall(pub IncomingGetResponse);
	///Container type for all input parameters for the `onGetTimeout` function with signature
	/// `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))` and selector `0xc492e426`
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
		abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))"
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
