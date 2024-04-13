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
                        ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
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
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x14\xB18\x03\x80a\x14\xB1\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\x8BV[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x83\x01Q`\x01U`@\x90\x92\x01Q`\x02\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xFDV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0\x86W`\0\x80\xFD[\x91\x90PV[`\0``\x82\x84\x03\x12\x15a\0\x9DW`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xCDWcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD9\x83a\0oV[\x81R` \x83\x01Q` \x82\x01Ra\0\xF1`@\x84\x01a\0oV[`@\x82\x01R\x93\x92PPPV[a\x13\xA5\x80a\x01\x0C`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xB5\xA9\x82K\x11a\0[W\x80c\xB5\xA9\x82K\x14a\0\xD6W\x80c\xBC\r\xD4G\x14a\0\xE9W\x80c\xC4\x92\xE4&\x14a\0\xFCW\x80c\xCF\xF0\xAB\x96\x14a\x01\nW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\x0F\xEE2\xCE\x14a\0\xB5W\x80c\xB2\xA0\x1B\xF5\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\nZV[a\x01\x8FV[\0[a\0\xA0a\0\xB06`\x04a\n\xADV[a\x01\xEBV[a\0\xA0a\0\xC36`\x04a\n\xCFV[a\x02nV[a\0\xA0a\0\x9B6`\x04a\x0B\tV[a\0\xA0a\0\xE46`\x04a\r#V[a\x05\x11V[a\0\xA0a\0\xF76`\x04a\x0E\xBDV[a\x05eV[a\0\xA0a\0\xE46`\x04a\x0E\xF1V[a\x01[`@\x80Q``\x81\x01\x82R`\0\x80\x82R` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91RP`@\x80Q``\x81\x01\x82R`\0T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R`\x01T` \x83\x01R`\x02T\x16\x91\x81\x01\x91\x90\x91R\x90V[`@\x80Q\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R` \x80\x85\x01Q\x90\x83\x01R\x92\x82\x01Q\x90\x92\x16\x90\x82\x01R``\x01`@Q\x80\x91\x03\x90\xF3[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02EW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xE2V[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x02T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xC8W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xE2V[6a\x02\xD3\x82\x80a\x0F%V[\x90Pa\x03-a\x02\xE6`\0`\x01\x01Ta\x05\xBBV[a\x02\xF0\x83\x80a\x0FEV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xEC\x90PV[a\x03pW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xE2V[`\0a\x03\x7F`\xC0\x83\x01\x83a\x0FEV[`\0\x81\x81\x10a\x03\x90Wa\x03\x90a\x0F\x92V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x03\xABWa\x03\xABa\x0F\xA8V[\x90P`\0\x81`\x01\x81\x11\x15a\x03\xC1Wa\x03\xC1a\x0F\xA8V[\x03a\x04dW`\0a\x03\xD5`\xC0\x84\x01\x84a\x0FEV[a\x03\xE3\x91`\x01\x90\x82\x90a\x0F\xBEV[\x81\x01\x90a\x03\xF0\x91\x90a\x0F\xE8V[`\x02T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04FW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04ZW=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04xWa\x04xa\x0F\xA8V[\x03a\x04\xD8W`\0a\x04\x8C`\xC0\x84\x01\x84a\x0FEV[a\x04\x9A\x91`\x01\x90\x82\x90a\x0F\xBEV[\x81\x01\x90a\x04\xA7\x91\x90a\x10\x99V[`\x02T`@QcH\xDA\x177`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cH\xDA\x177\x90a\x04,\x90\x84\x90`\x04\x01a\x12EV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xE2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xE2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xE2V[``a\x05\xC6\x82a\x06\x19V[`@Q` \x01a\x05\xD6\x91\x90a\x13>V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0\x81Q\x83Q\x14a\x05\xFFWP`\0a\x06\x13V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[```\0a\x06&\x83a\x06\xABV[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06EWa\x06Ea\x07\x83V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x06oW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x06yWP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x06\xEAWr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x07\x16Wm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x074Wf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x07LWc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x07`Wa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x07rW`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x06\x13W`\x01\x01\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@Qa\x01\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08.Wa\x08.a\x07\x83V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x08GW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08`Wa\x08`a\x07\x83V[a\x08s`\x1F\x82\x01`\x1F\x19\x16` \x01a\x08\x06V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08\x88W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xBCW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xD3W`\0\x80\xFD[a\x08\xDBa\x07\x99V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xF4W`\0\x80\xFD[a\t\0\x85\x83\x86\x01a\x086V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\t\x16W`\0\x80\xFD[a\t\"\x85\x83\x86\x01a\x086V[` \x84\x01Ra\t3`@\x85\x01a\x08\xA5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\tLW`\0\x80\xFD[a\tX\x85\x83\x86\x01a\x086V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tqW`\0\x80\xFD[a\t}\x85\x83\x86\x01a\x086V[`\x80\x84\x01Ra\t\x8E`\xA0\x85\x01a\x08\xA5V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\xA7W`\0\x80\xFD[Pa\t\xB4\x84\x82\x85\x01a\x086V[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\t\xD2W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\xF5Wa\t\xF5a\x07\x83V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\n\rW`\0\x80\xFD[a\n\x19\x86\x83\x87\x01a\x08\xC1V[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\n/W`\0\x80\xFD[Pa\n<\x85\x82\x86\x01a\x086V[` \x83\x01RPa\nN`@\x84\x01a\x08\xA5V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\nlW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x82W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\t\xC0V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xBCW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\xBFW`\0\x80\xFD[a\n\xC8\x82a\n\x96V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\xE1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xF7W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\n\xC8W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x1BW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B2W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0BFW`\0\x80\xFD[a\x0BNa\x07\xC1V[\x825\x82\x81\x11\x15a\x0B]W`\0\x80\xFD[a\x0Bi\x87\x82\x86\x01a\t\xC0V[\x82RPa\x0Bx` \x84\x01a\n\x96V[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0B\x9FWa\x0B\x9Fa\x07\x83V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\xBAW`\0\x80\xFD[\x815` a\x0B\xCFa\x0B\xCA\x83a\x0B\x86V[a\x08\x06V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B\xEEW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C-W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x11W`\0\x80\x81\xFD[a\x0C\x1F\x89\x86\x83\x8B\x01\x01a\x086V[\x84RP\x91\x83\x01\x91\x83\x01a\x0B\xF2V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0CJW`\0\x80\xFD[a\x0CRa\x07\x99V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0CkW`\0\x80\xFD[a\x0Cw\x85\x83\x86\x01a\x086V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0C\x8DW`\0\x80\xFD[a\x0C\x99\x85\x83\x86\x01a\x086V[` \x84\x01Ra\x0C\xAA`@\x85\x01a\x08\xA5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\xC3W`\0\x80\xFD[a\x0C\xCF\x85\x83\x86\x01a\x086V[``\x84\x01Ra\x0C\xE0`\x80\x85\x01a\x08\xA5V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0C\xF9W`\0\x80\xFD[Pa\r\x06\x84\x82\x85\x01a\x0B\xA9V[`\xA0\x83\x01RPa\r\x18`\xC0\x83\x01a\x08\xA5V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\r5W`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\rKW`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\r_W`\0\x80\xFD[a\rga\x07\xC1V[\x82\x825\x11\x15a\ruW`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\r\x89W`\0\x80\xFD[a\r\x91a\x07\xC1V[\x84\x825\x11\x15a\r\x9FW`\0\x80\xFD[a\r\xAC\x88\x835\x84\x01a\x0C8V[\x81R\x84` \x83\x015\x11\x15a\r\xBFW`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\r\xD7W`\0\x80\xFD[a\r\xE4a\x0B\xCA\x835a\x0B\x86V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x0E\x03W`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x0E\xA2W\x88\x815\x11\x15a\x0E\x1EW`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0E6W`\0\x80\xFD[a\x0E>a\x07\xC1V[\x8A` \x83\x015\x11\x15a\x0EOW`\0\x80\xFD[a\x0Ea\x8E` \x80\x85\x015\x85\x01\x01a\x086V[\x81R\x8A`@\x83\x015\x11\x15a\x0EtW`\0\x80\xFD[a\x0E\x87\x8E` `@\x85\x015\x85\x01\x01a\x086V[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x0E\x08V[PP\x80` \x84\x01RPP\x80\x83RPPa\x0Bx` \x83\x01a\n\x96V[`\0` \x82\x84\x03\x12\x15a\x0E\xCFW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xE5W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\x08\xC1V[`\0` \x82\x84\x03\x12\x15a\x0F\x03W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\x19W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\x0C8V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0F;W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\\W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0FvW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0F\x8BW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\xCEW`\0\x80\xFD[\x83\x86\x11\x15a\x0F\xDBW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F\xFAW`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10\x1CWa\x10\x1Ca\x07\x83V[`@Ra\x10(\x83a\n\x96V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x10OW`\0\x80\xFD[\x815` a\x10_a\x0B\xCA\x83a\x0B\x86V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10~W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C-W\x805\x83R\x91\x83\x01\x91\x83\x01a\x10\x82V[`\0` \x82\x84\x03\x12\x15a\x10\xABW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x10\xC2W`\0\x80\xFD[\x90\x83\x01\x90a\x01\x80\x82\x86\x03\x12\x15a\x10\xD7W`\0\x80\xFD[a\x10\xDFa\x07\xE3V[\x825\x81R` \x83\x015` \x82\x01Ra\x10\xF9`@\x84\x01a\n\x96V[`@\x82\x01Ra\x11\n``\x84\x01a\n\x96V[``\x82\x01Ra\x11\x1B`\x80\x84\x01a\n\x96V[`\x80\x82\x01Ra\x11,`\xA0\x84\x01a\n\x96V[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x11S\x81\x85\x01a\n\x96V[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x11kW`\0\x80\xFD[a\x11w\x88\x82\x87\x01a\x086V[\x82\x84\x01RPPa\x01@\x80\x84\x015\x81\x83\x01RPa\x01`\x80\x84\x015\x83\x81\x11\x15a\x11\x9DW`\0\x80\xFD[a\x11\xA9\x88\x82\x87\x01a\x10>V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0[\x83\x81\x10\x15a\x11\xD5W\x81\x81\x01Q\x83\x82\x01R` \x01a\x11\xBDV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x11\xF6\x81` \x86\x01` \x86\x01a\x11\xBAV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12:W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\x1EV[P\x94\x95\x94PPPPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x12v``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x12\xEB\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\x80a\x01@\x81\x81\x86\x01Ra\x13\x0Ba\x01\xA0\x86\x01\x84a\x11\xDEV[\x90\x86\x01Qa\x01`\x86\x81\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x134\x83\x82a\x12\nV[\x96\x95PPPPPPV[hPOLKADOT-`\xB8\x1B\x81R`\0\x82Qa\x13b\x81`\t\x85\x01` \x87\x01a\x11\xBAV[\x91\x90\x91\x01`\t\x01\x92\x91PPV\xFE\xA2dipfsX\"\x12 \xA8\x90\xE18\x19F\xA91U\x19(u-\x1EU\x19\x12)\x84?{\xFA\x1D\x10\xB8\xB3$\x08\t\xE4+\x03dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xB5\xA9\x82K\x11a\0[W\x80c\xB5\xA9\x82K\x14a\0\xD6W\x80c\xBC\r\xD4G\x14a\0\xE9W\x80c\xC4\x92\xE4&\x14a\0\xFCW\x80c\xCF\xF0\xAB\x96\x14a\x01\nW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\x0F\xEE2\xCE\x14a\0\xB5W\x80c\xB2\xA0\x1B\xF5\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\nZV[a\x01\x8FV[\0[a\0\xA0a\0\xB06`\x04a\n\xADV[a\x01\xEBV[a\0\xA0a\0\xC36`\x04a\n\xCFV[a\x02nV[a\0\xA0a\0\x9B6`\x04a\x0B\tV[a\0\xA0a\0\xE46`\x04a\r#V[a\x05\x11V[a\0\xA0a\0\xF76`\x04a\x0E\xBDV[a\x05eV[a\0\xA0a\0\xE46`\x04a\x0E\xF1V[a\x01[`@\x80Q``\x81\x01\x82R`\0\x80\x82R` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91RP`@\x80Q``\x81\x01\x82R`\0T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R`\x01T` \x83\x01R`\x02T\x16\x91\x81\x01\x91\x90\x91R\x90V[`@\x80Q\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R` \x80\x85\x01Q\x90\x83\x01R\x92\x82\x01Q\x90\x92\x16\x90\x82\x01R``\x01`@Q\x80\x91\x03\x90\xF3[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02EW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xE2V[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x02T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xC8W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xE2V[6a\x02\xD3\x82\x80a\x0F%V[\x90Pa\x03-a\x02\xE6`\0`\x01\x01Ta\x05\xBBV[a\x02\xF0\x83\x80a\x0FEV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xEC\x90PV[a\x03pW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xE2V[`\0a\x03\x7F`\xC0\x83\x01\x83a\x0FEV[`\0\x81\x81\x10a\x03\x90Wa\x03\x90a\x0F\x92V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x03\xABWa\x03\xABa\x0F\xA8V[\x90P`\0\x81`\x01\x81\x11\x15a\x03\xC1Wa\x03\xC1a\x0F\xA8V[\x03a\x04dW`\0a\x03\xD5`\xC0\x84\x01\x84a\x0FEV[a\x03\xE3\x91`\x01\x90\x82\x90a\x0F\xBEV[\x81\x01\x90a\x03\xF0\x91\x90a\x0F\xE8V[`\x02T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04FW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04ZW=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04xWa\x04xa\x0F\xA8V[\x03a\x04\xD8W`\0a\x04\x8C`\xC0\x84\x01\x84a\x0FEV[a\x04\x9A\x91`\x01\x90\x82\x90a\x0F\xBEV[\x81\x01\x90a\x04\xA7\x91\x90a\x10\x99V[`\x02T`@QcH\xDA\x177`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cH\xDA\x177\x90a\x04,\x90\x84\x90`\x04\x01a\x12EV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xE2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xE2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xE2V[``a\x05\xC6\x82a\x06\x19V[`@Q` \x01a\x05\xD6\x91\x90a\x13>V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0\x81Q\x83Q\x14a\x05\xFFWP`\0a\x06\x13V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[```\0a\x06&\x83a\x06\xABV[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06EWa\x06Ea\x07\x83V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x06oW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x06yWP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x06\xEAWr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x07\x16Wm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x074Wf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x07LWc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x07`Wa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x07rW`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x06\x13W`\x01\x01\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@Qa\x01\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xBBWa\x07\xBBa\x07\x83V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08.Wa\x08.a\x07\x83V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x08GW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08`Wa\x08`a\x07\x83V[a\x08s`\x1F\x82\x01`\x1F\x19\x16` \x01a\x08\x06V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08\x88W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xBCW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xD3W`\0\x80\xFD[a\x08\xDBa\x07\x99V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xF4W`\0\x80\xFD[a\t\0\x85\x83\x86\x01a\x086V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\t\x16W`\0\x80\xFD[a\t\"\x85\x83\x86\x01a\x086V[` \x84\x01Ra\t3`@\x85\x01a\x08\xA5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\tLW`\0\x80\xFD[a\tX\x85\x83\x86\x01a\x086V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tqW`\0\x80\xFD[a\t}\x85\x83\x86\x01a\x086V[`\x80\x84\x01Ra\t\x8E`\xA0\x85\x01a\x08\xA5V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\xA7W`\0\x80\xFD[Pa\t\xB4\x84\x82\x85\x01a\x086V[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\t\xD2W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\xF5Wa\t\xF5a\x07\x83V[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\n\rW`\0\x80\xFD[a\n\x19\x86\x83\x87\x01a\x08\xC1V[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\n/W`\0\x80\xFD[Pa\n<\x85\x82\x86\x01a\x086V[` \x83\x01RPa\nN`@\x84\x01a\x08\xA5V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\nlW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x82W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\t\xC0V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xBCW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\xBFW`\0\x80\xFD[a\n\xC8\x82a\n\x96V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\xE1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xF7W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\n\xC8W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x1BW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B2W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0BFW`\0\x80\xFD[a\x0BNa\x07\xC1V[\x825\x82\x81\x11\x15a\x0B]W`\0\x80\xFD[a\x0Bi\x87\x82\x86\x01a\t\xC0V[\x82RPa\x0Bx` \x84\x01a\n\x96V[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0B\x9FWa\x0B\x9Fa\x07\x83V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\xBAW`\0\x80\xFD[\x815` a\x0B\xCFa\x0B\xCA\x83a\x0B\x86V[a\x08\x06V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B\xEEW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C-W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x11W`\0\x80\x81\xFD[a\x0C\x1F\x89\x86\x83\x8B\x01\x01a\x086V[\x84RP\x91\x83\x01\x91\x83\x01a\x0B\xF2V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0CJW`\0\x80\xFD[a\x0CRa\x07\x99V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0CkW`\0\x80\xFD[a\x0Cw\x85\x83\x86\x01a\x086V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0C\x8DW`\0\x80\xFD[a\x0C\x99\x85\x83\x86\x01a\x086V[` \x84\x01Ra\x0C\xAA`@\x85\x01a\x08\xA5V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\xC3W`\0\x80\xFD[a\x0C\xCF\x85\x83\x86\x01a\x086V[``\x84\x01Ra\x0C\xE0`\x80\x85\x01a\x08\xA5V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0C\xF9W`\0\x80\xFD[Pa\r\x06\x84\x82\x85\x01a\x0B\xA9V[`\xA0\x83\x01RPa\r\x18`\xC0\x83\x01a\x08\xA5V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\r5W`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\rKW`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\r_W`\0\x80\xFD[a\rga\x07\xC1V[\x82\x825\x11\x15a\ruW`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\r\x89W`\0\x80\xFD[a\r\x91a\x07\xC1V[\x84\x825\x11\x15a\r\x9FW`\0\x80\xFD[a\r\xAC\x88\x835\x84\x01a\x0C8V[\x81R\x84` \x83\x015\x11\x15a\r\xBFW`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\r\xD7W`\0\x80\xFD[a\r\xE4a\x0B\xCA\x835a\x0B\x86V[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\x0E\x03W`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x0E\xA2W\x88\x815\x11\x15a\x0E\x1EW`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0E6W`\0\x80\xFD[a\x0E>a\x07\xC1V[\x8A` \x83\x015\x11\x15a\x0EOW`\0\x80\xFD[a\x0Ea\x8E` \x80\x85\x015\x85\x01\x01a\x086V[\x81R\x8A`@\x83\x015\x11\x15a\x0EtW`\0\x80\xFD[a\x0E\x87\x8E` `@\x85\x015\x85\x01\x01a\x086V[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\x0E\x08V[PP\x80` \x84\x01RPP\x80\x83RPPa\x0Bx` \x83\x01a\n\x96V[`\0` \x82\x84\x03\x12\x15a\x0E\xCFW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xE5W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\x08\xC1V[`\0` \x82\x84\x03\x12\x15a\x0F\x03W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\x19W`\0\x80\xFD[a\n\x8E\x84\x82\x85\x01a\x0C8V[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0F;W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\\W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0FvW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0F\x8BW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\xCEW`\0\x80\xFD[\x83\x86\x11\x15a\x0F\xDBW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F\xFAW`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10\x1CWa\x10\x1Ca\x07\x83V[`@Ra\x10(\x83a\n\x96V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x10OW`\0\x80\xFD[\x815` a\x10_a\x0B\xCA\x83a\x0B\x86V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x10~W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C-W\x805\x83R\x91\x83\x01\x91\x83\x01a\x10\x82V[`\0` \x82\x84\x03\x12\x15a\x10\xABW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x10\xC2W`\0\x80\xFD[\x90\x83\x01\x90a\x01\x80\x82\x86\x03\x12\x15a\x10\xD7W`\0\x80\xFD[a\x10\xDFa\x07\xE3V[\x825\x81R` \x83\x015` \x82\x01Ra\x10\xF9`@\x84\x01a\n\x96V[`@\x82\x01Ra\x11\n``\x84\x01a\n\x96V[``\x82\x01Ra\x11\x1B`\x80\x84\x01a\n\x96V[`\x80\x82\x01Ra\x11,`\xA0\x84\x01a\n\x96V[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x11S\x81\x85\x01a\n\x96V[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x11kW`\0\x80\xFD[a\x11w\x88\x82\x87\x01a\x086V[\x82\x84\x01RPPa\x01@\x80\x84\x015\x81\x83\x01RPa\x01`\x80\x84\x015\x83\x81\x11\x15a\x11\x9DW`\0\x80\xFD[a\x11\xA9\x88\x82\x87\x01a\x10>V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0[\x83\x81\x10\x15a\x11\xD5W\x81\x81\x01Q\x83\x82\x01R` \x01a\x11\xBDV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x11\xF6\x81` \x86\x01` \x86\x01a\x11\xBAV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x12:W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\x1EV[P\x94\x95\x94PPPPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x12v``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x12\xEB\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\x80a\x01@\x81\x81\x86\x01Ra\x13\x0Ba\x01\xA0\x86\x01\x84a\x11\xDEV[\x90\x86\x01Qa\x01`\x86\x81\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x134\x83\x82a\x12\nV[\x96\x95PPPPPPV[hPOLKADOT-`\xB8\x1B\x81R`\0\x82Qa\x13b\x81`\t\x85\x01` \x87\x01a\x11\xBAV[\x91\x90\x91\x01`\t\x01\x92\x91PPV\xFE\xA2dipfsX\"\x12 \xA8\x90\xE18\x19F\xA91U\x19(u-\x1EU\x19\x12)\x84?{\xFA\x1D\x10\xB8\xB3$\x08\t\xE4+\x03dsolcC\0\x08\x11\x003";
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
        /// arguments and sends it. Returns a new instance of a deployer that returns an
        /// instance of this contract after sending the transaction
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
    ///`HostManagerParams(address,uint256,address)`
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
        pub governor_state_machine_id: ::ethers::core::types::U256,
        pub host: ::ethers::core::types::Address,
    }
}
