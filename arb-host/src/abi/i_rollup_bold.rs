pub use i_rollup_bold::*;
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
pub mod i_rollup_bold {
	pub use super::super::shared_types::*;
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("amountStaked"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("amountStaked"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("staker"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                    ::std::borrow::ToOwned::to_owned("baseStake"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("baseStake"),
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
                    ::std::borrow::ToOwned::to_owned("bridge"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("bridge"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IBridge"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("challengeManager"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("challengeManager"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "contract IEdgeChallengeManager",
                                        ),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("confirmPeriodBlocks"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "confirmPeriodBlocks",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("genesisAssertionHash"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "genesisAssertionHash",
                            ),
                            inputs: ::std::vec![],
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("getAssertion"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("getAssertion"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct AssertionNode"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned(
                        "getAssertionCreationBlockForLogLookup",
                    ),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "getAssertionCreationBlockForLogLookup",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                    ::std::borrow::ToOwned::to_owned("getFirstChildCreationBlock"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "getFirstChildCreationBlock",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("getSecondChildCreationBlock"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "getSecondChildCreationBlock",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("getStaker"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("getStaker"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("staker"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct IRollupCore.Staker",
                                        ),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("getStakerAddress"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("getStakerAddress"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("stakerNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
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
                    ::std::borrow::ToOwned::to_owned("getValidators"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("getValidators"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address[]"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("isFirstChild"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("isFirstChild"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                    ::std::borrow::ToOwned::to_owned("isPending"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("isPending"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
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
                    ::std::borrow::ToOwned::to_owned("isStaked"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("isStaked"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("staker"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                    ::std::borrow::ToOwned::to_owned("isValidator"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("isValidator"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                    ::std::borrow::ToOwned::to_owned("latestConfirmed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("latestConfirmed"),
                            inputs: ::std::vec![],
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("latestStakedAssertion"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "latestStakedAssertion",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("staker"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("loserStakeEscrow"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("loserStakeEscrow"),
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
                    ::std::borrow::ToOwned::to_owned("minimumAssertionPeriod"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "minimumAssertionPeriod",
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
                    ::std::borrow::ToOwned::to_owned("outbox"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("outbox"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IOutbox"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("rollupEventInbox"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("rollupEventInbox"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "contract IRollupEventInbox",
                                        ),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("sequencerInbox"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("sequencerInbox"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract ISequencerInbox"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("stakeToken"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("stakeToken"),
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
                    ::std::borrow::ToOwned::to_owned("stakerCount"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("stakerCount"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validateAssertionHash"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "validateAssertionHash",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("state"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                        ),
                                                        2usize,
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                        ),
                                                        2usize,
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct AssertionState"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("prevAssertionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("inboxAcc"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validateConfig"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("validateConfig"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes32"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("configData"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct ConfigData"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validatorAfkBlocks"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("validatorAfkBlocks"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validatorWhitelistDisabled"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "validatorWhitelistDisabled",
                            ),
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
                    ::std::borrow::ToOwned::to_owned("wasmModuleRoot"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("wasmModuleRoot"),
                            inputs: ::std::vec![],
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawableFunds"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("withdrawableFunds"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("owner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                    ::std::borrow::ToOwned::to_owned("withdrawalAddress"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("withdrawalAddress"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("staker"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("AssertionConfirmed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("AssertionConfirmed"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("blockHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("sendRoot"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("AssertionCreated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("AssertionCreated"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("assertionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "parentAssertionHash",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("assertion"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                        ],
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                ),
                                                                2usize,
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                ),
                                                                2usize,
                                                            ),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                ),
                                                                2usize,
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::FixedArray(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                ),
                                                                2usize,
                                                            ),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                        ],
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "afterInboxBatchAcc",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("inboxMaxCount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("wasmModuleRoot"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("requiredStake"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("challengeManager"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "confirmPeriodBlocks",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("RollupChallengeStarted"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "RollupChallengeStarted",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("challengeIndex"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("asserter"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("challenger"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "challengedAssertion",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("RollupInitialized"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("RollupInitialized"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("machineHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("chainId"),
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
                    ::std::borrow::ToOwned::to_owned("UserStakeUpdated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("UserStakeUpdated"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("user"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("withdrawalAddress"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("initialBalance"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("finalBalance"),
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
                    ::std::borrow::ToOwned::to_owned("UserWithdrawableFundsUpdated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "UserWithdrawableFundsUpdated",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("user"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("initialBalance"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("finalBalance"),
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
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
	}
	///The parsed JSON ABI of the contract.
	pub static IROLLUPBOLD_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct IRollupBold<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for IRollupBold<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for IRollupBold<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for IRollupBold<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for IRollupBold<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(IRollupBold)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> IRollupBold<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), IROLLUPBOLD_ABI.clone(), client))
		}
		///Calls the contract's `amountStaked` (0xef40a670) function
		pub fn amount_staked(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([239, 64, 166, 112], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `baseStake` (0x76e7e23b) function
		pub fn base_stake(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([118, 231, 226, 59], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `bridge` (0xe78cea92) function
		pub fn bridge(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([231, 140, 234, 146], ())
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
		///Calls the contract's `challengeManager` (0x023a96fe) function
		pub fn challenge_manager(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([2, 58, 150, 254], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `confirmPeriodBlocks` (0x2e7acfa6) function
		pub fn confirm_period_blocks(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([46, 122, 207, 166], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `genesisAssertionHash` (0x353325e0) function
		pub fn genesis_assertion_hash(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([53, 51, 37, 224], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getAssertion` (0x88302884) function
		pub fn get_assertion(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, AssertionNode> {
			self.0
				.method_hash([136, 48, 40, 132], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getAssertionCreationBlockForLogLookup` (0x13c56ca7) function
		pub fn get_assertion_creation_block_for_log_lookup(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([19, 197, 108, 167], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getFirstChildCreationBlock` (0x11715585) function
		pub fn get_first_child_creation_block(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([17, 113, 85, 133], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getSecondChildCreationBlock` (0x56bbc9e6) function
		pub fn get_second_child_creation_block(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([86, 187, 201, 230], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getStaker` (0xa23c44b1) function
		pub fn get_staker(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, Staker> {
			self.0
				.method_hash([162, 60, 68, 177], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getStakerAddress` (0x6ddd3744) function
		pub fn get_staker_address(
			&self,
			staker_num: u64,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([109, 221, 55, 68], staker_num)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getValidators` (0xb7ab4db5) function
		pub fn get_validators(
			&self,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			::std::vec::Vec<::ethers::core::types::Address>,
		> {
			self.0
				.method_hash([183, 171, 77, 181], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `isFirstChild` (0x30836228) function
		pub fn is_first_child(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([48, 131, 98, 40], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `isPending` (0xe531d8c7) function
		pub fn is_pending(
			&self,
			assertion_hash: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([229, 49, 216, 199], assertion_hash)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `isStaked` (0x6177fd18) function
		pub fn is_staked(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([97, 119, 253, 24], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `isValidator` (0xfacd743b) function
		pub fn is_validator(
			&self,
			p0: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([250, 205, 116, 59], p0)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestConfirmed` (0x65f7f80d) function
		pub fn latest_confirmed(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([101, 247, 248, 13], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestStakedAssertion` (0x2abdd230) function
		pub fn latest_staked_assertion(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([42, 189, 210, 48], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `loserStakeEscrow` (0xf065de3f) function
		pub fn loser_stake_escrow(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([240, 101, 222, 63], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `minimumAssertionPeriod` (0x45e38b64) function
		pub fn minimum_assertion_period(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([69, 227, 139, 100], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `outbox` (0xce11e6ab) function
		pub fn outbox(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([206, 17, 230, 171], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `rollupEventInbox` (0xaa38a6e7) function
		pub fn rollup_event_inbox(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([170, 56, 166, 231], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `sequencerInbox` (0xee35f327) function
		pub fn sequencer_inbox(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([238, 53, 243, 39], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `stakeToken` (0x51ed6a30) function
		pub fn stake_token(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([81, 237, 106, 48], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `stakerCount` (0xdff69787) function
		pub fn staker_count(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([223, 246, 151, 135], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `validateAssertionHash` (0xe51019a6) function
		pub fn validate_assertion_hash(
			&self,
			assertion_hash: [u8; 32],
			state: AssertionState,
			prev_assertion_hash: [u8; 32],
			inbox_acc: [u8; 32],
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash(
					[229, 16, 25, 166],
					(assertion_hash, state, prev_assertion_hash, inbox_acc),
				)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `validateConfig` (0x04972af9) function
		pub fn validate_config(
			&self,
			assertion_hash: [u8; 32],
			config_data: ConfigData,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([4, 151, 42, 249], (assertion_hash, config_data))
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `validatorAfkBlocks` (0xe6b3082c) function
		pub fn validator_afk_blocks(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([230, 179, 8, 44], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `validatorWhitelistDisabled` (0x12ab3d3b) function
		pub fn validator_whitelist_disabled(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([18, 171, 61, 59], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `wasmModuleRoot` (0x8ee1a126) function
		pub fn wasm_module_root(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([142, 225, 161, 38], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `withdrawableFunds` (0x2f30cabd) function
		pub fn withdrawable_funds(
			&self,
			owner: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([47, 48, 202, 189], owner)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `withdrawalAddress` (0x84728cd0) function
		pub fn withdrawal_address(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([132, 114, 140, 208], staker)
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `AssertionConfirmed` event
		pub fn assertion_confirmed_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AssertionConfirmedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `AssertionCreated` event
		pub fn assertion_created_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AssertionCreatedFilter> {
			self.0.event()
		}
		///Gets the contract's `RollupChallengeStarted` event
		pub fn rollup_challenge_started_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, RollupChallengeStartedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `RollupInitialized` event
		pub fn rollup_initialized_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, RollupInitializedFilter>
		{
			self.0.event()
		}
		///Gets the contract's `UserStakeUpdated` event
		pub fn user_stake_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, UserStakeUpdatedFilter> {
			self.0.event()
		}
		///Gets the contract's `UserWithdrawableFundsUpdated` event
		pub fn user_withdrawable_funds_updated_filter(
			&self,
		) -> ::ethers::contract::builders::Event<
			::std::sync::Arc<M>,
			M,
			UserWithdrawableFundsUpdatedFilter,
		> {
			self.0.event()
		}
		/// Returns an `Event` builder for all the events of this contract.
		pub fn events(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, IRollupBoldEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for IRollupBold<M> {
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
		Hash,
	)]
	#[ethevent(name = "AssertionConfirmed", abi = "AssertionConfirmed(bytes32,bytes32,bytes32)")]
	pub struct AssertionConfirmedFilter {
		#[ethevent(indexed)]
		pub assertion_hash: [u8; 32],
		pub block_hash: [u8; 32],
		pub send_root: [u8; 32],
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
		name = "AssertionCreated",
		abi = "AssertionCreated(bytes32,bytes32,((bytes32,bytes32,(bytes32,uint256,address,uint64,uint64)),((bytes32[2],uint64[2]),uint8,bytes32),((bytes32[2],uint64[2]),uint8,bytes32)),bytes32,uint256,bytes32,uint256,address,uint64)"
	)]
	pub struct AssertionCreatedFilter {
		#[ethevent(indexed)]
		pub assertion_hash: [u8; 32],
		#[ethevent(indexed)]
		pub parent_assertion_hash: [u8; 32],
		pub assertion: AssertionInputs,
		pub after_inbox_batch_acc: [u8; 32],
		pub inbox_max_count: ::ethers::core::types::U256,
		pub wasm_module_root: [u8; 32],
		pub required_stake: ::ethers::core::types::U256,
		pub challenge_manager: ::ethers::core::types::Address,
		pub confirm_period_blocks: u64,
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
		name = "RollupChallengeStarted",
		abi = "RollupChallengeStarted(uint64,address,address,uint64)"
	)]
	pub struct RollupChallengeStartedFilter {
		#[ethevent(indexed)]
		pub challenge_index: u64,
		pub asserter: ::ethers::core::types::Address,
		pub challenger: ::ethers::core::types::Address,
		pub challenged_assertion: u64,
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
	#[ethevent(name = "RollupInitialized", abi = "RollupInitialized(bytes32,uint256)")]
	pub struct RollupInitializedFilter {
		pub machine_hash: [u8; 32],
		pub chain_id: ::ethers::core::types::U256,
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
		name = "UserStakeUpdated",
		abi = "UserStakeUpdated(address,address,uint256,uint256)"
	)]
	pub struct UserStakeUpdatedFilter {
		#[ethevent(indexed)]
		pub user: ::ethers::core::types::Address,
		#[ethevent(indexed)]
		pub withdrawal_address: ::ethers::core::types::Address,
		pub initial_balance: ::ethers::core::types::U256,
		pub final_balance: ::ethers::core::types::U256,
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
		name = "UserWithdrawableFundsUpdated",
		abi = "UserWithdrawableFundsUpdated(address,uint256,uint256)"
	)]
	pub struct UserWithdrawableFundsUpdatedFilter {
		#[ethevent(indexed)]
		pub user: ::ethers::core::types::Address,
		pub initial_balance: ::ethers::core::types::U256,
		pub final_balance: ::ethers::core::types::U256,
	}
	///Container type for all of the contract's events
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum IRollupBoldEvents {
		AssertionConfirmedFilter(AssertionConfirmedFilter),
		AssertionCreatedFilter(AssertionCreatedFilter),
		RollupChallengeStartedFilter(RollupChallengeStartedFilter),
		RollupInitializedFilter(RollupInitializedFilter),
		UserStakeUpdatedFilter(UserStakeUpdatedFilter),
		UserWithdrawableFundsUpdatedFilter(UserWithdrawableFundsUpdatedFilter),
	}
	impl ::ethers::contract::EthLogDecode for IRollupBoldEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = AssertionConfirmedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::AssertionConfirmedFilter(decoded));
			}
			if let Ok(decoded) = AssertionCreatedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::AssertionCreatedFilter(decoded));
			}
			if let Ok(decoded) = RollupChallengeStartedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::RollupChallengeStartedFilter(decoded));
			}
			if let Ok(decoded) = RollupInitializedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::RollupInitializedFilter(decoded));
			}
			if let Ok(decoded) = UserStakeUpdatedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::UserStakeUpdatedFilter(decoded));
			}
			if let Ok(decoded) = UserWithdrawableFundsUpdatedFilter::decode_log(log) {
				return Ok(IRollupBoldEvents::UserWithdrawableFundsUpdatedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for IRollupBoldEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::AssertionConfirmedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::AssertionCreatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::RollupChallengeStartedFilter(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::RollupInitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::UserStakeUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::UserWithdrawableFundsUpdatedFilter(element) =>
					::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<AssertionConfirmedFilter> for IRollupBoldEvents {
		fn from(value: AssertionConfirmedFilter) -> Self {
			Self::AssertionConfirmedFilter(value)
		}
	}
	impl ::core::convert::From<AssertionCreatedFilter> for IRollupBoldEvents {
		fn from(value: AssertionCreatedFilter) -> Self {
			Self::AssertionCreatedFilter(value)
		}
	}
	impl ::core::convert::From<RollupChallengeStartedFilter> for IRollupBoldEvents {
		fn from(value: RollupChallengeStartedFilter) -> Self {
			Self::RollupChallengeStartedFilter(value)
		}
	}
	impl ::core::convert::From<RollupInitializedFilter> for IRollupBoldEvents {
		fn from(value: RollupInitializedFilter) -> Self {
			Self::RollupInitializedFilter(value)
		}
	}
	impl ::core::convert::From<UserStakeUpdatedFilter> for IRollupBoldEvents {
		fn from(value: UserStakeUpdatedFilter) -> Self {
			Self::UserStakeUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<UserWithdrawableFundsUpdatedFilter> for IRollupBoldEvents {
		fn from(value: UserWithdrawableFundsUpdatedFilter) -> Self {
			Self::UserWithdrawableFundsUpdatedFilter(value)
		}
	}
	///Container type for all input parameters for the `amountStaked` function with signature
	/// `amountStaked(address)` and selector `0xef40a670`
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
	#[ethcall(name = "amountStaked", abi = "amountStaked(address)")]
	pub struct AmountStakedCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `baseStake` function with signature
	/// `baseStake()` and selector `0x76e7e23b`
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
	#[ethcall(name = "baseStake", abi = "baseStake()")]
	pub struct BaseStakeCall;
	///Container type for all input parameters for the `bridge` function with signature `bridge()`
	/// and selector `0xe78cea92`
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
	#[ethcall(name = "bridge", abi = "bridge()")]
	pub struct BridgeCall;
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
	///Container type for all input parameters for the `challengeManager` function with signature
	/// `challengeManager()` and selector `0x023a96fe`
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
	#[ethcall(name = "challengeManager", abi = "challengeManager()")]
	pub struct ChallengeManagerCall;
	///Container type for all input parameters for the `confirmPeriodBlocks` function with
	/// signature `confirmPeriodBlocks()` and selector `0x2e7acfa6`
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
	#[ethcall(name = "confirmPeriodBlocks", abi = "confirmPeriodBlocks()")]
	pub struct ConfirmPeriodBlocksCall;
	///Container type for all input parameters for the `genesisAssertionHash` function with
	/// signature `genesisAssertionHash()` and selector `0x353325e0`
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
	#[ethcall(name = "genesisAssertionHash", abi = "genesisAssertionHash()")]
	pub struct GenesisAssertionHashCall;
	///Container type for all input parameters for the `getAssertion` function with signature
	/// `getAssertion(bytes32)` and selector `0x88302884`
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
	#[ethcall(name = "getAssertion", abi = "getAssertion(bytes32)")]
	pub struct GetAssertionCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `getAssertionCreationBlockForLogLookup`
	/// function with signature `getAssertionCreationBlockForLogLookup(bytes32)` and selector
	/// `0x13c56ca7`
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
		name = "getAssertionCreationBlockForLogLookup",
		abi = "getAssertionCreationBlockForLogLookup(bytes32)"
	)]
	pub struct GetAssertionCreationBlockForLogLookupCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `getFirstChildCreationBlock` function with
	/// signature `getFirstChildCreationBlock(bytes32)` and selector `0x11715585`
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
	#[ethcall(name = "getFirstChildCreationBlock", abi = "getFirstChildCreationBlock(bytes32)")]
	pub struct GetFirstChildCreationBlockCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `getSecondChildCreationBlock` function with
	/// signature `getSecondChildCreationBlock(bytes32)` and selector `0x56bbc9e6`
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
	#[ethcall(name = "getSecondChildCreationBlock", abi = "getSecondChildCreationBlock(bytes32)")]
	pub struct GetSecondChildCreationBlockCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `getStaker` function with signature
	/// `getStaker(address)` and selector `0xa23c44b1`
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
	#[ethcall(name = "getStaker", abi = "getStaker(address)")]
	pub struct GetStakerCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `getStakerAddress` function with signature
	/// `getStakerAddress(uint64)` and selector `0x6ddd3744`
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
	#[ethcall(name = "getStakerAddress", abi = "getStakerAddress(uint64)")]
	pub struct GetStakerAddressCall {
		pub staker_num: u64,
	}
	///Container type for all input parameters for the `getValidators` function with signature
	/// `getValidators()` and selector `0xb7ab4db5`
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
	#[ethcall(name = "getValidators", abi = "getValidators()")]
	pub struct GetValidatorsCall;
	///Container type for all input parameters for the `isFirstChild` function with signature
	/// `isFirstChild(bytes32)` and selector `0x30836228`
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
	#[ethcall(name = "isFirstChild", abi = "isFirstChild(bytes32)")]
	pub struct IsFirstChildCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `isPending` function with signature
	/// `isPending(bytes32)` and selector `0xe531d8c7`
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
	#[ethcall(name = "isPending", abi = "isPending(bytes32)")]
	pub struct IsPendingCall {
		pub assertion_hash: [u8; 32],
	}
	///Container type for all input parameters for the `isStaked` function with signature
	/// `isStaked(address)` and selector `0x6177fd18`
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
	#[ethcall(name = "isStaked", abi = "isStaked(address)")]
	pub struct IsStakedCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `isValidator` function with signature
	/// `isValidator(address)` and selector `0xfacd743b`
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
	#[ethcall(name = "isValidator", abi = "isValidator(address)")]
	pub struct IsValidatorCall(pub ::ethers::core::types::Address);
	///Container type for all input parameters for the `latestConfirmed` function with signature
	/// `latestConfirmed()` and selector `0x65f7f80d`
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
	#[ethcall(name = "latestConfirmed", abi = "latestConfirmed()")]
	pub struct LatestConfirmedCall;
	///Container type for all input parameters for the `latestStakedAssertion` function with
	/// signature `latestStakedAssertion(address)` and selector `0x2abdd230`
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
	#[ethcall(name = "latestStakedAssertion", abi = "latestStakedAssertion(address)")]
	pub struct LatestStakedAssertionCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `loserStakeEscrow` function with signature
	/// `loserStakeEscrow()` and selector `0xf065de3f`
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
	#[ethcall(name = "loserStakeEscrow", abi = "loserStakeEscrow()")]
	pub struct LoserStakeEscrowCall;
	///Container type for all input parameters for the `minimumAssertionPeriod` function with
	/// signature `minimumAssertionPeriod()` and selector `0x45e38b64`
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
	#[ethcall(name = "minimumAssertionPeriod", abi = "minimumAssertionPeriod()")]
	pub struct MinimumAssertionPeriodCall;
	///Container type for all input parameters for the `outbox` function with signature `outbox()`
	/// and selector `0xce11e6ab`
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
	#[ethcall(name = "outbox", abi = "outbox()")]
	pub struct OutboxCall;
	///Container type for all input parameters for the `rollupEventInbox` function with signature
	/// `rollupEventInbox()` and selector `0xaa38a6e7`
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
	#[ethcall(name = "rollupEventInbox", abi = "rollupEventInbox()")]
	pub struct RollupEventInboxCall;
	///Container type for all input parameters for the `sequencerInbox` function with signature
	/// `sequencerInbox()` and selector `0xee35f327`
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
	#[ethcall(name = "sequencerInbox", abi = "sequencerInbox()")]
	pub struct SequencerInboxCall;
	///Container type for all input parameters for the `stakeToken` function with signature
	/// `stakeToken()` and selector `0x51ed6a30`
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
	#[ethcall(name = "stakeToken", abi = "stakeToken()")]
	pub struct StakeTokenCall;
	///Container type for all input parameters for the `stakerCount` function with signature
	/// `stakerCount()` and selector `0xdff69787`
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
	#[ethcall(name = "stakerCount", abi = "stakerCount()")]
	pub struct StakerCountCall;
	///Container type for all input parameters for the `validateAssertionHash` function with
	/// signature `validateAssertionHash(bytes32,((bytes32[2],uint64[2]),uint8,bytes32),bytes32,
	/// bytes32)` and selector `0xe51019a6`
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
		name = "validateAssertionHash",
		abi = "validateAssertionHash(bytes32,((bytes32[2],uint64[2]),uint8,bytes32),bytes32,bytes32)"
	)]
	pub struct ValidateAssertionHashCall {
		pub assertion_hash: [u8; 32],
		pub state: AssertionState,
		pub prev_assertion_hash: [u8; 32],
		pub inbox_acc: [u8; 32],
	}
	///Container type for all input parameters for the `validateConfig` function with signature
	/// `validateConfig(bytes32,(bytes32,uint256,address,uint64,uint64))` and selector `0x04972af9`
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
		name = "validateConfig",
		abi = "validateConfig(bytes32,(bytes32,uint256,address,uint64,uint64))"
	)]
	pub struct ValidateConfigCall {
		pub assertion_hash: [u8; 32],
		pub config_data: ConfigData,
	}
	///Container type for all input parameters for the `validatorAfkBlocks` function with signature
	/// `validatorAfkBlocks()` and selector `0xe6b3082c`
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
	#[ethcall(name = "validatorAfkBlocks", abi = "validatorAfkBlocks()")]
	pub struct ValidatorAfkBlocksCall;
	///Container type for all input parameters for the `validatorWhitelistDisabled` function with
	/// signature `validatorWhitelistDisabled()` and selector `0x12ab3d3b`
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
	#[ethcall(name = "validatorWhitelistDisabled", abi = "validatorWhitelistDisabled()")]
	pub struct ValidatorWhitelistDisabledCall;
	///Container type for all input parameters for the `wasmModuleRoot` function with signature
	/// `wasmModuleRoot()` and selector `0x8ee1a126`
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
	#[ethcall(name = "wasmModuleRoot", abi = "wasmModuleRoot()")]
	pub struct WasmModuleRootCall;
	///Container type for all input parameters for the `withdrawableFunds` function with signature
	/// `withdrawableFunds(address)` and selector `0x2f30cabd`
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
	#[ethcall(name = "withdrawableFunds", abi = "withdrawableFunds(address)")]
	pub struct WithdrawableFundsCall {
		pub owner: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `withdrawalAddress` function with signature
	/// `withdrawalAddress(address)` and selector `0x84728cd0`
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
	#[ethcall(name = "withdrawalAddress", abi = "withdrawalAddress(address)")]
	pub struct WithdrawalAddressCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum IRollupBoldCalls {
		AmountStaked(AmountStakedCall),
		BaseStake(BaseStakeCall),
		Bridge(BridgeCall),
		ChainId(ChainIdCall),
		ChallengeManager(ChallengeManagerCall),
		ConfirmPeriodBlocks(ConfirmPeriodBlocksCall),
		GenesisAssertionHash(GenesisAssertionHashCall),
		GetAssertion(GetAssertionCall),
		GetAssertionCreationBlockForLogLookup(GetAssertionCreationBlockForLogLookupCall),
		GetFirstChildCreationBlock(GetFirstChildCreationBlockCall),
		GetSecondChildCreationBlock(GetSecondChildCreationBlockCall),
		GetStaker(GetStakerCall),
		GetStakerAddress(GetStakerAddressCall),
		GetValidators(GetValidatorsCall),
		IsFirstChild(IsFirstChildCall),
		IsPending(IsPendingCall),
		IsStaked(IsStakedCall),
		IsValidator(IsValidatorCall),
		LatestConfirmed(LatestConfirmedCall),
		LatestStakedAssertion(LatestStakedAssertionCall),
		LoserStakeEscrow(LoserStakeEscrowCall),
		MinimumAssertionPeriod(MinimumAssertionPeriodCall),
		Outbox(OutboxCall),
		RollupEventInbox(RollupEventInboxCall),
		SequencerInbox(SequencerInboxCall),
		StakeToken(StakeTokenCall),
		StakerCount(StakerCountCall),
		ValidateAssertionHash(ValidateAssertionHashCall),
		ValidateConfig(ValidateConfigCall),
		ValidatorAfkBlocks(ValidatorAfkBlocksCall),
		ValidatorWhitelistDisabled(ValidatorWhitelistDisabledCall),
		WasmModuleRoot(WasmModuleRootCall),
		WithdrawableFunds(WithdrawableFundsCall),
		WithdrawalAddress(WithdrawalAddressCall),
	}
	impl ::ethers::core::abi::AbiDecode for IRollupBoldCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <AmountStakedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::AmountStaked(decoded));
			}
			if let Ok(decoded) = <BaseStakeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::BaseStake(decoded));
			}
			if let Ok(decoded) = <BridgeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Bridge(decoded));
			}
			if let Ok(decoded) = <ChainIdCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ChainId(decoded));
			}
			if let Ok(decoded) =
				<ChallengeManagerCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ChallengeManager(decoded));
			}
			if let Ok(decoded) =
				<ConfirmPeriodBlocksCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ConfirmPeriodBlocks(decoded));
			}
			if let Ok(decoded) =
				<GenesisAssertionHashCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GenesisAssertionHash(decoded));
			}
			if let Ok(decoded) = <GetAssertionCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetAssertion(decoded));
			}
			if let Ok(decoded) = <GetAssertionCreationBlockForLogLookupCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::GetAssertionCreationBlockForLogLookup(decoded));
            }
			if let Ok(decoded) =
				<GetFirstChildCreationBlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetFirstChildCreationBlock(decoded));
			}
			if let Ok(decoded) =
				<GetSecondChildCreationBlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetSecondChildCreationBlock(decoded));
			}
			if let Ok(decoded) = <GetStakerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetStaker(decoded));
			}
			if let Ok(decoded) =
				<GetStakerAddressCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetStakerAddress(decoded));
			}
			if let Ok(decoded) = <GetValidatorsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetValidators(decoded));
			}
			if let Ok(decoded) = <IsFirstChildCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::IsFirstChild(decoded));
			}
			if let Ok(decoded) = <IsPendingCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsPending(decoded));
			}
			if let Ok(decoded) = <IsStakedCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsStaked(decoded));
			}
			if let Ok(decoded) = <IsValidatorCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsValidator(decoded));
			}
			if let Ok(decoded) =
				<LatestConfirmedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestConfirmed(decoded));
			}
			if let Ok(decoded) =
				<LatestStakedAssertionCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestStakedAssertion(decoded));
			}
			if let Ok(decoded) =
				<LoserStakeEscrowCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LoserStakeEscrow(decoded));
			}
			if let Ok(decoded) =
				<MinimumAssertionPeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MinimumAssertionPeriod(decoded));
			}
			if let Ok(decoded) = <OutboxCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Outbox(decoded));
			}
			if let Ok(decoded) =
				<RollupEventInboxCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::RollupEventInbox(decoded));
			}
			if let Ok(decoded) =
				<SequencerInboxCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SequencerInbox(decoded));
			}
			if let Ok(decoded) = <StakeTokenCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::StakeToken(decoded));
			}
			if let Ok(decoded) = <StakerCountCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::StakerCount(decoded));
			}
			if let Ok(decoded) =
				<ValidateAssertionHashCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ValidateAssertionHash(decoded));
			}
			if let Ok(decoded) =
				<ValidateConfigCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ValidateConfig(decoded));
			}
			if let Ok(decoded) =
				<ValidatorAfkBlocksCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ValidatorAfkBlocks(decoded));
			}
			if let Ok(decoded) =
				<ValidatorWhitelistDisabledCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ValidatorWhitelistDisabled(decoded));
			}
			if let Ok(decoded) =
				<WasmModuleRootCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::WasmModuleRoot(decoded));
			}
			if let Ok(decoded) =
				<WithdrawableFundsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::WithdrawableFunds(decoded));
			}
			if let Ok(decoded) =
				<WithdrawalAddressCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::WithdrawalAddress(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for IRollupBoldCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::AmountStaked(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BaseStake(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Bridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChainId(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChallengeManager(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConfirmPeriodBlocks(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GenesisAssertionHash(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetAssertion(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetAssertionCreationBlockForLogLookup(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetFirstChildCreationBlock(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetSecondChildCreationBlock(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetStaker(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetStakerAddress(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetValidators(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsFirstChild(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsPending(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsStaked(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsValidator(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestConfirmed(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestStakedAssertion(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::LoserStakeEscrow(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::MinimumAssertionPeriod(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::Outbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RollupEventInbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SequencerInbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StakeToken(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StakerCount(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidateAssertionHash(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidateConfig(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidatorAfkBlocks(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidatorWhitelistDisabled(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::WasmModuleRoot(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::WithdrawableFunds(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::WithdrawalAddress(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for IRollupBoldCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::AmountStaked(element) => ::core::fmt::Display::fmt(element, f),
				Self::BaseStake(element) => ::core::fmt::Display::fmt(element, f),
				Self::Bridge(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChainId(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChallengeManager(element) => ::core::fmt::Display::fmt(element, f),
				Self::ConfirmPeriodBlocks(element) => ::core::fmt::Display::fmt(element, f),
				Self::GenesisAssertionHash(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetAssertion(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetAssertionCreationBlockForLogLookup(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::GetFirstChildCreationBlock(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetSecondChildCreationBlock(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetStaker(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetStakerAddress(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetValidators(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsFirstChild(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsPending(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsStaked(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsValidator(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestConfirmed(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestStakedAssertion(element) => ::core::fmt::Display::fmt(element, f),
				Self::LoserStakeEscrow(element) => ::core::fmt::Display::fmt(element, f),
				Self::MinimumAssertionPeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::Outbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::RollupEventInbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::SequencerInbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::StakeToken(element) => ::core::fmt::Display::fmt(element, f),
				Self::StakerCount(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidateAssertionHash(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidateConfig(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidatorAfkBlocks(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidatorWhitelistDisabled(element) => ::core::fmt::Display::fmt(element, f),
				Self::WasmModuleRoot(element) => ::core::fmt::Display::fmt(element, f),
				Self::WithdrawableFunds(element) => ::core::fmt::Display::fmt(element, f),
				Self::WithdrawalAddress(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<AmountStakedCall> for IRollupBoldCalls {
		fn from(value: AmountStakedCall) -> Self {
			Self::AmountStaked(value)
		}
	}
	impl ::core::convert::From<BaseStakeCall> for IRollupBoldCalls {
		fn from(value: BaseStakeCall) -> Self {
			Self::BaseStake(value)
		}
	}
	impl ::core::convert::From<BridgeCall> for IRollupBoldCalls {
		fn from(value: BridgeCall) -> Self {
			Self::Bridge(value)
		}
	}
	impl ::core::convert::From<ChainIdCall> for IRollupBoldCalls {
		fn from(value: ChainIdCall) -> Self {
			Self::ChainId(value)
		}
	}
	impl ::core::convert::From<ChallengeManagerCall> for IRollupBoldCalls {
		fn from(value: ChallengeManagerCall) -> Self {
			Self::ChallengeManager(value)
		}
	}
	impl ::core::convert::From<ConfirmPeriodBlocksCall> for IRollupBoldCalls {
		fn from(value: ConfirmPeriodBlocksCall) -> Self {
			Self::ConfirmPeriodBlocks(value)
		}
	}
	impl ::core::convert::From<GenesisAssertionHashCall> for IRollupBoldCalls {
		fn from(value: GenesisAssertionHashCall) -> Self {
			Self::GenesisAssertionHash(value)
		}
	}
	impl ::core::convert::From<GetAssertionCall> for IRollupBoldCalls {
		fn from(value: GetAssertionCall) -> Self {
			Self::GetAssertion(value)
		}
	}
	impl ::core::convert::From<GetAssertionCreationBlockForLogLookupCall> for IRollupBoldCalls {
		fn from(value: GetAssertionCreationBlockForLogLookupCall) -> Self {
			Self::GetAssertionCreationBlockForLogLookup(value)
		}
	}
	impl ::core::convert::From<GetFirstChildCreationBlockCall> for IRollupBoldCalls {
		fn from(value: GetFirstChildCreationBlockCall) -> Self {
			Self::GetFirstChildCreationBlock(value)
		}
	}
	impl ::core::convert::From<GetSecondChildCreationBlockCall> for IRollupBoldCalls {
		fn from(value: GetSecondChildCreationBlockCall) -> Self {
			Self::GetSecondChildCreationBlock(value)
		}
	}
	impl ::core::convert::From<GetStakerCall> for IRollupBoldCalls {
		fn from(value: GetStakerCall) -> Self {
			Self::GetStaker(value)
		}
	}
	impl ::core::convert::From<GetStakerAddressCall> for IRollupBoldCalls {
		fn from(value: GetStakerAddressCall) -> Self {
			Self::GetStakerAddress(value)
		}
	}
	impl ::core::convert::From<GetValidatorsCall> for IRollupBoldCalls {
		fn from(value: GetValidatorsCall) -> Self {
			Self::GetValidators(value)
		}
	}
	impl ::core::convert::From<IsFirstChildCall> for IRollupBoldCalls {
		fn from(value: IsFirstChildCall) -> Self {
			Self::IsFirstChild(value)
		}
	}
	impl ::core::convert::From<IsPendingCall> for IRollupBoldCalls {
		fn from(value: IsPendingCall) -> Self {
			Self::IsPending(value)
		}
	}
	impl ::core::convert::From<IsStakedCall> for IRollupBoldCalls {
		fn from(value: IsStakedCall) -> Self {
			Self::IsStaked(value)
		}
	}
	impl ::core::convert::From<IsValidatorCall> for IRollupBoldCalls {
		fn from(value: IsValidatorCall) -> Self {
			Self::IsValidator(value)
		}
	}
	impl ::core::convert::From<LatestConfirmedCall> for IRollupBoldCalls {
		fn from(value: LatestConfirmedCall) -> Self {
			Self::LatestConfirmed(value)
		}
	}
	impl ::core::convert::From<LatestStakedAssertionCall> for IRollupBoldCalls {
		fn from(value: LatestStakedAssertionCall) -> Self {
			Self::LatestStakedAssertion(value)
		}
	}
	impl ::core::convert::From<LoserStakeEscrowCall> for IRollupBoldCalls {
		fn from(value: LoserStakeEscrowCall) -> Self {
			Self::LoserStakeEscrow(value)
		}
	}
	impl ::core::convert::From<MinimumAssertionPeriodCall> for IRollupBoldCalls {
		fn from(value: MinimumAssertionPeriodCall) -> Self {
			Self::MinimumAssertionPeriod(value)
		}
	}
	impl ::core::convert::From<OutboxCall> for IRollupBoldCalls {
		fn from(value: OutboxCall) -> Self {
			Self::Outbox(value)
		}
	}
	impl ::core::convert::From<RollupEventInboxCall> for IRollupBoldCalls {
		fn from(value: RollupEventInboxCall) -> Self {
			Self::RollupEventInbox(value)
		}
	}
	impl ::core::convert::From<SequencerInboxCall> for IRollupBoldCalls {
		fn from(value: SequencerInboxCall) -> Self {
			Self::SequencerInbox(value)
		}
	}
	impl ::core::convert::From<StakeTokenCall> for IRollupBoldCalls {
		fn from(value: StakeTokenCall) -> Self {
			Self::StakeToken(value)
		}
	}
	impl ::core::convert::From<StakerCountCall> for IRollupBoldCalls {
		fn from(value: StakerCountCall) -> Self {
			Self::StakerCount(value)
		}
	}
	impl ::core::convert::From<ValidateAssertionHashCall> for IRollupBoldCalls {
		fn from(value: ValidateAssertionHashCall) -> Self {
			Self::ValidateAssertionHash(value)
		}
	}
	impl ::core::convert::From<ValidateConfigCall> for IRollupBoldCalls {
		fn from(value: ValidateConfigCall) -> Self {
			Self::ValidateConfig(value)
		}
	}
	impl ::core::convert::From<ValidatorAfkBlocksCall> for IRollupBoldCalls {
		fn from(value: ValidatorAfkBlocksCall) -> Self {
			Self::ValidatorAfkBlocks(value)
		}
	}
	impl ::core::convert::From<ValidatorWhitelistDisabledCall> for IRollupBoldCalls {
		fn from(value: ValidatorWhitelistDisabledCall) -> Self {
			Self::ValidatorWhitelistDisabled(value)
		}
	}
	impl ::core::convert::From<WasmModuleRootCall> for IRollupBoldCalls {
		fn from(value: WasmModuleRootCall) -> Self {
			Self::WasmModuleRoot(value)
		}
	}
	impl ::core::convert::From<WithdrawableFundsCall> for IRollupBoldCalls {
		fn from(value: WithdrawableFundsCall) -> Self {
			Self::WithdrawableFunds(value)
		}
	}
	impl ::core::convert::From<WithdrawalAddressCall> for IRollupBoldCalls {
		fn from(value: WithdrawalAddressCall) -> Self {
			Self::WithdrawalAddress(value)
		}
	}
	///Container type for all return fields from the `amountStaked` function with signature
	/// `amountStaked(address)` and selector `0xef40a670`
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
	pub struct AmountStakedReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `baseStake` function with signature
	/// `baseStake()` and selector `0x76e7e23b`
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
	pub struct BaseStakeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `bridge` function with signature `bridge()`
	/// and selector `0xe78cea92`
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
	pub struct BridgeReturn(pub ::ethers::core::types::Address);
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
	///Container type for all return fields from the `challengeManager` function with signature
	/// `challengeManager()` and selector `0x023a96fe`
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
	pub struct ChallengeManagerReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `confirmPeriodBlocks` function with signature
	/// `confirmPeriodBlocks()` and selector `0x2e7acfa6`
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
	pub struct ConfirmPeriodBlocksReturn(pub u64);
	///Container type for all return fields from the `genesisAssertionHash` function with signature
	/// `genesisAssertionHash()` and selector `0x353325e0`
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
	pub struct GenesisAssertionHashReturn(pub [u8; 32]);
	///Container type for all return fields from the `getAssertion` function with signature
	/// `getAssertion(bytes32)` and selector `0x88302884`
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
	pub struct GetAssertionReturn(pub AssertionNode);
	///Container type for all return fields from the `getAssertionCreationBlockForLogLookup`
	/// function with signature `getAssertionCreationBlockForLogLookup(bytes32)` and selector
	/// `0x13c56ca7`
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
	pub struct GetAssertionCreationBlockForLogLookupReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getFirstChildCreationBlock` function with
	/// signature `getFirstChildCreationBlock(bytes32)` and selector `0x11715585`
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
	pub struct GetFirstChildCreationBlockReturn(pub u64);
	///Container type for all return fields from the `getSecondChildCreationBlock` function with
	/// signature `getSecondChildCreationBlock(bytes32)` and selector `0x56bbc9e6`
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
	pub struct GetSecondChildCreationBlockReturn(pub u64);
	///Container type for all return fields from the `getStaker` function with signature
	/// `getStaker(address)` and selector `0xa23c44b1`
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
	pub struct GetStakerReturn(pub Staker);
	///Container type for all return fields from the `getStakerAddress` function with signature
	/// `getStakerAddress(uint64)` and selector `0x6ddd3744`
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
	pub struct GetStakerAddressReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `getValidators` function with signature
	/// `getValidators()` and selector `0xb7ab4db5`
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
	pub struct GetValidatorsReturn(pub ::std::vec::Vec<::ethers::core::types::Address>);
	///Container type for all return fields from the `isFirstChild` function with signature
	/// `isFirstChild(bytes32)` and selector `0x30836228`
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
	pub struct IsFirstChildReturn(pub bool);
	///Container type for all return fields from the `isPending` function with signature
	/// `isPending(bytes32)` and selector `0xe531d8c7`
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
	pub struct IsPendingReturn(pub bool);
	///Container type for all return fields from the `isStaked` function with signature
	/// `isStaked(address)` and selector `0x6177fd18`
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
	pub struct IsStakedReturn(pub bool);
	///Container type for all return fields from the `isValidator` function with signature
	/// `isValidator(address)` and selector `0xfacd743b`
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
	pub struct IsValidatorReturn(pub bool);
	///Container type for all return fields from the `latestConfirmed` function with signature
	/// `latestConfirmed()` and selector `0x65f7f80d`
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
	pub struct LatestConfirmedReturn(pub [u8; 32]);
	///Container type for all return fields from the `latestStakedAssertion` function with
	/// signature `latestStakedAssertion(address)` and selector `0x2abdd230`
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
	pub struct LatestStakedAssertionReturn(pub [u8; 32]);
	///Container type for all return fields from the `loserStakeEscrow` function with signature
	/// `loserStakeEscrow()` and selector `0xf065de3f`
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
	pub struct LoserStakeEscrowReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `minimumAssertionPeriod` function with
	/// signature `minimumAssertionPeriod()` and selector `0x45e38b64`
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
	pub struct MinimumAssertionPeriodReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `outbox` function with signature `outbox()`
	/// and selector `0xce11e6ab`
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
	pub struct OutboxReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `rollupEventInbox` function with signature
	/// `rollupEventInbox()` and selector `0xaa38a6e7`
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
	pub struct RollupEventInboxReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `sequencerInbox` function with signature
	/// `sequencerInbox()` and selector `0xee35f327`
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
	pub struct SequencerInboxReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `stakeToken` function with signature
	/// `stakeToken()` and selector `0x51ed6a30`
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
	pub struct StakeTokenReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `stakerCount` function with signature
	/// `stakerCount()` and selector `0xdff69787`
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
	pub struct StakerCountReturn(pub u64);
	///Container type for all return fields from the `validatorAfkBlocks` function with signature
	/// `validatorAfkBlocks()` and selector `0xe6b3082c`
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
	pub struct ValidatorAfkBlocksReturn(pub u64);
	///Container type for all return fields from the `validatorWhitelistDisabled` function with
	/// signature `validatorWhitelistDisabled()` and selector `0x12ab3d3b`
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
	pub struct ValidatorWhitelistDisabledReturn(pub bool);
	///Container type for all return fields from the `wasmModuleRoot` function with signature
	/// `wasmModuleRoot()` and selector `0x8ee1a126`
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
	pub struct WasmModuleRootReturn(pub [u8; 32]);
	///Container type for all return fields from the `withdrawableFunds` function with signature
	/// `withdrawableFunds(address)` and selector `0x2f30cabd`
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
	pub struct WithdrawableFundsReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `withdrawalAddress` function with signature
	/// `withdrawalAddress(address)` and selector `0x84728cd0`
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
	pub struct WithdrawalAddressReturn(pub ::ethers::core::types::Address);
	///`AssertionInputs((bytes32,bytes32,(bytes32,uint256,address,uint64,uint64)),((bytes32[2],
	/// uint64[2]),uint8,bytes32),((bytes32[2],uint64[2]),uint8,bytes32))`
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
	pub struct AssertionInputs {
		pub before_state_data: BeforeStateData,
		pub before_state: AssertionState,
		pub after_state: AssertionState,
	}
	///`AssertionNode(uint64,uint64,uint64,bool,uint8,bytes32)`
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
	pub struct AssertionNode {
		pub first_child_block: u64,
		pub second_child_block: u64,
		pub created_at_block: u64,
		pub is_first_child: bool,
		pub status: u8,
		pub config_hash: [u8; 32],
	}
	///`AssertionState((bytes32[2],uint64[2]),uint8,bytes32)`
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
	pub struct AssertionState {
		pub global_state: GlobalState,
		pub machine_status: u8,
		pub end_history_root: [u8; 32],
	}
	///`BeforeStateData(bytes32,bytes32,(bytes32,uint256,address,uint64,uint64))`
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
	pub struct BeforeStateData {
		pub prev_prev_assertion_hash: [u8; 32],
		pub sequencer_batch_acc: [u8; 32],
		pub config_data: ConfigData,
	}
	///`ConfigData(bytes32,uint256,address,uint64,uint64)`
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
	pub struct ConfigData {
		pub wasm_module_root: [u8; 32],
		pub required_stake: ::ethers::core::types::U256,
		pub challenge_manager: ::ethers::core::types::Address,
		pub confirm_period_blocks: u64,
		pub next_inbox_position: u64,
	}
}
