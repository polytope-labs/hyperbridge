pub use evm_host::*;
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
pub mod evm_host {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("admin"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("admin"),
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
                    ::std::borrow::ToOwned::to_owned("chainId"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("chainId"),
                            inputs: ::std::vec![],
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("challengePeriod"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("challengePeriod"),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("consensusClient"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("consensusClient"),
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
                    ::std::borrow::ToOwned::to_owned("consensusState"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("consensusState"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("consensusUpdateTime"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "consensusUpdateTime",
                            ),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("dai"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dai"),
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
                    ::std::borrow::ToOwned::to_owned("dispatch"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("post"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct DispatchPostResponse",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("post"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct DispatchPost"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("get"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct DispatchGet"),
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
                    ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("meta"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FeeMetadata"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("meta"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FeeMetadata"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("meta"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FeeMetadata"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchIncoming"),
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
                    ::std::borrow::ToOwned::to_owned("frozen"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("frozen"),
                            inputs: ::std::vec![],
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
                (
                    ::std::borrow::ToOwned::to_owned("host"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("host"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("hostParams"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("hostParams"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct HostParams"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("latestStateMachineHeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "latestStateMachineHeight",
                            ),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("perByteFee"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("perByteFee"),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("requestCommitments"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("requestCommitments"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FeeMetadata"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("requestReceipts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("requestReceipts"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
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
                    ::std::borrow::ToOwned::to_owned("responseCommitments"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "responseCommitments",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct FeeMetadata"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("responseReceipts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("responseReceipts"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct ResponseReceipt"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setConsensusState"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setConsensusState"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("state"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
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
                    ::std::borrow::ToOwned::to_owned("setFrozenState"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setFrozenState"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("newState"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bool"),
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
                    ::std::borrow::ToOwned::to_owned("setHostParams"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setHostParams"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("params"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct HostParams"),
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
                    ::std::borrow::ToOwned::to_owned("setHostParamsAdmin"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setHostParamsAdmin"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("params"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct HostParams"),
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
                    ::std::borrow::ToOwned::to_owned("stateMachineCommitment"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "stateMachineCommitment",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct StateMachineHeight",
                                        ),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct StateCommitment"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("stateMachineCommitmentUpdateTime"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "stateMachineCommitmentUpdateTime",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct StateMachineHeight",
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
                    ::std::borrow::ToOwned::to_owned("storeConsensusState"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "storeConsensusState",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("state"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
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
                    ::std::borrow::ToOwned::to_owned("storeConsensusUpdateTime"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "storeConsensusUpdateTime",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("time"),
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
                    ::std::borrow::ToOwned::to_owned("storeLatestStateMachineHeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "storeLatestStateMachineHeight",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
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
                    ::std::borrow::ToOwned::to_owned("storeStateMachineCommitment"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "storeStateMachineCommitment",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct StateMachineHeight",
                                        ),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct StateCommitment"),
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
                    ::std::borrow::ToOwned::to_owned(
                        "storeStateMachineCommitmentUpdateTime",
                    ),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "storeStateMachineCommitmentUpdateTime",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct StateMachineHeight",
                                        ),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("time"),
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
                    ::std::borrow::ToOwned::to_owned("timestamp"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("timestamp"),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("unStakingPeriod"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("unStakingPeriod"),
                            inputs: ::std::vec![],
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
                    ::std::borrow::ToOwned::to_owned("withdraw"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("withdraw"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("params"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct WithdrawParams"),
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
                    ::std::borrow::ToOwned::to_owned("GetRequestEvent"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("GetRequestEvent"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("source"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("keys"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ),
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nonce"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("gaslimit"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("fee"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("GetRequestHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("GetRequestHandled"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostRequestEvent"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostRequestEvent"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("source"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nonce"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("data"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("gaslimit"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("fee"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostRequestHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostRequestHandled"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostResponseEvent"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostResponseEvent"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("source"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nonce"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("timeoutTimestamp"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("data"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("gaslimit"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("resGaslimit"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "resTimeoutTimestamp",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("fee"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostResponseHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostResponseHandled",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
            ]),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static EVMHOST_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> = ::ethers::contract::Lazy::new(
        __abi,
    );
    pub struct EvmHost<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for EvmHost<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for EvmHost<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for EvmHost<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for EvmHost<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(EvmHost)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> EvmHost<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    EVMHOST_ABI.clone(),
                    client,
                ),
            )
        }
        ///Calls the contract's `admin` (0xf851a440) function
        pub fn admin(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([248, 81, 164, 64], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `chainId` (0x9a8a0592) function
        pub fn chain_id(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([154, 138, 5, 146], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `challengePeriod` (0xf3f480d9) function
        pub fn challenge_period(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([243, 244, 128, 217], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `consensusClient` (0x2476132b) function
        pub fn consensus_client(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([36, 118, 19, 43], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `consensusState` (0xbbad99d4) function
        pub fn consensus_state(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Bytes,
        > {
            self.0
                .method_hash([187, 173, 153, 212], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `consensusUpdateTime` (0x9a8425bc) function
        pub fn consensus_update_time(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([154, 132, 37, 188], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dai` (0xf4b9fa75) function
        pub fn dai(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([244, 185, 250, 117], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatch` (0x825d27c5) function
        pub fn dispatch(
            &self,
            post: DispatchPost,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([130, 93, 39, 197], (post,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatch` (0xa20a3174) function
        pub fn dispatch_with_post(
            &self,
            post: DispatchPost,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([162, 10, 49, 116], (post,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatch` (0xc94f48f3) function
        pub fn dispatch_with_get(
            &self,
            get: DispatchGet,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([201, 79, 72, 243], (get,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0x09cc21c3) function
        pub fn dispatch_incoming_3(
            &self,
            request: PostRequest,
            meta: FeeMetadata,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([9, 204, 33, 195], (request, meta, commitment))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0x1678619c) function
        pub fn dispatch_incoming_4(
            &self,
            request: PostRequest,
            meta: FeeMetadata,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([22, 120, 97, 156], (request, meta, commitment))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0x3b8c2bf7) function
        pub fn dispatch_incoming_0(
            &self,
            request: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([59, 140, 43, 247], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0x4f5b258a) function
        pub fn dispatch_incoming_1(
            &self,
            response: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([79, 91, 37, 138], (response,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0xe95c866c) function
        pub fn dispatch_incoming_5(
            &self,
            response: GetResponse,
            meta: FeeMetadata,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([233, 92, 134, 108], (response, meta, commitment))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `dispatchIncoming` (0xf0736091) function
        pub fn dispatch_incoming_2(
            &self,
            response: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([240, 115, 96, 145], (response,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `frozen` (0x054f7d9c) function
        pub fn frozen(&self) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([5, 79, 125, 156], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `host` (0xf437bc59) function
        pub fn host(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Bytes,
        > {
            self.0
                .method_hash([244, 55, 188, 89], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `hostParams` (0x2215364d) function
        pub fn host_params(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, HostParams> {
            self.0
                .method_hash([34, 21, 54, 77], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `latestStateMachineHeight` (0x56b65597) function
        pub fn latest_state_machine_height(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([86, 182, 85, 151], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `perByteFee` (0x641d729d) function
        pub fn per_byte_fee(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([100, 29, 114, 157], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `requestCommitments` (0x368bf464) function
        pub fn request_commitments(
            &self,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, FeeMetadata> {
            self.0
                .method_hash([54, 139, 244, 100], commitment)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `requestReceipts` (0x19667a3e) function
        pub fn request_receipts(
            &self,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([25, 102, 122, 62], commitment)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `responseCommitments` (0x2211f1dd) function
        pub fn response_commitments(
            &self,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, FeeMetadata> {
            self.0
                .method_hash([34, 17, 241, 221], commitment)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `responseReceipts` (0x8856337e) function
        pub fn response_receipts(
            &self,
            commitment: [u8; 32],
        ) -> ::ethers::contract::builders::ContractCall<M, ResponseReceipt> {
            self.0
                .method_hash([136, 86, 51, 126], commitment)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setConsensusState` (0xa15f7431) function
        pub fn set_consensus_state(
            &self,
            state: ::ethers::core::types::Bytes,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([161, 95, 116, 49], state)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setFrozenState` (0x19e8faf1) function
        pub fn set_frozen_state(
            &self,
            new_state: bool,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([25, 232, 250, 241], new_state)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setHostParams` (0x7772a61b) function
        pub fn set_host_params(
            &self,
            params: HostParams,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([119, 114, 166, 27], (params,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setHostParamsAdmin` (0x44febebd) function
        pub fn set_host_params_admin(
            &self,
            params: HostParams,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([68, 254, 190, 189], (params,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `stateMachineCommitment` (0xa70a8c47) function
        pub fn state_machine_commitment(
            &self,
            height: StateMachineHeight,
        ) -> ::ethers::contract::builders::ContractCall<M, StateCommitment> {
            self.0
                .method_hash([167, 10, 140, 71], (height,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `stateMachineCommitmentUpdateTime` (0x1a880a93) function
        pub fn state_machine_commitment_update_time(
            &self,
            height: StateMachineHeight,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([26, 136, 10, 147], (height,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `storeConsensusState` (0xb4974cf0) function
        pub fn store_consensus_state(
            &self,
            state: ::ethers::core::types::Bytes,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([180, 151, 76, 240], state)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `storeConsensusUpdateTime` (0xd860cb47) function
        pub fn store_consensus_update_time(
            &self,
            time: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([216, 96, 203, 71], time)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `storeLatestStateMachineHeight` (0xa0756ecd) function
        pub fn store_latest_state_machine_height(
            &self,
            height: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([160, 117, 110, 205], height)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `storeStateMachineCommitment` (0x559efe9e) function
        pub fn store_state_machine_commitment(
            &self,
            height: StateMachineHeight,
            commitment: StateCommitment,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([85, 158, 254, 158], (height, commitment))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `storeStateMachineCommitmentUpdateTime` (0x14863dcb) function
        pub fn store_state_machine_commitment_update_time(
            &self,
            height: StateMachineHeight,
            time: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([20, 134, 61, 203], (height, time))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `timestamp` (0xb80777ea) function
        pub fn timestamp(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([184, 7, 119, 234], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `unStakingPeriod` (0xd40784c7) function
        pub fn un_staking_period(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([212, 7, 132, 199], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `withdraw` (0x3c565417) function
        pub fn withdraw(
            &self,
            params: WithdrawParams,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([60, 86, 84, 23], (params,))
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `GetRequestEvent` event
        pub fn get_request_event_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            GetRequestEventFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `GetRequestHandled` event
        pub fn get_request_handled_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            GetRequestHandledFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostRequestEvent` event
        pub fn post_request_event_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostRequestEventFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostRequestHandled` event
        pub fn post_request_handled_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostRequestHandledFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostResponseEvent` event
        pub fn post_response_event_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostResponseEventFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `PostResponseHandled` event
        pub fn post_response_handled_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostResponseHandledFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, EvmHostEvents> {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for EvmHost<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
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
    #[ethevent(
        name = "GetRequestEvent",
        abi = "GetRequestEvent(bytes,bytes,bytes,bytes[],uint256,uint256,uint256,uint256,uint256)"
    )]
    pub struct GetRequestEventFilter {
        pub source: ::ethers::core::types::Bytes,
        pub dest: ::ethers::core::types::Bytes,
        pub from: ::ethers::core::types::Bytes,
        pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
        #[ethevent(indexed)]
        pub nonce: ::ethers::core::types::U256,
        pub height: ::ethers::core::types::U256,
        pub timeout_timestamp: ::ethers::core::types::U256,
        pub gaslimit: ::ethers::core::types::U256,
        pub fee: ::ethers::core::types::U256,
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
    #[ethevent(name = "GetRequestHandled", abi = "GetRequestHandled(bytes32,address)")]
    pub struct GetRequestHandledFilter {
        pub commitment: [u8; 32],
        pub relayer: ::ethers::core::types::Address,
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
        name = "PostRequestEvent",
        abi = "PostRequestEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,uint256,uint256)"
    )]
    pub struct PostRequestEventFilter {
        pub source: ::ethers::core::types::Bytes,
        pub dest: ::ethers::core::types::Bytes,
        pub from: ::ethers::core::types::Bytes,
        pub to: ::ethers::core::types::Bytes,
        #[ethevent(indexed)]
        pub nonce: ::ethers::core::types::U256,
        pub timeout_timestamp: ::ethers::core::types::U256,
        pub data: ::ethers::core::types::Bytes,
        pub gaslimit: ::ethers::core::types::U256,
        pub fee: ::ethers::core::types::U256,
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
    #[ethevent(name = "PostRequestHandled", abi = "PostRequestHandled(bytes32,address)")]
    pub struct PostRequestHandledFilter {
        pub commitment: [u8; 32],
        pub relayer: ::ethers::core::types::Address,
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
        name = "PostResponseEvent",
        abi = "PostResponseEvent(bytes,bytes,bytes,bytes,uint256,uint256,bytes,uint256,bytes,uint256,uint256,uint256)"
    )]
    pub struct PostResponseEventFilter {
        pub source: ::ethers::core::types::Bytes,
        pub dest: ::ethers::core::types::Bytes,
        pub from: ::ethers::core::types::Bytes,
        pub to: ::ethers::core::types::Bytes,
        #[ethevent(indexed)]
        pub nonce: ::ethers::core::types::U256,
        pub timeout_timestamp: ::ethers::core::types::U256,
        pub data: ::ethers::core::types::Bytes,
        pub gaslimit: ::ethers::core::types::U256,
        pub response: ::ethers::core::types::Bytes,
        pub res_gaslimit: ::ethers::core::types::U256,
        pub res_timeout_timestamp: ::ethers::core::types::U256,
        pub fee: ::ethers::core::types::U256,
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
        name = "PostResponseHandled",
        abi = "PostResponseHandled(bytes32,address)"
    )]
    pub struct PostResponseHandledFilter {
        pub commitment: [u8; 32],
        pub relayer: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum EvmHostEvents {
        GetRequestEventFilter(GetRequestEventFilter),
        GetRequestHandledFilter(GetRequestHandledFilter),
        PostRequestEventFilter(PostRequestEventFilter),
        PostRequestHandledFilter(PostRequestHandledFilter),
        PostResponseEventFilter(PostResponseEventFilter),
        PostResponseHandledFilter(PostResponseHandledFilter),
    }
    impl ::ethers::contract::EthLogDecode for EvmHostEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = GetRequestEventFilter::decode_log(log) {
                return Ok(EvmHostEvents::GetRequestEventFilter(decoded));
            }
            if let Ok(decoded) = GetRequestHandledFilter::decode_log(log) {
                return Ok(EvmHostEvents::GetRequestHandledFilter(decoded));
            }
            if let Ok(decoded) = PostRequestEventFilter::decode_log(log) {
                return Ok(EvmHostEvents::PostRequestEventFilter(decoded));
            }
            if let Ok(decoded) = PostRequestHandledFilter::decode_log(log) {
                return Ok(EvmHostEvents::PostRequestHandledFilter(decoded));
            }
            if let Ok(decoded) = PostResponseEventFilter::decode_log(log) {
                return Ok(EvmHostEvents::PostResponseEventFilter(decoded));
            }
            if let Ok(decoded) = PostResponseHandledFilter::decode_log(log) {
                return Ok(EvmHostEvents::PostResponseHandledFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for EvmHostEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::GetRequestEventFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::GetRequestHandledFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostRequestEventFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostRequestHandledFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostResponseEventFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PostResponseHandledFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
            }
        }
    }
    impl ::core::convert::From<GetRequestEventFilter> for EvmHostEvents {
        fn from(value: GetRequestEventFilter) -> Self {
            Self::GetRequestEventFilter(value)
        }
    }
    impl ::core::convert::From<GetRequestHandledFilter> for EvmHostEvents {
        fn from(value: GetRequestHandledFilter) -> Self {
            Self::GetRequestHandledFilter(value)
        }
    }
    impl ::core::convert::From<PostRequestEventFilter> for EvmHostEvents {
        fn from(value: PostRequestEventFilter) -> Self {
            Self::PostRequestEventFilter(value)
        }
    }
    impl ::core::convert::From<PostRequestHandledFilter> for EvmHostEvents {
        fn from(value: PostRequestHandledFilter) -> Self {
            Self::PostRequestHandledFilter(value)
        }
    }
    impl ::core::convert::From<PostResponseEventFilter> for EvmHostEvents {
        fn from(value: PostResponseEventFilter) -> Self {
            Self::PostResponseEventFilter(value)
        }
    }
    impl ::core::convert::From<PostResponseHandledFilter> for EvmHostEvents {
        fn from(value: PostResponseHandledFilter) -> Self {
            Self::PostResponseHandledFilter(value)
        }
    }
    ///Container type for all input parameters for the `admin` function with signature `admin()` and selector `0xf851a440`
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
    #[ethcall(name = "admin", abi = "admin()")]
    pub struct AdminCall;
    ///Container type for all input parameters for the `chainId` function with signature `chainId()` and selector `0x9a8a0592`
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
    #[ethcall(name = "chainId", abi = "chainId()")]
    pub struct ChainIdCall;
    ///Container type for all input parameters for the `challengePeriod` function with signature `challengePeriod()` and selector `0xf3f480d9`
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
    #[ethcall(name = "challengePeriod", abi = "challengePeriod()")]
    pub struct ChallengePeriodCall;
    ///Container type for all input parameters for the `consensusClient` function with signature `consensusClient()` and selector `0x2476132b`
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
    #[ethcall(name = "consensusClient", abi = "consensusClient()")]
    pub struct ConsensusClientCall;
    ///Container type for all input parameters for the `consensusState` function with signature `consensusState()` and selector `0xbbad99d4`
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
    #[ethcall(name = "consensusState", abi = "consensusState()")]
    pub struct ConsensusStateCall;
    ///Container type for all input parameters for the `consensusUpdateTime` function with signature `consensusUpdateTime()` and selector `0x9a8425bc`
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
    #[ethcall(name = "consensusUpdateTime", abi = "consensusUpdateTime()")]
    pub struct ConsensusUpdateTimeCall;
    ///Container type for all input parameters for the `dai` function with signature `dai()` and selector `0xf4b9fa75`
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
    #[ethcall(name = "dai", abi = "dai()")]
    pub struct DaiCall;
    ///Container type for all input parameters for the `dispatch` function with signature `dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64,uint256))` and selector `0x825d27c5`
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
        abi = "dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64,uint256))"
    )]
    pub struct DispatchCall {
        pub post: DispatchPost,
    }
    ///Container type for all input parameters for the `dispatch` function with signature `dispatch((bytes,bytes,bytes,uint64,uint64,uint256))` and selector `0xa20a3174`
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
        abi = "dispatch((bytes,bytes,bytes,uint64,uint64,uint256))"
    )]
    pub struct DispatchWithPostCall {
        pub post: DispatchPost,
    }
    ///Container type for all input parameters for the `dispatch` function with signature `dispatch((bytes,uint64,bytes[],uint64,uint64,uint256))` and selector `0xc94f48f3`
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
        abi = "dispatch((bytes,uint64,bytes[],uint64,uint64,uint256))"
    )]
    pub struct DispatchWithGetCall {
        pub get: DispatchGet,
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(uint256,address),bytes32)` and selector `0x09cc21c3`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(uint256,address),bytes32)"
    )]
    pub struct DispatchIncoming3Call {
        pub request: PostRequest,
        pub meta: FeeMetadata,
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),(uint256,address),bytes32)` and selector `0x1678619c`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),(uint256,address),bytes32)"
    )]
    pub struct DispatchIncoming4Call {
        pub request: PostRequest,
        pub meta: FeeMetadata,
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x3b8c2bf7`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct DispatchIncoming0Call {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))` and selector `0x4f5b258a`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64))"
    )]
    pub struct DispatchIncoming1Call {
        pub response: GetResponse,
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64),(uint256,address),bytes32)` and selector `0xe95c866c`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64),(uint256,address),bytes32)"
    )]
    pub struct DispatchIncoming5Call {
        pub response: GetResponse,
        pub meta: FeeMetadata,
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `dispatchIncoming` function with signature `dispatchIncoming(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))` and selector `0xf0736091`
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
        name = "dispatchIncoming",
        abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))"
    )]
    pub struct DispatchIncoming2Call {
        pub response: GetResponse,
    }
    ///Container type for all input parameters for the `frozen` function with signature `frozen()` and selector `0x054f7d9c`
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
    #[ethcall(name = "frozen", abi = "frozen()")]
    pub struct FrozenCall;
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
    ///Container type for all input parameters for the `hostParams` function with signature `hostParams()` and selector `0x2215364d`
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
    #[ethcall(name = "hostParams", abi = "hostParams()")]
    pub struct HostParamsCall;
    ///Container type for all input parameters for the `latestStateMachineHeight` function with signature `latestStateMachineHeight()` and selector `0x56b65597`
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
    #[ethcall(name = "latestStateMachineHeight", abi = "latestStateMachineHeight()")]
    pub struct LatestStateMachineHeightCall;
    ///Container type for all input parameters for the `perByteFee` function with signature `perByteFee()` and selector `0x641d729d`
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
    #[ethcall(name = "perByteFee", abi = "perByteFee()")]
    pub struct PerByteFeeCall;
    ///Container type for all input parameters for the `requestCommitments` function with signature `requestCommitments(bytes32)` and selector `0x368bf464`
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
    #[ethcall(name = "requestCommitments", abi = "requestCommitments(bytes32)")]
    pub struct RequestCommitmentsCall {
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `requestReceipts` function with signature `requestReceipts(bytes32)` and selector `0x19667a3e`
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
    #[ethcall(name = "requestReceipts", abi = "requestReceipts(bytes32)")]
    pub struct RequestReceiptsCall {
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `responseCommitments` function with signature `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
    #[ethcall(name = "responseCommitments", abi = "responseCommitments(bytes32)")]
    pub struct ResponseCommitmentsCall {
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `responseReceipts` function with signature `responseReceipts(bytes32)` and selector `0x8856337e`
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
    #[ethcall(name = "responseReceipts", abi = "responseReceipts(bytes32)")]
    pub struct ResponseReceiptsCall {
        pub commitment: [u8; 32],
    }
    ///Container type for all input parameters for the `setConsensusState` function with signature `setConsensusState(bytes)` and selector `0xa15f7431`
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
    #[ethcall(name = "setConsensusState", abi = "setConsensusState(bytes)")]
    pub struct SetConsensusStateCall {
        pub state: ::ethers::core::types::Bytes,
    }
    ///Container type for all input parameters for the `setFrozenState` function with signature `setFrozenState(bool)` and selector `0x19e8faf1`
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
    #[ethcall(name = "setFrozenState", abi = "setFrozenState(bool)")]
    pub struct SetFrozenStateCall {
        pub new_state: bool,
    }
    ///Container type for all input parameters for the `setHostParams` function with signature `setHostParams((uint256,uint256,uint256,address,address,address,address,uint256,uint256,address,bytes,uint256,uint256))` and selector `0x7772a61b`
    #[derive(Clone, ::ethers::contract::EthCall, ::ethers::contract::EthDisplay)]
    #[ethcall(
        name = "setHostParams",
        abi = "setHostParams((uint256,uint256,uint256,address,address,address,address,uint256,uint256,address,bytes,uint256,uint256))"
    )]
    pub struct SetHostParamsCall {
        pub params: HostParams,
    }
    ///Container type for all input parameters for the `setHostParamsAdmin` function with signature `setHostParamsAdmin((uint256,uint256,uint256,address,address,address,address,uint256,uint256,address,bytes,uint256,uint256))` and selector `0x44febebd`
    #[derive(Clone, ::ethers::contract::EthCall, ::ethers::contract::EthDisplay)]
    #[ethcall(
        name = "setHostParamsAdmin",
        abi = "setHostParamsAdmin((uint256,uint256,uint256,address,address,address,address,uint256,uint256,address,bytes,uint256,uint256))"
    )]
    pub struct SetHostParamsAdminCall {
        pub params: HostParams,
    }
    ///Container type for all input parameters for the `stateMachineCommitment` function with signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
        name = "stateMachineCommitment",
        abi = "stateMachineCommitment((uint256,uint256))"
    )]
    pub struct StateMachineCommitmentCall {
        pub height: StateMachineHeight,
    }
    ///Container type for all input parameters for the `stateMachineCommitmentUpdateTime` function with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector `0x1a880a93`
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
        name = "stateMachineCommitmentUpdateTime",
        abi = "stateMachineCommitmentUpdateTime((uint256,uint256))"
    )]
    pub struct StateMachineCommitmentUpdateTimeCall {
        pub height: StateMachineHeight,
    }
    ///Container type for all input parameters for the `storeConsensusState` function with signature `storeConsensusState(bytes)` and selector `0xb4974cf0`
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
    #[ethcall(name = "storeConsensusState", abi = "storeConsensusState(bytes)")]
    pub struct StoreConsensusStateCall {
        pub state: ::ethers::core::types::Bytes,
    }
    ///Container type for all input parameters for the `storeConsensusUpdateTime` function with signature `storeConsensusUpdateTime(uint256)` and selector `0xd860cb47`
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
        name = "storeConsensusUpdateTime",
        abi = "storeConsensusUpdateTime(uint256)"
    )]
    pub struct StoreConsensusUpdateTimeCall {
        pub time: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `storeLatestStateMachineHeight` function with signature `storeLatestStateMachineHeight(uint256)` and selector `0xa0756ecd`
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
        name = "storeLatestStateMachineHeight",
        abi = "storeLatestStateMachineHeight(uint256)"
    )]
    pub struct StoreLatestStateMachineHeightCall {
        pub height: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `storeStateMachineCommitment` function with signature `storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))` and selector `0x559efe9e`
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
        name = "storeStateMachineCommitment",
        abi = "storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))"
    )]
    pub struct StoreStateMachineCommitmentCall {
        pub height: StateMachineHeight,
        pub commitment: StateCommitment,
    }
    ///Container type for all input parameters for the `storeStateMachineCommitmentUpdateTime` function with signature `storeStateMachineCommitmentUpdateTime((uint256,uint256),uint256)` and selector `0x14863dcb`
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
        name = "storeStateMachineCommitmentUpdateTime",
        abi = "storeStateMachineCommitmentUpdateTime((uint256,uint256),uint256)"
    )]
    pub struct StoreStateMachineCommitmentUpdateTimeCall {
        pub height: StateMachineHeight,
        pub time: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `timestamp` function with signature `timestamp()` and selector `0xb80777ea`
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
    #[ethcall(name = "timestamp", abi = "timestamp()")]
    pub struct TimestampCall;
    ///Container type for all input parameters for the `unStakingPeriod` function with signature `unStakingPeriod()` and selector `0xd40784c7`
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
    #[ethcall(name = "unStakingPeriod", abi = "unStakingPeriod()")]
    pub struct UnStakingPeriodCall;
    ///Container type for all input parameters for the `withdraw` function with signature `withdraw((address,uint256))` and selector `0x3c565417`
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
    #[ethcall(name = "withdraw", abi = "withdraw((address,uint256))")]
    pub struct WithdrawCall {
        pub params: WithdrawParams,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType)]
    pub enum EvmHostCalls {
        Admin(AdminCall),
        ChainId(ChainIdCall),
        ChallengePeriod(ChallengePeriodCall),
        ConsensusClient(ConsensusClientCall),
        ConsensusState(ConsensusStateCall),
        ConsensusUpdateTime(ConsensusUpdateTimeCall),
        Dai(DaiCall),
        Dispatch(DispatchCall),
        DispatchWithPost(DispatchWithPostCall),
        DispatchWithGet(DispatchWithGetCall),
        DispatchIncoming3(DispatchIncoming3Call),
        DispatchIncoming4(DispatchIncoming4Call),
        DispatchIncoming0(DispatchIncoming0Call),
        DispatchIncoming1(DispatchIncoming1Call),
        DispatchIncoming5(DispatchIncoming5Call),
        DispatchIncoming2(DispatchIncoming2Call),
        Frozen(FrozenCall),
        Host(HostCall),
        HostParams(HostParamsCall),
        LatestStateMachineHeight(LatestStateMachineHeightCall),
        PerByteFee(PerByteFeeCall),
        RequestCommitments(RequestCommitmentsCall),
        RequestReceipts(RequestReceiptsCall),
        ResponseCommitments(ResponseCommitmentsCall),
        ResponseReceipts(ResponseReceiptsCall),
        SetConsensusState(SetConsensusStateCall),
        SetFrozenState(SetFrozenStateCall),
        SetHostParams(SetHostParamsCall),
        SetHostParamsAdmin(SetHostParamsAdminCall),
        StateMachineCommitment(StateMachineCommitmentCall),
        StateMachineCommitmentUpdateTime(StateMachineCommitmentUpdateTimeCall),
        StoreConsensusState(StoreConsensusStateCall),
        StoreConsensusUpdateTime(StoreConsensusUpdateTimeCall),
        StoreLatestStateMachineHeight(StoreLatestStateMachineHeightCall),
        StoreStateMachineCommitment(StoreStateMachineCommitmentCall),
        StoreStateMachineCommitmentUpdateTime(StoreStateMachineCommitmentUpdateTimeCall),
        Timestamp(TimestampCall),
        UnStakingPeriod(UnStakingPeriodCall),
        Withdraw(WithdrawCall),
    }
    impl ::ethers::core::abi::AbiDecode for EvmHostCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <AdminCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Admin(decoded));
            }
            if let Ok(decoded) = <ChainIdCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ChainId(decoded));
            }
            if let Ok(decoded) = <ChallengePeriodCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ChallengePeriod(decoded));
            }
            if let Ok(decoded) = <ConsensusClientCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ConsensusClient(decoded));
            }
            if let Ok(decoded) = <ConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ConsensusState(decoded));
            }
            if let Ok(decoded) = <ConsensusUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ConsensusUpdateTime(decoded));
            }
            if let Ok(decoded) = <DaiCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Dai(decoded));
            }
            if let Ok(decoded) = <DispatchCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Dispatch(decoded));
            }
            if let Ok(decoded) = <DispatchWithPostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchWithPost(decoded));
            }
            if let Ok(decoded) = <DispatchWithGetCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchWithGet(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming3Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming3(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming4Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming4(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming0Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming0(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming1Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming1(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming5Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming5(decoded));
            }
            if let Ok(decoded) = <DispatchIncoming2Call as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::DispatchIncoming2(decoded));
            }
            if let Ok(decoded) = <FrozenCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Frozen(decoded));
            }
            if let Ok(decoded) = <HostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Host(decoded));
            }
            if let Ok(decoded) = <HostParamsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::HostParams(decoded));
            }
            if let Ok(decoded) = <LatestStateMachineHeightCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::LatestStateMachineHeight(decoded));
            }
            if let Ok(decoded) = <PerByteFeeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::PerByteFee(decoded));
            }
            if let Ok(decoded) = <RequestCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RequestCommitments(decoded));
            }
            if let Ok(decoded) = <RequestReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RequestReceipts(decoded));
            }
            if let Ok(decoded) = <ResponseCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ResponseCommitments(decoded));
            }
            if let Ok(decoded) = <ResponseReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ResponseReceipts(decoded));
            }
            if let Ok(decoded) = <SetConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetConsensusState(decoded));
            }
            if let Ok(decoded) = <SetFrozenStateCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetFrozenState(decoded));
            }
            if let Ok(decoded) = <SetHostParamsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetHostParams(decoded));
            }
            if let Ok(decoded) = <SetHostParamsAdminCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetHostParamsAdmin(decoded));
            }
            if let Ok(decoded) = <StateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StateMachineCommitment(decoded));
            }
            if let Ok(decoded) = <StateMachineCommitmentUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StateMachineCommitmentUpdateTime(decoded));
            }
            if let Ok(decoded) = <StoreConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreConsensusState(decoded));
            }
            if let Ok(decoded) = <StoreConsensusUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreConsensusUpdateTime(decoded));
            }
            if let Ok(decoded) = <StoreLatestStateMachineHeightCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreLatestStateMachineHeight(decoded));
            }
            if let Ok(decoded) = <StoreStateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreStateMachineCommitment(decoded));
            }
            if let Ok(decoded) = <StoreStateMachineCommitmentUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::StoreStateMachineCommitmentUpdateTime(decoded));
            }
            if let Ok(decoded) = <TimestampCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Timestamp(decoded));
            }
            if let Ok(decoded) = <UnStakingPeriodCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::UnStakingPeriod(decoded));
            }
            if let Ok(decoded) = <WithdrawCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Withdraw(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for EvmHostCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::Admin(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::ChainId(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::ChallengePeriod(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ConsensusClient(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ConsensusState(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ConsensusUpdateTime(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Dai(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Dispatch(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchWithPost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchWithGet(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming3(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming4(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming0(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming1(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming5(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchIncoming2(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Frozen(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Host(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::HostParams(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::LatestStateMachineHeight(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::PerByteFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RequestCommitments(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RequestReceipts(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ResponseCommitments(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ResponseReceipts(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetConsensusState(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetFrozenState(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetHostParams(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetHostParamsAdmin(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StateMachineCommitment(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StateMachineCommitmentUpdateTime(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StoreConsensusState(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StoreConsensusUpdateTime(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StoreLatestStateMachineHeight(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StoreStateMachineCommitment(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::StoreStateMachineCommitmentUpdateTime(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Timestamp(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnStakingPeriod(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Withdraw(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for EvmHostCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::Admin(element) => ::core::fmt::Display::fmt(element, f),
                Self::ChainId(element) => ::core::fmt::Display::fmt(element, f),
                Self::ChallengePeriod(element) => ::core::fmt::Display::fmt(element, f),
                Self::ConsensusClient(element) => ::core::fmt::Display::fmt(element, f),
                Self::ConsensusState(element) => ::core::fmt::Display::fmt(element, f),
                Self::ConsensusUpdateTime(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::Dai(element) => ::core::fmt::Display::fmt(element, f),
                Self::Dispatch(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchWithPost(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchWithGet(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming3(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming4(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming0(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming1(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming5(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchIncoming2(element) => ::core::fmt::Display::fmt(element, f),
                Self::Frozen(element) => ::core::fmt::Display::fmt(element, f),
                Self::Host(element) => ::core::fmt::Display::fmt(element, f),
                Self::HostParams(element) => ::core::fmt::Display::fmt(element, f),
                Self::LatestStateMachineHeight(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::PerByteFee(element) => ::core::fmt::Display::fmt(element, f),
                Self::RequestCommitments(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RequestReceipts(element) => ::core::fmt::Display::fmt(element, f),
                Self::ResponseCommitments(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ResponseReceipts(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetConsensusState(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetFrozenState(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetHostParams(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetHostParamsAdmin(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StateMachineCommitment(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StateMachineCommitmentUpdateTime(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StoreConsensusState(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StoreConsensusUpdateTime(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StoreLatestStateMachineHeight(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StoreStateMachineCommitment(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::StoreStateMachineCommitmentUpdateTime(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::Timestamp(element) => ::core::fmt::Display::fmt(element, f),
                Self::UnStakingPeriod(element) => ::core::fmt::Display::fmt(element, f),
                Self::Withdraw(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<AdminCall> for EvmHostCalls {
        fn from(value: AdminCall) -> Self {
            Self::Admin(value)
        }
    }
    impl ::core::convert::From<ChainIdCall> for EvmHostCalls {
        fn from(value: ChainIdCall) -> Self {
            Self::ChainId(value)
        }
    }
    impl ::core::convert::From<ChallengePeriodCall> for EvmHostCalls {
        fn from(value: ChallengePeriodCall) -> Self {
            Self::ChallengePeriod(value)
        }
    }
    impl ::core::convert::From<ConsensusClientCall> for EvmHostCalls {
        fn from(value: ConsensusClientCall) -> Self {
            Self::ConsensusClient(value)
        }
    }
    impl ::core::convert::From<ConsensusStateCall> for EvmHostCalls {
        fn from(value: ConsensusStateCall) -> Self {
            Self::ConsensusState(value)
        }
    }
    impl ::core::convert::From<ConsensusUpdateTimeCall> for EvmHostCalls {
        fn from(value: ConsensusUpdateTimeCall) -> Self {
            Self::ConsensusUpdateTime(value)
        }
    }
    impl ::core::convert::From<DaiCall> for EvmHostCalls {
        fn from(value: DaiCall) -> Self {
            Self::Dai(value)
        }
    }
    impl ::core::convert::From<DispatchCall> for EvmHostCalls {
        fn from(value: DispatchCall) -> Self {
            Self::Dispatch(value)
        }
    }
    impl ::core::convert::From<DispatchWithPostCall> for EvmHostCalls {
        fn from(value: DispatchWithPostCall) -> Self {
            Self::DispatchWithPost(value)
        }
    }
    impl ::core::convert::From<DispatchWithGetCall> for EvmHostCalls {
        fn from(value: DispatchWithGetCall) -> Self {
            Self::DispatchWithGet(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming3Call> for EvmHostCalls {
        fn from(value: DispatchIncoming3Call) -> Self {
            Self::DispatchIncoming3(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming4Call> for EvmHostCalls {
        fn from(value: DispatchIncoming4Call) -> Self {
            Self::DispatchIncoming4(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming0Call> for EvmHostCalls {
        fn from(value: DispatchIncoming0Call) -> Self {
            Self::DispatchIncoming0(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming1Call> for EvmHostCalls {
        fn from(value: DispatchIncoming1Call) -> Self {
            Self::DispatchIncoming1(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming5Call> for EvmHostCalls {
        fn from(value: DispatchIncoming5Call) -> Self {
            Self::DispatchIncoming5(value)
        }
    }
    impl ::core::convert::From<DispatchIncoming2Call> for EvmHostCalls {
        fn from(value: DispatchIncoming2Call) -> Self {
            Self::DispatchIncoming2(value)
        }
    }
    impl ::core::convert::From<FrozenCall> for EvmHostCalls {
        fn from(value: FrozenCall) -> Self {
            Self::Frozen(value)
        }
    }
    impl ::core::convert::From<HostCall> for EvmHostCalls {
        fn from(value: HostCall) -> Self {
            Self::Host(value)
        }
    }
    impl ::core::convert::From<HostParamsCall> for EvmHostCalls {
        fn from(value: HostParamsCall) -> Self {
            Self::HostParams(value)
        }
    }
    impl ::core::convert::From<LatestStateMachineHeightCall> for EvmHostCalls {
        fn from(value: LatestStateMachineHeightCall) -> Self {
            Self::LatestStateMachineHeight(value)
        }
    }
    impl ::core::convert::From<PerByteFeeCall> for EvmHostCalls {
        fn from(value: PerByteFeeCall) -> Self {
            Self::PerByteFee(value)
        }
    }
    impl ::core::convert::From<RequestCommitmentsCall> for EvmHostCalls {
        fn from(value: RequestCommitmentsCall) -> Self {
            Self::RequestCommitments(value)
        }
    }
    impl ::core::convert::From<RequestReceiptsCall> for EvmHostCalls {
        fn from(value: RequestReceiptsCall) -> Self {
            Self::RequestReceipts(value)
        }
    }
    impl ::core::convert::From<ResponseCommitmentsCall> for EvmHostCalls {
        fn from(value: ResponseCommitmentsCall) -> Self {
            Self::ResponseCommitments(value)
        }
    }
    impl ::core::convert::From<ResponseReceiptsCall> for EvmHostCalls {
        fn from(value: ResponseReceiptsCall) -> Self {
            Self::ResponseReceipts(value)
        }
    }
    impl ::core::convert::From<SetConsensusStateCall> for EvmHostCalls {
        fn from(value: SetConsensusStateCall) -> Self {
            Self::SetConsensusState(value)
        }
    }
    impl ::core::convert::From<SetFrozenStateCall> for EvmHostCalls {
        fn from(value: SetFrozenStateCall) -> Self {
            Self::SetFrozenState(value)
        }
    }
    impl ::core::convert::From<SetHostParamsCall> for EvmHostCalls {
        fn from(value: SetHostParamsCall) -> Self {
            Self::SetHostParams(value)
        }
    }
    impl ::core::convert::From<SetHostParamsAdminCall> for EvmHostCalls {
        fn from(value: SetHostParamsAdminCall) -> Self {
            Self::SetHostParamsAdmin(value)
        }
    }
    impl ::core::convert::From<StateMachineCommitmentCall> for EvmHostCalls {
        fn from(value: StateMachineCommitmentCall) -> Self {
            Self::StateMachineCommitment(value)
        }
    }
    impl ::core::convert::From<StateMachineCommitmentUpdateTimeCall> for EvmHostCalls {
        fn from(value: StateMachineCommitmentUpdateTimeCall) -> Self {
            Self::StateMachineCommitmentUpdateTime(value)
        }
    }
    impl ::core::convert::From<StoreConsensusStateCall> for EvmHostCalls {
        fn from(value: StoreConsensusStateCall) -> Self {
            Self::StoreConsensusState(value)
        }
    }
    impl ::core::convert::From<StoreConsensusUpdateTimeCall> for EvmHostCalls {
        fn from(value: StoreConsensusUpdateTimeCall) -> Self {
            Self::StoreConsensusUpdateTime(value)
        }
    }
    impl ::core::convert::From<StoreLatestStateMachineHeightCall> for EvmHostCalls {
        fn from(value: StoreLatestStateMachineHeightCall) -> Self {
            Self::StoreLatestStateMachineHeight(value)
        }
    }
    impl ::core::convert::From<StoreStateMachineCommitmentCall> for EvmHostCalls {
        fn from(value: StoreStateMachineCommitmentCall) -> Self {
            Self::StoreStateMachineCommitment(value)
        }
    }
    impl ::core::convert::From<StoreStateMachineCommitmentUpdateTimeCall>
    for EvmHostCalls {
        fn from(value: StoreStateMachineCommitmentUpdateTimeCall) -> Self {
            Self::StoreStateMachineCommitmentUpdateTime(value)
        }
    }
    impl ::core::convert::From<TimestampCall> for EvmHostCalls {
        fn from(value: TimestampCall) -> Self {
            Self::Timestamp(value)
        }
    }
    impl ::core::convert::From<UnStakingPeriodCall> for EvmHostCalls {
        fn from(value: UnStakingPeriodCall) -> Self {
            Self::UnStakingPeriod(value)
        }
    }
    impl ::core::convert::From<WithdrawCall> for EvmHostCalls {
        fn from(value: WithdrawCall) -> Self {
            Self::Withdraw(value)
        }
    }
    ///Container type for all return fields from the `admin` function with signature `admin()` and selector `0xf851a440`
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
    pub struct AdminReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `chainId` function with signature `chainId()` and selector `0x9a8a0592`
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
    pub struct ChainIdReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `challengePeriod` function with signature `challengePeriod()` and selector `0xf3f480d9`
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
    pub struct ChallengePeriodReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `consensusClient` function with signature `consensusClient()` and selector `0x2476132b`
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
    pub struct ConsensusClientReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `consensusState` function with signature `consensusState()` and selector `0xbbad99d4`
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
    pub struct ConsensusStateReturn(pub ::ethers::core::types::Bytes);
    ///Container type for all return fields from the `consensusUpdateTime` function with signature `consensusUpdateTime()` and selector `0x9a8425bc`
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
    pub struct ConsensusUpdateTimeReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `dai` function with signature `dai()` and selector `0xf4b9fa75`
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
    pub struct DaiReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `frozen` function with signature `frozen()` and selector `0x054f7d9c`
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
    pub struct FrozenReturn(pub bool);
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
    pub struct HostReturn(pub ::ethers::core::types::Bytes);
    ///Container type for all return fields from the `hostParams` function with signature `hostParams()` and selector `0x2215364d`
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
    pub struct HostParamsReturn(pub HostParams);
    ///Container type for all return fields from the `latestStateMachineHeight` function with signature `latestStateMachineHeight()` and selector `0x56b65597`
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
    pub struct LatestStateMachineHeightReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `perByteFee` function with signature `perByteFee()` and selector `0x641d729d`
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
    pub struct PerByteFeeReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `requestCommitments` function with signature `requestCommitments(bytes32)` and selector `0x368bf464`
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
    pub struct RequestCommitmentsReturn(pub FeeMetadata);
    ///Container type for all return fields from the `requestReceipts` function with signature `requestReceipts(bytes32)` and selector `0x19667a3e`
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
    pub struct RequestReceiptsReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `responseCommitments` function with signature `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
    pub struct ResponseCommitmentsReturn(pub FeeMetadata);
    ///Container type for all return fields from the `responseReceipts` function with signature `responseReceipts(bytes32)` and selector `0x8856337e`
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
    pub struct ResponseReceiptsReturn(pub ResponseReceipt);
    ///Container type for all return fields from the `stateMachineCommitment` function with signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
    pub struct StateMachineCommitmentReturn(pub StateCommitment);
    ///Container type for all return fields from the `stateMachineCommitmentUpdateTime` function with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector `0x1a880a93`
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
    pub struct StateMachineCommitmentUpdateTimeReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `timestamp` function with signature `timestamp()` and selector `0xb80777ea`
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
    pub struct TimestampReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `unStakingPeriod` function with signature `unStakingPeriod()` and selector `0xd40784c7`
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
    pub struct UnStakingPeriodReturn(pub ::ethers::core::types::U256);
    ///`DispatchGet(bytes,uint64,bytes[],uint64,uint64,uint256)`
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
    pub struct DispatchGet {
        pub dest: ::ethers::core::types::Bytes,
        pub height: u64,
        pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
        pub timeout: u64,
        pub gaslimit: u64,
        pub fee: ::ethers::core::types::U256,
    }
    ///`DispatchPost(bytes,bytes,bytes,uint64,uint64,uint256)`
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
    pub struct DispatchPost {
        pub dest: ::ethers::core::types::Bytes,
        pub to: ::ethers::core::types::Bytes,
        pub body: ::ethers::core::types::Bytes,
        pub timeout: u64,
        pub gaslimit: u64,
        pub fee: ::ethers::core::types::U256,
    }
    ///`DispatchPostResponse((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes,uint64,uint64,uint256)`
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
    pub struct DispatchPostResponse {
        pub request: PostRequest,
        pub response: ::ethers::core::types::Bytes,
        pub timeout: u64,
        pub gaslimit: u64,
        pub fee: ::ethers::core::types::U256,
    }
    ///`FeeMetadata(uint256,address)`
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
    pub struct FeeMetadata {
        pub fee: ::ethers::core::types::U256,
        pub sender: ::ethers::core::types::Address,
    }
    ///`HostParams(uint256,uint256,uint256,address,address,address,address,uint256,uint256,address,bytes,uint256,uint256)`
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
    pub struct HostParams {
        pub default_timeout: ::ethers::core::types::U256,
        pub base_get_request_fee: ::ethers::core::types::U256,
        pub per_byte_fee: ::ethers::core::types::U256,
        pub fee_token_address: ::ethers::core::types::Address,
        pub admin: ::ethers::core::types::Address,
        pub handler: ::ethers::core::types::Address,
        pub host_manager: ::ethers::core::types::Address,
        pub un_staking_period: ::ethers::core::types::U256,
        pub challenge_period: ::ethers::core::types::U256,
        pub consensus_client: ::ethers::core::types::Address,
        pub consensus_state: ::ethers::core::types::Bytes,
        pub last_updated: ::ethers::core::types::U256,
        pub latest_state_machine_height: ::ethers::core::types::U256,
    }
    ///`ResponseReceipt(bytes32,address)`
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
    pub struct ResponseReceipt {
        pub response_commitment: [u8; 32],
        pub relayer: ::ethers::core::types::Address,
    }
    ///`WithdrawParams(address,uint256)`
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
    pub struct WithdrawParams {
        pub beneficiary: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
}
