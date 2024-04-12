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
                            name: ::std::borrow::ToOwned::to_owned("request"),
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
                    ::std::borrow::ToOwned::to_owned("onGetResponse"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("onGetResponse"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
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
                                ],),
                                ::ethers::core::abi::ethabi::ParamType::Array(
                                    ::std::boxed::Box::new(
                                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],),
                                    ),
                                ),
                            ],),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("struct GetResponse"),
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x12\r8\x03\x80a\x12\r\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\x83V[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xEBV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0~W`\0\x80\xFD[\x91\x90PV[`\0`@\x82\x84\x03\x12\x15a\0\x95W`\0\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xC5WcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD1\x83a\0gV[\x81Ra\0\xDF` \x84\x01a\0gV[` \x82\x01R\x93\x92PPPV[a\x11\x13\x80a\0\xFA`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xCF\xF0\xAB\x96\x11a\0[W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xDE\xAET\xF5\x14a\x01.W\x80c\xEA\xEE\x1C\xAA\x14a\x01<W\x80c\xFE\xFF\x7F\xA8\x14a\0\x8DW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\xBC\r\xD4G\x14a\0\xB5W\x80c\xC4\x92\xE4&\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x08EV[a\x01OV[\0[a\0\xA0a\0\xB06`\x04a\t\x13V[a\x01\xABV[a\0\xA0a\0\xC36`\x04a\t5V[a\x02.V[a\0\xA0a\0\xD66`\x04a\x0B\tV[a\x02\x84V[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[a\0\xA0a\0\xD66`\x04a\x0B=V[a\0\xA0a\x01J6`\x04a\x0C\x9AV[a\x02\xD8V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x05W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA2V[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xA2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xA2V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x032W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA2V[`\x01T`@\x80Qb/;\x1F`\xE1\x1B\x81R\x90Qa\x03\xEA\x92`\x01`\x01`\xA0\x1B\x03\x16\x91b^v>\x91`\x04\x80\x83\x01\x92`\0\x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\x03{W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\xA3\x91\x90\x81\x01\x90a\x0C\xF8V[a\x03\xAD\x83\x80a\rnV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xCD\x90PV[a\x04-W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xA2V[`\0a\x04<`\xC0\x83\x01\x83a\rnV[`\0\x81\x81\x10a\x04MWa\x04Ma\r\xBBV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04hWa\x04ha\r\xD1V[\x90P`\0\x81`\x01\x81\x11\x15a\x04~Wa\x04~a\r\xD1V[\x03a\x05 W`\0a\x04\x92`\xC0\x84\x01\x84a\rnV[a\x04\xA0\x91`\x01\x90\x82\x90a\r\xE7V[\x81\x01\x90a\x04\xAD\x91\x90a\x0E\x11V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05\x03W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x17W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x054Wa\x054a\r\xD1V[\x03a\x05\x94W`\0a\x05H`\xC0\x84\x01\x84a\rnV[a\x05V\x91`\x01\x90\x82\x90a\r\xE7V[\x81\x01\x90a\x05c\x91\x90a\x0EgV[`\x01T`@Qc:3\x81\x15`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xE8\xCE\x04T\x90a\x04\xE9\x90\x84\x90`\x04\x01a\x0F\xCCV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xA2V[`\0\x81Q\x83Q\x14a\x05\xE0WP`\0a\x05\xF4V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\xA5Wa\x06\xA5a\x05\xFAV[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x06\xC6Wa\x06\xC6a\x05\xFAV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x06\xE5W`\0\x80\xFD[\x815a\x06\xF8a\x06\xF3\x82a\x06\xADV[a\x06}V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x07\rW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07AW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x07XW`\0\x80\xFD[a\x07`a\x06\x10V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07yW`\0\x80\xFD[a\x07\x85\x85\x83\x86\x01a\x06\xD4V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\x9BW`\0\x80\xFD[a\x07\xA7\x85\x83\x86\x01a\x06\xD4V[` \x84\x01Ra\x07\xB8`@\x85\x01a\x07*V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xD1W`\0\x80\xFD[a\x07\xDD\x85\x83\x86\x01a\x06\xD4V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x07\xF6W`\0\x80\xFD[a\x08\x02\x85\x83\x86\x01a\x06\xD4V[`\x80\x84\x01Ra\x08\x13`\xA0\x85\x01a\x07*V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08,W`\0\x80\xFD[Pa\x089\x84\x82\x85\x01a\x06\xD4V[`\xC0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08WW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08nW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\x08\x82W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\x08\x9DWa\x08\x9Da\x05\xFAV[`@R\x825\x82\x81\x11\x15a\x08\xAFW`\0\x80\xFD[a\x08\xBB\x87\x82\x86\x01a\x07FV[\x82RP` \x83\x015\x82\x81\x11\x15a\x08\xD0W`\0\x80\xFD[a\x08\xDC\x87\x82\x86\x01a\x06\xD4V[` \x83\x01RPa\x08\xEE`@\x84\x01a\x07*V[`@\x82\x01R\x95\x94PPPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x07AW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\t%W`\0\x80\xFD[a\t.\x82a\x08\xFCV[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\tGW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t]W`\0\x80\xFD[a\ti\x84\x82\x85\x01a\x07FV[\x94\x93PPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\t\x8AWa\t\x8Aa\x05\xFAV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\t\xA5W`\0\x80\xFD[\x815` a\t\xB5a\x06\xF3\x83a\tqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\t\xD4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\x13W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xF7W`\0\x80\x81\xFD[a\n\x05\x89\x86\x83\x8B\x01\x01a\x06\xD4V[\x84RP\x91\x83\x01\x91\x83\x01a\t\xD8V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\n0W`\0\x80\xFD[a\n8a\x06\x10V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nQW`\0\x80\xFD[a\n]\x85\x83\x86\x01a\x06\xD4V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\nsW`\0\x80\xFD[a\n\x7F\x85\x83\x86\x01a\x06\xD4V[` \x84\x01Ra\n\x90`@\x85\x01a\x07*V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\n\xA9W`\0\x80\xFD[a\n\xB5\x85\x83\x86\x01a\x06\xD4V[``\x84\x01Ra\n\xC6`\x80\x85\x01a\x07*V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\n\xDFW`\0\x80\xFD[Pa\n\xEC\x84\x82\x85\x01a\t\x94V[`\xA0\x83\x01RPa\n\xFE`\xC0\x83\x01a\x07*V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0B\x1BW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B1W`\0\x80\xFD[a\ti\x84\x82\x85\x01a\n\x1EV[`\0` \x80\x83\x85\x03\x12\x15a\x0BPW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0BgW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B}W`\0\x80\xFD[a\x0B\x85a\x068V[\x835\x83\x81\x11\x15a\x0B\x94W`\0\x80\xFD[a\x0B\xA0\x89\x82\x87\x01a\n\x1EV[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0B\xB4W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0B\xC9W`\0\x80\xFD[\x835a\x0B\xD7a\x06\xF3\x82a\tqV[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0B\xF6W`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\x86W\x805\x87\x81\x11\x15a\x0C\x12W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0C(W`\0\x80\x81\xFD[a\x0C0a\x068V[\x8A\x82\x015\x89\x81\x11\x15a\x0CBW`\0\x80\x81\xFD[a\x0CP\x8F\x8D\x83\x86\x01\x01a\x06\xD4V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0CeW`\0\x80\x81\xFD[a\x0Cs\x8F\x8D\x83\x86\x01\x01a\x06\xD4V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0B\xFAV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x0C\xACW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xC2W`\0\x80\xFD[\x82\x01`\xE0\x81\x85\x03\x12\x15a\t.W`\0\x80\xFD[`\0[\x83\x81\x10\x15a\x0C\xEFW\x81\x81\x01Q\x83\x82\x01R` \x01a\x0C\xD7V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\r\nW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\r W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\r1W`\0\x80\xFD[\x80Qa\r?a\x06\xF3\x82a\x06\xADV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\rTW`\0\x80\xFD[a\re\x82` \x83\x01` \x86\x01a\x0C\xD4V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\r\x85W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\x9FW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\r\xB4W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\r\xF7W`\0\x80\xFD[\x83\x86\x11\x15a\x0E\x04W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E#W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0EEWa\x0EEa\x05\xFAV[`@Ra\x0EQ\x83a\x08\xFCV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0EyW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x90W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x0E\xA5W`\0\x80\xFD[a\x0E\xADa\x06ZV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0E\xD1``\x84\x01a\x08\xFCV[``\x82\x01Ra\x0E\xE2`\x80\x84\x01a\x08\xFCV[`\x80\x82\x01Ra\x0E\xF3`\xA0\x84\x01a\x08\xFCV[`\xA0\x82\x01Ra\x0F\x04`\xC0\x84\x01a\x08\xFCV[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x0F-\x81\x85\x01a\x08\xFCV[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x0FEW`\0\x80\xFD[a\x0FQ\x88\x82\x87\x01a\x06\xD4V[\x82\x84\x01RPPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x81\x83\x01RPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x0F\x83W`\0\x80\xFD[a\x0F\x8F\x88\x82\x87\x01a\x06\xD4V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84Ra\x0F\xB8\x81` \x86\x01` \x86\x01a\x0C\xD4V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10\x07`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x10}\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01`\x81\x81\x86\x01Ra\x10\x9Da\x01\xE0\x86\x01\x84a\x0F\xA0V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\xA0\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x10\xD3\x83\x82a\x0F\xA0V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xB2\x06g\x0C\xE6\xE9/,\xCA\xCATlh\x04\"H\xCFsj<*\xC1\xBF'q\x84A\xBEZF$YdsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xCF\xF0\xAB\x96\x11a\0[W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xDE\xAET\xF5\x14a\x01.W\x80c\xEA\xEE\x1C\xAA\x14a\x01<W\x80c\xFE\xFF\x7F\xA8\x14a\0\x8DW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\xBC\r\xD4G\x14a\0\xB5W\x80c\xC4\x92\xE4&\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x08EV[a\x01OV[\0[a\0\xA0a\0\xB06`\x04a\t\x13V[a\x01\xABV[a\0\xA0a\0\xC36`\x04a\t5V[a\x02.V[a\0\xA0a\0\xD66`\x04a\x0B\tV[a\x02\x84V[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[a\0\xA0a\0\xD66`\x04a\x0B=V[a\0\xA0a\x01J6`\x04a\x0C\x9AV[a\x02\xD8V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x05W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA2V[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xA2V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xA2V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x032W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA2V[`\x01T`@\x80Qb/;\x1F`\xE1\x1B\x81R\x90Qa\x03\xEA\x92`\x01`\x01`\xA0\x1B\x03\x16\x91b^v>\x91`\x04\x80\x83\x01\x92`\0\x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\x03{W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\xA3\x91\x90\x81\x01\x90a\x0C\xF8V[a\x03\xAD\x83\x80a\rnV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xCD\x90PV[a\x04-W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xA2V[`\0a\x04<`\xC0\x83\x01\x83a\rnV[`\0\x81\x81\x10a\x04MWa\x04Ma\r\xBBV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04hWa\x04ha\r\xD1V[\x90P`\0\x81`\x01\x81\x11\x15a\x04~Wa\x04~a\r\xD1V[\x03a\x05 W`\0a\x04\x92`\xC0\x84\x01\x84a\rnV[a\x04\xA0\x91`\x01\x90\x82\x90a\r\xE7V[\x81\x01\x90a\x04\xAD\x91\x90a\x0E\x11V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05\x03W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x17W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x054Wa\x054a\r\xD1V[\x03a\x05\x94W`\0a\x05H`\xC0\x84\x01\x84a\rnV[a\x05V\x91`\x01\x90\x82\x90a\r\xE7V[\x81\x01\x90a\x05c\x91\x90a\x0EgV[`\x01T`@Qc:3\x81\x15`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xE8\xCE\x04T\x90a\x04\xE9\x90\x84\x90`\x04\x01a\x0F\xCCV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xA2V[`\0\x81Q\x83Q\x14a\x05\xE0WP`\0a\x05\xF4V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x062Wa\x062a\x05\xFAV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\xA5Wa\x06\xA5a\x05\xFAV[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x06\xC6Wa\x06\xC6a\x05\xFAV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x06\xE5W`\0\x80\xFD[\x815a\x06\xF8a\x06\xF3\x82a\x06\xADV[a\x06}V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x07\rW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07AW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x07XW`\0\x80\xFD[a\x07`a\x06\x10V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07yW`\0\x80\xFD[a\x07\x85\x85\x83\x86\x01a\x06\xD4V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\x9BW`\0\x80\xFD[a\x07\xA7\x85\x83\x86\x01a\x06\xD4V[` \x84\x01Ra\x07\xB8`@\x85\x01a\x07*V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xD1W`\0\x80\xFD[a\x07\xDD\x85\x83\x86\x01a\x06\xD4V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x07\xF6W`\0\x80\xFD[a\x08\x02\x85\x83\x86\x01a\x06\xD4V[`\x80\x84\x01Ra\x08\x13`\xA0\x85\x01a\x07*V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08,W`\0\x80\xFD[Pa\x089\x84\x82\x85\x01a\x06\xD4V[`\xC0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08WW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08nW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\x08\x82W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\x08\x9DWa\x08\x9Da\x05\xFAV[`@R\x825\x82\x81\x11\x15a\x08\xAFW`\0\x80\xFD[a\x08\xBB\x87\x82\x86\x01a\x07FV[\x82RP` \x83\x015\x82\x81\x11\x15a\x08\xD0W`\0\x80\xFD[a\x08\xDC\x87\x82\x86\x01a\x06\xD4V[` \x83\x01RPa\x08\xEE`@\x84\x01a\x07*V[`@\x82\x01R\x95\x94PPPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x07AW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\t%W`\0\x80\xFD[a\t.\x82a\x08\xFCV[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\tGW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\t]W`\0\x80\xFD[a\ti\x84\x82\x85\x01a\x07FV[\x94\x93PPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\t\x8AWa\t\x8Aa\x05\xFAV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\t\xA5W`\0\x80\xFD[\x815` a\t\xB5a\x06\xF3\x83a\tqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\t\xD4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\x13W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xF7W`\0\x80\x81\xFD[a\n\x05\x89\x86\x83\x8B\x01\x01a\x06\xD4V[\x84RP\x91\x83\x01\x91\x83\x01a\t\xD8V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\n0W`\0\x80\xFD[a\n8a\x06\x10V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nQW`\0\x80\xFD[a\n]\x85\x83\x86\x01a\x06\xD4V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\nsW`\0\x80\xFD[a\n\x7F\x85\x83\x86\x01a\x06\xD4V[` \x84\x01Ra\n\x90`@\x85\x01a\x07*V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\n\xA9W`\0\x80\xFD[a\n\xB5\x85\x83\x86\x01a\x06\xD4V[``\x84\x01Ra\n\xC6`\x80\x85\x01a\x07*V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\n\xDFW`\0\x80\xFD[Pa\n\xEC\x84\x82\x85\x01a\t\x94V[`\xA0\x83\x01RPa\n\xFE`\xC0\x83\x01a\x07*V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0B\x1BW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B1W`\0\x80\xFD[a\ti\x84\x82\x85\x01a\n\x1EV[`\0` \x80\x83\x85\x03\x12\x15a\x0BPW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0BgW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B}W`\0\x80\xFD[a\x0B\x85a\x068V[\x835\x83\x81\x11\x15a\x0B\x94W`\0\x80\xFD[a\x0B\xA0\x89\x82\x87\x01a\n\x1EV[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0B\xB4W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0B\xC9W`\0\x80\xFD[\x835a\x0B\xD7a\x06\xF3\x82a\tqV[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0B\xF6W`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\x86W\x805\x87\x81\x11\x15a\x0C\x12W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0C(W`\0\x80\x81\xFD[a\x0C0a\x068V[\x8A\x82\x015\x89\x81\x11\x15a\x0CBW`\0\x80\x81\xFD[a\x0CP\x8F\x8D\x83\x86\x01\x01a\x06\xD4V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0CeW`\0\x80\x81\xFD[a\x0Cs\x8F\x8D\x83\x86\x01\x01a\x06\xD4V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0B\xFAV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x0C\xACW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xC2W`\0\x80\xFD[\x82\x01`\xE0\x81\x85\x03\x12\x15a\t.W`\0\x80\xFD[`\0[\x83\x81\x10\x15a\x0C\xEFW\x81\x81\x01Q\x83\x82\x01R` \x01a\x0C\xD7V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\r\nW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\r W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\r1W`\0\x80\xFD[\x80Qa\r?a\x06\xF3\x82a\x06\xADV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\rTW`\0\x80\xFD[a\re\x82` \x83\x01` \x86\x01a\x0C\xD4V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\r\x85W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\x9FW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\r\xB4W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\r\xF7W`\0\x80\xFD[\x83\x86\x11\x15a\x0E\x04W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E#W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0EEWa\x0EEa\x05\xFAV[`@Ra\x0EQ\x83a\x08\xFCV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0EyW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\x90W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x0E\xA5W`\0\x80\xFD[a\x0E\xADa\x06ZV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0E\xD1``\x84\x01a\x08\xFCV[``\x82\x01Ra\x0E\xE2`\x80\x84\x01a\x08\xFCV[`\x80\x82\x01Ra\x0E\xF3`\xA0\x84\x01a\x08\xFCV[`\xA0\x82\x01Ra\x0F\x04`\xC0\x84\x01a\x08\xFCV[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x0F-\x81\x85\x01a\x08\xFCV[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x0FEW`\0\x80\xFD[a\x0FQ\x88\x82\x87\x01a\x06\xD4V[\x82\x84\x01RPPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x81\x83\x01RPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x0F\x83W`\0\x80\xFD[a\x0F\x8F\x88\x82\x87\x01a\x06\xD4V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84Ra\x0F\xB8\x81` \x86\x01` \x86\x01a\x0C\xD4V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10\x07`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x10}\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01`\x81\x81\x86\x01Ra\x10\x9Da\x01\xE0\x86\x01\x84a\x0F\xA0V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\xA0\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x10\xD3\x83\x82a\x0F\xA0V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xB2\x06g\x0C\xE6\xE9/,\xCA\xCATlh\x04\"H\xCFsj<*\xC1\xBF'q\x84A\xBEZF$YdsolcC\0\x08\x11\x003";
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
        ///Calls the contract's `onAccept` (0xeaee1caa) function
        pub fn on_accept(
            &self,
            request: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([234, 238, 28, 170], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetResponse` (0xdeae54f5) function
        pub fn on_get_response(
            &self,
            p0: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([222, 174, 84, 245], (p0,))
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
        ///Calls the contract's `onPostResponse` (0xfeff7fa8) function
        pub fn on_post_response(
            &self,
            p0: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([254, 255, 127, 168], (p0,))
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
    /// `onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes))` and selector `0xeaee1caa`
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
    #[ethcall(name = "onAccept", abi = "onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes))")]
    pub struct OnAcceptCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature
    /// `onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]))` and
    /// selector `0xdeae54f5`
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
        abi = "onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]))"
    )]
    pub struct OnGetResponseCall(pub GetResponse);
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
    /// `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))` and selector
    /// `0xfeff7fa8`
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
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))"
    )]
    pub struct OnPostResponseCall(pub PostResponse);
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
