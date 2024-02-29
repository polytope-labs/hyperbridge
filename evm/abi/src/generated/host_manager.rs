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
                        ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                                ::ethers::core::abi::ethabi::ParamType::Bytes,
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x11W`\0\x80\xFD[P`@Qb\0\x15L8\x03\x80b\0\x15L\x839\x81\x01`@\x81\x90Rb\0\x004\x91b\0\x01\x1AV[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x92\x83\x16`\x01`\x01`\xA0\x1B\x03\x19\x91\x82\x16\x17\x82U` \x84\x01Q`\x01\x80T\x91\x90\x94\x16\x91\x16\x17\x90\x91U`@\x82\x01Q\x82\x91\x90`\x02\x90b\0\0~\x90\x82b\0\x02\xB5V[P\x90PPPb\0\x03\x81V[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\0\xC4Wb\0\0\xC4b\0\0\x89V[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\0\xF5Wb\0\0\xF5b\0\0\x89V[`@R\x91\x90PV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x01\x15W`\0\x80\xFD[\x91\x90PV[`\0` \x80\x83\x85\x03\x12\x15b\0\x01.W`\0\x80\xFD[\x82Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x01FW`\0\x80\xFD[\x90\x84\x01\x90``\x82\x87\x03\x12\x15b\0\x01[W`\0\x80\xFD[b\0\x01eb\0\0\x9FV[b\0\x01p\x83b\0\0\xFDV[\x81Rb\0\x01\x7F\x84\x84\x01b\0\0\xFDV[\x84\x82\x01R`@\x83\x01Q\x82\x81\x11\x15b\0\x01\x96W`\0\x80\xFD[\x80\x84\x01\x93PP\x86`\x1F\x84\x01\x12b\0\x01\xACW`\0\x80\xFD[\x82Q\x82\x81\x11\x15b\0\x01\xC1Wb\0\x01\xC1b\0\0\x89V[b\0\x01\xD5`\x1F\x82\x01`\x1F\x19\x16\x86\x01b\0\0\xCAV[\x92P\x80\x83R\x87\x85\x82\x86\x01\x01\x11\x15b\0\x01\xECW`\0\x80\xFD[`\0[\x81\x81\x10\x15b\0\x02\x0CW\x84\x81\x01\x86\x01Q\x84\x82\x01\x87\x01R\x85\x01b\0\x01\xEFV[P`\0\x90\x83\x01\x90\x94\x01\x93\x90\x93R`@\x83\x01RP\x93\x92PPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80b\0\x02;W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03b\0\x02\\WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15b\0\x02\xB0W`\0\x81\x81R` \x81 `\x1F\x85\x01`\x05\x1C\x81\x01` \x86\x10\x15b\0\x02\x8BWP\x80[`\x1F\x85\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15b\0\x02\xACW\x82\x81U`\x01\x01b\0\x02\x97V[PPP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15b\0\x02\xD1Wb\0\x02\xD1b\0\0\x89V[b\0\x02\xE9\x81b\0\x02\xE2\x84Tb\0\x02&V[\x84b\0\x02bV[` \x80`\x1F\x83\x11`\x01\x81\x14b\0\x03!W`\0\x84\x15b\0\x03\x08WP\x85\x83\x01Q[`\0\x19`\x03\x86\x90\x1B\x1C\x19\x16`\x01\x85\x90\x1B\x17\x85Ub\0\x02\xACV[`\0\x85\x81R` \x81 `\x1F\x19\x86\x16\x91[\x82\x81\x10\x15b\0\x03RW\x88\x86\x01Q\x82U\x94\x84\x01\x94`\x01\x90\x91\x01\x90\x84\x01b\0\x031V[P\x85\x82\x10\x15b\0\x03qW\x87\x85\x01Q`\0\x19`\x03\x88\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPPP`\x01\x90\x81\x1B\x01\x90UPV[a\x11\xBB\x80b\0\x03\x91`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xAF\xB7`\xAC\x11a\0[W\x80c\xAF\xB7`\xAC\x14a\0\xA2W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xD6;\xCF\x18\x14a\0\xF9W\x80c\xF3p\xFD\xBB\x14a\x01\x0CW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x8DW\x80c\x12\xB2RO\x14a\0\xA2W\x80cLF\xC05\x14a\0\xB5W\x80cN\x87\xBA\x19\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x06\xE1V[a\x01\x1AV[\0[a\0\xA0a\0\xB06`\x04a\tpV[a\x01\xA2V[a\0\xA0a\0\xC36`\x04a\x0B\xC6V[a\x01\xF9V[a\0\xA0a\0\xD66`\x04a\x0C\x02V[a\x02MV[a\0\xE3a\x05fV[`@Qa\0\xF0\x91\x90a\x0C\x83V[`@Q\x80\x91\x03\x90\xF3[a\0\xA0a\x01\x076`\x04a\x0C\xBEV[a\x06BV[a\0\xA0a\0\xC36`\x04a\x0C\xF2V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01yW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01a\x01pV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01pV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xA7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01pV[a\x03\x83`\0`\x02\x01\x80Ta\x02\xBA\x90a\x0EOV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xE6\x90a\x0EOV[\x80\x15a\x033W\x80`\x1F\x10a\x03\x08Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x033V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03\x16W\x82\x90\x03`\x1F\x16\x82\x01\x91[Pa\x03F\x93P\x86\x92P\x82\x91Pa\x0E\x89\x90PV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x06\x98\x90PV[a\x03\xC6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01pV[`\0a\x03\xD5`\xC0\x83\x01\x83a\x0E\x89V[`\0\x81\x81\x10a\x03\xE6Wa\x03\xE6a\x0E\xD6V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\x01Wa\x04\x01a\x0E\xECV[\x90P`\0\x81`\x01\x81\x11\x15a\x04\x17Wa\x04\x17a\x0E\xECV[\x03a\x04\xB9W`\0a\x04+`\xC0\x84\x01\x84a\x0E\x89V[a\x049\x91`\x01\x90\x82\x90a\x0F\x02V[\x81\x01\x90a\x04F\x91\x90a\x0F,V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\x9CW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xB0W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xCDWa\x04\xCDa\x0E\xECV[\x03a\x05-W`\0a\x04\xE1`\xC0\x84\x01\x84a\x0E\x89V[a\x04\xEF\x91`\x01\x90\x82\x90a\x0F\x02V[\x81\x01\x90a\x04\xFC\x91\x90a\x0F\x82V[`\x01T`@Qcwr\xA6\x1B`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cwr\xA6\x1B\x90a\x04\x82\x90\x84\x90`\x04\x01a\x10\x95V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01pV[`@\x80Q``\x80\x82\x01\x83R`\0\x80\x83R` \x83\x01R\x91\x81\x01\x91\x90\x91R`@\x80Q``\x81\x01\x82R`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x83R`\x01T\x16` \x83\x01R`\x02\x80T\x92\x93\x91\x92\x91\x84\x01\x91a\x05\xBB\x90a\x0EOV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05\xE7\x90a\x0EOV[\x80\x15a\x064W\x80`\x1F\x10a\x06\tWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x064V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x06\x17W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81RPP\x90P\x90V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01pV[`\0\x81Q\x83Q\x14a\x06\xABWP`\0a\x06\xBFV[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06\xDCW`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x06\xF3W`\0\x80\xFD[a\x06\xFC\x82a\x06\xC5V[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@Qa\x01\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xD1Wa\x07\xD1a\x07\x03V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x07\xEAW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08\x03Wa\x08\x03a\x07\x03V[a\x08\x16`\x1F\x82\x01`\x1F\x19\x16` \x01a\x07\xA9V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08+W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x06\xDCW`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x08rW`\0\x80\xFD[a\x08za\x07\x19V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x93W`\0\x80\xFD[a\x08\x9F\x85\x83\x86\x01a\x07\xD9V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xB5W`\0\x80\xFD[a\x08\xC1\x85\x83\x86\x01a\x07\xD9V[` \x84\x01Ra\x08\xD2`@\x85\x01a\x08HV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08\xEBW`\0\x80\xFD[a\x08\xF7\x85\x83\x86\x01a\x07\xD9V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\x10W`\0\x80\xFD[a\t\x1C\x85\x83\x86\x01a\x07\xD9V[`\x80\x84\x01Ra\t-`\xA0\x85\x01a\x08HV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\tFW`\0\x80\xFD[Pa\tS\x84\x82\x85\x01a\x07\xD9V[`\xC0\x83\x01RPa\te`\xE0\x83\x01a\x08HV[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\x82W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\x99W`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\t\xADW`\0\x80\xFD[a\t\xB5a\x07BV[\x825\x82\x81\x11\x15a\t\xC4W`\0\x80\xFD[a\t\xD0\x87\x82\x86\x01a\x08_V[\x82RP` \x83\x015\x82\x81\x11\x15a\t\xE5W`\0\x80\xFD[a\t\xF1\x87\x82\x86\x01a\x07\xD9V[` \x83\x01RPa\n\x03`@\x84\x01a\x08HV[`@\x82\x01Ra\n\x14``\x84\x01a\x08HV[``\x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n;Wa\n;a\x07\x03V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\nVW`\0\x80\xFD[\x815` a\nka\nf\x83a\n\"V[a\x07\xA9V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\x8AW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xC9W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xADW`\0\x80\x81\xFD[a\n\xBB\x89\x86\x83\x8B\x01\x01a\x07\xD9V[\x84RP\x91\x83\x01\x91\x83\x01a\n\x8EV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\n\xE7W`\0\x80\xFD[a\n\xEFa\x07\x19V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x08W`\0\x80\xFD[a\x0B\x14\x85\x83\x86\x01a\x07\xD9V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B*W`\0\x80\xFD[a\x0B6\x85\x83\x86\x01a\x07\xD9V[` \x84\x01Ra\x0BG`@\x85\x01a\x08HV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0B`W`\0\x80\xFD[a\x0Bl\x85\x83\x86\x01a\x07\xD9V[``\x84\x01Ra\x0B}`\x80\x85\x01a\x08HV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\x96W`\0\x80\xFD[Pa\x0B\xA3\x84\x82\x85\x01a\nEV[`\xA0\x83\x01RPa\x0B\xB5`\xC0\x83\x01a\x08HV[`\xC0\x82\x01Ra\te`\xE0\x83\x01a\x08HV[`\0` \x82\x84\x03\x12\x15a\x0B\xD8W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xEEW`\0\x80\xFD[a\x0B\xFA\x84\x82\x85\x01a\n\xD4V[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x0C\x14W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C*W`\0\x80\xFD[\x82\x01a\x01\0\x81\x85\x03\x12\x15a\x06\xFCW`\0\x80\xFD[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\x0CcW` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\x0CGV[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[` \x81R`\0`\x01\x80`\xA0\x1B\x03\x80\x84Q\x16` \x84\x01R\x80` \x85\x01Q\x16`@\x84\x01RP`@\x83\x01Q``\x80\x84\x01Ra\x0B\xFA`\x80\x84\x01\x82a\x0C=V[`\0` \x82\x84\x03\x12\x15a\x0C\xD0W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xE6W`\0\x80\xFD[a\x0B\xFA\x84\x82\x85\x01a\x08_V[`\0` \x80\x83\x85\x03\x12\x15a\r\x05W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\r\x1CW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\r2W`\0\x80\xFD[a\r:a\x07dV[\x835\x83\x81\x11\x15a\rIW`\0\x80\xFD[a\rU\x89\x82\x87\x01a\n\xD4V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\riW`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\r~W`\0\x80\xFD[\x835a\r\x8Ca\nf\x82a\n\"V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\r\xABW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0E;W\x805\x87\x81\x11\x15a\r\xC7W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\r\xDDW`\0\x80\x81\xFD[a\r\xE5a\x07dV[\x8A\x82\x015\x89\x81\x11\x15a\r\xF7W`\0\x80\x81\xFD[a\x0E\x05\x8F\x8D\x83\x86\x01\x01a\x07\xD9V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0E\x1AW`\0\x80\x81\xFD[a\x0E(\x8F\x8D\x83\x86\x01\x01a\x07\xD9V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\r\xAFV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0EcW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0E\x83WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0E\xA0W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0E\xBAW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E\xCFW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\x12W`\0\x80\xFD[\x83\x86\x11\x15a\x0F\x1FW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F>W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0F`Wa\x0F`a\x07\x03V[`@Ra\x0Fl\x83a\x06\xC5V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0F\x94W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0F\xABW`\0\x80\xFD[\x90\x83\x01\x90a\x01\xA0\x82\x86\x03\x12\x15a\x0F\xC0W`\0\x80\xFD[a\x0F\xC8a\x07\x86V[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0F\xEC``\x84\x01a\x06\xC5V[``\x82\x01Ra\x0F\xFD`\x80\x84\x01a\x06\xC5V[`\x80\x82\x01Ra\x10\x0E`\xA0\x84\x01a\x06\xC5V[`\xA0\x82\x01Ra\x10\x1F`\xC0\x84\x01a\x06\xC5V[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x10H\x81\x85\x01a\x06\xC5V[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x10`W`\0\x80\xFD[a\x10l\x88\x82\x87\x01a\x07\xD9V[\x91\x83\x01\x91\x90\x91RPa\x01`\x83\x81\x015\x90\x82\x01Ra\x01\x80\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x93\x92PPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10\xD0`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x11F\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xA0a\x01`\x81\x81\x86\x01Ra\x11fa\x01\xC0\x86\x01\x84a\x0C=V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x90\x95\x01Q\x93\x01\x92\x90\x92RP\x90\x91\x90PV\xFE\xA2dipfsX\"\x12 \xF7\xF2F\xC3\xA5~\x1D\x8C\xF5\x04\x0C\x9B\x08\x0B,\xB3\xBD\xA9Q\xAF\rA\x13[\xCA\x18\xD4\xF9M?\xE87dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xAF\xB7`\xAC\x11a\0[W\x80c\xAF\xB7`\xAC\x14a\0\xA2W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xD6;\xCF\x18\x14a\0\xF9W\x80c\xF3p\xFD\xBB\x14a\x01\x0CW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x8DW\x80c\x12\xB2RO\x14a\0\xA2W\x80cLF\xC05\x14a\0\xB5W\x80cN\x87\xBA\x19\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\x06\xE1V[a\x01\x1AV[\0[a\0\xA0a\0\xB06`\x04a\tpV[a\x01\xA2V[a\0\xA0a\0\xC36`\x04a\x0B\xC6V[a\x01\xF9V[a\0\xA0a\0\xD66`\x04a\x0C\x02V[a\x02MV[a\0\xE3a\x05fV[`@Qa\0\xF0\x91\x90a\x0C\x83V[`@Q\x80\x91\x03\x90\xF3[a\0\xA0a\x01\x076`\x04a\x0C\xBEV[a\x06BV[a\0\xA0a\0\xC36`\x04a\x0C\xF2V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01yW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01a\x01pV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01pV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xA7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01pV[a\x03\x83`\0`\x02\x01\x80Ta\x02\xBA\x90a\x0EOV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xE6\x90a\x0EOV[\x80\x15a\x033W\x80`\x1F\x10a\x03\x08Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x033V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03\x16W\x82\x90\x03`\x1F\x16\x82\x01\x91[Pa\x03F\x93P\x86\x92P\x82\x91Pa\x0E\x89\x90PV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x06\x98\x90PV[a\x03\xC6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01pV[`\0a\x03\xD5`\xC0\x83\x01\x83a\x0E\x89V[`\0\x81\x81\x10a\x03\xE6Wa\x03\xE6a\x0E\xD6V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\x01Wa\x04\x01a\x0E\xECV[\x90P`\0\x81`\x01\x81\x11\x15a\x04\x17Wa\x04\x17a\x0E\xECV[\x03a\x04\xB9W`\0a\x04+`\xC0\x84\x01\x84a\x0E\x89V[a\x049\x91`\x01\x90\x82\x90a\x0F\x02V[\x81\x01\x90a\x04F\x91\x90a\x0F,V[`\x01T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\x9CW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xB0W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x04\xCDWa\x04\xCDa\x0E\xECV[\x03a\x05-W`\0a\x04\xE1`\xC0\x84\x01\x84a\x0E\x89V[a\x04\xEF\x91`\x01\x90\x82\x90a\x0F\x02V[\x81\x01\x90a\x04\xFC\x91\x90a\x0F\x82V[`\x01T`@Qcwr\xA6\x1B`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cwr\xA6\x1B\x90a\x04\x82\x90\x84\x90`\x04\x01a\x10\x95V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01pV[`@\x80Q``\x80\x82\x01\x83R`\0\x80\x83R` \x83\x01R\x91\x81\x01\x91\x90\x91R`@\x80Q``\x81\x01\x82R`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x83R`\x01T\x16` \x83\x01R`\x02\x80T\x92\x93\x91\x92\x91\x84\x01\x91a\x05\xBB\x90a\x0EOV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05\xE7\x90a\x0EOV[\x80\x15a\x064W\x80`\x1F\x10a\x06\tWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x064V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x06\x17W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81RPP\x90P\x90V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01pV[`\0\x81Q\x83Q\x14a\x06\xABWP`\0a\x06\xBFV[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06\xDCW`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x06\xF3W`\0\x80\xFD[a\x06\xFC\x82a\x06\xC5V[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@Qa\x01\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07<Wa\x07<a\x07\x03V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\xD1Wa\x07\xD1a\x07\x03V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x07\xEAW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08\x03Wa\x08\x03a\x07\x03V[a\x08\x16`\x1F\x82\x01`\x1F\x19\x16` \x01a\x07\xA9V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08+W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x06\xDCW`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x08rW`\0\x80\xFD[a\x08za\x07\x19V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x93W`\0\x80\xFD[a\x08\x9F\x85\x83\x86\x01a\x07\xD9V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xB5W`\0\x80\xFD[a\x08\xC1\x85\x83\x86\x01a\x07\xD9V[` \x84\x01Ra\x08\xD2`@\x85\x01a\x08HV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08\xEBW`\0\x80\xFD[a\x08\xF7\x85\x83\x86\x01a\x07\xD9V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\x10W`\0\x80\xFD[a\t\x1C\x85\x83\x86\x01a\x07\xD9V[`\x80\x84\x01Ra\t-`\xA0\x85\x01a\x08HV[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\tFW`\0\x80\xFD[Pa\tS\x84\x82\x85\x01a\x07\xD9V[`\xC0\x83\x01RPa\te`\xE0\x83\x01a\x08HV[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\x82W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\x99W`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\t\xADW`\0\x80\xFD[a\t\xB5a\x07BV[\x825\x82\x81\x11\x15a\t\xC4W`\0\x80\xFD[a\t\xD0\x87\x82\x86\x01a\x08_V[\x82RP` \x83\x015\x82\x81\x11\x15a\t\xE5W`\0\x80\xFD[a\t\xF1\x87\x82\x86\x01a\x07\xD9V[` \x83\x01RPa\n\x03`@\x84\x01a\x08HV[`@\x82\x01Ra\n\x14``\x84\x01a\x08HV[``\x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n;Wa\n;a\x07\x03V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\nVW`\0\x80\xFD[\x815` a\nka\nf\x83a\n\"V[a\x07\xA9V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\n\x8AW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\n\xC9W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xADW`\0\x80\x81\xFD[a\n\xBB\x89\x86\x83\x8B\x01\x01a\x07\xD9V[\x84RP\x91\x83\x01\x91\x83\x01a\n\x8EV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\n\xE7W`\0\x80\xFD[a\n\xEFa\x07\x19V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x08W`\0\x80\xFD[a\x0B\x14\x85\x83\x86\x01a\x07\xD9V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B*W`\0\x80\xFD[a\x0B6\x85\x83\x86\x01a\x07\xD9V[` \x84\x01Ra\x0BG`@\x85\x01a\x08HV[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0B`W`\0\x80\xFD[a\x0Bl\x85\x83\x86\x01a\x07\xD9V[``\x84\x01Ra\x0B}`\x80\x85\x01a\x08HV[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0B\x96W`\0\x80\xFD[Pa\x0B\xA3\x84\x82\x85\x01a\nEV[`\xA0\x83\x01RPa\x0B\xB5`\xC0\x83\x01a\x08HV[`\xC0\x82\x01Ra\te`\xE0\x83\x01a\x08HV[`\0` \x82\x84\x03\x12\x15a\x0B\xD8W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xEEW`\0\x80\xFD[a\x0B\xFA\x84\x82\x85\x01a\n\xD4V[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x0C\x14W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C*W`\0\x80\xFD[\x82\x01a\x01\0\x81\x85\x03\x12\x15a\x06\xFCW`\0\x80\xFD[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\x0CcW` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\x0CGV[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[` \x81R`\0`\x01\x80`\xA0\x1B\x03\x80\x84Q\x16` \x84\x01R\x80` \x85\x01Q\x16`@\x84\x01RP`@\x83\x01Q``\x80\x84\x01Ra\x0B\xFA`\x80\x84\x01\x82a\x0C=V[`\0` \x82\x84\x03\x12\x15a\x0C\xD0W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xE6W`\0\x80\xFD[a\x0B\xFA\x84\x82\x85\x01a\x08_V[`\0` \x80\x83\x85\x03\x12\x15a\r\x05W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\r\x1CW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\r2W`\0\x80\xFD[a\r:a\x07dV[\x835\x83\x81\x11\x15a\rIW`\0\x80\xFD[a\rU\x89\x82\x87\x01a\n\xD4V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\riW`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\r~W`\0\x80\xFD[\x835a\r\x8Ca\nf\x82a\n\"V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\r\xABW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0E;W\x805\x87\x81\x11\x15a\r\xC7W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\r\xDDW`\0\x80\x81\xFD[a\r\xE5a\x07dV[\x8A\x82\x015\x89\x81\x11\x15a\r\xF7W`\0\x80\x81\xFD[a\x0E\x05\x8F\x8D\x83\x86\x01\x01a\x07\xD9V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0E\x1AW`\0\x80\x81\xFD[a\x0E(\x8F\x8D\x83\x86\x01\x01a\x07\xD9V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\r\xAFV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0EcW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0E\x83WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0E\xA0W`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0E\xBAW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E\xCFW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0F\x12W`\0\x80\xFD[\x83\x86\x11\x15a\x0F\x1FW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0F>W`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0F`Wa\x0F`a\x07\x03V[`@Ra\x0Fl\x83a\x06\xC5V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0F\x94W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0F\xABW`\0\x80\xFD[\x90\x83\x01\x90a\x01\xA0\x82\x86\x03\x12\x15a\x0F\xC0W`\0\x80\xFD[a\x0F\xC8a\x07\x86V[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x0F\xEC``\x84\x01a\x06\xC5V[``\x82\x01Ra\x0F\xFD`\x80\x84\x01a\x06\xC5V[`\x80\x82\x01Ra\x10\x0E`\xA0\x84\x01a\x06\xC5V[`\xA0\x82\x01Ra\x10\x1F`\xC0\x84\x01a\x06\xC5V[`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0\x80\x84\x015\x81\x83\x01RPa\x01 a\x10H\x81\x85\x01a\x06\xC5V[\x90\x82\x01Ra\x01@\x83\x81\x015\x83\x81\x11\x15a\x10`W`\0\x80\xFD[a\x10l\x88\x82\x87\x01a\x07\xD9V[\x91\x83\x01\x91\x90\x91RPa\x01`\x83\x81\x015\x90\x82\x01Ra\x01\x80\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x93\x92PPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x10\xD0`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0\x83\x81\x01\x91\x90\x91R\x83\x01Qa\x01 \x80\x84\x01\x91\x90\x91R\x83\x01Qa\x01@a\x11F\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xA0a\x01`\x81\x81\x86\x01Ra\x11fa\x01\xC0\x86\x01\x84a\x0C=V[\x90\x86\x01Qa\x01\x80\x86\x81\x01\x91\x90\x91R\x90\x95\x01Q\x93\x01\x92\x90\x92RP\x90\x91\x90PV\xFE\xA2dipfsX\"\x12 \xF7\xF2F\xC3\xA5~\x1D\x8C\xF5\x04\x0C\x9B\x08\x0B,\xB3\xBD\xA9Q\xAF\rA\x13[\xCA\x18\xD4\xF9M?\xE87dsolcC\0\x08\x11\x003";
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
    ///`HostManagerParams(address,address,bytes)`
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
        pub hyperbridge: ::ethers::core::types::Bytes,
    }
}
