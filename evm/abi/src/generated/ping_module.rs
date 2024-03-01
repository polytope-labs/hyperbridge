pub use ping_module::*;
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
pub mod ping_module {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("admin"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("dispatch"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostRequest"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetRequest"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("dispatchPostResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "dispatchPostResponse",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
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
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostResponse"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("dispatchToParachain"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "dispatchToParachain",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_paraId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
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
                    ::std::borrow::ToOwned::to_owned("host"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("host"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
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
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetResponse"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                    ::std::borrow::ToOwned::to_owned("ping"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("ping"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("pingMessage"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PingMessage"),
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
                    ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("hostAddr"),
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
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("GetResponseReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "GetResponseReceived",
                            ),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("GetTimeoutReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("GetTimeoutReceived"),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("MessageDispatched"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("MessageDispatched"),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostReceived"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostRequestTimeoutReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostRequestTimeoutReceived",
                            ),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostResponseReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostResponseReceived",
                            ),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostResponseTimeoutReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostResponseTimeoutReceived",
                            ),
                            inputs: ::std::vec![],
                            anonymous: false,
                        },
                    ],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("ExecutionFailed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("ExecutionFailed"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotIsmpHost"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotIsmpHost"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static PINGMODULE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> = ::ethers::contract::Lazy::new(
        __abi,
    );
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x1A\xAB8\x03\x80a\x1A\xAB\x839\x81\x01`@\x81\x90Ra\0/\x91a\0TV[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90Ua\0\x84V[`\0` \x82\x84\x03\x12\x15a\0fW`\0\x80\xFD[\x81Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0}W`\0\x80\xFD[\x93\x92PPPV[a\x1A\x18\x80a\0\x93`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xCFW`\x005`\xE0\x1C\x80cr5N\x9B\x11a\0\x8CW\x80c\xD1\xABF\xCF\x11a\0fW\x80c\xD1\xABF\xCF\x14a\x01\x94W\x80c\xD6;\xCF\x18\x14a\x01\xA7W\x80c\xF3p\xFD\xBB\x14a\x01\xBAW\x80c\xF47\xBCY\x14a\x01\xCDW`\0\x80\xFD[\x80cr5N\x9B\x14a\x01[W\x80c\xAF\xB7`\xAC\x14a\x01nW\x80c\xCAKh\xFC\x14a\x01\x81W`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\xD4W\x80c\x12\xB2RO\x14a\0\xE9W\x80c1&}\xEE\x14a\0\xFCW\x80cJi.\x06\x14a\x01\"W\x80cLF\xC05\x14a\x015W\x80cN\x87\xBA\x19\x14a\x01HW[`\0\x80\xFD[a\0\xE7a\0\xE26`\x04a\x0C\x8AV[a\x01\xE8V[\0[a\0\xE7a\0\xF76`\x04a\x0F\x1FV[a\x025V[a\x01\x0Fa\x01\n6`\x04a\x0F\xD1V[a\x02\x8CV[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xE7a\x0106`\x04a\x10\rV[a\x03fV[a\0\xE7a\x01C6`\x04a\x12QV[a\x05\nV[a\0\xE7a\x01V6`\x04a\x0F\xD1V[a\x05aV[a\0\xE7a\x01i6`\x04a\x12\x85V[a\x05\xCAV[a\0\xE7a\x01|6`\x04a\x0F\x1FV[a\x06\xB0V[a\x01\x0Fa\x01\x8F6`\x04a\x0F\x1FV[a\x07\x07V[a\x01\x0Fa\x01\xA26`\x04a\x12QV[a\x07\xB1V[a\0\xE7a\x01\xB56`\x04a\x0F\xD1V[a\x08\x8DV[a\0\xE7a\x01\xC86`\x04a\x12\x9EV[a\x08\xE4V[`\0T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x19V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x13W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02`W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7Fhv\xFA>\xCC}\x82\x1F!]\x82\x12B\xCB\xBE\x1F\x0E0\xA0\n\x85\xC2\"\xD6\x92\xA7\x96\x8F\xD3\xAF\xF1\x0B\x90`\0\x90\xA1PV[`\0\x80`@Q\x80`\xC0\x01`@R\x80\x84` \x01Q\x81R` \x01\x84`\x80\x01Q\x81R` \x01\x84`\xC0\x01Q\x81R` \x01\x84`\xA0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xE0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01`\0\x81RP\x90P`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xA2\n1t\x82`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x03$\x91\x90a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x03>W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x03RW=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t;V[\x93\x92PPPV[`\0[\x81``\x01Q\x81\x10\x15a\x05\x06W`\0`@Q\x80`\xC0\x01`@R\x80\x84`\0\x01Q\x81R` \x01\x84` \x01Q`@Q` \x01a\x03\xB9\x91\x90``\x91\x90\x91\x1Bk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16\x81R`\x14\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x81R` \x01`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xF47\xBCY`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\x1EW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04F\x91\x90\x81\x01\x90a\x14\xDDV[`@Q` \x01a\x04V\x91\x90a\x15SV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R\x85\x81\x01Q`\x01`\x01`@\x1B\x03\x16` \x83\x01R`\0\x82\x82\x01\x81\x90R`\x80\x87\x01Q``\x90\x93\x01\x92\x90\x92R\x90T\x90Qc(\x82\x8C]`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xA2\n1t\x90a\x04\xC0\x90\x84\x90`\x04\x01a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xDAW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xEEW=`\0\x80>=`\0\xFD[PPPPP\x80\x80a\x04\xFE\x90a\x15\x86V[\x91PPa\x03iV[PPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x055W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\x83\xE6 %\xE4\xBCXu\x16\xD0\xBC1^2\x9E\xAC\x0Cf6(T\xFE\xB7\xCDA5\xEF\x81C\xBA\x15\xF9\x90`\0\x90\xA1PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x05\x8CW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\xFB\x08{?\xFB\xBB\x0F\xC9\"\xDC\xCF\x87%\x08g\x1Av\x05\x85\x94#\xEB\x90\xEB\x01LV\xFD\xBA\x14\x84\xDC\x81`\xC0\x01Q`@Qa\x05\xBF\x91\x90a\x15\xADV[`@Q\x80\x91\x03\x90\xA1PV[`\0`@Q\x80`\xC0\x01`@R\x80a\x05\xE0\x84a\tTV[\x81R`@\x80Q\x80\x82\x01\x82R`\x08\x81Rg\x1A\\\xDB\\\x0BX\\\xDD`\xC2\x1B` \x82\x81\x01\x91\x90\x91R\x80\x84\x01\x91\x90\x91R\x81Q\x80\x83\x01\x83R`\x0E\x81Rmhello from evm`\x90\x1B\x91\x81\x01\x91\x90\x91R\x81\x83\x01R`\0``\x83\x01\x81\x90R`\x80\x83\x01\x81\x90R`\xA0\x90\x92\x01\x82\x90R\x90T\x90Qc(\x82\x8C]`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xA2\n1t\x90a\x06z\x90\x84\x90`\x04\x01a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x06\x94W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x06\xA8W=`\0\x80>=`\0\xFD[PPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x06\xDBW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xD7\xDC\x99\xAF\xB6\xC309\xCE\xA4PZ\x9E,\xAB4q\xD3Y\xCE\xBE\x02\x1E\xC1'\xDC\x94\xDD\xD3Y\xD3\xC5\x90`\0\x90\xA1PV[`@\x80Q`\xA0\x81\x01\x82R\x82Q\x81R` \x80\x84\x01Q\x90\x82\x01R\x82\x82\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16\x82\x84\x01R``\x80\x85\x01Q\x90\x91\x16\x90\x82\x01R`\0`\x80\x82\x01\x81\x90R\x80T\x92Qc\x82]'\xC5`\xE0\x1B\x81R\x90\x92`\x01`\x01`\xA0\x1B\x03\x16\x90c\x82]'\xC5\x90a\x07v\x90\x84\x90`\x04\x01a\x15\xC0V[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x07\x90W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x07\xA4W=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t\x85V[`\0\x80`@Q\x80`\xC0\x01`@R\x80\x84` \x01Q\x81R` \x01\x84`\xC0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xA0\x01Q\x81R` \x01\x84`\x80\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xE0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01`\0\x81RP\x90P`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xC9OH\xF3\x82`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x08R\x91\x90a\x17\x1EV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x08lW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x08\x80W=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t\xF0V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xB8W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xBB\xF4\x8AR\xB8>\xBC=\x9E9\xF0\x92\xA8\xB9\xB7\xE5o\x1D\xD0\xDCC\x8B\xEF@\xDC}\x92\x99Bp\xA5\x9F\x90`\0\x90\xA1PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\t\x0FW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xCB\xCB\xCAfM\xFE\xB9$\xCC\xD8P\xA0\x08h\x13\x0B\xFB\x1D\xF1W\t\x9A\x06\xF9)h\"\xCB{\xC3\xAD\x01\x90`\0\x90\xA1PV[`\0a\tF\x82a\n\xC2V[\x80Q\x90` \x01 \x90P\x91\x90PV[``a\t_\x82a\x0B\x03V[`@Q` \x01a\to\x91\x90a\x17\xEEV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0a\t\x94\x82`\0\x01Qa\n\xC2V[\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xB5\x93\x92\x91\x90a\x18\x1DV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra\t\xD3\x92\x91` \x01a\x18YV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`@\x80Q` \x81\x01\x90\x91R`\0\x80\x82R`\xA0\x83\x01QQ\x90\x91\x90\x82[\x81\x81\x10\x15a\ngW\x82\x85`\xA0\x01Q\x82\x81Q\x81\x10a\n*Wa\n*a\x18\x88V[` \x02` \x01\x01Q`@Q` \x01a\nC\x92\x91\x90a\x18YV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x92P\x80\x80a\n_\x90a\x15\x86V[\x91PPa\n\x0BV[P\x83`\0\x01Q\x84` \x01Q\x85`@\x01Q\x86`\xC0\x01Q\x87`\x80\x01Q\x88``\x01Q\x87\x8A`\xE0\x01Q`@Q` \x01a\n\xA3\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x18\x9EV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x92PPP\x91\x90PV[``\x81`\0\x01Q\x82` \x01Q\x83`@\x01Q\x84`\xA0\x01Q\x85``\x01Q\x86`\x80\x01Q\x87`\xC0\x01Q\x88`\xE0\x01Q`@Q` \x01a\to\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x19:V[```\0a\x0B\x10\x83a\x0B\x95V[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B/Wa\x0B/a\x0C\xA5V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x0BYW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x0BcWP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x0B\xD4Wr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x0C\0Wm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x0C\x1EWf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x0C6Wc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x0CJWa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x0C\\W`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x0ChW`\x01\x01[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x0C\x85W`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x0C\x9CW`\0\x80\xFD[a\x03_\x82a\x0CnV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\rrWa\rra\x0C\xA5V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\x93Wa\r\x93a\x0C\xA5V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\r\xB2W`\0\x80\xFD[\x815a\r\xC5a\r\xC0\x82a\rzV[a\rJV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\r\xDAW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x0C\x85W`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x0E!W`\0\x80\xFD[a\x0E)a\x0C\xBBV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0EBW`\0\x80\xFD[a\x0EN\x85\x83\x86\x01a\r\xA1V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0EdW`\0\x80\xFD[a\x0Ep\x85\x83\x86\x01a\r\xA1V[` \x84\x01Ra\x0E\x81`@\x85\x01a\r\xF7V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0E\x9AW`\0\x80\xFD[a\x0E\xA6\x85\x83\x86\x01a\r\xA1V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x0E\xBFW`\0\x80\xFD[a\x0E\xCB\x85\x83\x86\x01a\r\xA1V[`\x80\x84\x01Ra\x0E\xDC`\xA0\x85\x01a\r\xF7V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x0E\xF5W`\0\x80\xFD[Pa\x0F\x02\x84\x82\x85\x01a\r\xA1V[`\xC0\x83\x01RPa\x0F\x14`\xE0\x83\x01a\r\xF7V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0F1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0FHW`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\x0F\\W`\0\x80\xFD[a\x0Fda\x0C\xE4V[\x825\x82\x81\x11\x15a\x0FsW`\0\x80\xFD[a\x0F\x7F\x87\x82\x86\x01a\x0E\x0EV[\x82RP` \x83\x015\x82\x81\x11\x15a\x0F\x94W`\0\x80\xFD[a\x0F\xA0\x87\x82\x86\x01a\r\xA1V[` \x83\x01RPa\x0F\xB2`@\x84\x01a\r\xF7V[`@\x82\x01Ra\x0F\xC3``\x84\x01a\r\xF7V[``\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\x0F\xE3W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\xF9W`\0\x80\xFD[a\x10\x05\x84\x82\x85\x01a\x0E\x0EV[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x10\x1FW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x106W`\0\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\x10JW`\0\x80\xFD[a\x10Ra\r\x06V[\x825\x82\x81\x11\x15a\x10aW`\0\x80\xFD[a\x10m\x87\x82\x86\x01a\r\xA1V[\x82RPa\x10|` \x84\x01a\x0CnV[` \x82\x01Ra\x10\x8D`@\x84\x01a\r\xF7V[`@\x82\x01R``\x83\x015``\x82\x01R`\x80\x83\x015`\x80\x82\x01R\x80\x93PPPP\x92\x91PPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x10\xCBWa\x10\xCBa\x0C\xA5V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x10\xE6W`\0\x80\xFD[\x815` a\x10\xF6a\r\xC0\x83a\x10\xB2V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x15W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x11TW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x118W`\0\x80\x81\xFD[a\x11F\x89\x86\x83\x8B\x01\x01a\r\xA1V[\x84RP\x91\x83\x01\x91\x83\x01a\x11\x19V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x11rW`\0\x80\xFD[a\x11za\x0C\xBBV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\x93W`\0\x80\xFD[a\x11\x9F\x85\x83\x86\x01a\r\xA1V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x11\xB5W`\0\x80\xFD[a\x11\xC1\x85\x83\x86\x01a\r\xA1V[` \x84\x01Ra\x11\xD2`@\x85\x01a\r\xF7V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x11\xEBW`\0\x80\xFD[a\x11\xF7\x85\x83\x86\x01a\r\xA1V[``\x84\x01Ra\x12\x08`\x80\x85\x01a\r\xF7V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x12!W`\0\x80\xFD[Pa\x12.\x84\x82\x85\x01a\x10\xD5V[`\xA0\x83\x01RPa\x12@`\xC0\x83\x01a\r\xF7V[`\xC0\x82\x01Ra\x0F\x14`\xE0\x83\x01a\r\xF7V[`\0` \x82\x84\x03\x12\x15a\x12cW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12yW`\0\x80\xFD[a\x10\x05\x84\x82\x85\x01a\x11_V[`\0` \x82\x84\x03\x12\x15a\x12\x97W`\0\x80\xFD[P5\x91\x90PV[`\0` \x80\x83\x85\x03\x12\x15a\x12\xB1W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x12\xC8W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x12\xDEW`\0\x80\xFD[a\x12\xE6a\r(V[\x835\x83\x81\x11\x15a\x12\xF5W`\0\x80\xFD[a\x13\x01\x89\x82\x87\x01a\x11_V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x13\x15W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x13*W`\0\x80\xFD[\x835a\x138a\r\xC0\x82a\x10\xB2V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x13WW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x13\xE7W\x805\x87\x81\x11\x15a\x13sW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x13\x89W`\0\x80\x81\xFD[a\x13\x91a\r(V[\x8A\x82\x015\x89\x81\x11\x15a\x13\xA3W`\0\x80\x81\xFD[a\x13\xB1\x8F\x8D\x83\x86\x01\x01a\r\xA1V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x13\xC6W`\0\x80\x81\xFD[a\x13\xD4\x8F\x8D\x83\x86\x01\x01a\r\xA1V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x13[V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0[\x83\x81\x10\x15a\x14\x16W\x81\x81\x01Q\x83\x82\x01R` \x01a\x13\xFEV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x147\x81` \x86\x01` \x86\x01a\x13\xFBV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R`\0\x82Q`\xC0` \x84\x01Ra\x14g`\xE0\x84\x01\x82a\x14\x1FV[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x14\x85\x83\x83a\x14\x1FV[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x14\xA3\x82\x82a\x14\x1FV[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP`\xA0\x84\x01Q`\xC0\x84\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x14\xEFW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\x05W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x15\x16W`\0\x80\xFD[\x80Qa\x15$a\r\xC0\x82a\rzV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x159W`\0\x80\xFD[a\x15J\x82` \x83\x01` \x86\x01a\x13\xFBV[\x95\x94PPPPPV[j\x03C+ccy\x033\x93{i`\xAD\x1B\x81R`\0\x82Qa\x15y\x81`\x0B\x85\x01` \x87\x01a\x13\xFBV[\x91\x90\x91\x01`\x0B\x01\x92\x91PPV[`\0`\x01\x82\x01a\x15\xA6WcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[P`\x01\x01\x90V[` \x81R`\0a\x03_` \x83\x01\x84a\x14\x1FV[` \x81R`\0\x82Q`\xA0` \x84\x01R\x80Qa\x01\0\x80`\xC0\x86\x01Ra\x15\xE8a\x01\xC0\x86\x01\x83a\x14\x1FV[\x91P` \x83\x01Q`\xBF\x19\x80\x87\x85\x03\x01`\xE0\x88\x01Ra\x16\x06\x84\x83a\x14\x1FV[\x93P`@\x85\x01Q\x91Pa\x16#\x83\x88\x01\x83`\x01`\x01`@\x1B\x03\x16\x90RV[``\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01 \x88\x01Ra\x16?\x84\x84a\x14\x1FV[\x93P`\x80\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01@\x88\x01Ra\x16]\x84\x84a\x14\x1FV[\x93P`\xA0\x85\x01Q\x92Pa\x16|a\x01`\x88\x01\x84`\x01`\x01`@\x1B\x03\x16\x90RV[`\xC0\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01\x80\x88\x01RPPa\x16\x9A\x82\x82a\x14\x1FV[\x91PP`\xE0\x82\x01Q\x91Pa\x16\xBAa\x01\xA0\x85\x01\x83`\x01`\x01`@\x1B\x03\x16\x90RV[` \x85\x01Q\x84\x82\x03`\x1F\x19\x01`@\x86\x01R\x91Pa\x16\xD7\x81\x83a\x14\x1FV[\x91PP`@\x84\x01Qa\x16\xF4``\x85\x01\x82`\x01`\x01`@\x1B\x03\x16\x90RV[P``\x84\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\x80\x85\x01RP`\x80\x84\x01Q`\xA0\x84\x01R\x80\x91PP\x92\x91PPV[`\0` \x80\x83R\x83Q`\xC0\x82\x85\x01Ra\x17:`\xE0\x85\x01\x82a\x14\x1FV[\x90P`\x01`\x01`@\x1B\x03\x82\x86\x01Q\x16`@\x85\x01R`@\x85\x01Q`\x1F\x19\x80\x86\x84\x03\x01``\x87\x01R\x82\x82Q\x80\x85R\x85\x85\x01\x91P\x85\x81`\x05\x1B\x86\x01\x01\x86\x85\x01\x94P`\0[\x82\x81\x10\x15a\x17\xA7W\x84\x87\x83\x03\x01\x84Ra\x17\x95\x82\x87Qa\x14\x1FV[\x95\x88\x01\x95\x93\x88\x01\x93\x91P`\x01\x01a\x17{V[P``\x8A\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\x80\x8B\x01R\x96P`\x80\x8A\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\xA0\x8B\x01R\x96P`\xA0\x8A\x01Q`\xC0\x8A\x01R\x80\x97PPPPPPPP\x92\x91PPV[fKUSAMA-`\xC8\x1B\x81R`\0\x82Qa\x18\x10\x81`\x07\x85\x01` \x87\x01a\x13\xFBV[\x91\x90\x91\x01`\x07\x01\x92\x91PPV[`\0\x84Qa\x18/\x81\x84` \x89\x01a\x13\xFBV[`\x01`\x01`\xC0\x1B\x03\x19`\xC0\x95\x86\x1B\x81\x16\x91\x90\x93\x01\x90\x81R\x92\x90\x93\x1B\x16`\x08\x82\x01R`\x10\x01\x92\x91PPV[`\0\x83Qa\x18k\x81\x84` \x88\x01a\x13\xFBV[\x83Q\x90\x83\x01\x90a\x18\x7F\x81\x83` \x88\x01a\x13\xFBV[\x01\x94\x93PPPPV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[`\0\x89Qa\x18\xB0\x81\x84` \x8E\x01a\x13\xFBV[\x89Q\x90\x83\x01\x90a\x18\xC4\x81\x83` \x8E\x01a\x13\xFBV[`\xC0\x8A\x81\x1B`\x01`\x01`\xC0\x1B\x03\x19\x90\x81\x16\x93\x90\x92\x01\x92\x83R\x89\x81\x1B\x82\x16`\x08\x84\x01R\x88\x90\x1B\x81\x16`\x10\x83\x01R\x86Qa\x19\x03\x81`\x18\x85\x01` \x8B\x01a\x13\xFBV[\x86Q\x92\x01\x91a\x19\x19\x81`\x18\x85\x01` \x8A\x01a\x13\xFBV[`\xC0\x95\x90\x95\x1B\x16\x93\x01`\x18\x81\x01\x93\x90\x93RPP` \x01\x97\x96PPPPPPPV[`\0\x89Qa\x19L\x81\x84` \x8E\x01a\x13\xFBV[\x89Q\x90\x83\x01\x90a\x19`\x81\x83` \x8E\x01a\x13\xFBV[`\xC0\x8A\x81\x1B`\x01`\x01`\xC0\x1B\x03\x19\x90\x81\x16\x93\x90\x92\x01\x92\x83R\x89\x90\x1B\x81\x16`\x08\x83\x01R\x87Qa\x19\x95\x81`\x10\x85\x01` \x8C\x01a\x13\xFBV[\x87Q\x92\x01\x91a\x19\xAB\x81`\x10\x85\x01` \x8B\x01a\x13\xFBV[\x86Q\x92\x01\x91a\x19\xC1\x81`\x10\x85\x01` \x8A\x01a\x13\xFBV[`\xC0\x95\x90\x95\x1B\x16\x93\x01`\x10\x81\x01\x93\x90\x93RPP`\x18\x01\x97\x96PPPPPPPV\xFE\xA2dipfsX\"\x12 \x87e\x06\xD5\x16p\x18\r?^1\xD6\xA6&\xB7\x05\x90\xBC\x82\xD1\xD6\x0B\xFC\x07\x83\xB8\xF3x(\xD9\xB8\xC8dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static PINGMODULE_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xCFW`\x005`\xE0\x1C\x80cr5N\x9B\x11a\0\x8CW\x80c\xD1\xABF\xCF\x11a\0fW\x80c\xD1\xABF\xCF\x14a\x01\x94W\x80c\xD6;\xCF\x18\x14a\x01\xA7W\x80c\xF3p\xFD\xBB\x14a\x01\xBAW\x80c\xF47\xBCY\x14a\x01\xCDW`\0\x80\xFD[\x80cr5N\x9B\x14a\x01[W\x80c\xAF\xB7`\xAC\x14a\x01nW\x80c\xCAKh\xFC\x14a\x01\x81W`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\xD4W\x80c\x12\xB2RO\x14a\0\xE9W\x80c1&}\xEE\x14a\0\xFCW\x80cJi.\x06\x14a\x01\"W\x80cLF\xC05\x14a\x015W\x80cN\x87\xBA\x19\x14a\x01HW[`\0\x80\xFD[a\0\xE7a\0\xE26`\x04a\x0C\x8AV[a\x01\xE8V[\0[a\0\xE7a\0\xF76`\x04a\x0F\x1FV[a\x025V[a\x01\x0Fa\x01\n6`\x04a\x0F\xD1V[a\x02\x8CV[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xE7a\x0106`\x04a\x10\rV[a\x03fV[a\0\xE7a\x01C6`\x04a\x12QV[a\x05\nV[a\0\xE7a\x01V6`\x04a\x0F\xD1V[a\x05aV[a\0\xE7a\x01i6`\x04a\x12\x85V[a\x05\xCAV[a\0\xE7a\x01|6`\x04a\x0F\x1FV[a\x06\xB0V[a\x01\x0Fa\x01\x8F6`\x04a\x0F\x1FV[a\x07\x07V[a\x01\x0Fa\x01\xA26`\x04a\x12QV[a\x07\xB1V[a\0\xE7a\x01\xB56`\x04a\x0F\xD1V[a\x08\x8DV[a\0\xE7a\x01\xC86`\x04a\x12\x9EV[a\x08\xE4V[`\0T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x19V[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x13W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02`W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7Fhv\xFA>\xCC}\x82\x1F!]\x82\x12B\xCB\xBE\x1F\x0E0\xA0\n\x85\xC2\"\xD6\x92\xA7\x96\x8F\xD3\xAF\xF1\x0B\x90`\0\x90\xA1PV[`\0\x80`@Q\x80`\xC0\x01`@R\x80\x84` \x01Q\x81R` \x01\x84`\x80\x01Q\x81R` \x01\x84`\xC0\x01Q\x81R` \x01\x84`\xA0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xE0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01`\0\x81RP\x90P`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xA2\n1t\x82`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x03$\x91\x90a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x03>W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x03RW=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t;V[\x93\x92PPPV[`\0[\x81``\x01Q\x81\x10\x15a\x05\x06W`\0`@Q\x80`\xC0\x01`@R\x80\x84`\0\x01Q\x81R` \x01\x84` \x01Q`@Q` \x01a\x03\xB9\x91\x90``\x91\x90\x91\x1Bk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16\x81R`\x14\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x81R` \x01`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xF47\xBCY`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01`\0`@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\x1EW=`\0\x80>=`\0\xFD[PPPP`@Q=`\0\x82>`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01`@Ra\x04F\x91\x90\x81\x01\x90a\x14\xDDV[`@Q` \x01a\x04V\x91\x90a\x15SV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R\x85\x81\x01Q`\x01`\x01`@\x1B\x03\x16` \x83\x01R`\0\x82\x82\x01\x81\x90R`\x80\x87\x01Q``\x90\x93\x01\x92\x90\x92R\x90T\x90Qc(\x82\x8C]`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xA2\n1t\x90a\x04\xC0\x90\x84\x90`\x04\x01a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xDAW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xEEW=`\0\x80>=`\0\xFD[PPPPP\x80\x80a\x04\xFE\x90a\x15\x86V[\x91PPa\x03iV[PPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x055W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\x83\xE6 %\xE4\xBCXu\x16\xD0\xBC1^2\x9E\xAC\x0Cf6(T\xFE\xB7\xCDA5\xEF\x81C\xBA\x15\xF9\x90`\0\x90\xA1PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x05\x8CW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\xFB\x08{?\xFB\xBB\x0F\xC9\"\xDC\xCF\x87%\x08g\x1Av\x05\x85\x94#\xEB\x90\xEB\x01LV\xFD\xBA\x14\x84\xDC\x81`\xC0\x01Q`@Qa\x05\xBF\x91\x90a\x15\xADV[`@Q\x80\x91\x03\x90\xA1PV[`\0`@Q\x80`\xC0\x01`@R\x80a\x05\xE0\x84a\tTV[\x81R`@\x80Q\x80\x82\x01\x82R`\x08\x81Rg\x1A\\\xDB\\\x0BX\\\xDD`\xC2\x1B` \x82\x81\x01\x91\x90\x91R\x80\x84\x01\x91\x90\x91R\x81Q\x80\x83\x01\x83R`\x0E\x81Rmhello from evm`\x90\x1B\x91\x81\x01\x91\x90\x91R\x81\x83\x01R`\0``\x83\x01\x81\x90R`\x80\x83\x01\x81\x90R`\xA0\x90\x92\x01\x82\x90R\x90T\x90Qc(\x82\x8C]`\xE2\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xA2\n1t\x90a\x06z\x90\x84\x90`\x04\x01a\x14KV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x06\x94W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x06\xA8W=`\0\x80>=`\0\xFD[PPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x06\xDBW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xD7\xDC\x99\xAF\xB6\xC309\xCE\xA4PZ\x9E,\xAB4q\xD3Y\xCE\xBE\x02\x1E\xC1'\xDC\x94\xDD\xD3Y\xD3\xC5\x90`\0\x90\xA1PV[`@\x80Q`\xA0\x81\x01\x82R\x82Q\x81R` \x80\x84\x01Q\x90\x82\x01R\x82\x82\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16\x82\x84\x01R``\x80\x85\x01Q\x90\x91\x16\x90\x82\x01R`\0`\x80\x82\x01\x81\x90R\x80T\x92Qc\x82]'\xC5`\xE0\x1B\x81R\x90\x92`\x01`\x01`\xA0\x1B\x03\x16\x90c\x82]'\xC5\x90a\x07v\x90\x84\x90`\x04\x01a\x15\xC0V[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x07\x90W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x07\xA4W=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t\x85V[`\0\x80`@Q\x80`\xC0\x01`@R\x80\x84` \x01Q\x81R` \x01\x84`\xC0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xA0\x01Q\x81R` \x01\x84`\x80\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01\x84`\xE0\x01Q`\x01`\x01`@\x1B\x03\x16\x81R` \x01`\0\x81RP\x90P`\0\x80T\x90a\x01\0\n\x90\x04`\x01`\x01`\xA0\x1B\x03\x16`\x01`\x01`\xA0\x1B\x03\x16c\xC9OH\xF3\x82`@Q\x82c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x08R\x91\x90a\x17\x1EV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x08lW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x08\x80W=`\0\x80>=`\0\xFD[PPPPa\x03_\x83a\t\xF0V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xB8W`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xBB\xF4\x8AR\xB8>\xBC=\x9E9\xF0\x92\xA8\xB9\xB7\xE5o\x1D\xD0\xDCC\x8B\xEF@\xDC}\x92\x99Bp\xA5\x9F\x90`\0\x90\xA1PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\t\x0FW`@QcQ\xAB\x8D\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x7F\xCB\xCB\xCAfM\xFE\xB9$\xCC\xD8P\xA0\x08h\x13\x0B\xFB\x1D\xF1W\t\x9A\x06\xF9)h\"\xCB{\xC3\xAD\x01\x90`\0\x90\xA1PV[`\0a\tF\x82a\n\xC2V[\x80Q\x90` \x01 \x90P\x91\x90PV[``a\t_\x82a\x0B\x03V[`@Q` \x01a\to\x91\x90a\x17\xEEV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0a\t\x94\x82`\0\x01Qa\n\xC2V[\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xB5\x93\x92\x91\x90a\x18\x1DV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra\t\xD3\x92\x91` \x01a\x18YV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`@\x80Q` \x81\x01\x90\x91R`\0\x80\x82R`\xA0\x83\x01QQ\x90\x91\x90\x82[\x81\x81\x10\x15a\ngW\x82\x85`\xA0\x01Q\x82\x81Q\x81\x10a\n*Wa\n*a\x18\x88V[` \x02` \x01\x01Q`@Q` \x01a\nC\x92\x91\x90a\x18YV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x92P\x80\x80a\n_\x90a\x15\x86V[\x91PPa\n\x0BV[P\x83`\0\x01Q\x84` \x01Q\x85`@\x01Q\x86`\xC0\x01Q\x87`\x80\x01Q\x88``\x01Q\x87\x8A`\xE0\x01Q`@Q` \x01a\n\xA3\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x18\x9EV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x92PPP\x91\x90PV[``\x81`\0\x01Q\x82` \x01Q\x83`@\x01Q\x84`\xA0\x01Q\x85``\x01Q\x86`\x80\x01Q\x87`\xC0\x01Q\x88`\xE0\x01Q`@Q` \x01a\to\x98\x97\x96\x95\x94\x93\x92\x91\x90a\x19:V[```\0a\x0B\x10\x83a\x0B\x95V[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B/Wa\x0B/a\x0C\xA5V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x0BYW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x0BcWP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x0B\xD4Wr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x0C\0Wm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x0C\x1EWf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x0C6Wc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x0CJWa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x0C\\W`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x0ChW`\x01\x01[\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x0C\x85W`\0\x80\xFD[\x91\x90PV[`\0` \x82\x84\x03\x12\x15a\x0C\x9CW`\0\x80\xFD[a\x03_\x82a\x0CnV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x0C\xDEWa\x0C\xDEa\x0C\xA5V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\rrWa\rra\x0C\xA5V[`@R\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\r\x93Wa\r\x93a\x0C\xA5V[P`\x1F\x01`\x1F\x19\x16` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\r\xB2W`\0\x80\xFD[\x815a\r\xC5a\r\xC0\x82a\rzV[a\rJV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\r\xDAW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x0C\x85W`\0\x80\xFD[`\0a\x01\0\x82\x84\x03\x12\x15a\x0E!W`\0\x80\xFD[a\x0E)a\x0C\xBBV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0EBW`\0\x80\xFD[a\x0EN\x85\x83\x86\x01a\r\xA1V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0EdW`\0\x80\xFD[a\x0Ep\x85\x83\x86\x01a\r\xA1V[` \x84\x01Ra\x0E\x81`@\x85\x01a\r\xF7V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0E\x9AW`\0\x80\xFD[a\x0E\xA6\x85\x83\x86\x01a\r\xA1V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x0E\xBFW`\0\x80\xFD[a\x0E\xCB\x85\x83\x86\x01a\r\xA1V[`\x80\x84\x01Ra\x0E\xDC`\xA0\x85\x01a\r\xF7V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x0E\xF5W`\0\x80\xFD[Pa\x0F\x02\x84\x82\x85\x01a\r\xA1V[`\xC0\x83\x01RPa\x0F\x14`\xE0\x83\x01a\r\xF7V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0F1W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0FHW`\0\x80\xFD[\x90\x83\x01\x90`\x80\x82\x86\x03\x12\x15a\x0F\\W`\0\x80\xFD[a\x0Fda\x0C\xE4V[\x825\x82\x81\x11\x15a\x0FsW`\0\x80\xFD[a\x0F\x7F\x87\x82\x86\x01a\x0E\x0EV[\x82RP` \x83\x015\x82\x81\x11\x15a\x0F\x94W`\0\x80\xFD[a\x0F\xA0\x87\x82\x86\x01a\r\xA1V[` \x83\x01RPa\x0F\xB2`@\x84\x01a\r\xF7V[`@\x82\x01Ra\x0F\xC3``\x84\x01a\r\xF7V[``\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\x0F\xE3W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0F\xF9W`\0\x80\xFD[a\x10\x05\x84\x82\x85\x01a\x0E\x0EV[\x94\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\x10\x1FW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x106W`\0\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\x10JW`\0\x80\xFD[a\x10Ra\r\x06V[\x825\x82\x81\x11\x15a\x10aW`\0\x80\xFD[a\x10m\x87\x82\x86\x01a\r\xA1V[\x82RPa\x10|` \x84\x01a\x0CnV[` \x82\x01Ra\x10\x8D`@\x84\x01a\r\xF7V[`@\x82\x01R``\x83\x015``\x82\x01R`\x80\x83\x015`\x80\x82\x01R\x80\x93PPPP\x92\x91PPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x10\xCBWa\x10\xCBa\x0C\xA5V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x10\xE6W`\0\x80\xFD[\x815` a\x10\xF6a\r\xC0\x83a\x10\xB2V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x11\x15W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x11TW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x118W`\0\x80\x81\xFD[a\x11F\x89\x86\x83\x8B\x01\x01a\r\xA1V[\x84RP\x91\x83\x01\x91\x83\x01a\x11\x19V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x11rW`\0\x80\xFD[a\x11za\x0C\xBBV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x11\x93W`\0\x80\xFD[a\x11\x9F\x85\x83\x86\x01a\r\xA1V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x11\xB5W`\0\x80\xFD[a\x11\xC1\x85\x83\x86\x01a\r\xA1V[` \x84\x01Ra\x11\xD2`@\x85\x01a\r\xF7V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x11\xEBW`\0\x80\xFD[a\x11\xF7\x85\x83\x86\x01a\r\xA1V[``\x84\x01Ra\x12\x08`\x80\x85\x01a\r\xF7V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x12!W`\0\x80\xFD[Pa\x12.\x84\x82\x85\x01a\x10\xD5V[`\xA0\x83\x01RPa\x12@`\xC0\x83\x01a\r\xF7V[`\xC0\x82\x01Ra\x0F\x14`\xE0\x83\x01a\r\xF7V[`\0` \x82\x84\x03\x12\x15a\x12cW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x12yW`\0\x80\xFD[a\x10\x05\x84\x82\x85\x01a\x11_V[`\0` \x82\x84\x03\x12\x15a\x12\x97W`\0\x80\xFD[P5\x91\x90PV[`\0` \x80\x83\x85\x03\x12\x15a\x12\xB1W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x12\xC8W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x12\xDEW`\0\x80\xFD[a\x12\xE6a\r(V[\x835\x83\x81\x11\x15a\x12\xF5W`\0\x80\xFD[a\x13\x01\x89\x82\x87\x01a\x11_V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x13\x15W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x13*W`\0\x80\xFD[\x835a\x138a\r\xC0\x82a\x10\xB2V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x13WW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x13\xE7W\x805\x87\x81\x11\x15a\x13sW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x13\x89W`\0\x80\x81\xFD[a\x13\x91a\r(V[\x8A\x82\x015\x89\x81\x11\x15a\x13\xA3W`\0\x80\x81\xFD[a\x13\xB1\x8F\x8D\x83\x86\x01\x01a\r\xA1V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x13\xC6W`\0\x80\x81\xFD[a\x13\xD4\x8F\x8D\x83\x86\x01\x01a\r\xA1V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x13[V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0[\x83\x81\x10\x15a\x14\x16W\x81\x81\x01Q\x83\x82\x01R` \x01a\x13\xFEV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x147\x81` \x86\x01` \x86\x01a\x13\xFBV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R`\0\x82Q`\xC0` \x84\x01Ra\x14g`\xE0\x84\x01\x82a\x14\x1FV[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x14\x85\x83\x83a\x14\x1FV[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x14\xA3\x82\x82a\x14\x1FV[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP`\xA0\x84\x01Q`\xC0\x84\x01R\x80\x91PP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x14\xEFW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x15\x05W`\0\x80\xFD[\x82\x01`\x1F\x81\x01\x84\x13a\x15\x16W`\0\x80\xFD[\x80Qa\x15$a\r\xC0\x82a\rzV[\x81\x81R\x85` \x83\x85\x01\x01\x11\x15a\x159W`\0\x80\xFD[a\x15J\x82` \x83\x01` \x86\x01a\x13\xFBV[\x95\x94PPPPPV[j\x03C+ccy\x033\x93{i`\xAD\x1B\x81R`\0\x82Qa\x15y\x81`\x0B\x85\x01` \x87\x01a\x13\xFBV[\x91\x90\x91\x01`\x0B\x01\x92\x91PPV[`\0`\x01\x82\x01a\x15\xA6WcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[P`\x01\x01\x90V[` \x81R`\0a\x03_` \x83\x01\x84a\x14\x1FV[` \x81R`\0\x82Q`\xA0` \x84\x01R\x80Qa\x01\0\x80`\xC0\x86\x01Ra\x15\xE8a\x01\xC0\x86\x01\x83a\x14\x1FV[\x91P` \x83\x01Q`\xBF\x19\x80\x87\x85\x03\x01`\xE0\x88\x01Ra\x16\x06\x84\x83a\x14\x1FV[\x93P`@\x85\x01Q\x91Pa\x16#\x83\x88\x01\x83`\x01`\x01`@\x1B\x03\x16\x90RV[``\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01 \x88\x01Ra\x16?\x84\x84a\x14\x1FV[\x93P`\x80\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01@\x88\x01Ra\x16]\x84\x84a\x14\x1FV[\x93P`\xA0\x85\x01Q\x92Pa\x16|a\x01`\x88\x01\x84`\x01`\x01`@\x1B\x03\x16\x90RV[`\xC0\x85\x01Q\x92P\x80\x87\x85\x03\x01a\x01\x80\x88\x01RPPa\x16\x9A\x82\x82a\x14\x1FV[\x91PP`\xE0\x82\x01Q\x91Pa\x16\xBAa\x01\xA0\x85\x01\x83`\x01`\x01`@\x1B\x03\x16\x90RV[` \x85\x01Q\x84\x82\x03`\x1F\x19\x01`@\x86\x01R\x91Pa\x16\xD7\x81\x83a\x14\x1FV[\x91PP`@\x84\x01Qa\x16\xF4``\x85\x01\x82`\x01`\x01`@\x1B\x03\x16\x90RV[P``\x84\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\x80\x85\x01RP`\x80\x84\x01Q`\xA0\x84\x01R\x80\x91PP\x92\x91PPV[`\0` \x80\x83R\x83Q`\xC0\x82\x85\x01Ra\x17:`\xE0\x85\x01\x82a\x14\x1FV[\x90P`\x01`\x01`@\x1B\x03\x82\x86\x01Q\x16`@\x85\x01R`@\x85\x01Q`\x1F\x19\x80\x86\x84\x03\x01``\x87\x01R\x82\x82Q\x80\x85R\x85\x85\x01\x91P\x85\x81`\x05\x1B\x86\x01\x01\x86\x85\x01\x94P`\0[\x82\x81\x10\x15a\x17\xA7W\x84\x87\x83\x03\x01\x84Ra\x17\x95\x82\x87Qa\x14\x1FV[\x95\x88\x01\x95\x93\x88\x01\x93\x91P`\x01\x01a\x17{V[P``\x8A\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\x80\x8B\x01R\x96P`\x80\x8A\x01Q`\x01`\x01`@\x1B\x03\x81\x16`\xA0\x8B\x01R\x96P`\xA0\x8A\x01Q`\xC0\x8A\x01R\x80\x97PPPPPPPP\x92\x91PPV[fKUSAMA-`\xC8\x1B\x81R`\0\x82Qa\x18\x10\x81`\x07\x85\x01` \x87\x01a\x13\xFBV[\x91\x90\x91\x01`\x07\x01\x92\x91PPV[`\0\x84Qa\x18/\x81\x84` \x89\x01a\x13\xFBV[`\x01`\x01`\xC0\x1B\x03\x19`\xC0\x95\x86\x1B\x81\x16\x91\x90\x93\x01\x90\x81R\x92\x90\x93\x1B\x16`\x08\x82\x01R`\x10\x01\x92\x91PPV[`\0\x83Qa\x18k\x81\x84` \x88\x01a\x13\xFBV[\x83Q\x90\x83\x01\x90a\x18\x7F\x81\x83` \x88\x01a\x13\xFBV[\x01\x94\x93PPPPV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[`\0\x89Qa\x18\xB0\x81\x84` \x8E\x01a\x13\xFBV[\x89Q\x90\x83\x01\x90a\x18\xC4\x81\x83` \x8E\x01a\x13\xFBV[`\xC0\x8A\x81\x1B`\x01`\x01`\xC0\x1B\x03\x19\x90\x81\x16\x93\x90\x92\x01\x92\x83R\x89\x81\x1B\x82\x16`\x08\x84\x01R\x88\x90\x1B\x81\x16`\x10\x83\x01R\x86Qa\x19\x03\x81`\x18\x85\x01` \x8B\x01a\x13\xFBV[\x86Q\x92\x01\x91a\x19\x19\x81`\x18\x85\x01` \x8A\x01a\x13\xFBV[`\xC0\x95\x90\x95\x1B\x16\x93\x01`\x18\x81\x01\x93\x90\x93RPP` \x01\x97\x96PPPPPPPV[`\0\x89Qa\x19L\x81\x84` \x8E\x01a\x13\xFBV[\x89Q\x90\x83\x01\x90a\x19`\x81\x83` \x8E\x01a\x13\xFBV[`\xC0\x8A\x81\x1B`\x01`\x01`\xC0\x1B\x03\x19\x90\x81\x16\x93\x90\x92\x01\x92\x83R\x89\x90\x1B\x81\x16`\x08\x83\x01R\x87Qa\x19\x95\x81`\x10\x85\x01` \x8C\x01a\x13\xFBV[\x87Q\x92\x01\x91a\x19\xAB\x81`\x10\x85\x01` \x8B\x01a\x13\xFBV[\x86Q\x92\x01\x91a\x19\xC1\x81`\x10\x85\x01` \x8A\x01a\x13\xFBV[`\xC0\x95\x90\x95\x1B\x16\x93\x01`\x10\x81\x01\x93\x90\x93RPP`\x18\x01\x97\x96PPPPPPPV\xFE\xA2dipfsX\"\x12 \x87e\x06\xD5\x16p\x18\r?^1\xD6\xA6&\xB7\x05\x90\xBC\x82\xD1\xD6\x0B\xFC\x07\x83\xB8\xF3x(\xD9\xB8\xC8dsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static PINGMODULE_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
    pub struct PingModule<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for PingModule<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for PingModule<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for PingModule<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for PingModule<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(PingModule)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> PingModule<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    PINGMODULE_ABI.clone(),
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
                PINGMODULE_ABI.clone(),
                PINGMODULE_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        ///Calls the contract's `dispatch` (0x31267dee) function
        pub fn dispatch(
            &self,
            request: GetRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
            self.0
                .method_hash([49, 38, 125, 238], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatch` (0xd1ab46cf) function
        pub fn dispatch_with_request(
            &self,
            request: GetRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
            self.0
                .method_hash([209, 171, 70, 207], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchPostResponse` (0xca4b68fc) function
        pub fn dispatch_post_response(
            &self,
            response: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
            self.0
                .method_hash([202, 75, 104, 252], (response,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchToParachain` (0x72354e9b) function
        pub fn dispatch_to_parachain(
            &self,
            para_id: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([114, 53, 78, 155], para_id)
                .expect("method not found (this should never happen)")
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
        ///Calls the contract's `ping` (0x4a692e06) function
        pub fn ping(
            &self,
            ping_message: PingMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([74, 105, 46, 6], (ping_message,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setIsmpHost` (0x0e8324a2) function
        pub fn set_ismp_host(
            &self,
            host_addr: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([14, 131, 36, 162], host_addr)
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `GetResponseReceived` event
        pub fn get_response_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            GetResponseReceivedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `GetTimeoutReceived` event
        pub fn get_timeout_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            GetTimeoutReceivedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `MessageDispatched` event
        pub fn message_dispatched_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            MessageDispatchedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostReceived` event
        pub fn post_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostReceivedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostRequestTimeoutReceived` event
        pub fn post_request_timeout_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostRequestTimeoutReceivedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostResponseReceived` event
        pub fn post_response_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostResponseReceivedFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostResponseTimeoutReceived` event
        pub fn post_response_timeout_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostResponseTimeoutReceivedFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PingModuleEvents,
        > {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for PingModule<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `ExecutionFailed` with signature `ExecutionFailed()` and selector `0xacfdb444`
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
    #[etherror(name = "ExecutionFailed", abi = "ExecutionFailed()")]
    pub struct ExecutionFailed;
    ///Custom Error type `NotIsmpHost` with signature `NotIsmpHost()` and selector `0x51ab8de5`
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
    #[etherror(name = "NotIsmpHost", abi = "NotIsmpHost()")]
    pub struct NotIsmpHost;
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum PingModuleErrors {
        ExecutionFailed(ExecutionFailed),
        NotIsmpHost(NotIsmpHost),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for PingModuleErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) = <ExecutionFailed as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ExecutionFailed(decoded));
            }
            if let Ok(decoded) = <NotIsmpHost as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::NotIsmpHost(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for PingModuleErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::ExecutionFailed(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::NotIsmpHost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for PingModuleErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <ExecutionFailed as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NotIsmpHost as ::ethers::contract::EthError>::selector() => true,
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for PingModuleErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ExecutionFailed(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for PingModuleErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<ExecutionFailed> for PingModuleErrors {
        fn from(value: ExecutionFailed) -> Self {
            Self::ExecutionFailed(value)
        }
    }
    impl ::core::convert::From<NotIsmpHost> for PingModuleErrors {
        fn from(value: NotIsmpHost) -> Self {
            Self::NotIsmpHost(value)
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
        Hash
    )]
    #[ethevent(name = "GetResponseReceived", abi = "GetResponseReceived()")]
    pub struct GetResponseReceivedFilter;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "GetTimeoutReceived", abi = "GetTimeoutReceived()")]
    pub struct GetTimeoutReceivedFilter;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "MessageDispatched", abi = "MessageDispatched()")]
    pub struct MessageDispatchedFilter;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "PostReceived", abi = "PostReceived(string)")]
    pub struct PostReceivedFilter {
        pub message: ::std::string::String,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(
        name = "PostRequestTimeoutReceived",
        abi = "PostRequestTimeoutReceived()"
    )]
    pub struct PostRequestTimeoutReceivedFilter;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "PostResponseReceived", abi = "PostResponseReceived()")]
    pub struct PostResponseReceivedFilter;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(
        name = "PostResponseTimeoutReceived",
        abi = "PostResponseTimeoutReceived()"
    )]
    pub struct PostResponseTimeoutReceivedFilter;
    ///Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum PingModuleEvents {
        GetResponseReceivedFilter(GetResponseReceivedFilter),
        GetTimeoutReceivedFilter(GetTimeoutReceivedFilter),
        MessageDispatchedFilter(MessageDispatchedFilter),
        PostReceivedFilter(PostReceivedFilter),
        PostRequestTimeoutReceivedFilter(PostRequestTimeoutReceivedFilter),
        PostResponseReceivedFilter(PostResponseReceivedFilter),
        PostResponseTimeoutReceivedFilter(PostResponseTimeoutReceivedFilter),
    }
    impl ::ethers::contract::EthLogDecode for PingModuleEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = GetResponseReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::GetResponseReceivedFilter(decoded));
            }
            if let Ok(decoded) = GetTimeoutReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::GetTimeoutReceivedFilter(decoded));
            }
            if let Ok(decoded) = MessageDispatchedFilter::decode_log(log) {
                return Ok(PingModuleEvents::MessageDispatchedFilter(decoded));
            }
            if let Ok(decoded) = PostReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::PostReceivedFilter(decoded));
            }
            if let Ok(decoded) = PostRequestTimeoutReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::PostRequestTimeoutReceivedFilter(decoded));
            }
            if let Ok(decoded) = PostResponseReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::PostResponseReceivedFilter(decoded));
            }
            if let Ok(decoded) = PostResponseTimeoutReceivedFilter::decode_log(log) {
                return Ok(PingModuleEvents::PostResponseTimeoutReceivedFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for PingModuleEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::GetResponseReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::GetTimeoutReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::MessageDispatchedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostRequestTimeoutReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostResponseReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostResponseTimeoutReceivedFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
            }
        }
    }
    impl ::core::convert::From<GetResponseReceivedFilter> for PingModuleEvents {
        fn from(value: GetResponseReceivedFilter) -> Self {
            Self::GetResponseReceivedFilter(value)
        }
    }
    impl ::core::convert::From<GetTimeoutReceivedFilter> for PingModuleEvents {
        fn from(value: GetTimeoutReceivedFilter) -> Self {
            Self::GetTimeoutReceivedFilter(value)
        }
    }
    impl ::core::convert::From<MessageDispatchedFilter> for PingModuleEvents {
        fn from(value: MessageDispatchedFilter) -> Self {
            Self::MessageDispatchedFilter(value)
        }
    }
    impl ::core::convert::From<PostReceivedFilter> for PingModuleEvents {
        fn from(value: PostReceivedFilter) -> Self {
            Self::PostReceivedFilter(value)
        }
    }
    impl ::core::convert::From<PostRequestTimeoutReceivedFilter> for PingModuleEvents {
        fn from(value: PostRequestTimeoutReceivedFilter) -> Self {
            Self::PostRequestTimeoutReceivedFilter(value)
        }
    }
    impl ::core::convert::From<PostResponseReceivedFilter> for PingModuleEvents {
        fn from(value: PostResponseReceivedFilter) -> Self {
            Self::PostResponseReceivedFilter(value)
        }
    }
    impl ::core::convert::From<PostResponseTimeoutReceivedFilter> for PingModuleEvents {
        fn from(value: PostResponseTimeoutReceivedFilter) -> Self {
            Self::PostResponseTimeoutReceivedFilter(value)
        }
    }
    ///Container type for all input parameters for the `dispatch` function with signature `dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x31267dee`
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
        name = "dispatch",
        abi = "dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct DispatchCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `dispatch` function with signature `dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector `0xd1ab46cf`
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
        name = "dispatch",
        abi = "dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
    )]
    pub struct DispatchWithRequestCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `dispatchPostResponse` function with signature `dispatchPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))` and selector `0xca4b68fc`
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
        name = "dispatchPostResponse",
        abi = "dispatchPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
    )]
    pub struct DispatchPostResponseCall {
        pub response: PostResponse,
    }
    ///Container type for all input parameters for the `dispatchToParachain` function with signature `dispatchToParachain(uint256)` and selector `0x72354e9b`
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
    #[ethcall(name = "dispatchToParachain", abi = "dispatchToParachain(uint256)")]
    pub struct DispatchToParachainCall {
        pub para_id: ::ethers::core::types::U256,
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
    ///Container type for all input parameters for the `onAccept` function with signature `onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x4e87ba19`
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
        abi = "onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnAcceptCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature `onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))` and selector `0xf370fdbb`
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
        abi = "onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))"
    )]
    pub struct OnGetResponseCall(pub GetResponse);
    ///Container type for all input parameters for the `onGetTimeout` function with signature `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector `0x4c46c035`
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
        abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
    )]
    pub struct OnGetTimeoutCall(pub GetRequest);
    ///Container type for all input parameters for the `onPostRequestTimeout` function with signature `onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0xd63bcf18`
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
        abi = "onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnPostRequestTimeoutCall(pub PostRequest);
    ///Container type for all input parameters for the `onPostResponse` function with signature `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))` and selector `0xafb760ac`
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
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
    )]
    pub struct OnPostResponseCall(pub PostResponse);
    ///Container type for all input parameters for the `onPostResponseTimeout` function with signature `onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))` and selector `0x12b2524f`
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
        abi = "onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
    )]
    pub struct OnPostResponseTimeoutCall(pub PostResponse);
    ///Container type for all input parameters for the `ping` function with signature `ping((bytes,address,uint64,uint256,uint256))` and selector `0x4a692e06`
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
    #[ethcall(name = "ping", abi = "ping((bytes,address,uint64,uint256,uint256))")]
    pub struct PingCall {
        pub ping_message: PingMessage,
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
        pub host_addr: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum PingModuleCalls {
        Dispatch(DispatchCall),
        DispatchWithRequest(DispatchWithRequestCall),
        DispatchPostResponse(DispatchPostResponseCall),
        DispatchToParachain(DispatchToParachainCall),
        Host(HostCall),
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostRequestTimeout(OnPostRequestTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostResponseTimeout(OnPostResponseTimeoutCall),
        Ping(PingCall),
        SetIsmpHost(SetIsmpHostCall),
    }
    impl ::ethers::core::abi::AbiDecode for PingModuleCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <DispatchCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Dispatch(decoded));
            }
            if let Ok(decoded) = <DispatchWithRequestCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchWithRequest(decoded));
            }
            if let Ok(decoded) = <DispatchPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchPostResponse(decoded));
            }
            if let Ok(decoded) = <DispatchToParachainCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchToParachain(decoded));
            }
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
            if let Ok(decoded) = <PingCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Ping(decoded));
            }
            if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetIsmpHost(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for PingModuleCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::Dispatch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchWithRequest(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchPostResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchToParachain(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
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
                Self::Ping(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::SetIsmpHost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for PingModuleCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::Dispatch(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchWithRequest(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::DispatchPostResponse(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::DispatchToParachain(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
                Self::Ping(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<DispatchCall> for PingModuleCalls {
        fn from(value: DispatchCall) -> Self {
            Self::Dispatch(value)
        }
    }
    impl ::core::convert::From<DispatchWithRequestCall> for PingModuleCalls {
        fn from(value: DispatchWithRequestCall) -> Self {
            Self::DispatchWithRequest(value)
        }
    }
    impl ::core::convert::From<DispatchPostResponseCall> for PingModuleCalls {
        fn from(value: DispatchPostResponseCall) -> Self {
            Self::DispatchPostResponse(value)
        }
    }
    impl ::core::convert::From<DispatchToParachainCall> for PingModuleCalls {
        fn from(value: DispatchToParachainCall) -> Self {
            Self::DispatchToParachain(value)
        }
    }
    impl ::core::convert::From<HostCall> for PingModuleCalls {
        fn from(value: HostCall) -> Self {
            Self::Host(value)
        }
    }
    impl ::core::convert::From<OnAcceptCall> for PingModuleCalls {
        fn from(value: OnAcceptCall) -> Self {
            Self::OnAccept(value)
        }
    }
    impl ::core::convert::From<OnGetResponseCall> for PingModuleCalls {
        fn from(value: OnGetResponseCall) -> Self {
            Self::OnGetResponse(value)
        }
    }
    impl ::core::convert::From<OnGetTimeoutCall> for PingModuleCalls {
        fn from(value: OnGetTimeoutCall) -> Self {
            Self::OnGetTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostRequestTimeoutCall> for PingModuleCalls {
        fn from(value: OnPostRequestTimeoutCall) -> Self {
            Self::OnPostRequestTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostResponseCall> for PingModuleCalls {
        fn from(value: OnPostResponseCall) -> Self {
            Self::OnPostResponse(value)
        }
    }
    impl ::core::convert::From<OnPostResponseTimeoutCall> for PingModuleCalls {
        fn from(value: OnPostResponseTimeoutCall) -> Self {
            Self::OnPostResponseTimeout(value)
        }
    }
    impl ::core::convert::From<PingCall> for PingModuleCalls {
        fn from(value: PingCall) -> Self {
            Self::Ping(value)
        }
    }
    impl ::core::convert::From<SetIsmpHostCall> for PingModuleCalls {
        fn from(value: SetIsmpHostCall) -> Self {
            Self::SetIsmpHost(value)
        }
    }
    ///Container type for all return fields from the `dispatch` function with signature `dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x31267dee`
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
    pub struct DispatchReturn(pub [u8; 32]);
    ///Container type for all return fields from the `dispatch` function with signature `dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector `0xd1ab46cf`
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
    pub struct DispatchWithRequestReturn(pub [u8; 32]);
    ///Container type for all return fields from the `dispatchPostResponse` function with signature `dispatchPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))` and selector `0xca4b68fc`
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
    pub struct DispatchPostResponseReturn(pub [u8; 32]);
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
    pub struct HostReturn(pub ::ethers::core::types::Address);
    ///`PingMessage(bytes,address,uint64,uint256,uint256)`
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
    pub struct PingMessage {
        pub dest: ::ethers::core::types::Bytes,
        pub module: ::ethers::core::types::Address,
        pub timeout: u64,
        pub count: ::ethers::core::types::U256,
        pub fee: ::ethers::core::types::U256,
    }
}
