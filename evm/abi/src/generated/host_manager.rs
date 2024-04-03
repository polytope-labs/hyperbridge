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
                                ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                ],),
                                ::ethers::core::abi::ethabi::ParamType::Bytes,
                                ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                ],),
                                ::ethers::core::abi::ethabi::ParamType::Bytes,
                                ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x12E8\x03\x80a\x12E\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\x83V[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xEBV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0~W`\0\x80\xFD[\x91\x90PV[`\0`@\x82\x84\x03\x12\x15a\0\x95W`\0\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xC5WcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD1\x83a\0gV[\x81Ra\0\xDF` \x84\x01a\0gV[` \x82\x01R\x93\x92PPPV[a\x11K\x80a\0\xFA`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xAF\xB7`\xAC\x11a\0[W\x80c\xAF\xB7`\xAC\x14a\0\xA2W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xD6;\xCF\x18\x14a\x01.W\x80c\xF3p\xFD\xBB\x14a\x01AW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x8DW\x80c\x12\xB2RO\x14a\0\xA2W\x80cLF\xC05\x14a\0\xB5W\x80cN\x87\xBA\x19\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x06\x16V[a\x01OV[\0[a\0\xA0a\0\xB06`\x04a\x08\xB3V[a\x01\xD7V[a\0\xA0a\0\xC36`\x04a\x0B\x04V[a\x02.V[a\0\xA0a\0\xD66`\x04a\x0B@V[a\x02\x82V[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[a\0\xA0a\x01<6`\x04a\x0B{V[a\x05wV[a\0\xA0a\0\xC36`\x04a\x0B\xAFV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xDCW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA5V[`\x01T`@\x80Qb/;\x1F`\xE1\x1B\x81R\x90Qa\x03\x94\x92`\x01`\x01`\xA0\x1B\x03\x16\x91b^v>\x91`\x04\x80\x83\x01\x92`\0\x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\x03%W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03M\x91\x90\x81\x01\x90a\r0V[a\x03W\x83\x80a\r\xA6V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xCD\x90PV[a\x03\xD7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xA5V[`\0a\x03\xE6`\xC0\x83\x01\x83a\r\xA6V[`\0\x81\x81\x10a\x03\xF7Wa\x03\xF7a\r\xF3V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\x12Wa\x04\x12a\x0E\tV[\x90P`\0\x81`\x01\x81\x11\x15a\x04(Wa\x04(a\x0E\tV[\x03a\x04\xCAW`\0a\x04<`\xC0\x84\x01\x84a\r\xA6V[a\x04J\x91`\x01\x90\x82\x90a\x0E\x1FV[\x81\x01\x90a\x04W\x91\x90a\x0EIV[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xADW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xC1W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xDEWa\x04\xDEa\x0E\tV[\x03a\x05>W`\0a\x04\xF2`\xC0\x84\x01\x84a\r\xA6V[a\x05\0\x91`\x01\x90\x82\x90a\x0E\x1FV[\x81\x01\x90a\x05\r\x91\x90a\x0E\x9FV[`\x01T`@Qc:3\x81\x15`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xE8\xCE\x04T\x90a\x04\x93\x90\x84\x90`\x04\x01a\x10\x04V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xA5V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`\0\x81Q\x83Q\x14a\x05\xE0WP`\0a\x05\xF4V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06\x11W`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x06(W`\0\x80\xFD[a\x061\x82a\x05\xFAV[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x06Wa\x07\x06a\x068V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07'Wa\x07'a\x068V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07FW`\0\x80\xFD[\x815a\x07Ya\x07T\x82a\x07\x0EV[a\x06\xDEV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x07nW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x06\x11W`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x07\xB5W`\0\x80\xFD[a\x07\xBDa\x06NV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xD6W`\0\x80\xFD[a\x07\xE2\x85\x83\x86\x01a\x075V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\xF8W`\0\x80\xFD[a\x08\x04\x85\x83\x86\x01a\x075V[` \x84\x01Ra\x08\x15`@\x85\x01a\x07\x8BV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08.W`\0\x80\xFD[a\x08:\x85\x83\x86\x01a\x075V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x08SW`\0\x80\xFD[a\x08_\x85\x83\x86\x01a\x075V[`\x80\x84\x01Ra\x08p`\xA0\x85\x01a\x07\x8BV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08\x89W`\0\x80\xFD[Pa\x08\x96\x84\x82\x85\x01a\x075V[`\xC0\x83\x01RPa\x08\xA8`\xE0\x83\x01a\x07\x8BV[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08\xC5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xDCW`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\x08\xF0W`\0\x80\xFD[a\x08\xF8a\x06wV[\x825\x82\x81\x11\x15a\t\x07W`\0\x80\xFD[a\t\x13\x87\x82\x86\x01a\x07\xA2V[\x82RP` \x83\x015\x82\x81\x11\x15a\t(W`\0\x80\xFD[a\t4\x87\x82\x86\x01a\x075V[` \x83\x01RPa\tF`@\x84\x01a\x07\x8BV[`@\x82\x01Ra\tW``\x84\x01a\x07\x8BV[``\x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\t~Wa\t~a\x068V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\t\x99W`\0\x80\xFD[\x815` a\t\xA9a\x07T\x83a\teV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\t\xC8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\x07W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xEBW`\0\x80\x81\xFD[a\t\xF9\x89\x86\x83\x8B\x01\x01a\x075V[\x84RP\x91\x83\x01\x91\x83\x01a\t\xCCV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\n%W`\0\x80\xFD[a\n-a\x06NV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nFW`\0\x80\xFD[a\nR\x85\x83\x86\x01a\x075V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\nhW`\0\x80\xFD[a\nt\x85\x83\x86\x01a\x075V[` \x84\x01Ra\n\x85`@\x85\x01a\x07\x8BV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\n\x9EW`\0\x80\xFD[a\n\xAA\x85\x83\x86\x01a\x075V[``\x84\x01Ra\n\xBB`\x80\x85\x01a\x07\x8BV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\n\xD4W`\0\x80\xFD[Pa\n\xE1\x84\x82\x85\x01a\t\x88V[`\xA0\x83\x01RPa\n\xF3`\xC0\x83\x01a\x07\x8BV[`\xC0\x82\x01Ra\x08\xA8`\xE0\x83\x01a\x07\x8BV[`\0` \x82\x84\x03\x12\x15a\x0B\x16W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B,W`\0\x80\xFD[a\x0B8\x84\x82\x85\x01a\n\x12V[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x0BRW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0BhW`\0\x80\xFD[\x82\x01a\x01\0\x81\x85\x03\x12\x15a\x061W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x8DW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xA3W`\0\x80\xFD[a\x0B8\x84\x82\x85\x01a\x07\xA2V[`\0` \x80\x83\x85\x03\x12\x15a\x0B\xC2W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xD9W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B\xEFW`\0\x80\xFD[a\x0B\xF7a\x06\x99V[\x835\x83\x81\x11\x15a\x0C\x06W`\0\x80\xFD[a\x0C\x12\x89\x82\x87\x01a\n\x12V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0C&W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0C;W`\0\x80\xFD[\x835a\x0CIa\x07T\x82a\teV[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0ChW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\xF8W\x805\x87\x81\x11\x15a\x0C\x84W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0C\x9AW`\0\x80\x81\xFD[a\x0C\xA2a\x06\x99V[\x8A\x82\x015\x89\x81\x11\x15a\x0C\xB4W`\0\x80\x81\xFD[a\x0C\xC2\x8F\x8D\x83\x86\x01\x01a\x075V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0C\xD7W`\0\x80\x81\xFD[a\x0C\xE5\x8F\x8D\x83\x86\x01\x01a\x075V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0ClV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0[\x83\x81\x10\x15a\r'W\x81\x81\x01Q\x83\x82\x01R` \x01a\r\x0FV[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\rBW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\rXW`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\riW`\0\x80\xFD[\x80Qa\rwa\x07T\x82a\x07\x0EV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\r\x8CW`\0\x80\xFD[a\r\x9D\x82` \x83\x01` \x86\x01a\r\x0CV[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\r\xBDW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\xD7W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\r\xECW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0E/W`\0\x80\xFD[\x83\x86\x11\x15a\x0E<W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E[W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0E}Wa\x0E}a\x068V[`@Ra\x0E\x89\x83a\x05\xFAV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0E\xB1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\xC8W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x0E\xDDW`\0\x80\xFD[a\x0E\xE5a\x06\xBBV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0F\t``\x84\x01a\x05\xFAV[``\x82\x01Ra\x0F\x1A`\x80\x84\x01a\x05\xFAV[`\x80\x82\x01Ra\x0F+`\xA0\x84\x01a\x05\xFAV[`\xA0\x82\x01Ra\x0F<`\xC0\x84\x01a\x05\xFAV[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x0Fe\x81\x85\x01a\x05\xFAV[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x0F}W`\0\x80\xFD[a\x0F\x89\x88\x82\x87\x01a\x075V[\x82\x84\x01RPPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x81\x83\x01RPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x0F\xBBW`\0\x80\xFD[a\x0F\xC7\x88\x82\x87\x01a\x075V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84Ra\x0F\xF0\x81` \x86\x01` \x86\x01a\r\x0CV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10?`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x10\xB5\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01`\x81\x81\x86\x01Ra\x10\xD5a\x01\xE0\x86\x01\x84a\x0F\xD8V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\xA0\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x11\x0B\x83\x82a\x0F\xD8V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xD5\x81c\x0C\x06\xD0!:\x91D\xC7D\x99\xE5\0\xF56\xAE<\xF1\xA4\x04 \xFBKZ\xC47\xC9\xAB\xC2/dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xAF\xB7`\xAC\x11a\0[W\x80c\xAF\xB7`\xAC\x14a\0\xA2W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xD6;\xCF\x18\x14a\x01.W\x80c\xF3p\xFD\xBB\x14a\x01AW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x8DW\x80c\x12\xB2RO\x14a\0\xA2W\x80cLF\xC05\x14a\0\xB5W\x80cN\x87\xBA\x19\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x06\x16V[a\x01OV[\0[a\0\xA0a\0\xB06`\x04a\x08\xB3V[a\x01\xD7V[a\0\xA0a\0\xC36`\x04a\x0B\x04V[a\x02.V[a\0\xA0a\0\xD66`\x04a\x0B@V[a\x02\x82V[`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x81Q\x90\x81\x90\x03\x90\x91\x01\x90\xF3[a\0\xA0a\x01<6`\x04a\x0B{V[a\x05wV[a\0\xA0a\0\xC36`\x04a\x0B\xAFV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xDCW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xA5V[`\x01T`@\x80Qb/;\x1F`\xE1\x1B\x81R\x90Qa\x03\x94\x92`\x01`\x01`\xA0\x1B\x03\x16\x91b^v>\x91`\x04\x80\x83\x01\x92`\0\x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\x03%W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03M\x91\x90\x81\x01\x90a\r0V[a\x03W\x83\x80a\r\xA6V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xCD\x90PV[a\x03\xD7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xA5V[`\0a\x03\xE6`\xC0\x83\x01\x83a\r\xA6V[`\0\x81\x81\x10a\x03\xF7Wa\x03\xF7a\r\xF3V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\x12Wa\x04\x12a\x0E\tV[\x90P`\0\x81`\x01\x81\x11\x15a\x04(Wa\x04(a\x0E\tV[\x03a\x04\xCAW`\0a\x04<`\xC0\x84\x01\x84a\r\xA6V[a\x04J\x91`\x01\x90\x82\x90a\x0E\x1FV[\x81\x01\x90a\x04W\x91\x90a\x0EIV[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xADW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xC1W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xDEWa\x04\xDEa\x0E\tV[\x03a\x05>W`\0a\x04\xF2`\xC0\x84\x01\x84a\r\xA6V[a\x05\0\x91`\x01\x90\x82\x90a\x0E\x1FV[\x81\x01\x90a\x05\r\x91\x90a\x0E\x9FV[`\x01T`@Qc:3\x81\x15`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xE8\xCE\x04T\x90a\x04\x93\x90\x84\x90`\x04\x01a\x10\x04V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xA5V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xA5V[`\0\x81Q\x83Q\x14a\x05\xE0WP`\0a\x05\xF4V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06\x11W`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x06(W`\0\x80\xFD[a\x061\x82a\x05\xFAV[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06qWa\x06qa\x068V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x06Wa\x07\x06a\x068V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07'Wa\x07'a\x068V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07FW`\0\x80\xFD[\x815a\x07Ya\x07T\x82a\x07\x0EV[a\x06\xDEV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x07nW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x06\x11W`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x07\xB5W`\0\x80\xFD[a\x07\xBDa\x06NV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xD6W`\0\x80\xFD[a\x07\xE2\x85\x83\x86\x01a\x075V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\xF8W`\0\x80\xFD[a\x08\x04\x85\x83\x86\x01a\x075V[` \x84\x01Ra\x08\x15`@\x85\x01a\x07\x8BV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08.W`\0\x80\xFD[a\x08:\x85\x83\x86\x01a\x075V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x08SW`\0\x80\xFD[a\x08_\x85\x83\x86\x01a\x075V[`\x80\x84\x01Ra\x08p`\xA0\x85\x01a\x07\x8BV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08\x89W`\0\x80\xFD[Pa\x08\x96\x84\x82\x85\x01a\x075V[`\xC0\x83\x01RPa\x08\xA8`\xE0\x83\x01a\x07\x8BV[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08\xC5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xDCW`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\x08\xF0W`\0\x80\xFD[a\x08\xF8a\x06wV[\x825\x82\x81\x11\x15a\t\x07W`\0\x80\xFD[a\t\x13\x87\x82\x86\x01a\x07\xA2V[\x82RP` \x83\x015\x82\x81\x11\x15a\t(W`\0\x80\xFD[a\t4\x87\x82\x86\x01a\x075V[` \x83\x01RPa\tF`@\x84\x01a\x07\x8BV[`@\x82\x01Ra\tW``\x84\x01a\x07\x8BV[``\x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\t~Wa\t~a\x068V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\t\x99W`\0\x80\xFD[\x815` a\t\xA9a\x07T\x83a\teV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\t\xC8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\x07W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\t\xEBW`\0\x80\x81\xFD[a\t\xF9\x89\x86\x83\x8B\x01\x01a\x075V[\x84RP\x91\x83\x01\x91\x83\x01a\t\xCCV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\n%W`\0\x80\xFD[a\n-a\x06NV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nFW`\0\x80\xFD[a\nR\x85\x83\x86\x01a\x075V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\nhW`\0\x80\xFD[a\nt\x85\x83\x86\x01a\x075V[` \x84\x01Ra\n\x85`@\x85\x01a\x07\x8BV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\n\x9EW`\0\x80\xFD[a\n\xAA\x85\x83\x86\x01a\x075V[``\x84\x01Ra\n\xBB`\x80\x85\x01a\x07\x8BV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\n\xD4W`\0\x80\xFD[Pa\n\xE1\x84\x82\x85\x01a\t\x88V[`\xA0\x83\x01RPa\n\xF3`\xC0\x83\x01a\x07\x8BV[`\xC0\x82\x01Ra\x08\xA8`\xE0\x83\x01a\x07\x8BV[`\0` \x82\x84\x03\x12\x15a\x0B\x16W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B,W`\0\x80\xFD[a\x0B8\x84\x82\x85\x01a\n\x12V[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x0BRW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0BhW`\0\x80\xFD[\x82\x01a\x01\0\x81\x85\x03\x12\x15a\x061W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x8DW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xA3W`\0\x80\xFD[a\x0B8\x84\x82\x85\x01a\x07\xA2V[`\0` \x80\x83\x85\x03\x12\x15a\x0B\xC2W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xD9W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B\xEFW`\0\x80\xFD[a\x0B\xF7a\x06\x99V[\x835\x83\x81\x11\x15a\x0C\x06W`\0\x80\xFD[a\x0C\x12\x89\x82\x87\x01a\n\x12V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0C&W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0C;W`\0\x80\xFD[\x835a\x0CIa\x07T\x82a\teV[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0ChW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\xF8W\x805\x87\x81\x11\x15a\x0C\x84W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0C\x9AW`\0\x80\x81\xFD[a\x0C\xA2a\x06\x99V[\x8A\x82\x015\x89\x81\x11\x15a\x0C\xB4W`\0\x80\x81\xFD[a\x0C\xC2\x8F\x8D\x83\x86\x01\x01a\x075V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0C\xD7W`\0\x80\x81\xFD[a\x0C\xE5\x8F\x8D\x83\x86\x01\x01a\x075V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0ClV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0[\x83\x81\x10\x15a\r'W\x81\x81\x01Q\x83\x82\x01R` \x01a\r\x0FV[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\rBW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\rXW`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\riW`\0\x80\xFD[\x80Qa\rwa\x07T\x82a\x07\x0EV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\r\x8CW`\0\x80\xFD[a\r\x9D\x82` \x83\x01` \x86\x01a\r\x0CV[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\r\xBDW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\xD7W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\r\xECW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0E/W`\0\x80\xFD[\x83\x86\x11\x15a\x0E<W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E[W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0E}Wa\x0E}a\x068V[`@Ra\x0E\x89\x83a\x05\xFAV[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0E\xB1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0E\xC8W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x0E\xDDW`\0\x80\xFD[a\x0E\xE5a\x06\xBBV[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0F\t``\x84\x01a\x05\xFAV[``\x82\x01Ra\x0F\x1A`\x80\x84\x01a\x05\xFAV[`\x80\x82\x01Ra\x0F+`\xA0\x84\x01a\x05\xFAV[`\xA0\x82\x01Ra\x0F<`\xC0\x84\x01a\x05\xFAV[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x0Fe\x81\x85\x01a\x05\xFAV[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x0F}W`\0\x80\xFD[a\x0F\x89\x88\x82\x87\x01a\x075V[\x82\x84\x01RPPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x81\x83\x01RPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x0F\xBBW`\0\x80\xFD[a\x0F\xC7\x88\x82\x87\x01a\x075V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84Ra\x0F\xF0\x81` \x86\x01` \x86\x01a\r\x0CV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10?`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x10\xB5\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01`\x81\x81\x86\x01Ra\x10\xD5a\x01\xE0\x86\x01\x84a\x0F\xD8V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\xA0\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x11\x0B\x83\x82a\x0F\xD8V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \xD5\x81c\x0C\x06\xD0!:\x91D\xC7D\x99\xE5\0\xF56\xAE<\xF1\xA4\x04 \xFBKZ\xC47\xC9\xAB\xC2/dsolcC\0\x08\x11\x003";
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
        ///Calls the contract's `onAccept` (0x4e87ba19) function
        pub fn on_accept(
            &self,
            request: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([78, 135, 186, 25], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetResponse` (0xf370fdbb) function
        pub fn on_get_response(
            &self,
            p0: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([243, 112, 253, 187], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetTimeout` (0x4c46c035) function
        pub fn on_get_timeout(
            &self,
            p0: GetRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([76, 70, 192, 53], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostRequestTimeout` (0xd63bcf18) function
        pub fn on_post_request_timeout(
            &self,
            p0: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([214, 59, 207, 24], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostResponse` (0xafb760ac) function
        pub fn on_post_response(
            &self,
            p0: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([175, 183, 96, 172], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostResponseTimeout` (0x12b2524f) function
        pub fn on_post_response_timeout(
            &self,
            p0: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([18, 178, 82, 79], (p0,))
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
    /// `onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x4e87ba19`
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
        abi = "onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnAcceptCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature
    /// `onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))`
    /// and selector `0xf370fdbb`
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
        abi = "onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))"
    )]
    pub struct OnGetResponseCall(pub GetResponse);
    ///Container type for all input parameters for the `onGetTimeout` function with signature
    /// `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector
    /// `0x4c46c035`
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
        abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
    )]
    pub struct OnGetTimeoutCall(pub GetRequest);
    ///Container type for all input parameters for the `onPostRequestTimeout` function with
    /// signature `onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and
    /// selector `0xd63bcf18`
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
        abi = "onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnPostRequestTimeoutCall(pub PostRequest);
    ///Container type for all input parameters for the `onPostResponse` function with signature
    /// `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))`
    /// and selector `0xafb760ac`
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
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
    )]
    pub struct OnPostResponseCall(pub PostResponse);
    ///Container type for all input parameters for the `onPostResponseTimeout` function with
    /// signature `onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),
    /// bytes,uint64,uint64))` and selector `0x12b2524f`
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
        abi = "onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
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
