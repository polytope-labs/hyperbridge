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
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("managerParams"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                            ::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Address,
                                ::ethers::core::abi::ethabi::ParamType::Address,
                            ],
                        ),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("struct HostManagerParams"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("onAccept"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onAccept"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("incoming"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct IncomingPostRequest",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onGetResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onGetResponse"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
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
                                                        ],
                                                    ),
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
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct IncomingGetResponse",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onGetTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onGetTimeout"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
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
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetRequest"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onPostRequestTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "onPostRequestTimeout",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostRequest"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onPostResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onPostResponse"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct IncomingPostResponse",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onPostResponseTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "onPostResponseTimeout",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostResponse"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("params"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("params"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct HostManagerParams"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("supportsInterface"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("supportsInterface"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("interfaceId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        4usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes4"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bool"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
            ]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("UnauthorizedAccount"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "UnauthorizedAccount",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnexpectedCall"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnexpectedCall"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnsupportedChain"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnsupportedChain"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
            ]),
            receive: true,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static HOSTMANAGER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> = ::ethers::contract::Lazy::new(
        __abi,
    );
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qb\0\x15\xEF8\x03\x80b\0\x15\xEF\x839\x81\x01`@\x81\x90Ra\x001\x91a\0\x85V[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xEDV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0\x80W`\0\x80\xFD[\x91\x90PV[`\0`@\x82\x84\x03\x12\x15a\0\x97W`\0\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xC7WcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD3\x83a\0iV[\x81Ra\0\xE1` \x84\x01a\0iV[` \x82\x01R\x93\x92PPPV[a\x14\xF2\x80b\0\0\xFD`\09`\0\xF3\xFE`\x80`@R`\x046\x10a\0\x8AW`\x005`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0YW\x80c\xB2\xA0\x1B\xF5\x14a\x01-W\x80c\xB5\xA9\x82K\x14a\x01HW\x80c\xBC\r\xD4G\x14a\x01cW\x80c\xC4\x92\xE4&\x14a\x01~W\x80c\xCF\xF0\xAB\x96\x14a\x01\x99W`\0\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\x96W\x80c\x0B\xC3{\xAB\x14a\0\xCBW\x80c\x0E\x83$\xA2\x14a\0\xEDW\x80c\x0F\xEE2\xCE\x14a\x01\rW`\0\x80\xFD[6a\0\x91W\0[`\0\x80\xFD[4\x80\x15a\0\xA2W`\0\x80\xFD[Pa\0\xB6a\0\xB16`\x04a\x076V[a\x01\xF3V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xD7W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\nLV[a\x02*V[\0[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\0\xEBa\x01\x086`\x04a\n\x9FV[a\x02|V[4\x80\x15a\x01\x19W`\0\x80\xFD[Pa\0\xEBa\x01(6`\x04a\n\xBAV[a\x03\x04V[4\x80\x15a\x019W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\n\xF4V[4\x80\x15a\x01TW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\r\tV[4\x80\x15a\x01oW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\xA3V[4\x80\x15a\x01\x8AW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\xD7V[4\x80\x15a\x01\xA5W`\0\x80\xFD[P`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xC2V[`\0`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c=\xDD\xF0]`\xE1\x1B\x14\x80a\x02$WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x022a\x06!V[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02cW`@QcT\xBF\xF8E`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xDBW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03^W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x02\xD2V[6a\x03i\x82\x80a\x0F\x0BV[\x90Pa\x042`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03\xC3W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\xEB\x91\x90\x81\x01\x90a\x0FOV[a\x03\xF5\x83\x80a\x0F\xC5V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x07\x0C\x90PV[a\x04uW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x02\xD2V[`\0a\x04\x84`\xC0\x83\x01\x83a\x0F\xC5V[`\0\x81\x81\x10a\x04\x95Wa\x04\x95a\x10\x12V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xB0Wa\x04\xB0a\x10(V[\x90P`\0\x81`\x01\x81\x11\x15a\x04\xC6Wa\x04\xC6a\x10(V[\x03a\x05tW`\0a\x04\xDA`\xC0\x84\x01\x84a\x0F\xC5V[a\x04\xE8\x91`\x01\x90\x82\x90a\x10>V[\x81\x01\x90a\x04\xF5\x91\x90a\x10hV[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05VW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05jW=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x05\x88Wa\x05\x88a\x10(V[\x03a\x05\xE8W`\0a\x05\x9C`\xC0\x84\x01\x84a\x0F\xC5V[a\x05\xAA\x91`\x01\x90\x82\x90a\x10>V[\x81\x01\x90a\x05\xB7\x91\x90a\x11\x90V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x05<\x90\x84\x90`\x04\x01a\x13\x88V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x02\xD2V[`\0Fb\xAA6\xA7\x81\x14a\x06YWb\x06n\xEE\x81\x14a\x06uWb\xAA7\xDC\x81\x14a\x06\x91Wb\x01J4\x81\x14a\x06\xADW`a\x81\x14a\x06\xC9Wa\x06\xE1V[s\xF0\xBEe\x1F8,\xD7\x94\xAA\xB1\xB85\x84\xAAE\x8Buk\xD4\xCF\x91Pa\x06\xE1V[s}\xA4o\xB3\xB7{4\xEFn\xCF\x05Y\x15\xAC\xB1\xD4ee\xFBA\x91Pa\x06\xE1V[s\x8A\xC3\x9D\xFC\x1F&\x16\xE5\xE1\x9B\x93B\x0Cm\0\x8A\x8A\x8E\xE6_\x91Pa\x06\xE1V[s\xF8\xDB\xA4\xEB\0b\x1CWxv4\xF8\xDE\xBD\xDB\x18\x8B\xC7#\x8E\x91Pa\x06\xE1V[s\xA3\xF0|\x94\xA7\xE6\xCD\x93g\xA2\xE0\xC0\xF4$~\xB2\xACF|\x86\x91P[P`\x01`\x01`\xA0\x1B\x03\x81\x16a\x07\tW`@Qc\xD2\x1E\xAB7`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x90V[`\0\x81Q\x83Q\x14a\x07\x1FWP`\0a\x02$V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[`\0` \x82\x84\x03\x12\x15a\x07HW`\0\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x07`W`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\x12Wa\x08\x12a\x07gV[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x083Wa\x083a\x07gV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x08RW`\0\x80\xFD[\x815a\x08ea\x08`\x82a\x08\x1AV[a\x07\xEAV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08zW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xAEW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xC5W`\0\x80\xFD[a\x08\xCDa\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xE6W`\0\x80\xFD[a\x08\xF2\x85\x83\x86\x01a\x08AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\t\x08W`\0\x80\xFD[a\t\x14\x85\x83\x86\x01a\x08AV[` \x84\x01Ra\t%`@\x85\x01a\x08\x97V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t>W`\0\x80\xFD[a\tJ\x85\x83\x86\x01a\x08AV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tcW`\0\x80\xFD[a\to\x85\x83\x86\x01a\x08AV[`\x80\x84\x01Ra\t\x80`\xA0\x85\x01a\x08\x97V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\x99W`\0\x80\xFD[Pa\t\xA6\x84\x82\x85\x01a\x08AV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\t\xC4W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\xE7Wa\t\xE7a\x07gV[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\t\xFFW`\0\x80\xFD[a\n\x0B\x86\x83\x87\x01a\x08\xB3V[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\n!W`\0\x80\xFD[Pa\n.\x85\x82\x86\x01a\x08AV[` \x83\x01RPa\n@`@\x84\x01a\x08\x97V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\n^W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\ntW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\t\xB2V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xAEW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\xB1W`\0\x80\xFD[a\x07`\x82a\n\x88V[`\0` \x82\x84\x03\x12\x15a\n\xCCW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xE2W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x07`W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x06W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x1DW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0B1W`\0\x80\xFD[a\x0B9a\x07\xA5V[\x825\x82\x81\x11\x15a\x0BHW`\0\x80\xFD[a\x0BT\x87\x82\x86\x01a\t\xB2V[\x82RPa\x0Bc` \x84\x01a\n\x88V[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0B\x8AWa\x0B\x8Aa\x07gV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\xA5W`\0\x80\xFD[\x815` a\x0B\xB5a\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B\xD4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xF7W`\0\x80\x81\xFD[a\x0C\x05\x89\x86\x83\x8B\x01\x01a\x08AV[\x84RP\x91\x83\x01\x91\x83\x01a\x0B\xD8V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0C0W`\0\x80\xFD[a\x0C8a\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0CQW`\0\x80\xFD[a\x0C]\x85\x83\x86\x01a\x08AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0CsW`\0\x80\xFD[a\x0C\x7F\x85\x83\x86\x01a\x08AV[` \x84\x01Ra\x0C\x90`@\x85\x01a\x08\x97V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\xA9W`\0\x80\xFD[a\x0C\xB5\x85\x83\x86\x01a\x08AV[``\x84\x01Ra\x0C\xC6`\x80\x85\x01a\x08\x97V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0C\xDFW`\0\x80\xFD[Pa\x0C\xEC\x84\x82\x85\x01a\x0B\x94V[`\xA0\x83\x01RPa\x0C\xFE`\xC0\x83\x01a\x08\x97V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\r\x1BW`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\r1W`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\rEW`\0\x80\xFD[a\rMa\x07\xA5V[\x82\x825\x11\x15a\r[W`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\roW`\0\x80\xFD[a\rwa\x07\xA5V[\x84\x825\x11\x15a\r\x85W`\0\x80\xFD[a\r\x92\x88\x835\x84\x01a\x0C\x1EV[\x81R\x84` \x83\x015\x11\x15a\r\xA5W`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\r\xBDW`\0\x80\xFD[a\r\xCAa\x08`\x835a\x0BqV[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\r\xE9W`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x0E\x88W\x88\x815\x11\x15a\x0E\x04W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0E\x1CW`\0\x80\xFD[a\x0E$a\x07\xA5V[\x8A` \x83\x015\x11\x15a\x0E5W`\0\x80\xFD[a\x0EG\x8E` \x80\x85\x015\x85\x01\x01a\x08AV[\x81R\x8A`@\x83\x015\x11\x15a\x0EZW`\0\x80\xFD[a\x0Em\x8E` `@\x85\x015\x85\x01\x01a\x08AV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\r\xEEV[PP\x80` \x84\x01RPP\x80\x83RPPa\x0Bc` \x83\x01a\n\x88V[`\0` \x82\x84\x03\x12\x15a\x0E\xB5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xCBW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\x08\xB3V[`\0` \x82\x84\x03\x12\x15a\x0E\xE9W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xFFW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\x0C\x1EV[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0F!W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0FFW\x81\x81\x01Q\x83\x82\x01R` \x01a\x0F.V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0FaW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0FwW`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0F\x88W`\0\x80\xFD[\x80Qa\x0F\x96a\x08`\x82a\x08\x1AV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0F\xABW`\0\x80\xFD[a\x0F\xBC\x82` \x83\x01` \x86\x01a\x0F+V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\xDCW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0F\xF6W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x10\x0BW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x10NW`\0\x80\xFD[\x83\x86\x11\x15a\x10[W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0``\x82\x84\x03\x12\x15a\x10zW`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10\x9CWa\x10\x9Ca\x07gV[`@Ra\x10\xA8\x83a\n\x88V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x10\xC7W`\0\x80\xFD[`@\x82\x01R\x93\x92PPPV[`\0\x82`\x1F\x83\x01\x12a\x10\xE4W`\0\x80\xFD[\x815` a\x10\xF4a\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x13W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13W\x805\x83R\x91\x83\x01\x91\x83\x01a\x11\x17V[`\0\x82`\x1F\x83\x01\x12a\x11?W`\0\x80\xFD[\x815` a\x11Oa\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11nW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13Wa\x11\x83\x81a\n\x88V[\x83R\x91\x83\x01\x91\x83\x01a\x11rV[`\0` \x82\x84\x03\x12\x15a\x11\xA2W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\xB9W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x11\xCEW`\0\x80\xFD[a\x11\xD6a\x07\xC7V[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x11\xFA``\x84\x01a\n\x88V[``\x82\x01Ra\x12\x0B`\x80\x84\x01a\n\x88V[`\x80\x82\x01Ra\x12\x1C`\xA0\x84\x01a\n\x88V[`\xA0\x82\x01Ra\x12-`\xC0\x84\x01a\n\x88V[`\xC0\x82\x01Ra\x12>`\xE0\x84\x01a\n\x88V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x12g\x81\x85\x01a\n\x88V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x12\x7FW`\0\x80\xFD[a\x12\x8B\x88\x82\x87\x01a\x10\xD3V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x12\xA5W`\0\x80\xFD[a\x12\xB1\x88\x82\x87\x01a\x11.V[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x12\xCBW`\0\x80\xFD[a\x12\xD7\x88\x82\x87\x01a\x08AV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x13\x18W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xFCV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x13\x18W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x137V[`\0\x81Q\x80\x84Ra\x13t\x81` \x86\x01` \x86\x01a\x0F+V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x13\xC3`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x14\x1F\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x14U\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x14ua\x01\xE0\x86\x01\x84a\x12\xE8V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x14\x96\x85\x84a\x13#V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x14\xB2\x83\x82a\x13\\V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \x05\xE8^\xA0\xCA\xEBj\xCF\x894\xE1\xB9\x0C\xA7\x9E\xA1\xB0\xA9\xB9\xD9H3u6j\xB4\xE7\x16\xCAz\xFB\xE8dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\x8AW`\x005`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0YW\x80c\xB2\xA0\x1B\xF5\x14a\x01-W\x80c\xB5\xA9\x82K\x14a\x01HW\x80c\xBC\r\xD4G\x14a\x01cW\x80c\xC4\x92\xE4&\x14a\x01~W\x80c\xCF\xF0\xAB\x96\x14a\x01\x99W`\0\x80\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\x96W\x80c\x0B\xC3{\xAB\x14a\0\xCBW\x80c\x0E\x83$\xA2\x14a\0\xEDW\x80c\x0F\xEE2\xCE\x14a\x01\rW`\0\x80\xFD[6a\0\x91W\0[`\0\x80\xFD[4\x80\x15a\0\xA2W`\0\x80\xFD[Pa\0\xB6a\0\xB16`\x04a\x076V[a\x01\xF3V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xD7W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\nLV[a\x02*V[\0[4\x80\x15a\0\xF9W`\0\x80\xFD[Pa\0\xEBa\x01\x086`\x04a\n\x9FV[a\x02|V[4\x80\x15a\x01\x19W`\0\x80\xFD[Pa\0\xEBa\x01(6`\x04a\n\xBAV[a\x03\x04V[4\x80\x15a\x019W`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\n\xF4V[4\x80\x15a\x01TW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\r\tV[4\x80\x15a\x01oW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\xA3V[4\x80\x15a\x01\x8AW`\0\x80\xFD[Pa\0\xEBa\0\xE66`\x04a\x0E\xD7V[4\x80\x15a\x01\xA5W`\0\x80\xFD[P`@\x80Q\x80\x82\x01\x82R`\0\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xC2V[`\0`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c=\xDD\xF0]`\xE1\x1B\x14\x80a\x02$WPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x022a\x06!V[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02cW`@QcT\xBF\xF8E`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\xDBW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03^W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x02\xD2V[6a\x03i\x82\x80a\x0F\x0BV[\x90Pa\x042`\0`\x01\x01`\0\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03\xC3W=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x03\xEB\x91\x90\x81\x01\x90a\x0FOV[a\x03\xF5\x83\x80a\x0F\xC5V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x07\x0C\x90PV[a\x04uW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x02\xD2V[`\0a\x04\x84`\xC0\x83\x01\x83a\x0F\xC5V[`\0\x81\x81\x10a\x04\x95Wa\x04\x95a\x10\x12V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xB0Wa\x04\xB0a\x10(V[\x90P`\0\x81`\x01\x81\x11\x15a\x04\xC6Wa\x04\xC6a\x10(V[\x03a\x05tW`\0a\x04\xDA`\xC0\x84\x01\x84a\x0F\xC5V[a\x04\xE8\x91`\x01\x90\x82\x90a\x10>V[\x81\x01\x90a\x04\xF5\x91\x90a\x10hV[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05VW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05jW=`\0\x80>=`\0\xFD[PPPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x05\x88Wa\x05\x88a\x10(V[\x03a\x05\xE8W`\0a\x05\x9C`\xC0\x84\x01\x84a\x0F\xC5V[a\x05\xAA\x91`\x01\x90\x82\x90a\x10>V[\x81\x01\x90a\x05\xB7\x91\x90a\x11\x90V[`\x01T`@Qc\nl^m`\xE3\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cSb\xF3h\x90a\x05<\x90\x84\x90`\x04\x01a\x13\x88V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x02\xD2V[`\0Fb\xAA6\xA7\x81\x14a\x06YWb\x06n\xEE\x81\x14a\x06uWb\xAA7\xDC\x81\x14a\x06\x91Wb\x01J4\x81\x14a\x06\xADW`a\x81\x14a\x06\xC9Wa\x06\xE1V[s\xF0\xBEe\x1F8,\xD7\x94\xAA\xB1\xB85\x84\xAAE\x8Buk\xD4\xCF\x91Pa\x06\xE1V[s}\xA4o\xB3\xB7{4\xEFn\xCF\x05Y\x15\xAC\xB1\xD4ee\xFBA\x91Pa\x06\xE1V[s\x8A\xC3\x9D\xFC\x1F&\x16\xE5\xE1\x9B\x93B\x0Cm\0\x8A\x8A\x8E\xE6_\x91Pa\x06\xE1V[s\xF8\xDB\xA4\xEB\0b\x1CWxv4\xF8\xDE\xBD\xDB\x18\x8B\xC7#\x8E\x91Pa\x06\xE1V[s\xA3\xF0|\x94\xA7\xE6\xCD\x93g\xA2\xE0\xC0\xF4$~\xB2\xACF|\x86\x91P[P`\x01`\x01`\xA0\x1B\x03\x81\x16a\x07\tW`@Qc\xD2\x1E\xAB7`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x90V[`\0\x81Q\x83Q\x14a\x07\x1FWP`\0a\x02$V[P\x81Q` \x91\x82\x01\x81\x90 \x91\x90\x92\x01\x91\x90\x91 \x14\x90V[`\0` \x82\x84\x03\x12\x15a\x07HW`\0\x80\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x07`W`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\x12Wa\x08\x12a\x07gV[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x083Wa\x083a\x07gV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x08RW`\0\x80\xFD[\x815a\x08ea\x08`\x82a\x08\x1AV[a\x07\xEAV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08zW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xAEW`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xC5W`\0\x80\xFD[a\x08\xCDa\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xE6W`\0\x80\xFD[a\x08\xF2\x85\x83\x86\x01a\x08AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\t\x08W`\0\x80\xFD[a\t\x14\x85\x83\x86\x01a\x08AV[` \x84\x01Ra\t%`@\x85\x01a\x08\x97V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t>W`\0\x80\xFD[a\tJ\x85\x83\x86\x01a\x08AV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tcW`\0\x80\xFD[a\to\x85\x83\x86\x01a\x08AV[`\x80\x84\x01Ra\t\x80`\xA0\x85\x01a\x08\x97V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\x99W`\0\x80\xFD[Pa\t\xA6\x84\x82\x85\x01a\x08AV[`\xC0\x83\x01RP\x92\x91PPV[`\0``\x82\x84\x03\x12\x15a\t\xC4W`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x82\x82\x10\x81\x83\x11\x17\x15a\t\xE7Wa\t\xE7a\x07gV[\x81`@R\x82\x93P\x845\x91P\x80\x82\x11\x15a\t\xFFW`\0\x80\xFD[a\n\x0B\x86\x83\x87\x01a\x08\xB3V[\x83R` \x85\x015\x91P\x80\x82\x11\x15a\n!W`\0\x80\xFD[Pa\n.\x85\x82\x86\x01a\x08AV[` \x83\x01RPa\n@`@\x84\x01a\x08\x97V[`@\x82\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\n^W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\ntW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\t\xB2V[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xAEW`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\xB1W`\0\x80\xFD[a\x07`\x82a\n\x88V[`\0` \x82\x84\x03\x12\x15a\n\xCCW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xE2W`\0\x80\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x07`W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\x0B\x06W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x1DW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0B1W`\0\x80\xFD[a\x0B9a\x07\xA5V[\x825\x82\x81\x11\x15a\x0BHW`\0\x80\xFD[a\x0BT\x87\x82\x86\x01a\t\xB2V[\x82RPa\x0Bc` \x84\x01a\n\x88V[` \x82\x01R\x95\x94PPPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0B\x8AWa\x0B\x8Aa\x07gV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\xA5W`\0\x80\xFD[\x815` a\x0B\xB5a\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B\xD4W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xF7W`\0\x80\x81\xFD[a\x0C\x05\x89\x86\x83\x8B\x01\x01a\x08AV[\x84RP\x91\x83\x01\x91\x83\x01a\x0B\xD8V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0C0W`\0\x80\xFD[a\x0C8a\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0CQW`\0\x80\xFD[a\x0C]\x85\x83\x86\x01a\x08AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0CsW`\0\x80\xFD[a\x0C\x7F\x85\x83\x86\x01a\x08AV[` \x84\x01Ra\x0C\x90`@\x85\x01a\x08\x97V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\xA9W`\0\x80\xFD[a\x0C\xB5\x85\x83\x86\x01a\x08AV[``\x84\x01Ra\x0C\xC6`\x80\x85\x01a\x08\x97V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0C\xDFW`\0\x80\xFD[Pa\x0C\xEC\x84\x82\x85\x01a\x0B\x94V[`\xA0\x83\x01RPa\x0C\xFE`\xC0\x83\x01a\x08\x97V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\r\x1BW`\0\x80\xFD[`\x01`\x01`@\x1B\x03\x80\x835\x11\x15a\r1W`\0\x80\xFD[\x825\x83\x01`@\x81\x86\x03\x12\x15a\rEW`\0\x80\xFD[a\rMa\x07\xA5V[\x82\x825\x11\x15a\r[W`\0\x80\xFD[\x815\x82\x01`@\x81\x88\x03\x12\x15a\roW`\0\x80\xFD[a\rwa\x07\xA5V[\x84\x825\x11\x15a\r\x85W`\0\x80\xFD[a\r\x92\x88\x835\x84\x01a\x0C\x1EV[\x81R\x84` \x83\x015\x11\x15a\r\xA5W`\0\x80\xFD[` \x82\x015\x82\x01\x91P\x87`\x1F\x83\x01\x12a\r\xBDW`\0\x80\xFD[a\r\xCAa\x08`\x835a\x0BqV[\x825\x80\x82R` \x80\x83\x01\x92\x91`\x05\x1B\x85\x01\x01\x8A\x81\x11\x15a\r\xE9W`\0\x80\xFD[` \x85\x01[\x81\x81\x10\x15a\x0E\x88W\x88\x815\x11\x15a\x0E\x04W`\0\x80\xFD[\x805\x86\x01`@\x81\x8E\x03`\x1F\x19\x01\x12\x15a\x0E\x1CW`\0\x80\xFD[a\x0E$a\x07\xA5V[\x8A` \x83\x015\x11\x15a\x0E5W`\0\x80\xFD[a\x0EG\x8E` \x80\x85\x015\x85\x01\x01a\x08AV[\x81R\x8A`@\x83\x015\x11\x15a\x0EZW`\0\x80\xFD[a\x0Em\x8E` `@\x85\x015\x85\x01\x01a\x08AV[` \x82\x01R\x80\x86RPP` \x84\x01\x93P` \x81\x01\x90Pa\r\xEEV[PP\x80` \x84\x01RPP\x80\x83RPPa\x0Bc` \x83\x01a\n\x88V[`\0` \x82\x84\x03\x12\x15a\x0E\xB5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xCBW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\x08\xB3V[`\0` \x82\x84\x03\x12\x15a\x0E\xE9W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xFFW`\0\x80\xFD[a\n\x80\x84\x82\x85\x01a\x0C\x1EV[`\0\x825`\xDE\x19\x836\x03\x01\x81\x12a\x0F!W`\0\x80\xFD[\x91\x90\x91\x01\x92\x91PPV[`\0[\x83\x81\x10\x15a\x0FFW\x81\x81\x01Q\x83\x82\x01R` \x01a\x0F.V[PP`\0\x91\x01RV[`\0` \x82\x84\x03\x12\x15a\x0FaW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0FwW`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x0F\x88W`\0\x80\xFD[\x80Qa\x0F\x96a\x08`\x82a\x08\x1AV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x0F\xABW`\0\x80\xFD[a\x0F\xBC\x82` \x83\x01` \x86\x01a\x0F+V[\x95\x94PPPPPV[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0F\xDCW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0F\xF6W`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x10\x0BW`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x10NW`\0\x80\xFD[\x83\x86\x11\x15a\x10[W`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0``\x82\x84\x03\x12\x15a\x10zW`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x10\x9CWa\x10\x9Ca\x07gV[`@Ra\x10\xA8\x83a\n\x88V[\x81R` \x83\x015` \x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x10\xC7W`\0\x80\xFD[`@\x82\x01R\x93\x92PPPV[`\0\x82`\x1F\x83\x01\x12a\x10\xE4W`\0\x80\xFD[\x815` a\x10\xF4a\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x13W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13W\x805\x83R\x91\x83\x01\x91\x83\x01a\x11\x17V[`\0\x82`\x1F\x83\x01\x12a\x11?W`\0\x80\xFD[\x815` a\x11Oa\x08`\x83a\x0BqV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11nW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0C\x13Wa\x11\x83\x81a\n\x88V[\x83R\x91\x83\x01\x91\x83\x01a\x11rV[`\0` \x82\x84\x03\x12\x15a\x11\xA2W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\xB9W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xC0\x82\x86\x03\x12\x15a\x11\xCEW`\0\x80\xFD[a\x11\xD6a\x07\xC7V[\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01Ra\x11\xFA``\x84\x01a\n\x88V[``\x82\x01Ra\x12\x0B`\x80\x84\x01a\n\x88V[`\x80\x82\x01Ra\x12\x1C`\xA0\x84\x01a\n\x88V[`\xA0\x82\x01Ra\x12-`\xC0\x84\x01a\n\x88V[`\xC0\x82\x01Ra\x12>`\xE0\x84\x01a\n\x88V[`\xE0\x82\x01Ra\x01\0\x83\x81\x015\x90\x82\x01Ra\x01 \x80\x84\x015\x90\x82\x01Ra\x01@a\x12g\x81\x85\x01a\n\x88V[\x90\x82\x01Ra\x01`\x83\x81\x015\x83\x81\x11\x15a\x12\x7FW`\0\x80\xFD[a\x12\x8B\x88\x82\x87\x01a\x10\xD3V[\x82\x84\x01RPPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x12\xA5W`\0\x80\xFD[a\x12\xB1\x88\x82\x87\x01a\x11.V[\x82\x84\x01RPPa\x01\xA0\x80\x84\x015\x83\x81\x11\x15a\x12\xCBW`\0\x80\xFD[a\x12\xD7\x88\x82\x87\x01a\x08AV[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x13\x18W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x12\xFCV[P\x94\x95\x94PPPPPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x13\x18W\x81Q`\x01`\x01`\xA0\x1B\x03\x16\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x137V[`\0\x81Q\x80\x84Ra\x13t\x81` \x86\x01` \x86\x01a\x0F+V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R`\0``\x83\x01Qa\x13\xC3`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Qa\x01\0a\x14\x1F\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x84\x01Qa\x01 \x84\x81\x01\x91\x90\x91R\x84\x01Qa\x01@\x80\x85\x01\x91\x90\x91R\x84\x01Q\x90Pa\x01`a\x14U\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xC0a\x01\x80\x81\x81\x86\x01Ra\x14ua\x01\xE0\x86\x01\x84a\x12\xE8V[\x92P\x80\x86\x01Q\x90P`\x1F\x19a\x01\xA0\x81\x87\x86\x03\x01\x81\x88\x01Ra\x14\x96\x85\x84a\x13#V[\x90\x88\x01Q\x87\x82\x03\x90\x92\x01\x84\x88\x01R\x93P\x90Pa\x14\xB2\x83\x82a\x13\\V[\x96\x95PPPPPPV\xFE\xA2dipfsX\"\x12 \x05\xE8^\xA0\xCA\xEBj\xCF\x894\xE1\xB9\x0C\xA7\x9E\xA1\xB0\xA9\xB9\xD9H3u6j\xB4\xE7\x16\xCAz\xFB\xE8dsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static HOSTMANAGER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
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
            f.debug_tuple(::core::stringify!(HostManager))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> HostManager<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    HOSTMANAGER_ABI.clone(),
                    client,
                ),
            )
        }
        /// Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it.
        /// Returns a new instance of a deployer that returns an instance of this contract after sending the transaction
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
        pub fn params(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, HostManagerParams> {
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
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for HostManager<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `UnauthorizedAccount` with signature `UnauthorizedAccount()` and selector `0xa97ff08a`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "UnauthorizedAccount", abi = "UnauthorizedAccount()")]
    pub struct UnauthorizedAccount;
    ///Custom Error type `UnexpectedCall` with signature `UnexpectedCall()` and selector `0x02cbc79f`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "UnexpectedCall", abi = "UnexpectedCall()")]
    pub struct UnexpectedCall;
    ///Custom Error type `UnsupportedChain` with signature `UnsupportedChain()` and selector `0xd21eab37`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "UnsupportedChain", abi = "UnsupportedChain()")]
    pub struct UnsupportedChain;
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum HostManagerErrors {
        UnauthorizedAccount(UnauthorizedAccount),
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
            if let Ok(decoded) = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) = <UnauthorizedAccount as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnauthorizedAccount(decoded));
            }
            if let Ok(decoded) = <UnexpectedCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnexpectedCall(decoded));
            }
            if let Ok(decoded) = <UnsupportedChain as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnsupportedChain(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for HostManagerErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::UnauthorizedAccount(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnexpectedCall(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnsupportedChain(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for HostManagerErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <UnauthorizedAccount as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <UnexpectedCall as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <UnsupportedChain as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for HostManagerErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::UnauthorizedAccount(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
    ///Container type for all input parameters for the `onAccept` function with signature `onAccept(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address))` and selector `0x0fee32ce`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onAccept",
        abi = "onAccept(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address))"
    )]
    pub struct OnAcceptCall {
        pub incoming: IncomingPostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature `onGetResponse((((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]),address))` and selector `0xb5a9824b`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onGetResponse",
        abi = "onGetResponse((((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]),address))"
    )]
    pub struct OnGetResponseCall(pub IncomingGetResponse);
    ///Container type for all input parameters for the `onGetTimeout` function with signature `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))` and selector `0xc492e426`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onGetTimeout",
        abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))"
    )]
    pub struct OnGetTimeoutCall(pub GetRequest);
    ///Container type for all input parameters for the `onPostRequestTimeout` function with signature `onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))` and selector `0xbc0dd447`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onPostRequestTimeout",
        abi = "onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))"
    )]
    pub struct OnPostRequestTimeoutCall(pub PostRequest);
    ///Container type for all input parameters for the `onPostResponse` function with signature `onPostResponse((((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address))` and selector `0xb2a01bf5`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onPostResponse",
        abi = "onPostResponse((((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address))"
    )]
    pub struct OnPostResponseCall(pub IncomingPostResponse);
    ///Container type for all input parameters for the `onPostResponseTimeout` function with signature `onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))` and selector `0x0bc37bab`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onPostResponseTimeout",
        abi = "onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))"
    )]
    pub struct OnPostResponseTimeoutCall(pub PostResponse);
    ///Container type for all input parameters for the `params` function with signature `params()` and selector `0xcff0ab96`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "params", abi = "params()")]
    pub struct ParamsCall;
    ///Container type for all input parameters for the `setIsmpHost` function with signature `setIsmpHost(address)` and selector `0x0e8324a2`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "setIsmpHost", abi = "setIsmpHost(address)")]
    pub struct SetIsmpHostCall {
        pub host: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `supportsInterface` function with signature `supportsInterface(bytes4)` and selector `0x01ffc9a7`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
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
            if let Ok(decoded) = <OnAcceptCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnAccept(decoded));
            }
            if let Ok(decoded) = <OnGetResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnGetResponse(decoded));
            }
            if let Ok(decoded) = <OnGetTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnGetTimeout(decoded));
            }
            if let Ok(decoded) = <OnPostRequestTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostRequestTimeout(decoded));
            }
            if let Ok(decoded) = <OnPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostResponse(decoded));
            }
            if let Ok(decoded) = <OnPostResponseTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostResponseTimeout(decoded));
            }
            if let Ok(decoded) = <ParamsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Params(decoded));
            }
            if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetIsmpHost(decoded));
            }
            if let Ok(decoded) = <SupportsInterfaceCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SupportsInterface(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for HostManagerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::OnAccept(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnGetResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnGetTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostRequestTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostResponseTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Params(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::SetIsmpHost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SupportsInterface(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for HostManagerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostRequestTimeout(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostResponseTimeout(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
    ///Container type for all return fields from the `params` function with signature `params()` and selector `0xcff0ab96`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct ParamsReturn(pub HostManagerParams);
    ///Container type for all return fields from the `supportsInterface` function with signature `supportsInterface(bytes4)` and selector `0x01ffc9a7`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
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
        Hash
    )]
    pub struct HostManagerParams {
        pub admin: ::ethers::core::types::Address,
        pub host: ::ethers::core::types::Address,
    }
}
