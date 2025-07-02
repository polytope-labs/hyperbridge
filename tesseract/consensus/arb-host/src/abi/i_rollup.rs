pub use i_rollup::*;
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
pub mod i_rollup {
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
                                            "contract IChallengeManager",
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
                    ::std::borrow::ToOwned::to_owned("currentChallenge"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("currentChallenge"),
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
                    ::std::borrow::ToOwned::to_owned("extraChallengeTimeBlocks"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "extraChallengeTimeBlocks",
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
                    ::std::borrow::ToOwned::to_owned("firstUnresolvedNode"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "firstUnresolvedNode",
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
                    ::std::borrow::ToOwned::to_owned("getNode"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("getNode"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("nodeNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct Node"),
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
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
                    ::std::borrow::ToOwned::to_owned("isZombie"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("isZombie"),
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
                    ::std::borrow::ToOwned::to_owned("lastStakeBlock"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("lastStakeBlock"),
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
                    ::std::borrow::ToOwned::to_owned("latestConfirmed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("latestConfirmed"),
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
                    ::std::borrow::ToOwned::to_owned("latestNodeCreated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("latestNodeCreated"),
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
                    ::std::borrow::ToOwned::to_owned("latestStakedNode"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("latestStakedNode"),
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
                    ::std::borrow::ToOwned::to_owned("nodeHasStaker"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("nodeHasStaker"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("nodeNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint64"),
                                    ),
                                },
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
                    ::std::borrow::ToOwned::to_owned("zombieAddress"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("zombieAddress"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("zombieNum"),
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
                    ::std::borrow::ToOwned::to_owned("zombieCount"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("zombieCount"),
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
                    ::std::borrow::ToOwned::to_owned("zombieLatestStakedNode"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "zombieLatestStakedNode",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("zombieNum"),
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
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("NodeConfirmed"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("NodeConfirmed"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nodeNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                    ::std::borrow::ToOwned::to_owned("NodeCreated"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("NodeCreated"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nodeNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("parentNodeHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nodeHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("executionHash"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        32usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("assertion"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
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
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
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
                                    name: ::std::borrow::ToOwned::to_owned("wasmModuleRoot"),
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
                            ],
                            anonymous: false,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NodeRejected"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("NodeRejected"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nodeNum"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                    indexed: true,
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
                                    name: ::std::borrow::ToOwned::to_owned("challengedNode"),
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
	pub static IROLLUP_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct IRollup<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for IRollup<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for IRollup<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for IRollup<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for IRollup<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(IRollup)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> IRollup<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), IROLLUP_ABI.clone(), client))
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
		///Calls the contract's `currentChallenge` (0x69fd251c) function
		pub fn current_challenge(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([105, 253, 37, 28], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `extraChallengeTimeBlocks` (0x771b2f97) function
		pub fn extra_challenge_time_blocks(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([119, 27, 47, 151], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `firstUnresolvedNode` (0xd735e21d) function
		pub fn first_unresolved_node(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([215, 53, 226, 29], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getNode` (0x92c8134c) function
		pub fn get_node(
			&self,
			node_num: u64,
		) -> ::ethers::contract::builders::ContractCall<M, Node> {
			self.0
				.method_hash([146, 200, 19, 76], node_num)
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
		///Calls the contract's `isZombie` (0x91c657e8) function
		pub fn is_zombie(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([145, 198, 87, 232], staker)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `lastStakeBlock` (0x8640ce5f) function
		pub fn last_stake_block(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([134, 64, 206, 95], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestConfirmed` (0x65f7f80d) function
		pub fn latest_confirmed(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([101, 247, 248, 13], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestNodeCreated` (0x7ba9534a) function
		pub fn latest_node_created(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([123, 169, 83, 74], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `latestStakedNode` (0x3e96576e) function
		pub fn latest_staked_node(
			&self,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([62, 150, 87, 110], staker)
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
		///Calls the contract's `nodeHasStaker` (0xaa65af48) function
		pub fn node_has_staker(
			&self,
			node_num: u64,
			staker: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([170, 101, 175, 72], (node_num, staker))
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
		///Calls the contract's `zombieAddress` (0xd01e6602) function
		pub fn zombie_address(
			&self,
			zombie_num: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([208, 30, 102, 2], zombie_num)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `zombieCount` (0x63721d6b) function
		pub fn zombie_count(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([99, 114, 29, 107], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `zombieLatestStakedNode` (0xf33e1fac) function
		pub fn zombie_latest_staked_node(
			&self,
			zombie_num: ::ethers::core::types::U256,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([243, 62, 31, 172], zombie_num)
				.expect("method not found (this should never happen)")
		}
		///Gets the contract's `NodeConfirmed` event
		pub fn node_confirmed_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, NodeConfirmedFilter> {
			self.0.event()
		}
		///Gets the contract's `NodeCreated` event
		pub fn node_created_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, NodeCreatedFilter> {
			self.0.event()
		}
		///Gets the contract's `NodeRejected` event
		pub fn node_rejected_filter(
			&self,
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, NodeRejectedFilter> {
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
		) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, IRollupEvents> {
			self.0.event_with_filter(::core::default::Default::default())
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for IRollup<M> {
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
	#[ethevent(name = "NodeConfirmed", abi = "NodeConfirmed(uint64,bytes32,bytes32)")]
	pub struct NodeConfirmedFilter {
		#[ethevent(indexed)]
		pub node_num: u64,
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
		name = "NodeCreated",
		abi = "NodeCreated(uint64,bytes32,bytes32,bytes32,(((bytes32[2],uint64[2]),uint8),((bytes32[2],uint64[2]),uint8),uint64),bytes32,bytes32,uint256)"
	)]
	pub struct NodeCreatedFilter {
		#[ethevent(indexed)]
		pub node_num: u64,
		#[ethevent(indexed)]
		pub parent_node_hash: [u8; 32],
		#[ethevent(indexed)]
		pub node_hash: [u8; 32],
		pub execution_hash: [u8; 32],
		pub assertion: Assertion,
		pub after_inbox_batch_acc: [u8; 32],
		pub wasm_module_root: [u8; 32],
		pub inbox_max_count: ::ethers::core::types::U256,
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
	#[ethevent(name = "NodeRejected", abi = "NodeRejected(uint64)")]
	pub struct NodeRejectedFilter {
		#[ethevent(indexed)]
		pub node_num: u64,
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
		pub challenged_node: u64,
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
	#[ethevent(name = "UserStakeUpdated", abi = "UserStakeUpdated(address,uint256,uint256)")]
	pub struct UserStakeUpdatedFilter {
		#[ethevent(indexed)]
		pub user: ::ethers::core::types::Address,
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
	pub enum IRollupEvents {
		NodeConfirmedFilter(NodeConfirmedFilter),
		NodeCreatedFilter(NodeCreatedFilter),
		NodeRejectedFilter(NodeRejectedFilter),
		RollupChallengeStartedFilter(RollupChallengeStartedFilter),
		RollupInitializedFilter(RollupInitializedFilter),
		UserStakeUpdatedFilter(UserStakeUpdatedFilter),
		UserWithdrawableFundsUpdatedFilter(UserWithdrawableFundsUpdatedFilter),
	}
	impl ::ethers::contract::EthLogDecode for IRollupEvents {
		fn decode_log(
			log: &::ethers::core::abi::RawLog,
		) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
			if let Ok(decoded) = NodeConfirmedFilter::decode_log(log) {
				return Ok(IRollupEvents::NodeConfirmedFilter(decoded));
			}
			if let Ok(decoded) = NodeCreatedFilter::decode_log(log) {
				return Ok(IRollupEvents::NodeCreatedFilter(decoded));
			}
			if let Ok(decoded) = NodeRejectedFilter::decode_log(log) {
				return Ok(IRollupEvents::NodeRejectedFilter(decoded));
			}
			if let Ok(decoded) = RollupChallengeStartedFilter::decode_log(log) {
				return Ok(IRollupEvents::RollupChallengeStartedFilter(decoded));
			}
			if let Ok(decoded) = RollupInitializedFilter::decode_log(log) {
				return Ok(IRollupEvents::RollupInitializedFilter(decoded));
			}
			if let Ok(decoded) = UserStakeUpdatedFilter::decode_log(log) {
				return Ok(IRollupEvents::UserStakeUpdatedFilter(decoded));
			}
			if let Ok(decoded) = UserWithdrawableFundsUpdatedFilter::decode_log(log) {
				return Ok(IRollupEvents::UserWithdrawableFundsUpdatedFilter(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData)
		}
	}
	impl ::core::fmt::Display for IRollupEvents {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::NodeConfirmedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::NodeCreatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::NodeRejectedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::RollupChallengeStartedFilter(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::RollupInitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::UserStakeUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
				Self::UserWithdrawableFundsUpdatedFilter(element) =>
					::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<NodeConfirmedFilter> for IRollupEvents {
		fn from(value: NodeConfirmedFilter) -> Self {
			Self::NodeConfirmedFilter(value)
		}
	}
	impl ::core::convert::From<NodeCreatedFilter> for IRollupEvents {
		fn from(value: NodeCreatedFilter) -> Self {
			Self::NodeCreatedFilter(value)
		}
	}
	impl ::core::convert::From<NodeRejectedFilter> for IRollupEvents {
		fn from(value: NodeRejectedFilter) -> Self {
			Self::NodeRejectedFilter(value)
		}
	}
	impl ::core::convert::From<RollupChallengeStartedFilter> for IRollupEvents {
		fn from(value: RollupChallengeStartedFilter) -> Self {
			Self::RollupChallengeStartedFilter(value)
		}
	}
	impl ::core::convert::From<RollupInitializedFilter> for IRollupEvents {
		fn from(value: RollupInitializedFilter) -> Self {
			Self::RollupInitializedFilter(value)
		}
	}
	impl ::core::convert::From<UserStakeUpdatedFilter> for IRollupEvents {
		fn from(value: UserStakeUpdatedFilter) -> Self {
			Self::UserStakeUpdatedFilter(value)
		}
	}
	impl ::core::convert::From<UserWithdrawableFundsUpdatedFilter> for IRollupEvents {
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
	///Container type for all input parameters for the `currentChallenge` function with signature
	/// `currentChallenge(address)` and selector `0x69fd251c`
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
	#[ethcall(name = "currentChallenge", abi = "currentChallenge(address)")]
	pub struct CurrentChallengeCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `extraChallengeTimeBlocks` function with
	/// signature `extraChallengeTimeBlocks()` and selector `0x771b2f97`
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
	#[ethcall(name = "extraChallengeTimeBlocks", abi = "extraChallengeTimeBlocks()")]
	pub struct ExtraChallengeTimeBlocksCall;
	///Container type for all input parameters for the `firstUnresolvedNode` function with
	/// signature `firstUnresolvedNode()` and selector `0xd735e21d`
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
	#[ethcall(name = "firstUnresolvedNode", abi = "firstUnresolvedNode()")]
	pub struct FirstUnresolvedNodeCall;
	///Container type for all input parameters for the `getNode` function with signature
	/// `getNode(uint64)` and selector `0x92c8134c`
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
	#[ethcall(name = "getNode", abi = "getNode(uint64)")]
	pub struct GetNodeCall {
		pub node_num: u64,
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
	///Container type for all input parameters for the `isZombie` function with signature
	/// `isZombie(address)` and selector `0x91c657e8`
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
	#[ethcall(name = "isZombie", abi = "isZombie(address)")]
	pub struct IsZombieCall {
		pub staker: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `lastStakeBlock` function with signature
	/// `lastStakeBlock()` and selector `0x8640ce5f`
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
	#[ethcall(name = "lastStakeBlock", abi = "lastStakeBlock()")]
	pub struct LastStakeBlockCall;
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
	///Container type for all input parameters for the `latestNodeCreated` function with signature
	/// `latestNodeCreated()` and selector `0x7ba9534a`
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
	#[ethcall(name = "latestNodeCreated", abi = "latestNodeCreated()")]
	pub struct LatestNodeCreatedCall;
	///Container type for all input parameters for the `latestStakedNode` function with signature
	/// `latestStakedNode(address)` and selector `0x3e96576e`
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
	#[ethcall(name = "latestStakedNode", abi = "latestStakedNode(address)")]
	pub struct LatestStakedNodeCall {
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
	///Container type for all input parameters for the `nodeHasStaker` function with signature
	/// `nodeHasStaker(uint64,address)` and selector `0xaa65af48`
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
	#[ethcall(name = "nodeHasStaker", abi = "nodeHasStaker(uint64,address)")]
	pub struct NodeHasStakerCall {
		pub node_num: u64,
		pub staker: ::ethers::core::types::Address,
	}
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
	///Container type for all input parameters for the `zombieAddress` function with signature
	/// `zombieAddress(uint256)` and selector `0xd01e6602`
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
	#[ethcall(name = "zombieAddress", abi = "zombieAddress(uint256)")]
	pub struct ZombieAddressCall {
		pub zombie_num: ::ethers::core::types::U256,
	}
	///Container type for all input parameters for the `zombieCount` function with signature
	/// `zombieCount()` and selector `0x63721d6b`
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
	#[ethcall(name = "zombieCount", abi = "zombieCount()")]
	pub struct ZombieCountCall;
	///Container type for all input parameters for the `zombieLatestStakedNode` function with
	/// signature `zombieLatestStakedNode(uint256)` and selector `0xf33e1fac`
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
	#[ethcall(name = "zombieLatestStakedNode", abi = "zombieLatestStakedNode(uint256)")]
	pub struct ZombieLatestStakedNodeCall {
		pub zombie_num: ::ethers::core::types::U256,
	}
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum IRollupCalls {
		AmountStaked(AmountStakedCall),
		BaseStake(BaseStakeCall),
		Bridge(BridgeCall),
		ChainId(ChainIdCall),
		ChallengeManager(ChallengeManagerCall),
		ConfirmPeriodBlocks(ConfirmPeriodBlocksCall),
		CurrentChallenge(CurrentChallengeCall),
		ExtraChallengeTimeBlocks(ExtraChallengeTimeBlocksCall),
		FirstUnresolvedNode(FirstUnresolvedNodeCall),
		GetNode(GetNodeCall),
		GetStaker(GetStakerCall),
		GetStakerAddress(GetStakerAddressCall),
		IsStaked(IsStakedCall),
		IsValidator(IsValidatorCall),
		IsZombie(IsZombieCall),
		LastStakeBlock(LastStakeBlockCall),
		LatestConfirmed(LatestConfirmedCall),
		LatestNodeCreated(LatestNodeCreatedCall),
		LatestStakedNode(LatestStakedNodeCall),
		LoserStakeEscrow(LoserStakeEscrowCall),
		MinimumAssertionPeriod(MinimumAssertionPeriodCall),
		NodeHasStaker(NodeHasStakerCall),
		Outbox(OutboxCall),
		RollupEventInbox(RollupEventInboxCall),
		SequencerInbox(SequencerInboxCall),
		StakeToken(StakeTokenCall),
		StakerCount(StakerCountCall),
		ValidatorWhitelistDisabled(ValidatorWhitelistDisabledCall),
		WasmModuleRoot(WasmModuleRootCall),
		WithdrawableFunds(WithdrawableFundsCall),
		ZombieAddress(ZombieAddressCall),
		ZombieCount(ZombieCountCall),
		ZombieLatestStakedNode(ZombieLatestStakedNodeCall),
	}
	impl ::ethers::core::abi::AbiDecode for IRollupCalls {
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
				<CurrentChallengeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::CurrentChallenge(decoded));
			}
			if let Ok(decoded) =
				<ExtraChallengeTimeBlocksCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ExtraChallengeTimeBlocks(decoded));
			}
			if let Ok(decoded) =
				<FirstUnresolvedNodeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::FirstUnresolvedNode(decoded));
			}
			if let Ok(decoded) = <GetNodeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetNode(decoded));
			}
			if let Ok(decoded) = <GetStakerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetStaker(decoded));
			}
			if let Ok(decoded) =
				<GetStakerAddressCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetStakerAddress(decoded));
			}
			if let Ok(decoded) = <IsStakedCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsStaked(decoded));
			}
			if let Ok(decoded) = <IsValidatorCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsValidator(decoded));
			}
			if let Ok(decoded) = <IsZombieCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsZombie(decoded));
			}
			if let Ok(decoded) =
				<LastStakeBlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LastStakeBlock(decoded));
			}
			if let Ok(decoded) =
				<LatestConfirmedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestConfirmed(decoded));
			}
			if let Ok(decoded) =
				<LatestNodeCreatedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestNodeCreated(decoded));
			}
			if let Ok(decoded) =
				<LatestStakedNodeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::LatestStakedNode(decoded));
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
			if let Ok(decoded) = <NodeHasStakerCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::NodeHasStaker(decoded));
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
			if let Ok(decoded) = <ZombieAddressCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ZombieAddress(decoded));
			}
			if let Ok(decoded) = <ZombieCountCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::ZombieCount(decoded));
			}
			if let Ok(decoded) =
				<ZombieLatestStakedNodeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::ZombieLatestStakedNode(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for IRollupCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::AmountStaked(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BaseStake(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Bridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChainId(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ChallengeManager(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ConfirmPeriodBlocks(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::CurrentChallenge(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ExtraChallengeTimeBlocks(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::FirstUnresolvedNode(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetNode(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetStaker(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetStakerAddress(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsStaked(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsValidator(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsZombie(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LastStakeBlock(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestConfirmed(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestNodeCreated(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LatestStakedNode(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::LoserStakeEscrow(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::MinimumAssertionPeriod(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::NodeHasStaker(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Outbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::RollupEventInbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SequencerInbox(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StakeToken(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::StakerCount(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ValidatorWhitelistDisabled(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::WasmModuleRoot(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::WithdrawableFunds(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ZombieAddress(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ZombieCount(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::ZombieLatestStakedNode(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for IRollupCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::AmountStaked(element) => ::core::fmt::Display::fmt(element, f),
				Self::BaseStake(element) => ::core::fmt::Display::fmt(element, f),
				Self::Bridge(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChainId(element) => ::core::fmt::Display::fmt(element, f),
				Self::ChallengeManager(element) => ::core::fmt::Display::fmt(element, f),
				Self::ConfirmPeriodBlocks(element) => ::core::fmt::Display::fmt(element, f),
				Self::CurrentChallenge(element) => ::core::fmt::Display::fmt(element, f),
				Self::ExtraChallengeTimeBlocks(element) => ::core::fmt::Display::fmt(element, f),
				Self::FirstUnresolvedNode(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetNode(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetStaker(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetStakerAddress(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsStaked(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsValidator(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsZombie(element) => ::core::fmt::Display::fmt(element, f),
				Self::LastStakeBlock(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestConfirmed(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestNodeCreated(element) => ::core::fmt::Display::fmt(element, f),
				Self::LatestStakedNode(element) => ::core::fmt::Display::fmt(element, f),
				Self::LoserStakeEscrow(element) => ::core::fmt::Display::fmt(element, f),
				Self::MinimumAssertionPeriod(element) => ::core::fmt::Display::fmt(element, f),
				Self::NodeHasStaker(element) => ::core::fmt::Display::fmt(element, f),
				Self::Outbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::RollupEventInbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::SequencerInbox(element) => ::core::fmt::Display::fmt(element, f),
				Self::StakeToken(element) => ::core::fmt::Display::fmt(element, f),
				Self::StakerCount(element) => ::core::fmt::Display::fmt(element, f),
				Self::ValidatorWhitelistDisabled(element) => ::core::fmt::Display::fmt(element, f),
				Self::WasmModuleRoot(element) => ::core::fmt::Display::fmt(element, f),
				Self::WithdrawableFunds(element) => ::core::fmt::Display::fmt(element, f),
				Self::ZombieAddress(element) => ::core::fmt::Display::fmt(element, f),
				Self::ZombieCount(element) => ::core::fmt::Display::fmt(element, f),
				Self::ZombieLatestStakedNode(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<AmountStakedCall> for IRollupCalls {
		fn from(value: AmountStakedCall) -> Self {
			Self::AmountStaked(value)
		}
	}
	impl ::core::convert::From<BaseStakeCall> for IRollupCalls {
		fn from(value: BaseStakeCall) -> Self {
			Self::BaseStake(value)
		}
	}
	impl ::core::convert::From<BridgeCall> for IRollupCalls {
		fn from(value: BridgeCall) -> Self {
			Self::Bridge(value)
		}
	}
	impl ::core::convert::From<ChainIdCall> for IRollupCalls {
		fn from(value: ChainIdCall) -> Self {
			Self::ChainId(value)
		}
	}
	impl ::core::convert::From<ChallengeManagerCall> for IRollupCalls {
		fn from(value: ChallengeManagerCall) -> Self {
			Self::ChallengeManager(value)
		}
	}
	impl ::core::convert::From<ConfirmPeriodBlocksCall> for IRollupCalls {
		fn from(value: ConfirmPeriodBlocksCall) -> Self {
			Self::ConfirmPeriodBlocks(value)
		}
	}
	impl ::core::convert::From<CurrentChallengeCall> for IRollupCalls {
		fn from(value: CurrentChallengeCall) -> Self {
			Self::CurrentChallenge(value)
		}
	}
	impl ::core::convert::From<ExtraChallengeTimeBlocksCall> for IRollupCalls {
		fn from(value: ExtraChallengeTimeBlocksCall) -> Self {
			Self::ExtraChallengeTimeBlocks(value)
		}
	}
	impl ::core::convert::From<FirstUnresolvedNodeCall> for IRollupCalls {
		fn from(value: FirstUnresolvedNodeCall) -> Self {
			Self::FirstUnresolvedNode(value)
		}
	}
	impl ::core::convert::From<GetNodeCall> for IRollupCalls {
		fn from(value: GetNodeCall) -> Self {
			Self::GetNode(value)
		}
	}
	impl ::core::convert::From<GetStakerCall> for IRollupCalls {
		fn from(value: GetStakerCall) -> Self {
			Self::GetStaker(value)
		}
	}
	impl ::core::convert::From<GetStakerAddressCall> for IRollupCalls {
		fn from(value: GetStakerAddressCall) -> Self {
			Self::GetStakerAddress(value)
		}
	}
	impl ::core::convert::From<IsStakedCall> for IRollupCalls {
		fn from(value: IsStakedCall) -> Self {
			Self::IsStaked(value)
		}
	}
	impl ::core::convert::From<IsValidatorCall> for IRollupCalls {
		fn from(value: IsValidatorCall) -> Self {
			Self::IsValidator(value)
		}
	}
	impl ::core::convert::From<IsZombieCall> for IRollupCalls {
		fn from(value: IsZombieCall) -> Self {
			Self::IsZombie(value)
		}
	}
	impl ::core::convert::From<LastStakeBlockCall> for IRollupCalls {
		fn from(value: LastStakeBlockCall) -> Self {
			Self::LastStakeBlock(value)
		}
	}
	impl ::core::convert::From<LatestConfirmedCall> for IRollupCalls {
		fn from(value: LatestConfirmedCall) -> Self {
			Self::LatestConfirmed(value)
		}
	}
	impl ::core::convert::From<LatestNodeCreatedCall> for IRollupCalls {
		fn from(value: LatestNodeCreatedCall) -> Self {
			Self::LatestNodeCreated(value)
		}
	}
	impl ::core::convert::From<LatestStakedNodeCall> for IRollupCalls {
		fn from(value: LatestStakedNodeCall) -> Self {
			Self::LatestStakedNode(value)
		}
	}
	impl ::core::convert::From<LoserStakeEscrowCall> for IRollupCalls {
		fn from(value: LoserStakeEscrowCall) -> Self {
			Self::LoserStakeEscrow(value)
		}
	}
	impl ::core::convert::From<MinimumAssertionPeriodCall> for IRollupCalls {
		fn from(value: MinimumAssertionPeriodCall) -> Self {
			Self::MinimumAssertionPeriod(value)
		}
	}
	impl ::core::convert::From<NodeHasStakerCall> for IRollupCalls {
		fn from(value: NodeHasStakerCall) -> Self {
			Self::NodeHasStaker(value)
		}
	}
	impl ::core::convert::From<OutboxCall> for IRollupCalls {
		fn from(value: OutboxCall) -> Self {
			Self::Outbox(value)
		}
	}
	impl ::core::convert::From<RollupEventInboxCall> for IRollupCalls {
		fn from(value: RollupEventInboxCall) -> Self {
			Self::RollupEventInbox(value)
		}
	}
	impl ::core::convert::From<SequencerInboxCall> for IRollupCalls {
		fn from(value: SequencerInboxCall) -> Self {
			Self::SequencerInbox(value)
		}
	}
	impl ::core::convert::From<StakeTokenCall> for IRollupCalls {
		fn from(value: StakeTokenCall) -> Self {
			Self::StakeToken(value)
		}
	}
	impl ::core::convert::From<StakerCountCall> for IRollupCalls {
		fn from(value: StakerCountCall) -> Self {
			Self::StakerCount(value)
		}
	}
	impl ::core::convert::From<ValidatorWhitelistDisabledCall> for IRollupCalls {
		fn from(value: ValidatorWhitelistDisabledCall) -> Self {
			Self::ValidatorWhitelistDisabled(value)
		}
	}
	impl ::core::convert::From<WasmModuleRootCall> for IRollupCalls {
		fn from(value: WasmModuleRootCall) -> Self {
			Self::WasmModuleRoot(value)
		}
	}
	impl ::core::convert::From<WithdrawableFundsCall> for IRollupCalls {
		fn from(value: WithdrawableFundsCall) -> Self {
			Self::WithdrawableFunds(value)
		}
	}
	impl ::core::convert::From<ZombieAddressCall> for IRollupCalls {
		fn from(value: ZombieAddressCall) -> Self {
			Self::ZombieAddress(value)
		}
	}
	impl ::core::convert::From<ZombieCountCall> for IRollupCalls {
		fn from(value: ZombieCountCall) -> Self {
			Self::ZombieCount(value)
		}
	}
	impl ::core::convert::From<ZombieLatestStakedNodeCall> for IRollupCalls {
		fn from(value: ZombieLatestStakedNodeCall) -> Self {
			Self::ZombieLatestStakedNode(value)
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
	///Container type for all return fields from the `currentChallenge` function with signature
	/// `currentChallenge(address)` and selector `0x69fd251c`
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
	pub struct CurrentChallengeReturn(pub u64);
	///Container type for all return fields from the `extraChallengeTimeBlocks` function with
	/// signature `extraChallengeTimeBlocks()` and selector `0x771b2f97`
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
	pub struct ExtraChallengeTimeBlocksReturn(pub u64);
	///Container type for all return fields from the `firstUnresolvedNode` function with signature
	/// `firstUnresolvedNode()` and selector `0xd735e21d`
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
	pub struct FirstUnresolvedNodeReturn(pub u64);
	///Container type for all return fields from the `getNode` function with signature
	/// `getNode(uint64)` and selector `0x92c8134c`
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
	pub struct GetNodeReturn(pub Node);
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
	///Container type for all return fields from the `isZombie` function with signature
	/// `isZombie(address)` and selector `0x91c657e8`
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
	pub struct IsZombieReturn(pub bool);
	///Container type for all return fields from the `lastStakeBlock` function with signature
	/// `lastStakeBlock()` and selector `0x8640ce5f`
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
	pub struct LastStakeBlockReturn(pub u64);
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
	pub struct LatestConfirmedReturn(pub u64);
	///Container type for all return fields from the `latestNodeCreated` function with signature
	/// `latestNodeCreated()` and selector `0x7ba9534a`
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
	pub struct LatestNodeCreatedReturn(pub u64);
	///Container type for all return fields from the `latestStakedNode` function with signature
	/// `latestStakedNode(address)` and selector `0x3e96576e`
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
	pub struct LatestStakedNodeReturn(pub u64);
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
	///Container type for all return fields from the `nodeHasStaker` function with signature
	/// `nodeHasStaker(uint64,address)` and selector `0xaa65af48`
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
	pub struct NodeHasStakerReturn(pub bool);
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
	///Container type for all return fields from the `zombieAddress` function with signature
	/// `zombieAddress(uint256)` and selector `0xd01e6602`
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
	pub struct ZombieAddressReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `zombieCount` function with signature
	/// `zombieCount()` and selector `0x63721d6b`
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
	pub struct ZombieCountReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `zombieLatestStakedNode` function with
	/// signature `zombieLatestStakedNode(uint256)` and selector `0xf33e1fac`
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
	pub struct ZombieLatestStakedNodeReturn(pub u64);
	///`Node(bytes32,bytes32,bytes32,uint64,uint64,uint64,uint64,uint64,uint64,uint64,uint64,
	/// bytes32)`
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
	pub struct Node {
		pub state_hash: [u8; 32],
		pub challenge_hash: [u8; 32],
		pub confirm_data: [u8; 32],
		pub prev_num: u64,
		pub deadline_block: u64,
		pub no_child_confirmed_before_block: u64,
		pub staker_count: u64,
		pub child_staker_count: u64,
		pub first_child_block: u64,
		pub latest_child_number: u64,
		pub created_at_block: u64,
		pub node_hash: [u8; 32],
	}
	///`Assertion(((bytes32[2],uint64[2]),uint8),((bytes32[2],uint64[2]),uint8),uint64)`
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
	pub struct Assertion {
		pub before_state: ExecutionState,
		pub after_state: ExecutionState,
		pub num_blocks: u64,
	}
	///`ExecutionState((bytes32[2],uint64[2]),uint8)`
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
	pub struct ExecutionState {
		pub global_state: GlobalState,
		pub machine_status: u8,
	}
}
