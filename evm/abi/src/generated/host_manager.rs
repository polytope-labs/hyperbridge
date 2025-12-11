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
                    ::std::borrow::ToOwned::to_owned("host"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("host"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("h"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
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
                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                ),
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                    ::std::borrow::ToOwned::to_owned("quote"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("quote"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("post"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct DispatchPost"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("quote"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("res"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct DispatchPostResponse",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
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
                    ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnauthorizedCall"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnauthorizedCall"),
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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`@Qa\x1B\x818\x03\x80a\x1B\x81\x839\x81\x01`@\x81\x90Ra\0.\x91a\x02PV[_a\x007a\x01TV[\x90P`\x01`\x01`\xA0\x1B\x03\x81\x16\x15a\x01\x1CW\x80`\x01`\x01`\xA0\x1B\x03\x16cdxF\xA5`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\0\x84W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\0\xA8\x91\x90a\x02\xB6V[`@Qc\t^\xA7\xB3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R_\x19`$\x83\x01R\x91\x90\x91\x16\x90c\t^\xA7\xB3\x90`D\x01` `@Q\x80\x83\x03\x81_\x87Z\xF1\x15\x80\x15a\0\xF6W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x01\x1A\x91\x90a\x02\xD6V[P[P\x80Q_\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x90\x92\x01Q`\x01\x80T\x90\x93\x16\x91\x16\x17\x90Ua\x02\xF5V[_Fb\xAA6\xA7\x81\x14a\x01\x93Wb\x06n\xEE\x81\x14a\x01\xAEWb\xAA7\xDC\x81\x14a\x01\xC9Wb\x01J4\x81\x14a\x01\xE4W`a\x81\x14a\x01\xFFWa'\xD8\x81\x14a\x02\x1AWP\x90V[s.\xDBt\xC2i\x94\x8B`\xEC\x10\0\x04\x0E\x10L\xEF\x0E\xAB\xAA\xE8\x91PP\x90V[s45\xBD~X\x955e5E\x9D`\x87\xD1\xEB\x98-\xAD\x90\xE7\x91PP\x90V[smQ\xB6x\x83m\x80`\xD9\x80`])\x99\xEF!\x18\t\xF3\xC2\x91PP\x90V[s\xD1\x98\xC0\x189\xDDHC\x91\x86\x17\xAF\xD1\xE4\xDD\xF4L\xC3\xBBJ\x91PP\x90V[s\x8A\xA0\xDE\xA6\xD6u\xD7\x85\xA8\x82\x96{\xF3\x81\x83\xF6\x11|\t\xB7\x91PP\x90V[sX\xA4\x1B\x89\xF4\x87\x17%\xE5\xD8\x98\xD9\x8E\xF4\xBF\x91v\x01\xC5\xEB\x91PP\x90V[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02KW__\xFD[\x91\x90PV[_`@\x82\x84\x03\x12\x80\x15a\x02aW__\xFD[P`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x02\x90WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Ra\x02\x9C\x83a\x025V[\x81Ra\x02\xAA` \x84\x01a\x025V[` \x82\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15a\x02\xC6W__\xFD[a\x02\xCF\x82a\x025V[\x93\x92PPPV[_` \x82\x84\x03\x12\x15a\x02\xE6W__\xFD[\x81Q\x80\x15\x15\x81\x14a\x02\xCFW__\xFD[a\x18\x7F\x80a\x03\x02_9_\xF3\xFE`\x80`@R`\x046\x10a\0\xA8W_5`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0bW\x80c\xB2\xA0\x1B\xF5\x14a\x01\x8DW\x80c\xBC\r\xD4G\x14a\x01\xA7W\x80c\xCF\xF0\xAB\x96\x14a\x01\xC1W\x80c\xD0\xFF\xF3f\x14a\x02\x19W\x80c\xDD\x92\xA3\x16\x14a\x023W\x80c\xF47\xBCY\x14a\x02RW__\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\xB3W\x80c\x0B\xC3{\xAB\x14a\0\xE7W\x80c\x0E\x83$\xA2\x14a\x01\x08W\x80c\x0F\xEE2\xCE\x14a\x01'W\x80c\x10\x8B\xC1\xDD\x14a\x01FW\x80cD\xAB \xF8\x14a\x01sW__\xFD[6a\0\xAFW\0[__\xFD[4\x80\x15a\0\xBEW__\xFD[Pa\0\xD2a\0\xCD6`\x04a\x08GV[a\x02~V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xF2W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x0B\xCFV[a\x02\xB4V[\0[4\x80\x15a\x01\x13W__\xFD[Pa\x01\x06a\x01\"6`\x04a\x0C\x1EV[a\x03\x06V[4\x80\x15a\x012W__\xFD[Pa\x01\x06a\x01A6`\x04a\x0C7V[a\x03ZV[4\x80\x15a\x01QW__\xFD[Pa\x01ea\x01`6`\x04a\x0CmV[a\x06\rV[`@Q\x90\x81R` \x01a\0\xDEV[4\x80\x15a\x01~W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x0F\x08V[4\x80\x15a\x01\x98W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x10\xC7V[4\x80\x15a\x01\xB2W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x115V[4\x80\x15a\x01\xCCW__\xFD[P`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xDEV[4\x80\x15a\x02$W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x11fV[4\x80\x15a\x02>W__\xFD[Pa\x01ea\x02M6`\x04a\x11\x97V[a\x06\xA2V[4\x80\x15a\x02]W__\xFD[Pa\x02fa\x078V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\xDEV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02\xAEWPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x02\xBCa\x078V[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02\xEDW`@Qc{\xF6\xA1o`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x031W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U_\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03\x86W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\x91\x83\x80a\x12WV[\x90Pa\x04S_`\x01\x01_\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01_`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03\xE6W=__>=_\xFD[PPPP`@Q=_\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04\r\x91\x90\x81\x01\x90a\x12\x97V[a\x04\x17\x83\x80a\x12\xFFV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847_\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x08\x19\x90PV[a\x04pW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04~`\xC0\x83\x01\x83a\x12\xFFV[_\x81\x81\x10a\x04\x8EWa\x04\x8Ea\x13HV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xA9Wa\x04\xA9a\x13\\V[\x90P_\x81`\x01\x81\x11\x15a\x04\xBEWa\x04\xBEa\x13\\V[\x03a\x05eW_a\x04\xD1`\xC0\x84\x01\x84a\x12\xFFV[a\x04\xDF\x91`\x01\x90\x82\x90a\x13pV[\x81\x01\x90a\x04\xEC\x91\x90a\x13\x97V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05IW__\xFD[PZ\xF1\x15\x80\x15a\x05[W=__>=_\xFD[PPPPPa\x06\x07V[`\x01\x81`\x01\x81\x11\x15a\x05yWa\x05ya\x13\\V[\x03a\x06\x07W_a\x05\x8C`\xC0\x84\x01\x84a\x12\xFFV[a\x05\x9A\x91`\x01\x90\x82\x90a\x13pV[\x81\x01\x90a\x05\xA7\x91\x90a\x14\xBFV[`\x01T`@Qcj\xD7\xDFG`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cj\xD7\xDFG\x90a\x05\xD8\x90\x84\x90`\x04\x01a\x16\xC1V[_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05\xEFW__\xFD[PZ\xF1\x15\x80\x15a\x06\x01W=__>=_\xFD[PPPPP[PPPPV[_a\x06\x16a\x078V[\x82Q`@Qc \x08\xF6\x05`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91c@\x11\xEC\n\x91a\x06D\x91`\x04\x01a\x17\xE2V[` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06_W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06\x83\x91\x90a\x17\xF4V[\x82`@\x01QQa\x06\x93\x91\x90a\x18\x1FV[\x82`\x80\x01Qa\x02\xAE\x91\x90a\x186V[_a\x06\xABa\x078V[\x82QQ`@Qc \x08\xF6\x05`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91c@\x11\xEC\n\x91a\x06\xDA\x91`\x04\x01a\x17\xE2V[` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\xF5W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x07\x19\x91\x90a\x17\xF4V[\x82` \x01QQa\x07)\x91\x90a\x18\x1FV[\x82``\x01Qa\x02\xAE\x91\x90a\x186V[_Fb\xAA6\xA7\x81\x14a\x07wWb\x06n\xEE\x81\x14a\x07\x92Wb\xAA7\xDC\x81\x14a\x07\xADWb\x01J4\x81\x14a\x07\xC8W`a\x81\x14a\x07\xE3Wa'\xD8\x81\x14a\x07\xFEWP\x90V[s.\xDBt\xC2i\x94\x8B`\xEC\x10\0\x04\x0E\x10L\xEF\x0E\xAB\xAA\xE8\x91PP\x90V[s45\xBD~X\x955e5E\x9D`\x87\xD1\xEB\x98-\xAD\x90\xE7\x91PP\x90V[smQ\xB6x\x83m\x80`\xD9\x80`])\x99\xEF!\x18\t\xF3\xC2\x91PP\x90V[s\xD1\x98\xC0\x189\xDDHC\x91\x86\x17\xAF\xD1\xE4\xDD\xF4L\xC3\xBBJ\x91PP\x90V[s\x8A\xA0\xDE\xA6\xD6u\xD7\x85\xA8\x82\x96{\xF3\x81\x83\xF6\x11|\t\xB7\x91PP\x90V[sX\xA4\x1B\x89\xF4\x87\x17%\xE5\xD8\x98\xD9\x8E\xF4\xBF\x91v\x01\xC5\xEB\x91PP\x90V[_\x81Q\x83Q\x14a\x08*WP_a\x02\xAEV[\x82Q` \x83\x81\x01\x82\x81 \x91\x86\x01\x92\x83 \x90\x91\x14[\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x08WW__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08nW__\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@R\x90V[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t\xA7Wa\t\xA7a\x08uV[`@R\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\t\xC7Wa\t\xC7a\x08uV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[_\x82`\x1F\x83\x01\x12a\t\xE4W__\xFD[\x815a\t\xF7a\t\xF2\x82a\t\xAFV[a\t\x7FV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\n\x0BW__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\n=W__\xFD[\x91\x90PV[_`\xE0\x82\x84\x03\x12\x15a\nRW__\xFD[a\nZa\x08\x89V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\nqW__\xFD[a\n}\x84\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x98W__\xFD[a\n\xA4\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\n\xB6`@\x83\x01a\n'V[`@\x82\x01R``\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xD3W__\xFD[a\n\xDF\x84\x82\x85\x01a\t\xD5V[``\x83\x01RP`\x80\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xFDW__\xFD[a\x0B\t\x84\x82\x85\x01a\t\xD5V[`\x80\x83\x01RPa\x0B\x1B`\xA0\x83\x01a\n'V[`\xA0\x82\x01R`\xC0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B8W__\xFD[a\x0BD\x84\x82\x85\x01a\t\xD5V[`\xC0\x83\x01RP\x92\x91PPV[_``\x82\x84\x03\x12\x15a\x0B`W__\xFD[a\x0Bha\x08\xB1V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\x7FW__\xFD[a\x0B\x8B\x84\x82\x85\x01a\nBV[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xA6W__\xFD[a\x0B\xB2\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x0B\xC4`@\x83\x01a\n'V[`@\x82\x01R\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0B\xDFW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xF4W__\xFD[a\x0C\0\x84\x82\x85\x01a\x0BPV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n=W__\xFD[_` \x82\x84\x03\x12\x15a\x0C.W__\xFD[a\x08n\x82a\x0C\x08V[_` \x82\x84\x03\x12\x15a\x0CGW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\\W__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x08nW__\xFD[_` \x82\x84\x03\x12\x15a\x0C}W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x92W__\xFD[\x82\x01`\xC0\x81\x85\x03\x12\x15a\x0C\xA3W__\xFD[a\x0C\xABa\x08\xD3V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xC0W__\xFD[a\x0C\xCC\x86\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xE7W__\xFD[a\x0C\xF3\x86\x82\x85\x01a\t\xD5V[` \x83\x01RP`@\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\x11W__\xFD[a\r\x1D\x86\x82\x85\x01a\t\xD5V[`@\x83\x01RPa\r/``\x83\x01a\n'V[``\x82\x01R`\x80\x82\x81\x015\x90\x82\x01Ra\rJ`\xA0\x83\x01a\x0C\x08V[`\xA0\x82\x01R\x94\x93PPPPV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\roWa\roa\x08uV[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\r\x88W__\xFD[\x815a\r\x96a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\r\xB7W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\xD9W__\xFD[a\r\xE8\x88` \x83\x8A\x01\x01a\t\xD5V[\x84RP` \x92\x83\x01\x92\x01a\r\xBCV[P\x95\x94PPPPPV[_a\x01\0\x82\x84\x03\x12\x15a\x0E\x12W__\xFD[a\x0E\x1Aa\x08\xF5V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E1W__\xFD[a\x0E=\x84\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0EXW__\xFD[a\x0Ed\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x0Ev`@\x83\x01a\n'V[`@\x82\x01Ra\x0E\x87``\x83\x01a\x0C\x08V[``\x82\x01Ra\x0E\x98`\x80\x83\x01a\n'V[`\x80\x82\x01R`\xA0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xB5W__\xFD[a\x0E\xC1\x84\x82\x85\x01a\ryV[`\xA0\x83\x01RPa\x0E\xD3`\xC0\x83\x01a\n'V[`\xC0\x82\x01R`\xE0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xF0W__\xFD[a\x0E\xFC\x84\x82\x85\x01a\t\xD5V[`\xE0\x83\x01RP\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0F\x18W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F-W__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x0F>W__\xFD[a\x0FFa\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F[W__\xFD[\x82\x01`@\x81\x87\x03\x12\x15a\x0FlW__\xFD[a\x0Fta\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\x89W__\xFD[a\x0F\x95\x88\x82\x85\x01a\x0E\x01V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\xB0W__\xFD[\x80\x83\x01\x92PP\x86`\x1F\x83\x01\x12a\x0F\xC4W__\xFD[\x815a\x0F\xD2a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x89\x83\x11\x15a\x0F\xF3W__\xFD[` \x85\x01[\x83\x81\x10\x15a\x10\x9FW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10\x15W__\xFD[\x86\x01`@\x81\x8D\x03`\x1F\x19\x01\x12\x15a\x10*W__\xFD[a\x102a\t\x18V[` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10JW__\xFD[a\x10Y\x8E` \x83\x86\x01\x01a\t\xD5V[\x82RP`@\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10tW__\xFD[a\x10\x83\x8E` \x83\x86\x01\x01a\t\xD5V[` \x83\x01RP\x80\x85RPP` \x83\x01\x92P` \x81\x01\x90Pa\x0F\xF8V[P\x80` \x85\x01RPPP\x80\x83RPPa\x10\xBA` \x83\x01a\x0C\x08V[` \x82\x01R\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a\x10\xD7W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10\xECW__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x10\xFDW__\xFD[a\x11\x05a\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x1AW__\xFD[a\x11&\x86\x82\x85\x01a\x0BPV[\x82RPa\x10\xBA` \x83\x01a\x0C\x08V[_` \x82\x84\x03\x12\x15a\x11EW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11ZW__\xFD[a\x0C\0\x84\x82\x85\x01a\nBV[_` \x82\x84\x03\x12\x15a\x11vW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x8BW__\xFD[a\x0C\0\x84\x82\x85\x01a\x0E\x01V[_` \x82\x84\x03\x12\x15a\x11\xA7W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xBCW__\xFD[\x82\x01`\xA0\x81\x85\x03\x12\x15a\x11\xCDW__\xFD[a\x11\xD5a\t:V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xEAW__\xFD[a\x11\xF6\x86\x82\x85\x01a\nBV[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\x11W__\xFD[a\x12\x1D\x86\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x12/`@\x83\x01a\n'V[`@\x82\x01R``\x82\x81\x015\x90\x82\x01Ra\x12J`\x80\x83\x01a\x0C\x08V[`\x80\x82\x01R\x94\x93PPPPV[_\x825`\xDE\x19\x836\x03\x01\x81\x12a\x12kW__\xFD[\x91\x90\x91\x01\x92\x91PPV[_[\x83\x81\x10\x15a\x12\x8FW\x81\x81\x01Q\x83\x82\x01R` \x01a\x12wV[PP_\x91\x01RV[_` \x82\x84\x03\x12\x15a\x12\xA7W__\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\xBCW__\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x12\xCCW__\xFD[\x80Qa\x12\xDAa\t\xF2\x82a\t\xAFV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x12\xEEW__\xFD[a\x08>\x82` \x83\x01` \x86\x01a\x12uV[__\x835`\x1E\x19\x846\x03\x01\x81\x12a\x13\x14W__\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x13-W__\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x13AW__\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[__\x85\x85\x11\x15a\x13~W__\xFD[\x83\x86\x11\x15a\x13\x8AW__\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[_``\x82\x84\x03\x12\x80\x15a\x13\xA8W__\xFD[Pa\x13\xB1a\x08\xB1V[a\x13\xBA\x83a\x0C\x08V[\x81R` \x83\x81\x015\x90\x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x13\xD8W__\xFD[`@\x82\x01R\x93\x92PPPV[_\x82`\x1F\x83\x01\x12a\x13\xF3W__\xFD[\x815a\x14\x01a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\x14\"W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W\x805\x83R` \x92\x83\x01\x92\x01a\x14'V[_\x82`\x1F\x83\x01\x12a\x14NW__\xFD[\x815a\x14\\a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x06\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\x14}W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W`@\x81\x88\x03\x12\x15a\x14\x99W__\xFD[a\x14\xA1a\t\x18V[\x815\x81R` \x80\x83\x015\x81\x83\x01R\x90\x84R\x92\x90\x92\x01\x91`@\x01a\x14\x82V[_` \x82\x84\x03\x12\x15a\x14\xCFW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\xE4W__\xFD[\x82\x01a\x01\xC0\x81\x85\x03\x12\x15a\x14\xF6W__\xFD[a\x14\xFEa\t\\V[\x815\x81R` \x80\x83\x015\x90\x82\x01R`@\x80\x83\x015\x90\x82\x01Ra\x15\"``\x83\x01a\x0C\x08V[``\x82\x01Ra\x153`\x80\x83\x01a\x0C\x08V[`\x80\x82\x01Ra\x15D`\xA0\x83\x01a\x0C\x08V[`\xA0\x82\x01Ra\x15U`\xC0\x83\x01a\x0C\x08V[`\xC0\x82\x01Ra\x15f`\xE0\x83\x01a\x0C\x08V[`\xE0\x82\x01Ra\x01\0\x82\x81\x015\x90\x82\x01Ra\x01 \x80\x83\x015\x90\x82\x01Ra\x15\x8Ea\x01@\x83\x01a\x0C\x08V[a\x01@\x82\x01Ra\x01`\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xADW__\xFD[a\x15\xB9\x86\x82\x85\x01a\x13\xE4V[a\x01`\x83\x01RPa\x01\x80\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xD9W__\xFD[a\x15\xE5\x86\x82\x85\x01a\x14?V[a\x01\x80\x83\x01RPa\x01\xA0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x05W__\xFD[a\x16\x11\x86\x82\x85\x01a\t\xD5V[a\x01\xA0\x83\x01RP\x94\x93PPPPV[_\x81Q\x80\x84R` \x84\x01\x93P` \x83\x01_[\x82\x81\x10\x15a\x16PW\x81Q\x86R` \x95\x86\x01\x95\x90\x91\x01\x90`\x01\x01a\x162V[P\x93\x94\x93PPPPV[_\x81Q\x80\x84R` \x84\x01\x93P` \x83\x01_[\x82\x81\x10\x15a\x16PW\x81Q\x80Q\x87R` \x90\x81\x01Q\x81\x88\x01R`@\x90\x96\x01\x95\x90\x91\x01\x90`\x01\x01a\x16lV[_\x81Q\x80\x84Ra\x16\xAD\x81` \x86\x01` \x86\x01a\x12uV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R_``\x83\x01Qa\x16\xFB`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16a\x01\0\x84\x01RPa\x01\0\x83\x01Qa\x01 \x83\x01Ra\x01 \x83\x01Qa\x01@\x83\x01Ra\x01@\x83\x01Qa\x17\x85a\x01`\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[Pa\x01`\x83\x01Qa\x01\xC0a\x01\x80\x84\x01Ra\x17\xA3a\x01\xE0\x84\x01\x82a\x16 V[\x90Pa\x01\x80\x84\x01Q`\x1F\x19\x84\x83\x03\x01a\x01\xA0\x85\x01Ra\x17\xC2\x82\x82a\x16ZV[\x91PPa\x01\xA0\x84\x01Q`\x1F\x19\x84\x83\x03\x01a\x01\xC0\x85\x01Ra\x08>\x82\x82a\x16\x96V[` \x81R_a\x08n` \x83\x01\x84a\x16\x96V[_` \x82\x84\x03\x12\x15a\x18\x04W__\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x02\xAEWa\x02\xAEa\x18\x0BV[\x80\x82\x01\x80\x82\x11\x15a\x02\xAEWa\x02\xAEa\x18\x0BV\xFE\xA2dipfsX\"\x12 \xA7\x0F\xE0f\xDA\xCB\x0Ei{\xEB]\x1E\x18\xAC\x97;\x01\xAA\r\x13\xD0\xEBF\x03\x1A;\xC83p\xA5CydsolcC\0\x08\x1E\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\xA8W_5`\xE0\x1C\x80c\xB2\xA0\x1B\xF5\x11a\0bW\x80c\xB2\xA0\x1B\xF5\x14a\x01\x8DW\x80c\xBC\r\xD4G\x14a\x01\xA7W\x80c\xCF\xF0\xAB\x96\x14a\x01\xC1W\x80c\xD0\xFF\xF3f\x14a\x02\x19W\x80c\xDD\x92\xA3\x16\x14a\x023W\x80c\xF47\xBCY\x14a\x02RW__\xFD[\x80c\x01\xFF\xC9\xA7\x14a\0\xB3W\x80c\x0B\xC3{\xAB\x14a\0\xE7W\x80c\x0E\x83$\xA2\x14a\x01\x08W\x80c\x0F\xEE2\xCE\x14a\x01'W\x80c\x10\x8B\xC1\xDD\x14a\x01FW\x80cD\xAB \xF8\x14a\x01sW__\xFD[6a\0\xAFW\0[__\xFD[4\x80\x15a\0\xBEW__\xFD[Pa\0\xD2a\0\xCD6`\x04a\x08GV[a\x02~V[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\0\xF2W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x0B\xCFV[a\x02\xB4V[\0[4\x80\x15a\x01\x13W__\xFD[Pa\x01\x06a\x01\"6`\x04a\x0C\x1EV[a\x03\x06V[4\x80\x15a\x012W__\xFD[Pa\x01\x06a\x01A6`\x04a\x0C7V[a\x03ZV[4\x80\x15a\x01QW__\xFD[Pa\x01ea\x01`6`\x04a\x0CmV[a\x06\rV[`@Q\x90\x81R` \x01a\0\xDEV[4\x80\x15a\x01~W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x0F\x08V[4\x80\x15a\x01\x98W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x10\xC7V[4\x80\x15a\x01\xB2W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x115V[4\x80\x15a\x01\xCCW__\xFD[P`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01\x81\x90R\x82Q\x80\x84\x01\x84R\x90T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x80\x83R`\x01T\x82\x16\x92\x84\x01\x92\x83R\x84Q\x90\x81R\x91Q\x16\x91\x81\x01\x91\x90\x91R\x01a\0\xDEV[4\x80\x15a\x02$W__\xFD[Pa\x01\x06a\x01\x016`\x04a\x11fV[4\x80\x15a\x02>W__\xFD[Pa\x01ea\x02M6`\x04a\x11\x97V[a\x06\xA2V[4\x80\x15a\x02]W__\xFD[Pa\x02fa\x078V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\xDEV[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16c\x9E\xD4UI`\xE0\x1B\x14\x80a\x02\xAEWPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x02\xBCa\x078V[`\x01`\x01`\xA0\x1B\x03\x163`\x01`\x01`\xA0\x1B\x03\x16\x14a\x02\xEDW`@Qc{\xF6\xA1o`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Qc\x02\xCB\xC7\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x031W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U_\x80T\x90\x91\x16\x90UV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x81\x14a\x03\x86W`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[6a\x03\x91\x83\x80a\x12WV[\x90Pa\x04S_`\x01\x01_\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16b^v>`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01_`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03\xE6W=__>=_\xFD[PPPP`@Q=_\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04\r\x91\x90\x81\x01\x90a\x12\x97V[a\x04\x17\x83\x80a\x12\xFFV[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847_\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x08\x19\x90PV[a\x04pW`@QcB\x1C\0}`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04~`\xC0\x83\x01\x83a\x12\xFFV[_\x81\x81\x10a\x04\x8EWa\x04\x8Ea\x13HV[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04\xA9Wa\x04\xA9a\x13\\V[\x90P_\x81`\x01\x81\x11\x15a\x04\xBEWa\x04\xBEa\x13\\V[\x03a\x05eW_a\x04\xD1`\xC0\x84\x01\x84a\x12\xFFV[a\x04\xDF\x91`\x01\x90\x82\x90a\x13pV[\x81\x01\x90a\x04\xEC\x91\x90a\x13\x97V[`\x01T`@\x80Qc\xCB\x1An/`\xE0\x1B\x81R\x83Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x85\x01Q`$\x83\x01R\x91\x84\x01Q\x15\x15`D\x82\x01R\x92\x93P\x16\x90c\xCB\x1An/\x90`d\x01_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05IW__\xFD[PZ\xF1\x15\x80\x15a\x05[W=__>=_\xFD[PPPPPa\x06\x07V[`\x01\x81`\x01\x81\x11\x15a\x05yWa\x05ya\x13\\V[\x03a\x06\x07W_a\x05\x8C`\xC0\x84\x01\x84a\x12\xFFV[a\x05\x9A\x91`\x01\x90\x82\x90a\x13pV[\x81\x01\x90a\x05\xA7\x91\x90a\x14\xBFV[`\x01T`@Qcj\xD7\xDFG`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90cj\xD7\xDFG\x90a\x05\xD8\x90\x84\x90`\x04\x01a\x16\xC1V[_`@Q\x80\x83\x03\x81_\x87\x80;\x15\x80\x15a\x05\xEFW__\xFD[PZ\xF1\x15\x80\x15a\x06\x01W=__>=_\xFD[PPPPP[PPPPV[_a\x06\x16a\x078V[\x82Q`@Qc \x08\xF6\x05`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91c@\x11\xEC\n\x91a\x06D\x91`\x04\x01a\x17\xE2V[` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06_W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x06\x83\x91\x90a\x17\xF4V[\x82`@\x01QQa\x06\x93\x91\x90a\x18\x1FV[\x82`\x80\x01Qa\x02\xAE\x91\x90a\x186V[_a\x06\xABa\x078V[\x82QQ`@Qc \x08\xF6\x05`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91c@\x11\xEC\n\x91a\x06\xDA\x91`\x04\x01a\x17\xE2V[` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x06\xF5W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x07\x19\x91\x90a\x17\xF4V[\x82` \x01QQa\x07)\x91\x90a\x18\x1FV[\x82``\x01Qa\x02\xAE\x91\x90a\x186V[_Fb\xAA6\xA7\x81\x14a\x07wWb\x06n\xEE\x81\x14a\x07\x92Wb\xAA7\xDC\x81\x14a\x07\xADWb\x01J4\x81\x14a\x07\xC8W`a\x81\x14a\x07\xE3Wa'\xD8\x81\x14a\x07\xFEWP\x90V[s.\xDBt\xC2i\x94\x8B`\xEC\x10\0\x04\x0E\x10L\xEF\x0E\xAB\xAA\xE8\x91PP\x90V[s45\xBD~X\x955e5E\x9D`\x87\xD1\xEB\x98-\xAD\x90\xE7\x91PP\x90V[smQ\xB6x\x83m\x80`\xD9\x80`])\x99\xEF!\x18\t\xF3\xC2\x91PP\x90V[s\xD1\x98\xC0\x189\xDDHC\x91\x86\x17\xAF\xD1\xE4\xDD\xF4L\xC3\xBBJ\x91PP\x90V[s\x8A\xA0\xDE\xA6\xD6u\xD7\x85\xA8\x82\x96{\xF3\x81\x83\xF6\x11|\t\xB7\x91PP\x90V[sX\xA4\x1B\x89\xF4\x87\x17%\xE5\xD8\x98\xD9\x8E\xF4\xBF\x91v\x01\xC5\xEB\x91PP\x90V[_\x81Q\x83Q\x14a\x08*WP_a\x02\xAEV[\x82Q` \x83\x81\x01\x82\x81 \x91\x86\x01\x92\x83 \x90\x91\x14[\x95\x94PPPPPV[_` \x82\x84\x03\x12\x15a\x08WW__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a\x08nW__\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@R\x90V[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Qa\x01\xC0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\xABWa\x08\xABa\x08uV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\t\xA7Wa\t\xA7a\x08uV[`@R\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\t\xC7Wa\t\xC7a\x08uV[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[_\x82`\x1F\x83\x01\x12a\t\xE4W__\xFD[\x815a\t\xF7a\t\xF2\x82a\t\xAFV[a\t\x7FV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\n\x0BW__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\n=W__\xFD[\x91\x90PV[_`\xE0\x82\x84\x03\x12\x15a\nRW__\xFD[a\nZa\x08\x89V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\nqW__\xFD[a\n}\x84\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\x98W__\xFD[a\n\xA4\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\n\xB6`@\x83\x01a\n'V[`@\x82\x01R``\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xD3W__\xFD[a\n\xDF\x84\x82\x85\x01a\t\xD5V[``\x83\x01RP`\x80\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xFDW__\xFD[a\x0B\t\x84\x82\x85\x01a\t\xD5V[`\x80\x83\x01RPa\x0B\x1B`\xA0\x83\x01a\n'V[`\xA0\x82\x01R`\xC0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B8W__\xFD[a\x0BD\x84\x82\x85\x01a\t\xD5V[`\xC0\x83\x01RP\x92\x91PPV[_``\x82\x84\x03\x12\x15a\x0B`W__\xFD[a\x0Bha\x08\xB1V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\x7FW__\xFD[a\x0B\x8B\x84\x82\x85\x01a\nBV[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xA6W__\xFD[a\x0B\xB2\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x0B\xC4`@\x83\x01a\n'V[`@\x82\x01R\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0B\xDFW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B\xF4W__\xFD[a\x0C\0\x84\x82\x85\x01a\x0BPV[\x94\x93PPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n=W__\xFD[_` \x82\x84\x03\x12\x15a\x0C.W__\xFD[a\x08n\x82a\x0C\x08V[_` \x82\x84\x03\x12\x15a\x0CGW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\\W__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x08nW__\xFD[_` \x82\x84\x03\x12\x15a\x0C}W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x92W__\xFD[\x82\x01`\xC0\x81\x85\x03\x12\x15a\x0C\xA3W__\xFD[a\x0C\xABa\x08\xD3V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xC0W__\xFD[a\x0C\xCC\x86\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\xE7W__\xFD[a\x0C\xF3\x86\x82\x85\x01a\t\xD5V[` \x83\x01RP`@\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\x11W__\xFD[a\r\x1D\x86\x82\x85\x01a\t\xD5V[`@\x83\x01RPa\r/``\x83\x01a\n'V[``\x82\x01R`\x80\x82\x81\x015\x90\x82\x01Ra\rJ`\xA0\x83\x01a\x0C\x08V[`\xA0\x82\x01R\x94\x93PPPPV[_`\x01`\x01`@\x1B\x03\x82\x11\x15a\roWa\roa\x08uV[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\r\x88W__\xFD[\x815a\r\x96a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\r\xB7W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\r\xD9W__\xFD[a\r\xE8\x88` \x83\x8A\x01\x01a\t\xD5V[\x84RP` \x92\x83\x01\x92\x01a\r\xBCV[P\x95\x94PPPPPV[_a\x01\0\x82\x84\x03\x12\x15a\x0E\x12W__\xFD[a\x0E\x1Aa\x08\xF5V[\x90P\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E1W__\xFD[a\x0E=\x84\x82\x85\x01a\t\xD5V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0EXW__\xFD[a\x0Ed\x84\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x0Ev`@\x83\x01a\n'V[`@\x82\x01Ra\x0E\x87``\x83\x01a\x0C\x08V[``\x82\x01Ra\x0E\x98`\x80\x83\x01a\n'V[`\x80\x82\x01R`\xA0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xB5W__\xFD[a\x0E\xC1\x84\x82\x85\x01a\ryV[`\xA0\x83\x01RPa\x0E\xD3`\xC0\x83\x01a\n'V[`\xC0\x82\x01R`\xE0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E\xF0W__\xFD[a\x0E\xFC\x84\x82\x85\x01a\t\xD5V[`\xE0\x83\x01RP\x92\x91PPV[_` \x82\x84\x03\x12\x15a\x0F\x18W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F-W__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x0F>W__\xFD[a\x0FFa\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F[W__\xFD[\x82\x01`@\x81\x87\x03\x12\x15a\x0FlW__\xFD[a\x0Fta\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\x89W__\xFD[a\x0F\x95\x88\x82\x85\x01a\x0E\x01V[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\xB0W__\xFD[\x80\x83\x01\x92PP\x86`\x1F\x83\x01\x12a\x0F\xC4W__\xFD[\x815a\x0F\xD2a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x89\x83\x11\x15a\x0F\xF3W__\xFD[` \x85\x01[\x83\x81\x10\x15a\x10\x9FW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10\x15W__\xFD[\x86\x01`@\x81\x8D\x03`\x1F\x19\x01\x12\x15a\x10*W__\xFD[a\x102a\t\x18V[` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10JW__\xFD[a\x10Y\x8E` \x83\x86\x01\x01a\t\xD5V[\x82RP`@\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10tW__\xFD[a\x10\x83\x8E` \x83\x86\x01\x01a\t\xD5V[` \x83\x01RP\x80\x85RPP` \x83\x01\x92P` \x81\x01\x90Pa\x0F\xF8V[P\x80` \x85\x01RPPP\x80\x83RPPa\x10\xBA` \x83\x01a\x0C\x08V[` \x82\x01R\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a\x10\xD7W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x10\xECW__\xFD[\x82\x01`@\x81\x85\x03\x12\x15a\x10\xFDW__\xFD[a\x11\x05a\t\x18V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x1AW__\xFD[a\x11&\x86\x82\x85\x01a\x0BPV[\x82RPa\x10\xBA` \x83\x01a\x0C\x08V[_` \x82\x84\x03\x12\x15a\x11EW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11ZW__\xFD[a\x0C\0\x84\x82\x85\x01a\nBV[_` \x82\x84\x03\x12\x15a\x11vW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\x8BW__\xFD[a\x0C\0\x84\x82\x85\x01a\x0E\x01V[_` \x82\x84\x03\x12\x15a\x11\xA7W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xBCW__\xFD[\x82\x01`\xA0\x81\x85\x03\x12\x15a\x11\xCDW__\xFD[a\x11\xD5a\t:V[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x11\xEAW__\xFD[a\x11\xF6\x86\x82\x85\x01a\nBV[\x82RP` \x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\x11W__\xFD[a\x12\x1D\x86\x82\x85\x01a\t\xD5V[` \x83\x01RPa\x12/`@\x83\x01a\n'V[`@\x82\x01R``\x82\x81\x015\x90\x82\x01Ra\x12J`\x80\x83\x01a\x0C\x08V[`\x80\x82\x01R\x94\x93PPPPV[_\x825`\xDE\x19\x836\x03\x01\x81\x12a\x12kW__\xFD[\x91\x90\x91\x01\x92\x91PPV[_[\x83\x81\x10\x15a\x12\x8FW\x81\x81\x01Q\x83\x82\x01R` \x01a\x12wV[PP_\x91\x01RV[_` \x82\x84\x03\x12\x15a\x12\xA7W__\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12\xBCW__\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x12\xCCW__\xFD[\x80Qa\x12\xDAa\t\xF2\x82a\t\xAFV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x12\xEEW__\xFD[a\x08>\x82` \x83\x01` \x86\x01a\x12uV[__\x835`\x1E\x19\x846\x03\x01\x81\x12a\x13\x14W__\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x13-W__\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x13AW__\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[__\x85\x85\x11\x15a\x13~W__\xFD[\x83\x86\x11\x15a\x13\x8AW__\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[_``\x82\x84\x03\x12\x80\x15a\x13\xA8W__\xFD[Pa\x13\xB1a\x08\xB1V[a\x13\xBA\x83a\x0C\x08V[\x81R` \x83\x81\x015\x90\x82\x01R`@\x83\x015\x80\x15\x15\x81\x14a\x13\xD8W__\xFD[`@\x82\x01R\x93\x92PPPV[_\x82`\x1F\x83\x01\x12a\x13\xF3W__\xFD[\x815a\x14\x01a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x05\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\x14\"W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W\x805\x83R` \x92\x83\x01\x92\x01a\x14'V[_\x82`\x1F\x83\x01\x12a\x14NW__\xFD[\x815a\x14\\a\t\xF2\x82a\rWV[\x80\x82\x82R` \x82\x01\x91P` \x83`\x06\x1B\x86\x01\x01\x92P\x85\x83\x11\x15a\x14}W__\xFD[` \x85\x01[\x83\x81\x10\x15a\r\xF7W`@\x81\x88\x03\x12\x15a\x14\x99W__\xFD[a\x14\xA1a\t\x18V[\x815\x81R` \x80\x83\x015\x81\x83\x01R\x90\x84R\x92\x90\x92\x01\x91`@\x01a\x14\x82V[_` \x82\x84\x03\x12\x15a\x14\xCFW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x14\xE4W__\xFD[\x82\x01a\x01\xC0\x81\x85\x03\x12\x15a\x14\xF6W__\xFD[a\x14\xFEa\t\\V[\x815\x81R` \x80\x83\x015\x90\x82\x01R`@\x80\x83\x015\x90\x82\x01Ra\x15\"``\x83\x01a\x0C\x08V[``\x82\x01Ra\x153`\x80\x83\x01a\x0C\x08V[`\x80\x82\x01Ra\x15D`\xA0\x83\x01a\x0C\x08V[`\xA0\x82\x01Ra\x15U`\xC0\x83\x01a\x0C\x08V[`\xC0\x82\x01Ra\x15f`\xE0\x83\x01a\x0C\x08V[`\xE0\x82\x01Ra\x01\0\x82\x81\x015\x90\x82\x01Ra\x01 \x80\x83\x015\x90\x82\x01Ra\x15\x8Ea\x01@\x83\x01a\x0C\x08V[a\x01@\x82\x01Ra\x01`\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xADW__\xFD[a\x15\xB9\x86\x82\x85\x01a\x13\xE4V[a\x01`\x83\x01RPa\x01\x80\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\xD9W__\xFD[a\x15\xE5\x86\x82\x85\x01a\x14?V[a\x01\x80\x83\x01RPa\x01\xA0\x82\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a\x16\x05W__\xFD[a\x16\x11\x86\x82\x85\x01a\t\xD5V[a\x01\xA0\x83\x01RP\x94\x93PPPPV[_\x81Q\x80\x84R` \x84\x01\x93P` \x83\x01_[\x82\x81\x10\x15a\x16PW\x81Q\x86R` \x95\x86\x01\x95\x90\x91\x01\x90`\x01\x01a\x162V[P\x93\x94\x93PPPPV[_\x81Q\x80\x84R` \x84\x01\x93P` \x83\x01_[\x82\x81\x10\x15a\x16PW\x81Q\x80Q\x87R` \x90\x81\x01Q\x81\x88\x01R`@\x90\x96\x01\x95\x90\x91\x01\x90`\x01\x01a\x16lV[_\x81Q\x80\x84Ra\x16\xAD\x81` \x86\x01` \x86\x01a\x12uV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`@\x82\x01Q``\x82\x01R_``\x83\x01Qa\x16\xFB`\x80\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xE0\x84\x01RP`\xE0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16a\x01\0\x84\x01RPa\x01\0\x83\x01Qa\x01 \x83\x01Ra\x01 \x83\x01Qa\x01@\x83\x01Ra\x01@\x83\x01Qa\x17\x85a\x01`\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[Pa\x01`\x83\x01Qa\x01\xC0a\x01\x80\x84\x01Ra\x17\xA3a\x01\xE0\x84\x01\x82a\x16 V[\x90Pa\x01\x80\x84\x01Q`\x1F\x19\x84\x83\x03\x01a\x01\xA0\x85\x01Ra\x17\xC2\x82\x82a\x16ZV[\x91PPa\x01\xA0\x84\x01Q`\x1F\x19\x84\x83\x03\x01a\x01\xC0\x85\x01Ra\x08>\x82\x82a\x16\x96V[` \x81R_a\x08n` \x83\x01\x84a\x16\x96V[_` \x82\x84\x03\x12\x15a\x18\x04W__\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x02\xAEWa\x02\xAEa\x18\x0BV[\x80\x82\x01\x80\x82\x11\x15a\x02\xAEWa\x02\xAEa\x18\x0BV\xFE\xA2dipfsX\"\x12 \xA7\x0F\xE0f\xDA\xCB\x0Ei{\xEB]\x1E\x18\xAC\x97;\x01\xAA\r\x13\xD0\xEBF\x03\x1A;\xC83p\xA5CydsolcC\0\x08\x1E\x003";
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
        ///Calls the contract's `host` (0xf437bc59) function
        pub fn host(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
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
        pub fn params(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, HostManagerParams> {
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
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for HostManager<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `UnauthorizedAction` with signature `UnauthorizedAction()` and selector `0x843800fa`
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
    #[etherror(name = "UnauthorizedAction", abi = "UnauthorizedAction()")]
    pub struct UnauthorizedAction;
    ///Custom Error type `UnauthorizedCall` with signature `UnauthorizedCall()` and selector `0x7bf6a16f`
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
    #[etherror(name = "UnauthorizedCall", abi = "UnauthorizedCall()")]
    pub struct UnauthorizedCall;
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
            if let Ok(decoded) = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) = <UnauthorizedAction as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnauthorizedAction(decoded));
            }
            if let Ok(decoded) = <UnauthorizedCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnauthorizedCall(decoded));
            }
            if let Ok(decoded) = <UnexpectedCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnexpectedCall(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for HostManagerErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::UnauthorizedAction(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnauthorizedCall(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnexpectedCall(element) => {
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
                    == <UnauthorizedAction as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <UnauthorizedCall as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <UnexpectedCall as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for HostManagerErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::UnauthorizedAction(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
    ///Container type for all input parameters for the `host` function with signature `host()` and selector `0xf437bc59`
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
    #[ethcall(name = "host", abi = "host()")]
    pub struct HostCall;
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
    ///Container type for all input parameters for the `onGetResponse` function with signature `onGetResponse((((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),address))` and selector `0x44ab20f8`
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
        abi = "onGetResponse((((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),address))"
    )]
    pub struct OnGetResponseCall(pub IncomingGetResponse);
    ///Container type for all input parameters for the `onGetTimeout` function with signature `onGetTimeout((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes))` and selector `0xd0fff366`
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
        abi = "onGetTimeout((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes))"
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
    ///Container type for all input parameters for the `quote` function with signature `quote((bytes,bytes,bytes,uint64,uint256,address))` and selector `0x108bc1dd`
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
    #[ethcall(name = "quote", abi = "quote((bytes,bytes,bytes,uint64,uint256,address))")]
    pub struct QuoteCall {
        pub post: DispatchPost,
    }
    ///Container type for all input parameters for the `quote` function with signature `quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and selector `0xdd92a316`
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
        name = "quote",
        abi = "quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))"
    )]
    pub struct QuoteWithResCall {
        pub res: DispatchPostResponse,
    }
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
        Host(HostCall),
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostRequestTimeout(OnPostRequestTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostResponseTimeout(OnPostResponseTimeoutCall),
        Params(ParamsCall),
        Quote(QuoteCall),
        QuoteWithRes(QuoteWithResCall),
        SetIsmpHost(SetIsmpHostCall),
        SupportsInterface(SupportsInterfaceCall),
    }
    impl ::ethers::core::abi::AbiDecode for HostManagerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <HostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Host(decoded));
            }
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
            if let Ok(decoded) = <QuoteCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Quote(decoded));
            }
            if let Ok(decoded) = <QuoteWithResCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::QuoteWithRes(decoded));
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
                Self::Host(element) => ::ethers::core::abi::AbiEncode::encode(element),
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
                Self::Quote(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::QuoteWithRes(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
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
                Self::Host(element) => ::core::fmt::Display::fmt(element, f),
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
                Self::Quote(element) => ::core::fmt::Display::fmt(element, f),
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
    ///Container type for all return fields from the `host` function with signature `host()` and selector `0xf437bc59`
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
    pub struct HostReturn {
        pub h: ::ethers::core::types::Address,
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
    ///Container type for all return fields from the `quote` function with signature `quote((bytes,bytes,bytes,uint64,uint256,address))` and selector `0x108bc1dd`
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
    pub struct QuoteReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `quote` function with signature `quote(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and selector `0xdd92a316`
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
    pub struct QuoteWithResReturn(pub ::ethers::core::types::U256);
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
