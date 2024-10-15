pub use evm_host::*;
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
                    ::std::borrow::ToOwned::to_owned("deleteStateMachineCommitment"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "deleteStateMachineCommitment",
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
                                    name: ::std::borrow::ToOwned::to_owned("fisherman"),
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
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct DispatchGet"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
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
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
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
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostRequest"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetResponse"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("relayer"),
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
                    ::std::borrow::ToOwned::to_owned("dispatchTimeOut"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatchTimeOut"),
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
                            name: ::std::borrow::ToOwned::to_owned("dispatchTimeOut"),
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
                            name: ::std::borrow::ToOwned::to_owned("dispatchTimeOut"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
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
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("feeToken"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("feeToken"),
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
                    ::std::borrow::ToOwned::to_owned("frozen"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("frozen"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("enum FrozenStatus"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("fundRequest"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("fundRequest"),
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("fundResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("fundResponse"),
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
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                    ::std::borrow::ToOwned::to_owned("hyperbridge"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("hyperbridge"),
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
                    ::std::borrow::ToOwned::to_owned("latestStateMachineHeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "latestStateMachineHeight",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("id"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
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
                    ::std::borrow::ToOwned::to_owned("nonce"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("nonce"),
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
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("stateId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
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
                    ::std::borrow::ToOwned::to_owned("responded"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("responded"),
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
                    ::std::borrow::ToOwned::to_owned("setFrozenState"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setFrozenState"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("newState"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("enum FrozenStatus"),
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
                    ::std::borrow::ToOwned::to_owned("stateCommitmentFee"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("stateCommitmentFee"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
                    ::std::borrow::ToOwned::to_owned("stateMachineId"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("stateMachineId"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("parachainId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("id"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
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
                    ::std::borrow::ToOwned::to_owned("uniswapV2Router"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("uniswapV2Router"),
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
                    ::std::borrow::ToOwned::to_owned("updateHostParams"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("updateHostParams"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                    ::std::borrow::ToOwned::to_owned("vetoes"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("vetoes"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("paraId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
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
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
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
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nonce"),
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
                                    name: ::std::borrow::ToOwned::to_owned("context"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                                    indexed: true,
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
                    ::std::borrow::ToOwned::to_owned("GetRequestTimeoutHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "GetRequestTimeoutHandled",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("HostFrozen"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("HostFrozen"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("status"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("HostParamsUpdated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("HostParamsUpdated"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("oldParams"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("newParams"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("HostWithdrawal"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("HostWithdrawal"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("beneficiary"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("native"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
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
                                    name: ::std::borrow::ToOwned::to_owned("body"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
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
                                    indexed: true,
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
                    ::std::borrow::ToOwned::to_owned("PostRequestTimeoutHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostRequestTimeoutHandled",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
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
                                    name: ::std::borrow::ToOwned::to_owned("body"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "responseTimeoutTimestamp",
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
                    ::std::borrow::ToOwned::to_owned("PostResponseFunded"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostResponseFunded"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("newFee"),
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
                                    indexed: true,
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
                    ::std::borrow::ToOwned::to_owned("PostResponseTimeoutHandled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "PostResponseTimeoutHandled",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("dest"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("RequestFunded"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("RequestFunded"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("commitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("newFee"),
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
                    ::std::borrow::ToOwned::to_owned("StateCommitmentRead"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "StateCommitmentRead",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("caller"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
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
                    ::std::borrow::ToOwned::to_owned("StateCommitmentVetoed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "StateCommitmentVetoed",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("stateMachineId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("stateCommitment"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("fisherman"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("StateMachineUpdated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "StateMachineUpdated",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("stateMachineId"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("height"),
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
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("CannotChangeFeeToken"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "CannotChangeFeeToken",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("DuplicateResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("DuplicateResponse"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("FrozenHost"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("FrozenHost"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidAddressLength"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidAddressLength",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidConsensusClient"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidConsensusClient",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidHandler"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("InvalidHandler"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidHostManager"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("InvalidHostManager"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidHyperbridgeId"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidHyperbridgeId",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidStateMachinesLength"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidStateMachinesLength",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidUnstakingPeriod"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidUnstakingPeriod",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
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
                    ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnauthorizedAction"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnauthorizedResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "UnauthorizedResponse",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnknownRequest"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnknownRequest"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnknownResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("UnknownResponse"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("WithdrawalFailed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("WithdrawalFailed"),
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
	pub static EVMHOST_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
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
			Self(::ethers::contract::Contract::new(address.into(), EVMHOST_ABI.clone(), client))
		}
		///Calls the contract's `admin` (0xf851a440) function
		pub fn admin(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
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
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([36, 118, 19, 43], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `consensusState` (0xbbad99d4) function
		pub fn consensus_state(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
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
		///Calls the contract's `deleteStateMachineCommitment` (0xf8ddf259) function
		pub fn delete_state_machine_commitment(
			&self,
			height: StateMachineHeight,
			fisherman: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([248, 221, 242, 89], (height, fisherman))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0x94480805) function
		pub fn dispatch(
			&self,
			post: DispatchPost,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([148, 72, 8, 5], (post,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0xb8f3e8f5) function
		pub fn dispatch_with_post(
			&self,
			post: DispatchPost,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([184, 243, 232, 245], (post,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatch` (0xd22e3343) function
		pub fn dispatch_with_get(
			&self,
			get: DispatchGet,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([210, 46, 51, 67], (get,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0xab013de1) function
		pub fn dispatch_incoming(
			&self,
			response: GetResponse,
			relayer: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([171, 1, 61, 225], (response, relayer))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0xb85e6fbb) function
		pub fn dispatch_incoming_with_request(
			&self,
			request: PostRequest,
			relayer: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([184, 94, 111, 187], (request, relayer))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchIncoming` (0xfc8a341c) function
		pub fn dispatch_incoming_with_response(
			&self,
			response: GetResponse,
			relayer: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([252, 138, 52, 28], (response, relayer))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchTimeOut` (0x0446fc47) function
		pub fn dispatch_time_out_0(
			&self,
			response: PostResponse,
			meta: FeeMetadata,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([4, 70, 252, 71], (response, meta, commitment))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchTimeOut` (0x4b46efaa) function
		pub fn dispatch_time_out_1(
			&self,
			request: GetRequest,
			meta: FeeMetadata,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([75, 70, 239, 170], (request, meta, commitment))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `dispatchTimeOut` (0x6d6c2313) function
		pub fn dispatch_time_out_2(
			&self,
			request: GetRequest,
			meta: FeeMetadata,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([109, 108, 35, 19], (request, meta, commitment))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `feeToken` (0x647846a5) function
		pub fn fee_token(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([100, 120, 70, 165], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `frozen` (0x054f7d9c) function
		pub fn frozen(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
			self.0
				.method_hash([5, 79, 125, 156], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `fundRequest` (0xb9ea3289) function
		pub fn fund_request(
			&self,
			commitment: [u8; 32],
			amount: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([185, 234, 50, 137], (commitment, amount))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `fundResponse` (0xfadce3c7) function
		pub fn fund_response(
			&self,
			commitment: [u8; 32],
			amount: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([250, 220, 227, 199], (commitment, amount))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `host` (0xf437bc59) function
		pub fn host(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([244, 55, 188, 89], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `hostParams` (0x2215364d) function
		pub fn host_params(&self) -> ::ethers::contract::builders::ContractCall<M, HostParams> {
			self.0
				.method_hash([34, 21, 54, 77], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `hyperbridge` (0x005e763e) function
		pub fn hyperbridge(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Bytes> {
			self.0
				.method_hash([0, 94, 118, 62], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestStateMachineHeight` (0x9c095f86) function
		pub fn latest_state_machine_height(
			&self,
			id: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([156, 9, 95, 134], id)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `nonce` (0xaffed0e0) function
		pub fn nonce(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([175, 254, 208, 224], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `perByteFee` (0x4011ec0a) function
		pub fn per_byte_fee(
			&self,
			state_id: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([64, 17, 236, 10], state_id)
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
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([25, 102, 122, 62], commitment)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `responded` (0x73bfa8ae) function
		pub fn responded(
			&self,
			commitment: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([115, 191, 168, 174], commitment)
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
		///Calls the contract's `setConsensusState` (0x8d48f3c7) function
		pub fn set_consensus_state(
			&self,
			state: ::ethers::core::types::Bytes,
			height: StateMachineHeight,
			commitment: StateCommitment,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([141, 72, 243, 199], (state, height, commitment))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setFrozenState` (0x6ac88bad) function
		pub fn set_frozen_state(
			&self,
			new_state: u8,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([106, 200, 139, 173], new_state)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `stateCommitmentFee` (0xb5c8a08e) function
		pub fn state_commitment_fee(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([181, 200, 160, 142], ())
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
		///Calls the contract's `stateMachineId` (0x7a1bd7c1) function
		pub fn state_machine_id(
			&self,
			parachain_id: ::ethers::core::types::Bytes,
			id: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
			self.0
				.method_hash([122, 27, 215, 193], (parachain_id, id))
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
		///Calls the contract's `uniswapV2Router` (0x1694505e) function
		pub fn uniswap_v2_router(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([22, 148, 80, 94], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `updateHostParams` (0x6ad7df47) function
		pub fn update_host_params(
			&self,
			params: HostParams,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([106, 215, 223, 71], (params,))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `vetoes` (0x5218d908) function
		pub fn vetoes(
			&self,
			para_id: ::ethers::core::types::U256,
			height: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([82, 24, 217, 8], (para_id, height))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `withdraw` (0xcb1a6e2f) function
		pub fn withdraw(
			&self,
			params: WithdrawParams,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([203, 26, 110, 47], (params,))
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `GetRequestEvent` event
		pub fn get_request_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GetRequestEventFilter> {
			self.0.event()
		}
		///Gets the contract's `GetRequestHandled` event
		pub fn get_request_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GetRequestHandledFilter>
		{
			self.0.event()
		}
		///Gets the contract's `GetRequestTimeoutHandled` event
		pub fn get_request_timeout_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<
			::std::sync::Arc<M>,
			M,
			GetRequestTimeoutHandledFilter,
		> {
			self.0.event()
		}
		///Gets the contract's `HostFrozen` event
		pub fn host_frozen_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, HostFrozenFilter> {
			self.0.event()
		}
		///Gets the contract's `HostParamsUpdated` event
		pub fn host_params_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, HostParamsUpdatedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `HostWithdrawal` event
		pub fn host_withdrawal_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, HostWithdrawalFilter> {
			self.0.event()
		}
		///Gets the contract's `PostRequestEvent` event
		pub fn post_request_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostRequestEventFilter> {
			self.0.event()
		}
		///Gets the contract's `PostRequestHandled` event
		pub fn post_request_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostRequestHandledFilter>
		{
			self.0.event()
		}
		///Gets the contract's `PostRequestTimeoutHandled` event
		pub fn post_request_timeout_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<
			::std::sync::Arc<M>,
			M,
			PostRequestTimeoutHandledFilter,
		> {
			self.0.event()
		}
		///Gets the contract's `PostResponseEvent` event
		pub fn post_response_event_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostResponseEventFilter>
		{
			self.0.event()
		}
		///Gets the contract's `PostResponseFunded` event
		pub fn post_response_funded_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostResponseFundedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `PostResponseHandled` event
		pub fn post_response_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostResponseHandledFilter>
		{
			self.0.event()
		}
		///Gets the contract's `PostResponseTimeoutHandled` event
		pub fn post_response_timeout_handled_filter(
			&self,
		) -> ::ethers::contract::builders::Event<
			::std::sync::Arc<M>,
			M,
			PostResponseTimeoutHandledFilter,
		> {
			self.0.event()
		}
		///Gets the contract's `RequestFunded` event
		pub fn request_funded_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, RequestFundedFilter> {
			self.0.event()
		}
		///Gets the contract's `StateCommitmentRead` event
		pub fn state_commitment_read_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StateCommitmentReadFilter>
		{
			self.0.event()
		}
		///Gets the contract's `StateCommitmentVetoed` event
		pub fn state_commitment_vetoed_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StateCommitmentVetoedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `StateMachineUpdated` event
		pub fn state_machine_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StateMachineUpdatedFilter>
		{
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, EvmHostEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for EvmHost<M> {
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Custom Error type `CannotChangeFeeToken` with signature `CannotChangeFeeToken()` and
	/// selector `0x50b79e6a`
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
	#[etherror(name = "CannotChangeFeeToken", abi = "CannotChangeFeeToken()")]
	pub struct CannotChangeFeeToken;
	///Custom Error type `DuplicateResponse` with signature `DuplicateResponse()` and selector
	/// `0x276d57d8`
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
	#[etherror(name = "DuplicateResponse", abi = "DuplicateResponse()")]
	pub struct DuplicateResponse;
	///Custom Error type `FrozenHost` with signature `FrozenHost()` and selector `0x61dfed09`
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
	#[etherror(name = "FrozenHost", abi = "FrozenHost()")]
	pub struct FrozenHost;
	///Custom Error type `InvalidAddressLength` with signature `InvalidAddressLength()` and
	/// selector `0xcc2cec02`
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
	#[etherror(name = "InvalidAddressLength", abi = "InvalidAddressLength()")]
	pub struct InvalidAddressLength;
	///Custom Error type `InvalidConsensusClient` with signature `InvalidConsensusClient()` and
	/// selector `0x1435c60b`
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
	#[etherror(name = "InvalidConsensusClient", abi = "InvalidConsensusClient()")]
	pub struct InvalidConsensusClient;
	///Custom Error type `InvalidHandler` with signature `InvalidHandler()` and selector
	/// `0xd8f59fa5`
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
	#[etherror(name = "InvalidHandler", abi = "InvalidHandler()")]
	pub struct InvalidHandler;
	///Custom Error type `InvalidHostManager` with signature `InvalidHostManager()` and selector
	/// `0xd445bb14`
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
	#[etherror(name = "InvalidHostManager", abi = "InvalidHostManager()")]
	pub struct InvalidHostManager;
	///Custom Error type `InvalidHyperbridgeId` with signature `InvalidHyperbridgeId()` and
	/// selector `0x9ea9869f`
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
	#[etherror(name = "InvalidHyperbridgeId", abi = "InvalidHyperbridgeId()")]
	pub struct InvalidHyperbridgeId;
	///Custom Error type `InvalidStateMachinesLength` with signature `InvalidStateMachinesLength()`
	/// and selector `0x4f450476`
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
	#[etherror(name = "InvalidStateMachinesLength", abi = "InvalidStateMachinesLength()")]
	pub struct InvalidStateMachinesLength;
	///Custom Error type `InvalidUnstakingPeriod` with signature `InvalidUnstakingPeriod()` and
	/// selector `0xd641157e`
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
	#[etherror(name = "InvalidUnstakingPeriod", abi = "InvalidUnstakingPeriod()")]
	pub struct InvalidUnstakingPeriod;
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
	///Custom Error type `UnauthorizedResponse` with signature `UnauthorizedResponse()` and
	/// selector `0x850dc39c`
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
	#[etherror(name = "UnauthorizedResponse", abi = "UnauthorizedResponse()")]
	pub struct UnauthorizedResponse;
	///Custom Error type `UnknownRequest` with signature `UnknownRequest()` and selector
	/// `0x6d080297`
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
	#[etherror(name = "UnknownRequest", abi = "UnknownRequest()")]
	pub struct UnknownRequest;
	///Custom Error type `UnknownResponse` with signature `UnknownResponse()` and selector
	/// `0x950a5b24`
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
	#[etherror(name = "UnknownResponse", abi = "UnknownResponse()")]
	pub struct UnknownResponse;
	///Custom Error type `WithdrawalFailed` with signature `WithdrawalFailed()` and selector
	/// `0x27fcd9d1`
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
	#[etherror(name = "WithdrawalFailed", abi = "WithdrawalFailed()")]
	pub struct WithdrawalFailed;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum EvmHostErrors {
		CannotChangeFeeToken(CannotChangeFeeToken),
		DuplicateResponse(DuplicateResponse),
		FrozenHost(FrozenHost),
		InvalidAddressLength(InvalidAddressLength),
		InvalidConsensusClient(InvalidConsensusClient),
		InvalidHandler(InvalidHandler),
		InvalidHostManager(InvalidHostManager),
		InvalidHyperbridgeId(InvalidHyperbridgeId),
		InvalidStateMachinesLength(InvalidStateMachinesLength),
		InvalidUnstakingPeriod(InvalidUnstakingPeriod),
		UnauthorizedAccount(UnauthorizedAccount),
		UnauthorizedAction(UnauthorizedAction),
		UnauthorizedResponse(UnauthorizedResponse),
		UnknownRequest(UnknownRequest),
		UnknownResponse(UnknownResponse),
		WithdrawalFailed(WithdrawalFailed),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for EvmHostErrors {
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
				<CannotChangeFeeToken as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::CannotChangeFeeToken(decoded));
			}
			if let Ok(decoded) = <DuplicateResponse as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DuplicateResponse(decoded));
			}
			if let Ok(decoded) = <FrozenHost as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::FrozenHost(decoded));
			}
			if let Ok(decoded) =
				<InvalidAddressLength as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidAddressLength(decoded));
			}
			if let Ok(decoded) =
				<InvalidConsensusClient as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidConsensusClient(decoded));
			}
			if let Ok(decoded) = <InvalidHandler as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InvalidHandler(decoded));
			}
			if let Ok(decoded) =
				<InvalidHostManager as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidHostManager(decoded));
			}
			if let Ok(decoded) =
				<InvalidHyperbridgeId as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidHyperbridgeId(decoded));
			}
			if let Ok(decoded) =
				<InvalidStateMachinesLength as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidStateMachinesLength(decoded));
			}
			if let Ok(decoded) =
				<InvalidUnstakingPeriod as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidUnstakingPeriod(decoded));
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
			if let Ok(decoded) =
				<UnauthorizedResponse as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnauthorizedResponse(decoded));
			}
			if let Ok(decoded) = <UnknownRequest as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::UnknownRequest(decoded));
			}
			if let Ok(decoded) = <UnknownResponse as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::UnknownResponse(decoded));
			}
			if let Ok(decoded) = <WithdrawalFailed as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::WithdrawalFailed(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for EvmHostErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::CannotChangeFeeToken(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DuplicateResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FrozenHost(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidAddressLength(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidConsensusClient(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidHandler(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidHostManager(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidHyperbridgeId(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidStateMachinesLength(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidUnstakingPeriod(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnauthorizedAccount(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnauthorizedAction(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnauthorizedResponse(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnknownRequest(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnknownResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::WithdrawalFailed(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for EvmHostErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<CannotChangeFeeToken as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<DuplicateResponse as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <FrozenHost as ::ethers::contract::EthError>::selector() => true,
				_ if selector ==
					<InvalidAddressLength as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidConsensusClient as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidHandler as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidHostManager as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidHyperbridgeId as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidStateMachinesLength as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidUnstakingPeriod as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnauthorizedAccount as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnauthorizedAction as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnauthorizedResponse as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnknownRequest as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <UnknownResponse as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <WithdrawalFailed as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for EvmHostErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::CannotChangeFeeToken(element) => ::core::fmt::Display::fmt(element, f),
				Self::DuplicateResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::FrozenHost(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidAddressLength(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidConsensusClient(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidHandler(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidHostManager(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidHyperbridgeId(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidStateMachinesLength(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidUnstakingPeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnauthorizedAccount(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnauthorizedAction(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnauthorizedResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnknownRequest(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnknownResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::WithdrawalFailed(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for EvmHostErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<CannotChangeFeeToken> for EvmHostErrors {
		fn from(value: CannotChangeFeeToken) -> Self {
			Self::CannotChangeFeeToken(value)
		}
	}
	impl ::core::convert::From<DuplicateResponse> for EvmHostErrors {
		fn from(value: DuplicateResponse) -> Self {
			Self::DuplicateResponse(value)
		}
	}
	impl ::core::convert::From<FrozenHost> for EvmHostErrors {
		fn from(value: FrozenHost) -> Self {
			Self::FrozenHost(value)
		}
	}
	impl ::core::convert::From<InvalidAddressLength> for EvmHostErrors {
		fn from(value: InvalidAddressLength) -> Self {
			Self::InvalidAddressLength(value)
		}
	}
	impl ::core::convert::From<InvalidConsensusClient> for EvmHostErrors {
		fn from(value: InvalidConsensusClient) -> Self {
			Self::InvalidConsensusClient(value)
		}
	}
	impl ::core::convert::From<InvalidHandler> for EvmHostErrors {
		fn from(value: InvalidHandler) -> Self {
			Self::InvalidHandler(value)
		}
	}
	impl ::core::convert::From<InvalidHostManager> for EvmHostErrors {
		fn from(value: InvalidHostManager) -> Self {
			Self::InvalidHostManager(value)
		}
	}
	impl ::core::convert::From<InvalidHyperbridgeId> for EvmHostErrors {
		fn from(value: InvalidHyperbridgeId) -> Self {
			Self::InvalidHyperbridgeId(value)
		}
	}
	impl ::core::convert::From<InvalidStateMachinesLength> for EvmHostErrors {
		fn from(value: InvalidStateMachinesLength) -> Self {
			Self::InvalidStateMachinesLength(value)
		}
	}
	impl ::core::convert::From<InvalidUnstakingPeriod> for EvmHostErrors {
		fn from(value: InvalidUnstakingPeriod) -> Self {
			Self::InvalidUnstakingPeriod(value)
		}
	}
	impl ::core::convert::From<UnauthorizedAccount> for EvmHostErrors {
		fn from(value: UnauthorizedAccount) -> Self {
			Self::UnauthorizedAccount(value)
		}
	}
	impl ::core::convert::From<UnauthorizedAction> for EvmHostErrors {
		fn from(value: UnauthorizedAction) -> Self {
			Self::UnauthorizedAction(value)
		}
	}
	impl ::core::convert::From<UnauthorizedResponse> for EvmHostErrors {
		fn from(value: UnauthorizedResponse) -> Self {
			Self::UnauthorizedResponse(value)
		}
	}
	impl ::core::convert::From<UnknownRequest> for EvmHostErrors {
		fn from(value: UnknownRequest) -> Self {
			Self::UnknownRequest(value)
		}
	}
	impl ::core::convert::From<UnknownResponse> for EvmHostErrors {
		fn from(value: UnknownResponse) -> Self {
			Self::UnknownResponse(value)
		}
	}
	impl ::core::convert::From<WithdrawalFailed> for EvmHostErrors {
		fn from(value: WithdrawalFailed) -> Self {
			Self::WithdrawalFailed(value)
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
		Hash,
	)]
	#[ethevent(
		name = "GetRequestEvent",
		abi = "GetRequestEvent(string,string,address,bytes[],uint256,uint256,uint256,bytes,uint256)"
	)]
	pub struct GetRequestEventFilter {
		pub source: ::std::string::String,
		pub dest: ::std::string::String,
		#[ethevent(indexed)]
		pub from: ::ethers::core::types::Address,
		pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
		pub height: ::ethers::core::types::U256,
		pub nonce: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub context: ::ethers::core::types::Bytes,
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
		Hash,
	)]
	#[ethevent(name = "GetRequestHandled", abi = "GetRequestHandled(bytes32,address)")]
	pub struct GetRequestHandledFilter {
		#[ethevent(indexed)]
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
		Hash,
	)]
	#[ethevent(name = "GetRequestTimeoutHandled", abi = "GetRequestTimeoutHandled(bytes32,string)")]
	pub struct GetRequestTimeoutHandledFilter {
		#[ethevent(indexed)]
		pub commitment: [u8; 32],
		pub dest: ::std::string::String,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "HostFrozen", abi = "HostFrozen(uint8)")]
	pub struct HostFrozenFilter {
		pub status: u8,
	}
	#[derive(Clone, ::ethers::contract::EthEvent, ::ethers::contract::EthDisplay)]
	#[ethevent(
		name = "HostParamsUpdated",
		abi = "HostParamsUpdated((uint256,uint256,uint256,address,address,address,address,address,uint256,uint256,address,uint256[],(bytes32,uint256)[],bytes),(uint256,uint256,uint256,address,address,address,address,address,uint256,uint256,address,uint256[],(bytes32,uint256)[],bytes))"
	)]
	pub struct HostParamsUpdatedFilter {
		pub old_params: HostParams,
		pub new_params: HostParams,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "HostWithdrawal", abi = "HostWithdrawal(uint256,address,bool)")]
	pub struct HostWithdrawalFilter {
		pub amount: ::ethers::core::types::U256,
		pub beneficiary: ::ethers::core::types::Address,
		pub native: bool,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(
		name = "PostRequestEvent",
		abi = "PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)"
	)]
	pub struct PostRequestEventFilter {
		pub source: ::std::string::String,
		pub dest: ::std::string::String,
		#[ethevent(indexed)]
		pub from: ::ethers::core::types::Address,
		pub to: ::ethers::core::types::Bytes,
		pub nonce: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub body: ::ethers::core::types::Bytes,
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
		Hash,
	)]
	#[ethevent(name = "PostRequestHandled", abi = "PostRequestHandled(bytes32,address)")]
	pub struct PostRequestHandledFilter {
		#[ethevent(indexed)]
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
		Hash,
	)]
	#[ethevent(
		name = "PostRequestTimeoutHandled",
		abi = "PostRequestTimeoutHandled(bytes32,string)"
	)]
	pub struct PostRequestTimeoutHandledFilter {
		#[ethevent(indexed)]
		pub commitment: [u8; 32],
		pub dest: ::std::string::String,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(
		name = "PostResponseEvent",
		abi = "PostResponseEvent(string,string,address,bytes,uint256,uint256,bytes,bytes,uint256,uint256)"
	)]
	pub struct PostResponseEventFilter {
		pub source: ::std::string::String,
		pub dest: ::std::string::String,
		#[ethevent(indexed)]
		pub from: ::ethers::core::types::Address,
		pub to: ::ethers::core::types::Bytes,
		pub nonce: ::ethers::core::types::U256,
		pub timeout_timestamp: ::ethers::core::types::U256,
		pub body: ::ethers::core::types::Bytes,
		pub response: ::ethers::core::types::Bytes,
		pub response_timeout_timestamp: ::ethers::core::types::U256,
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
		Hash,
	)]
	#[ethevent(name = "PostResponseFunded", abi = "PostResponseFunded(bytes32,uint256)")]
	pub struct PostResponseFundedFilter {
		#[ethevent(indexed)]
		pub commitment: [u8; 32],
		pub new_fee: ::ethers::core::types::U256,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "PostResponseHandled", abi = "PostResponseHandled(bytes32,address)")]
	pub struct PostResponseHandledFilter {
		#[ethevent(indexed)]
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
		Hash,
	)]
	#[ethevent(
		name = "PostResponseTimeoutHandled",
		abi = "PostResponseTimeoutHandled(bytes32,string)"
	)]
	pub struct PostResponseTimeoutHandledFilter {
		#[ethevent(indexed)]
		pub commitment: [u8; 32],
		pub dest: ::std::string::String,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "RequestFunded", abi = "RequestFunded(bytes32,uint256)")]
	pub struct RequestFundedFilter {
		#[ethevent(indexed)]
		pub commitment: [u8; 32],
		pub new_fee: ::ethers::core::types::U256,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "StateCommitmentRead", abi = "StateCommitmentRead(address,uint256)")]
	pub struct StateCommitmentReadFilter {
		#[ethevent(indexed)]
		pub caller: ::ethers::core::types::Address,
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
		Hash,
	)]
	#[ethevent(
		name = "StateCommitmentVetoed",
		abi = "StateCommitmentVetoed(string,uint256,(uint256,bytes32,bytes32),address)"
	)]
	pub struct StateCommitmentVetoedFilter {
		pub state_machine_id: ::std::string::String,
		pub height: ::ethers::core::types::U256,
		pub state_commitment: StateCommitment,
		#[ethevent(indexed)]
		pub fisherman: ::ethers::core::types::Address,
	}
	#[derive(
		Clone,
		::ethers::contract::EthEvent,
		::ethers::contract::EthDisplay,
		Default,
		Debug,
		PartialEq,
		Eq,
		Hash,
	)]
	#[ethevent(name = "StateMachineUpdated", abi = "StateMachineUpdated(string,uint256)")]
	pub struct StateMachineUpdatedFilter {
		pub state_machine_id: ::std::string::String,
		pub height: ::ethers::core::types::U256,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType)]
	pub enum EvmHostEvents {
		GetRequestEventFilter(GetRequestEventFilter),
		GetRequestHandledFilter(GetRequestHandledFilter),
		GetRequestTimeoutHandledFilter(GetRequestTimeoutHandledFilter),
		HostFrozenFilter(HostFrozenFilter),
		HostParamsUpdatedFilter(HostParamsUpdatedFilter),
		HostWithdrawalFilter(HostWithdrawalFilter),
		PostRequestEventFilter(PostRequestEventFilter),
		PostRequestHandledFilter(PostRequestHandledFilter),
		PostRequestTimeoutHandledFilter(PostRequestTimeoutHandledFilter),
		PostResponseEventFilter(PostResponseEventFilter),
		PostResponseFundedFilter(PostResponseFundedFilter),
		PostResponseHandledFilter(PostResponseHandledFilter),
		PostResponseTimeoutHandledFilter(PostResponseTimeoutHandledFilter),
		RequestFundedFilter(RequestFundedFilter),
		StateCommitmentReadFilter(StateCommitmentReadFilter),
		StateCommitmentVetoedFilter(StateCommitmentVetoedFilter),
		StateMachineUpdatedFilter(StateMachineUpdatedFilter),
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
			if let Ok(decoded) = GetRequestTimeoutHandledFilter::decode_log(log) {
				return Ok(EvmHostEvents::GetRequestTimeoutHandledFilter(decoded));
			}
			if let Ok(decoded) = HostFrozenFilter::decode_log(log) {
				return Ok(EvmHostEvents::HostFrozenFilter(decoded));
			}
			if let Ok(decoded) = HostParamsUpdatedFilter::decode_log(log) {
				return Ok(EvmHostEvents::HostParamsUpdatedFilter(decoded));
			}
			if let Ok(decoded) = HostWithdrawalFilter::decode_log(log) {
				return Ok(EvmHostEvents::HostWithdrawalFilter(decoded));
			}
			if let Ok(decoded) = PostRequestEventFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostRequestEventFilter(decoded));
			}
			if let Ok(decoded) = PostRequestHandledFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostRequestHandledFilter(decoded));
			}
			if let Ok(decoded) = PostRequestTimeoutHandledFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostRequestTimeoutHandledFilter(decoded));
			}
			if let Ok(decoded) = PostResponseEventFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostResponseEventFilter(decoded));
			}
			if let Ok(decoded) = PostResponseFundedFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostResponseFundedFilter(decoded));
			}
			if let Ok(decoded) = PostResponseHandledFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostResponseHandledFilter(decoded));
			}
			if let Ok(decoded) = PostResponseTimeoutHandledFilter::decode_log(log) {
				return Ok(EvmHostEvents::PostResponseTimeoutHandledFilter(decoded));
			}
			if let Ok(decoded) = RequestFundedFilter::decode_log(log) {
				return Ok(EvmHostEvents::RequestFundedFilter(decoded));
			}
			if let Ok(decoded) = StateCommitmentReadFilter::decode_log(log) {
				return Ok(EvmHostEvents::StateCommitmentReadFilter(decoded));
			}
			if let Ok(decoded) = StateCommitmentVetoedFilter::decode_log(log) {
				return Ok(EvmHostEvents::StateCommitmentVetoedFilter(decoded));
			}
			if let Ok(decoded) = StateMachineUpdatedFilter::decode_log(log) {
				return Ok(EvmHostEvents::StateMachineUpdatedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for EvmHostEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::GetRequestEventFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetRequestHandledFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetRequestTimeoutHandledFilter(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::HostFrozenFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::HostParamsUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::HostWithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostRequestEventFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostRequestHandledFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostRequestTimeoutHandledFilter(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::PostResponseEventFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostResponseFundedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostResponseHandledFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::PostResponseTimeoutHandledFilter(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::RequestFundedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateCommitmentReadFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateCommitmentVetoedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateMachineUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
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
	impl ::core::convert::From<GetRequestTimeoutHandledFilter> for EvmHostEvents {
		fn from(value: GetRequestTimeoutHandledFilter) -> Self {
			Self::GetRequestTimeoutHandledFilter(value)
		}
	}
	impl ::core::convert::From<HostFrozenFilter> for EvmHostEvents {
		fn from(value: HostFrozenFilter) -> Self {
			Self::HostFrozenFilter(value)
		}
	}
	impl ::core::convert::From<HostParamsUpdatedFilter> for EvmHostEvents {
		fn from(value: HostParamsUpdatedFilter) -> Self {
			Self::HostParamsUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<HostWithdrawalFilter> for EvmHostEvents {
		fn from(value: HostWithdrawalFilter) -> Self {
			Self::HostWithdrawalFilter(value)
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
	impl ::core::convert::From<PostRequestTimeoutHandledFilter> for EvmHostEvents {
		fn from(value: PostRequestTimeoutHandledFilter) -> Self {
			Self::PostRequestTimeoutHandledFilter(value)
		}
	}
	impl ::core::convert::From<PostResponseEventFilter> for EvmHostEvents {
		fn from(value: PostResponseEventFilter) -> Self {
			Self::PostResponseEventFilter(value)
		}
	}
	impl ::core::convert::From<PostResponseFundedFilter> for EvmHostEvents {
		fn from(value: PostResponseFundedFilter) -> Self {
			Self::PostResponseFundedFilter(value)
		}
	}
	impl ::core::convert::From<PostResponseHandledFilter> for EvmHostEvents {
		fn from(value: PostResponseHandledFilter) -> Self {
			Self::PostResponseHandledFilter(value)
		}
	}
	impl ::core::convert::From<PostResponseTimeoutHandledFilter> for EvmHostEvents {
		fn from(value: PostResponseTimeoutHandledFilter) -> Self {
			Self::PostResponseTimeoutHandledFilter(value)
		}
	}
	impl ::core::convert::From<RequestFundedFilter> for EvmHostEvents {
		fn from(value: RequestFundedFilter) -> Self {
			Self::RequestFundedFilter(value)
		}
	}
	impl ::core::convert::From<StateCommitmentReadFilter> for EvmHostEvents {
		fn from(value: StateCommitmentReadFilter) -> Self {
			Self::StateCommitmentReadFilter(value)
		}
	}
	impl ::core::convert::From<StateCommitmentVetoedFilter> for EvmHostEvents {
		fn from(value: StateCommitmentVetoedFilter) -> Self {
			Self::StateCommitmentVetoedFilter(value)
		}
	}
	impl ::core::convert::From<StateMachineUpdatedFilter> for EvmHostEvents {
		fn from(value: StateMachineUpdatedFilter) -> Self {
			Self::StateMachineUpdatedFilter(value)
		}
	}
	///Container type for all input parameters for the `admin` function with signature `admin()`
	/// and selector `0xf851a440`
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
	#[ethcall(name = "admin", abi = "admin()")]
	pub struct AdminCall;
	///Container type for all input parameters for the `chainId` function with signature
	/// `chainId()` and selector `0x9a8a0592`
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
	#[ethcall(name = "chainId", abi = "chainId()")]
	pub struct ChainIdCall;
	///Container type for all input parameters for the `challengePeriod` function with signature
	/// `challengePeriod()` and selector `0xf3f480d9`
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
	#[ethcall(name = "challengePeriod", abi = "challengePeriod()")]
	pub struct ChallengePeriodCall;
	///Container type for all input parameters for the `consensusClient` function with signature
	/// `consensusClient()` and selector `0x2476132b`
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
	#[ethcall(name = "consensusClient", abi = "consensusClient()")]
	pub struct ConsensusClientCall;
	///Container type for all input parameters for the `consensusState` function with signature
	/// `consensusState()` and selector `0xbbad99d4`
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
	#[ethcall(name = "consensusState", abi = "consensusState()")]
	pub struct ConsensusStateCall;
	///Container type for all input parameters for the `consensusUpdateTime` function with
	/// signature `consensusUpdateTime()` and selector `0x9a8425bc`
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
	#[ethcall(name = "consensusUpdateTime", abi = "consensusUpdateTime()")]
	pub struct ConsensusUpdateTimeCall;
	///Container type for all input parameters for the `deleteStateMachineCommitment` function with
	/// signature `deleteStateMachineCommitment((uint256,uint256),address)` and selector
	/// `0xf8ddf259`
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
		name = "deleteStateMachineCommitment",
		abi = "deleteStateMachineCommitment((uint256,uint256),address)"
	)]
	pub struct DeleteStateMachineCommitmentCall {
		pub height: StateMachineHeight,
		pub fisherman: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and
	/// selector `0x94480805`
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
		name = "dispatch",
		abi = "dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))"
	)]
	pub struct DispatchCall {
		pub post: DispatchPost,
	}
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch((bytes,bytes,bytes,uint64,uint256,address))` and selector `0xb8f3e8f5`
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
	#[ethcall(name = "dispatch", abi = "dispatch((bytes,bytes,bytes,uint64,uint256,address))")]
	pub struct DispatchWithPostCall {
		pub post: DispatchPost,
	}
	///Container type for all input parameters for the `dispatch` function with signature
	/// `dispatch((bytes,uint64,bytes[],uint64,uint256,bytes))` and selector `0xd22e3343`
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
	#[ethcall(name = "dispatch", abi = "dispatch((bytes,uint64,bytes[],uint64,uint256,bytes))")]
	pub struct DispatchWithGetCall {
		pub get: DispatchGet,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address)` and
	/// selector `0xab013de1`
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
		name = "dispatchIncoming",
		abi = "dispatchIncoming(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),address)"
	)]
	pub struct DispatchIncomingCall {
		pub response: GetResponse,
		pub relayer: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address)` and selector
	/// `0xb85e6fbb`
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
		name = "dispatchIncoming",
		abi = "dispatchIncoming((bytes,bytes,uint64,bytes,bytes,uint64,bytes),address)"
	)]
	pub struct DispatchIncomingWithRequestCall {
		pub request: PostRequest,
		pub relayer: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `dispatchIncoming` function with signature
	/// `dispatchIncoming(((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,
	/// bytes)[]),address)` and selector `0xfc8a341c`
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
		name = "dispatchIncoming",
		abi = "dispatchIncoming(((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(bytes,bytes)[]),address)"
	)]
	pub struct DispatchIncomingWithResponseCall {
		pub response: GetResponse,
		pub relayer: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `dispatchTimeOut` function with signature
	/// `dispatchTimeOut(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),(uint256,
	/// address),bytes32)` and selector `0x0446fc47`
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
		name = "dispatchTimeOut",
		abi = "dispatchTimeOut(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),(uint256,address),bytes32)"
	)]
	pub struct DispatchTimeOut0Call {
		pub response: PostResponse,
		pub meta: FeeMetadata,
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `dispatchTimeOut` function with signature
	/// `dispatchTimeOut((bytes,bytes,uint64,bytes,bytes,uint64,bytes),(uint256,address),bytes32)`
	/// and selector `0x4b46efaa`
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
		name = "dispatchTimeOut",
		abi = "dispatchTimeOut((bytes,bytes,uint64,bytes,bytes,uint64,bytes),(uint256,address),bytes32)"
	)]
	pub struct DispatchTimeOut1Call {
		pub request: GetRequest,
		pub meta: FeeMetadata,
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `dispatchTimeOut` function with signature
	/// `dispatchTimeOut((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(uint256,address),
	/// bytes32)` and selector `0x6d6c2313`
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
		name = "dispatchTimeOut",
		abi = "dispatchTimeOut((bytes,bytes,uint64,address,uint64,bytes[],uint64,bytes),(uint256,address),bytes32)"
	)]
	pub struct DispatchTimeOut2Call {
		pub request: GetRequest,
		pub meta: FeeMetadata,
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `feeToken` function with signature
	/// `feeToken()` and selector `0x647846a5`
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
	#[ethcall(name = "feeToken", abi = "feeToken()")]
	pub struct FeeTokenCall;
	///Container type for all input parameters for the `frozen` function with signature `frozen()`
	/// and selector `0x054f7d9c`
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
	#[ethcall(name = "frozen", abi = "frozen()")]
	pub struct FrozenCall;
	///Container type for all input parameters for the `fundRequest` function with signature
	/// `fundRequest(bytes32,uint256)` and selector `0xb9ea3289`
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
	#[ethcall(name = "fundRequest", abi = "fundRequest(bytes32,uint256)")]
	pub struct FundRequestCall {
		pub commitment: [u8; 32],
		pub amount: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `fundResponse` function with signature
	/// `fundResponse(bytes32,uint256)` and selector `0xfadce3c7`
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
	#[ethcall(name = "fundResponse", abi = "fundResponse(bytes32,uint256)")]
	pub struct FundResponseCall {
		pub commitment: [u8; 32],
		pub amount: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `host` function with signature `host()` and
	/// selector `0xf437bc59`
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
	#[ethcall(name = "host", abi = "host()")]
	pub struct HostCall;
	///Container type for all input parameters for the `hostParams` function with signature
	/// `hostParams()` and selector `0x2215364d`
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
	#[ethcall(name = "hostParams", abi = "hostParams()")]
	pub struct HostParamsCall;
	///Container type for all input parameters for the `hyperbridge` function with signature
	/// `hyperbridge()` and selector `0x005e763e`
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
	#[ethcall(name = "hyperbridge", abi = "hyperbridge()")]
	pub struct HyperbridgeCall;
	///Container type for all input parameters for the `latestStateMachineHeight` function with
	/// signature `latestStateMachineHeight(uint256)` and selector `0x9c095f86`
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
	#[ethcall(name = "latestStateMachineHeight", abi = "latestStateMachineHeight(uint256)")]
	pub struct LatestStateMachineHeightCall {
		pub id: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `nonce` function with signature `nonce()`
	/// and selector `0xaffed0e0`
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
	#[ethcall(name = "nonce", abi = "nonce()")]
	pub struct NonceCall;
	///Container type for all input parameters for the `perByteFee` function with signature
	/// `perByteFee(bytes)` and selector `0x4011ec0a`
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
	#[ethcall(name = "perByteFee", abi = "perByteFee(bytes)")]
	pub struct PerByteFeeCall {
		pub state_id: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `requestCommitments` function with signature
	/// `requestCommitments(bytes32)` and selector `0x368bf464`
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
	#[ethcall(name = "requestCommitments", abi = "requestCommitments(bytes32)")]
	pub struct RequestCommitmentsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `requestReceipts` function with signature
	/// `requestReceipts(bytes32)` and selector `0x19667a3e`
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
	#[ethcall(name = "requestReceipts", abi = "requestReceipts(bytes32)")]
	pub struct RequestReceiptsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `responded` function with signature
	/// `responded(bytes32)` and selector `0x73bfa8ae`
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
	#[ethcall(name = "responded", abi = "responded(bytes32)")]
	pub struct RespondedCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `responseCommitments` function with
	/// signature `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
	#[ethcall(name = "responseCommitments", abi = "responseCommitments(bytes32)")]
	pub struct ResponseCommitmentsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `responseReceipts` function with signature
	/// `responseReceipts(bytes32)` and selector `0x8856337e`
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
	#[ethcall(name = "responseReceipts", abi = "responseReceipts(bytes32)")]
	pub struct ResponseReceiptsCall {
		pub commitment: [u8; 32],
	}
	///Container type for all input parameters for the `setConsensusState` function with signature
	/// `setConsensusState(bytes,(uint256,uint256),(uint256,bytes32,bytes32))` and selector
	/// `0x8d48f3c7`
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
		name = "setConsensusState",
		abi = "setConsensusState(bytes,(uint256,uint256),(uint256,bytes32,bytes32))"
	)]
	pub struct SetConsensusStateCall {
		pub state: ::ethers::core::types::Bytes,
		pub height: StateMachineHeight,
		pub commitment: StateCommitment,
	}
	///Container type for all input parameters for the `setFrozenState` function with signature
	/// `setFrozenState(uint8)` and selector `0x6ac88bad`
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
	#[ethcall(name = "setFrozenState", abi = "setFrozenState(uint8)")]
	pub struct SetFrozenStateCall {
		pub new_state: u8,
	}
	///Container type for all input parameters for the `stateCommitmentFee` function with signature
	/// `stateCommitmentFee()` and selector `0xb5c8a08e`
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
	#[ethcall(name = "stateCommitmentFee", abi = "stateCommitmentFee()")]
	pub struct StateCommitmentFeeCall;
	///Container type for all input parameters for the `stateMachineCommitment` function with
	/// signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
	#[ethcall(name = "stateMachineCommitment", abi = "stateMachineCommitment((uint256,uint256))")]
	pub struct StateMachineCommitmentCall {
		pub height: StateMachineHeight,
	}
	///Container type for all input parameters for the `stateMachineCommitmentUpdateTime` function
	/// with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector
	/// `0x1a880a93`
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
		name = "stateMachineCommitmentUpdateTime",
		abi = "stateMachineCommitmentUpdateTime((uint256,uint256))"
	)]
	pub struct StateMachineCommitmentUpdateTimeCall {
		pub height: StateMachineHeight,
	}
	///Container type for all input parameters for the `stateMachineId` function with signature
	/// `stateMachineId(bytes,uint256)` and selector `0x7a1bd7c1`
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
	#[ethcall(name = "stateMachineId", abi = "stateMachineId(bytes,uint256)")]
	pub struct StateMachineIdCall {
		pub parachain_id: ::ethers::core::types::Bytes,
		pub id: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `storeConsensusState` function with
	/// signature `storeConsensusState(bytes)` and selector `0xb4974cf0`
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
	#[ethcall(name = "storeConsensusState", abi = "storeConsensusState(bytes)")]
	pub struct StoreConsensusStateCall {
		pub state: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `storeStateMachineCommitment` function with
	/// signature `storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))` and
	/// selector `0x559efe9e`
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
		name = "storeStateMachineCommitment",
		abi = "storeStateMachineCommitment((uint256,uint256),(uint256,bytes32,bytes32))"
	)]
	pub struct StoreStateMachineCommitmentCall {
		pub height: StateMachineHeight,
		pub commitment: StateCommitment,
	}
	///Container type for all input parameters for the `timestamp` function with signature
	/// `timestamp()` and selector `0xb80777ea`
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
	#[ethcall(name = "timestamp", abi = "timestamp()")]
	pub struct TimestampCall;
	///Container type for all input parameters for the `unStakingPeriod` function with signature
	/// `unStakingPeriod()` and selector `0xd40784c7`
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
	#[ethcall(name = "unStakingPeriod", abi = "unStakingPeriod()")]
	pub struct UnStakingPeriodCall;
	///Container type for all input parameters for the `uniswapV2Router` function with signature
	/// `uniswapV2Router()` and selector `0x1694505e`
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
	#[ethcall(name = "uniswapV2Router", abi = "uniswapV2Router()")]
	pub struct UniswapV2RouterCall;
	///Container type for all input parameters for the `updateHostParams` function with signature
	/// `updateHostParams((uint256,uint256,uint256,address,address,address,address,address,uint256,
	/// uint256,address,uint256[],(bytes32,uint256)[],bytes))` and selector `0x6ad7df47`
	#[derive(Clone, ::ethers::contract::EthCall, ::ethers::contract::EthDisplay)]
	#[ethcall(
		name = "updateHostParams",
		abi = "updateHostParams((uint256,uint256,uint256,address,address,address,address,address,uint256,uint256,address,uint256[],(bytes32,uint256)[],bytes))"
	)]
	pub struct UpdateHostParamsCall {
		pub params: HostParams,
	}
	///Container type for all input parameters for the `vetoes` function with signature
	/// `vetoes(uint256,uint256)` and selector `0x5218d908`
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
	#[ethcall(name = "vetoes", abi = "vetoes(uint256,uint256)")]
	pub struct VetoesCall {
		pub para_id: ::ethers::core::types::U256,
		pub height: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `withdraw` function with signature
	/// `withdraw((address,uint256,bool))` and selector `0xcb1a6e2f`
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
	#[ethcall(name = "withdraw", abi = "withdraw((address,uint256,bool))")]
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
		DeleteStateMachineCommitment(DeleteStateMachineCommitmentCall),
		Dispatch(DispatchCall),
		DispatchWithPost(DispatchWithPostCall),
		DispatchWithGet(DispatchWithGetCall),
		DispatchIncoming(DispatchIncomingCall),
		DispatchIncomingWithRequest(DispatchIncomingWithRequestCall),
		DispatchIncomingWithResponse(DispatchIncomingWithResponseCall),
		DispatchTimeOut0(DispatchTimeOut0Call),
		DispatchTimeOut1(DispatchTimeOut1Call),
		DispatchTimeOut2(DispatchTimeOut2Call),
		FeeToken(FeeTokenCall),
		Frozen(FrozenCall),
		FundRequest(FundRequestCall),
		FundResponse(FundResponseCall),
		Host(HostCall),
		HostParams(HostParamsCall),
		Hyperbridge(HyperbridgeCall),
		LatestStateMachineHeight(LatestStateMachineHeightCall),
		Nonce(NonceCall),
		PerByteFee(PerByteFeeCall),
		RequestCommitments(RequestCommitmentsCall),
		RequestReceipts(RequestReceiptsCall),
		Responded(RespondedCall),
		ResponseCommitments(ResponseCommitmentsCall),
		ResponseReceipts(ResponseReceiptsCall),
		SetConsensusState(SetConsensusStateCall),
		SetFrozenState(SetFrozenStateCall),
		StateCommitmentFee(StateCommitmentFeeCall),
		StateMachineCommitment(StateMachineCommitmentCall),
		StateMachineCommitmentUpdateTime(StateMachineCommitmentUpdateTimeCall),
		StateMachineId(StateMachineIdCall),
		StoreConsensusState(StoreConsensusStateCall),
		StoreStateMachineCommitment(StoreStateMachineCommitmentCall),
		Timestamp(TimestampCall),
		UnStakingPeriod(UnStakingPeriodCall),
		UniswapV2Router(UniswapV2RouterCall),
		UpdateHostParams(UpdateHostParamsCall),
		Vetoes(VetoesCall),
		Withdraw(WithdrawCall),
	}
	impl ::ethers::core::abi::AbiDecode for EvmHostCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <AdminCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Admin(decoded));
			}
			if let Ok(decoded) = <ChainIdCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ChainId(decoded));
			}
			if let Ok(decoded) =
				<ChallengePeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ChallengePeriod(decoded));
			}
			if let Ok(decoded) =
				<ConsensusClientCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusClient(decoded));
			}
			if let Ok(decoded) =
				<ConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusState(decoded));
			}
			if let Ok(decoded) =
				<ConsensusUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConsensusUpdateTime(decoded));
			}
			if let Ok(decoded) =
				<DeleteStateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DeleteStateMachineCommitment(decoded));
			}
			if let Ok(decoded) = <DispatchCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Dispatch(decoded));
			}
			if let Ok(decoded) =
				<DispatchWithPostCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchWithPost(decoded));
			}
			if let Ok(decoded) =
				<DispatchWithGetCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchWithGet(decoded));
			}
			if let Ok(decoded) =
				<DispatchIncomingCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncoming(decoded));
			}
			if let Ok(decoded) =
				<DispatchIncomingWithRequestCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncomingWithRequest(decoded));
			}
			if let Ok(decoded) =
				<DispatchIncomingWithResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchIncomingWithResponse(decoded));
			}
			if let Ok(decoded) =
				<DispatchTimeOut0Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchTimeOut0(decoded));
			}
			if let Ok(decoded) =
				<DispatchTimeOut1Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchTimeOut1(decoded));
			}
			if let Ok(decoded) =
				<DispatchTimeOut2Call as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::DispatchTimeOut2(decoded));
			}
			if let Ok(decoded) = <FeeTokenCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::FeeToken(decoded));
			}
			if let Ok(decoded) = <FrozenCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Frozen(decoded));
			}
			if let Ok(decoded) = <FundRequestCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::FundRequest(decoded));
			}
			if let Ok(decoded) = <FundResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::FundResponse(decoded));
			}
			if let Ok(decoded) = <HostCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Host(decoded));
			}
			if let Ok(decoded) = <HostParamsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::HostParams(decoded));
			}
			if let Ok(decoded) = <HyperbridgeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Hyperbridge(decoded));
			}
			if let Ok(decoded) =
				<LatestStateMachineHeightCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestStateMachineHeight(decoded));
			}
			if let Ok(decoded) = <NonceCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Nonce(decoded));
			}
			if let Ok(decoded) = <PerByteFeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::PerByteFee(decoded));
			}
			if let Ok(decoded) =
				<RequestCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RequestCommitments(decoded));
			}
			if let Ok(decoded) =
				<RequestReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RequestReceipts(decoded));
			}
			if let Ok(decoded) = <RespondedCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Responded(decoded));
			}
			if let Ok(decoded) =
				<ResponseCommitmentsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResponseCommitments(decoded));
			}
			if let Ok(decoded) =
				<ResponseReceiptsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ResponseReceipts(decoded));
			}
			if let Ok(decoded) =
				<SetConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetConsensusState(decoded));
			}
			if let Ok(decoded) =
				<SetFrozenStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SetFrozenState(decoded));
			}
			if let Ok(decoded) =
				<StateCommitmentFeeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StateCommitmentFee(decoded));
			}
			if let Ok(decoded) =
				<StateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StateMachineCommitment(decoded));
			}
			if let Ok(decoded) =
				<StateMachineCommitmentUpdateTimeCall as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::StateMachineCommitmentUpdateTime(decoded));
			}
			if let Ok(decoded) =
				<StateMachineIdCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StateMachineId(decoded));
			}
			if let Ok(decoded) =
				<StoreConsensusStateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreConsensusState(decoded));
			}
			if let Ok(decoded) =
				<StoreStateMachineCommitmentCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::StoreStateMachineCommitment(decoded));
			}
			if let Ok(decoded) = <TimestampCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Timestamp(decoded));
			}
			if let Ok(decoded) =
				<UnStakingPeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnStakingPeriod(decoded));
			}
			if let Ok(decoded) =
				<UniswapV2RouterCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UniswapV2Router(decoded));
			}
			if let Ok(decoded) =
				<UpdateHostParamsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UpdateHostParams(decoded));
			}
			if let Ok(decoded) = <VetoesCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Vetoes(decoded));
			}
			if let Ok(decoded) = <WithdrawCall as ::ethers::core::abi::AbiDecode>::decode(data) {
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
				Self::ChallengePeriod(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusClient(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConsensusUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DeleteStateMachineCommitment(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Dispatch(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchWithPost(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchWithGet(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncoming(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncomingWithRequest(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchIncomingWithResponse(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchTimeOut0(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchTimeOut1(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::DispatchTimeOut2(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FeeToken(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Frozen(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FundRequest(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::FundResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Host(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::HostParams(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Hyperbridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestStateMachineHeight(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Nonce(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::PerByteFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RequestCommitments(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RequestReceipts(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Responded(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ResponseCommitments(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ResponseReceipts(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetConsensusState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetFrozenState(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StateCommitmentFee(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StateMachineCommitment(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StateMachineCommitmentUpdateTime(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StateMachineId(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreConsensusState(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StoreStateMachineCommitment(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Timestamp(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnStakingPeriod(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UniswapV2Router(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UpdateHostParams(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Vetoes(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Withdraw(element) => ::ethers::core::abi::AbiEncode::encode(element),
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
				Self::ConsensusUpdateTime(element) => ::core::fmt::Display::fmt(element, f),
				Self::DeleteStateMachineCommitment(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::Dispatch(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchWithPost(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchWithGet(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncoming(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncomingWithRequest(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchIncomingWithResponse(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::DispatchTimeOut0(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchTimeOut1(element) => ::core::fmt::Display::fmt(element, f),
				Self::DispatchTimeOut2(element) => ::core::fmt::Display::fmt(element, f),
				Self::FeeToken(element) => ::core::fmt::Display::fmt(element, f),
				Self::Frozen(element) => ::core::fmt::Display::fmt(element, f),
				Self::FundRequest(element) => ::core::fmt::Display::fmt(element, f),
				Self::FundResponse(element) => ::core::fmt::Display::fmt(element, f),
				Self::Host(element) => ::core::fmt::Display::fmt(element, f),
				Self::HostParams(element) => ::core::fmt::Display::fmt(element, f),
				Self::Hyperbridge(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestStateMachineHeight(element) => ::core::fmt::Display::fmt(element, f),
				Self::Nonce(element) => ::core::fmt::Display::fmt(element, f),
				Self::PerByteFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::RequestCommitments(element) => ::core::fmt::Display::fmt(element, f),
				Self::RequestReceipts(element) => ::core::fmt::Display::fmt(element, f),
				Self::Responded(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResponseCommitments(element) => ::core::fmt::Display::fmt(element, f),
				Self::ResponseReceipts(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetConsensusState(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetFrozenState(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateCommitmentFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateMachineCommitment(element) => ::core::fmt::Display::fmt(element, f),
				Self::StateMachineCommitmentUpdateTime(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::StateMachineId(element) => ::core::fmt::Display::fmt(element, f),
				Self::StoreConsensusState(element) => ::core::fmt::Display::fmt(element, f),
				Self::StoreStateMachineCommitment(element) => ::core::fmt::Display::fmt(element, f),
				Self::Timestamp(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnStakingPeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::UniswapV2Router(element) => ::core::fmt::Display::fmt(element, f),
				Self::UpdateHostParams(element) => ::core::fmt::Display::fmt(element, f),
				Self::Vetoes(element) => ::core::fmt::Display::fmt(element, f),
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
	impl ::core::convert::From<DeleteStateMachineCommitmentCall> for EvmHostCalls {
		fn from(value: DeleteStateMachineCommitmentCall) -> Self {
			Self::DeleteStateMachineCommitment(value)
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
	impl ::core::convert::From<DispatchIncomingCall> for EvmHostCalls {
		fn from(value: DispatchIncomingCall) -> Self {
			Self::DispatchIncoming(value)
		}
	}
	impl ::core::convert::From<DispatchIncomingWithRequestCall> for EvmHostCalls {
		fn from(value: DispatchIncomingWithRequestCall) -> Self {
			Self::DispatchIncomingWithRequest(value)
		}
	}
	impl ::core::convert::From<DispatchIncomingWithResponseCall> for EvmHostCalls {
		fn from(value: DispatchIncomingWithResponseCall) -> Self {
			Self::DispatchIncomingWithResponse(value)
		}
	}
	impl ::core::convert::From<DispatchTimeOut0Call> for EvmHostCalls {
		fn from(value: DispatchTimeOut0Call) -> Self {
			Self::DispatchTimeOut0(value)
		}
	}
	impl ::core::convert::From<DispatchTimeOut1Call> for EvmHostCalls {
		fn from(value: DispatchTimeOut1Call) -> Self {
			Self::DispatchTimeOut1(value)
		}
	}
	impl ::core::convert::From<DispatchTimeOut2Call> for EvmHostCalls {
		fn from(value: DispatchTimeOut2Call) -> Self {
			Self::DispatchTimeOut2(value)
		}
	}
	impl ::core::convert::From<FeeTokenCall> for EvmHostCalls {
		fn from(value: FeeTokenCall) -> Self {
			Self::FeeToken(value)
		}
	}
	impl ::core::convert::From<FrozenCall> for EvmHostCalls {
		fn from(value: FrozenCall) -> Self {
			Self::Frozen(value)
		}
	}
	impl ::core::convert::From<FundRequestCall> for EvmHostCalls {
		fn from(value: FundRequestCall) -> Self {
			Self::FundRequest(value)
		}
	}
	impl ::core::convert::From<FundResponseCall> for EvmHostCalls {
		fn from(value: FundResponseCall) -> Self {
			Self::FundResponse(value)
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
	impl ::core::convert::From<HyperbridgeCall> for EvmHostCalls {
		fn from(value: HyperbridgeCall) -> Self {
			Self::Hyperbridge(value)
		}
	}
	impl ::core::convert::From<LatestStateMachineHeightCall> for EvmHostCalls {
		fn from(value: LatestStateMachineHeightCall) -> Self {
			Self::LatestStateMachineHeight(value)
		}
	}
	impl ::core::convert::From<NonceCall> for EvmHostCalls {
		fn from(value: NonceCall) -> Self {
			Self::Nonce(value)
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
	impl ::core::convert::From<RespondedCall> for EvmHostCalls {
		fn from(value: RespondedCall) -> Self {
			Self::Responded(value)
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
	impl ::core::convert::From<StateCommitmentFeeCall> for EvmHostCalls {
		fn from(value: StateCommitmentFeeCall) -> Self {
			Self::StateCommitmentFee(value)
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
	impl ::core::convert::From<StateMachineIdCall> for EvmHostCalls {
		fn from(value: StateMachineIdCall) -> Self {
			Self::StateMachineId(value)
		}
	}
	impl ::core::convert::From<StoreConsensusStateCall> for EvmHostCalls {
		fn from(value: StoreConsensusStateCall) -> Self {
			Self::StoreConsensusState(value)
		}
	}
	impl ::core::convert::From<StoreStateMachineCommitmentCall> for EvmHostCalls {
		fn from(value: StoreStateMachineCommitmentCall) -> Self {
			Self::StoreStateMachineCommitment(value)
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
	impl ::core::convert::From<UniswapV2RouterCall> for EvmHostCalls {
		fn from(value: UniswapV2RouterCall) -> Self {
			Self::UniswapV2Router(value)
		}
	}
	impl ::core::convert::From<UpdateHostParamsCall> for EvmHostCalls {
		fn from(value: UpdateHostParamsCall) -> Self {
			Self::UpdateHostParams(value)
		}
	}
	impl ::core::convert::From<VetoesCall> for EvmHostCalls {
		fn from(value: VetoesCall) -> Self {
			Self::Vetoes(value)
		}
	}
	impl ::core::convert::From<WithdrawCall> for EvmHostCalls {
		fn from(value: WithdrawCall) -> Self {
			Self::Withdraw(value)
		}
	}
	///Container type for all return fields from the `admin` function with signature `admin()` and
	/// selector `0xf851a440`
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
	pub struct AdminReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `chainId` function with signature `chainId()`
	/// and selector `0x9a8a0592`
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
	pub struct ChainIdReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `challengePeriod` function with signature
	/// `challengePeriod()` and selector `0xf3f480d9`
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
	pub struct ChallengePeriodReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `consensusClient` function with signature
	/// `consensusClient()` and selector `0x2476132b`
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
	pub struct ConsensusClientReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `consensusState` function with signature
	/// `consensusState()` and selector `0xbbad99d4`
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
	pub struct ConsensusStateReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `consensusUpdateTime` function with signature
	/// `consensusUpdateTime()` and selector `0x9a8425bc`
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
	pub struct ConsensusUpdateTimeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `dispatch` function with signature
	/// `dispatch(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64,uint256,address))` and
	/// selector `0x94480805`
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
	pub struct DispatchReturn {
		pub commitment: [u8; 32],
	}
	///Container type for all return fields from the `dispatch` function with signature
	/// `dispatch((bytes,bytes,bytes,uint64,uint256,address))` and selector `0xb8f3e8f5`
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
	pub struct DispatchWithPostReturn {
		pub commitment: [u8; 32],
	}
	///Container type for all return fields from the `dispatch` function with signature
	/// `dispatch((bytes,uint64,bytes[],uint64,uint256,bytes))` and selector `0xd22e3343`
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
	pub struct DispatchWithGetReturn {
		pub commitment: [u8; 32],
	}
	///Container type for all return fields from the `feeToken` function with signature
	/// `feeToken()` and selector `0x647846a5`
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
	pub struct FeeTokenReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `frozen` function with signature `frozen()`
	/// and selector `0x054f7d9c`
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
	pub struct FrozenReturn(pub u8);
	///Container type for all return fields from the `host` function with signature `host()` and
	/// selector `0xf437bc59`
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
	pub struct HostReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `hostParams` function with signature
	/// `hostParams()` and selector `0x2215364d`
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
	pub struct HostParamsReturn(pub HostParams);
	///Container type for all return fields from the `hyperbridge` function with signature
	/// `hyperbridge()` and selector `0x005e763e`
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
	pub struct HyperbridgeReturn(pub ::ethers::core::types::Bytes);
	///Container type for all return fields from the `latestStateMachineHeight` function with
	/// signature `latestStateMachineHeight(uint256)` and selector `0x9c095f86`
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
	pub struct LatestStateMachineHeightReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `nonce` function with signature `nonce()` and
	/// selector `0xaffed0e0`
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
	pub struct NonceReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `perByteFee` function with signature
	/// `perByteFee(bytes)` and selector `0x4011ec0a`
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
	pub struct PerByteFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `requestCommitments` function with signature
	/// `requestCommitments(bytes32)` and selector `0x368bf464`
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
	pub struct RequestCommitmentsReturn(pub FeeMetadata);
	///Container type for all return fields from the `requestReceipts` function with signature
	/// `requestReceipts(bytes32)` and selector `0x19667a3e`
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
	pub struct RequestReceiptsReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `responded` function with signature
	/// `responded(bytes32)` and selector `0x73bfa8ae`
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
	pub struct RespondedReturn(pub bool);
	///Container type for all return fields from the `responseCommitments` function with signature
	/// `responseCommitments(bytes32)` and selector `0x2211f1dd`
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
	pub struct ResponseCommitmentsReturn(pub FeeMetadata);
	///Container type for all return fields from the `responseReceipts` function with signature
	/// `responseReceipts(bytes32)` and selector `0x8856337e`
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
	pub struct ResponseReceiptsReturn(pub ResponseReceipt);
	///Container type for all return fields from the `stateCommitmentFee` function with signature
	/// `stateCommitmentFee()` and selector `0xb5c8a08e`
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
	pub struct StateCommitmentFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `stateMachineCommitment` function with
	/// signature `stateMachineCommitment((uint256,uint256))` and selector `0xa70a8c47`
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
	pub struct StateMachineCommitmentReturn(pub StateCommitment);
	///Container type for all return fields from the `stateMachineCommitmentUpdateTime` function
	/// with signature `stateMachineCommitmentUpdateTime((uint256,uint256))` and selector
	/// `0x1a880a93`
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
	pub struct StateMachineCommitmentUpdateTimeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `stateMachineId` function with signature
	/// `stateMachineId(bytes,uint256)` and selector `0x7a1bd7c1`
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
	pub struct StateMachineIdReturn(pub ::std::string::String);
	///Container type for all return fields from the `timestamp` function with signature
	/// `timestamp()` and selector `0xb80777ea`
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
	pub struct TimestampReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `unStakingPeriod` function with signature
	/// `unStakingPeriod()` and selector `0xd40784c7`
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
	pub struct UnStakingPeriodReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `uniswapV2Router` function with signature
	/// `uniswapV2Router()` and selector `0x1694505e`
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
	pub struct UniswapV2RouterReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `vetoes` function with signature
	/// `vetoes(uint256,uint256)` and selector `0x5218d908`
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
	pub struct VetoesReturn(pub ::ethers::core::types::Address);
	///`DispatchGet(bytes,uint64,bytes[],uint64,uint256,bytes)`
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
	pub struct DispatchGet {
		pub dest: ::ethers::core::types::Bytes,
		pub height: u64,
		pub keys: ::std::vec::Vec<::ethers::core::types::Bytes>,
		pub timeout: u64,
		pub fee: ::ethers::core::types::U256,
		pub context: ::ethers::core::types::Bytes,
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
		Hash,
	)]
	pub struct FeeMetadata {
		pub fee: ::ethers::core::types::U256,
		pub sender: ::ethers::core::types::Address,
	}
	///`HostParams(uint256,uint256,uint256,address,address,address,address,address,uint256,uint256,
	/// address,uint256[],(bytes32,uint256)[],bytes)`
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
	pub struct HostParams {
		pub default_timeout: ::ethers::core::types::U256,
		pub default_per_byte_fee: ::ethers::core::types::U256,
		pub state_commitment_fee: ::ethers::core::types::U256,
		pub fee_token: ::ethers::core::types::Address,
		pub admin: ::ethers::core::types::Address,
		pub handler: ::ethers::core::types::Address,
		pub host_manager: ::ethers::core::types::Address,
		pub uniswap_v2: ::ethers::core::types::Address,
		pub un_staking_period: ::ethers::core::types::U256,
		pub challenge_period: ::ethers::core::types::U256,
		pub consensus_client: ::ethers::core::types::Address,
		pub state_machines: ::std::vec::Vec<::ethers::core::types::U256>,
		pub per_byte_fees: ::std::vec::Vec<PerByteFee>,
		pub hyperbridge: ::ethers::core::types::Bytes,
	}
	///`PerByteFee(bytes32,uint256)`
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
	pub struct PerByteFee {
		pub state_id_hash: [u8; 32],
		pub per_byte_fee: ::ethers::core::types::U256,
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
		Hash,
	)]
	pub struct ResponseReceipt {
		pub response_commitment: [u8; 32],
		pub relayer: ::ethers::core::types::Address,
	}
	///`WithdrawParams(address,uint256,bool)`
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
	pub struct WithdrawParams {
		pub beneficiary: ::ethers::core::types::Address,
		pub amount: ::ethers::core::types::U256,
		pub native: bool,
	}
}
