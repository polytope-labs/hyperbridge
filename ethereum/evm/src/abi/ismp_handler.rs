pub use ismp_handler::*;
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
pub mod ismp_handler {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("handleConsensus"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("handleConsensus"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("proof"),
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
                    ::std::borrow::ToOwned::to_owned("handleGetResponses"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("handleGetResponses"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
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
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct GetResponseMessage",
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
                    ::std::borrow::ToOwned::to_owned("handleGetTimeouts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("handleGetTimeouts"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
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
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetTimeoutMessage"),
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
                    ::std::borrow::ToOwned::to_owned("handlePostRequests"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("handlePostRequests"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                        ),
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
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
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct PostRequestMessage",
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
                    ::std::borrow::ToOwned::to_owned("handlePostResponses"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "handlePostResponses",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                        ),
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                                                        ],
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct PostResponseMessage",
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
                    ::std::borrow::ToOwned::to_owned("handlePostTimeouts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("handlePostTimeouts"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("contract IIsmpHost"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
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
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct PostTimeoutMessage",
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
            ]),
            events: ::core::convert::From::from([
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
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
    pub static ISMPHANDLER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    pub struct IsmpHandler<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for IsmpHandler<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for IsmpHandler<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for IsmpHandler<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for IsmpHandler<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(IsmpHandler)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> IsmpHandler<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(address.into(), ISMPHANDLER_ABI.clone(), client))
        }
        ///Calls the contract's `handleConsensus` (0xbb1689be) function
        pub fn handle_consensus(
            &self,
            host: ::ethers::core::types::Address,
            proof: ::ethers::core::types::Bytes,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([187, 22, 137, 190], (host, proof))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handleGetResponses` (0x873ce1ce) function
        pub fn handle_get_responses(
            &self,
            host: ::ethers::core::types::Address,
            message: GetResponseMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([135, 60, 225, 206], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handleGetTimeouts` (0xac269bd6) function
        pub fn handle_get_timeouts(
            &self,
            host: ::ethers::core::types::Address,
            message: GetTimeoutMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([172, 38, 155, 214], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostRequests` (0xfda626c3) function
        pub fn handle_post_requests(
            &self,
            host: ::ethers::core::types::Address,
            request: PostRequestMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([253, 166, 38, 195], (host, request))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostResponses` (0x20d71c7a) function
        pub fn handle_post_responses(
            &self,
            host: ::ethers::core::types::Address,
            response: PostResponseMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([32, 215, 28, 122], (host, response))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostTimeouts` (0xd95e4fbb) function
        pub fn handle_post_timeouts(
            &self,
            host: ::ethers::core::types::Address,
            message: PostTimeoutMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([217, 94, 79, 187], (host, message))
                .expect("method not found (this should never happen)")
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
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StateMachineUpdatedFilter>
        {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for IsmpHandler<M> {
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
    #[ethevent(name = "StateMachineUpdated", abi = "StateMachineUpdated(uint256,uint256)")]
    pub struct StateMachineUpdatedFilter {
        pub state_machine_id: ::ethers::core::types::U256,
        pub height: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `handleConsensus` function with signature
    /// `handleConsensus(address,bytes)` and selector `0xbb1689be`
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
    #[ethcall(name = "handleConsensus", abi = "handleConsensus(address,bytes)")]
    pub struct HandleConsensusCall {
        pub host: ::ethers::core::types::Address,
        pub proof: ::ethers::core::types::Bytes,
    }
    ///Container type for all input parameters for the `handleGetResponses` function with signature
    /// `handleGetResponses(address,(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,
    /// bytes[],uint64,uint64)[]))` and selector `0x873ce1ce`
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
        name = "handleGetResponses",
        abi = "handleGetResponses(address,(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64)[]))"
    )]
    pub struct HandleGetResponsesCall {
        pub host: ::ethers::core::types::Address,
        pub message: GetResponseMessage,
    }
    ///Container type for all input parameters for the `handleGetTimeouts` function with signature
    /// `handleGetTimeouts(address,((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64)[]))` and
    /// selector `0xac269bd6`
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
        name = "handleGetTimeouts",
        abi = "handleGetTimeouts(address,((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64)[]))"
    )]
    pub struct HandleGetTimeoutsCall {
        pub host: ::ethers::core::types::Address,
        pub message: GetTimeoutMessage,
    }
    ///Container type for all input parameters for the `handlePostRequests` function with signature
    /// `handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,
    /// bytes,bytes,uint64,bytes,uint64),uint256,uint256)[]))` and selector `0xfda626c3`
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
        name = "handlePostRequests",
        abi = "handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),uint256,uint256)[]))"
    )]
    pub struct HandlePostRequestsCall {
        pub host: ::ethers::core::types::Address,
        pub request: PostRequestMessage,
    }
    ///Container type for all input parameters for the `handlePostResponses` function with
    /// signature `handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,
    /// bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes),uint256,uint256)[]))` and selector
    /// `0x20d71c7a`
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
        name = "handlePostResponses",
        abi = "handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes),uint256,uint256)[]))"
    )]
    pub struct HandlePostResponsesCall {
        pub host: ::ethers::core::types::Address,
        pub response: PostResponseMessage,
    }
    ///Container type for all input parameters for the `handlePostTimeouts` function with signature
    /// `handlePostTimeouts(address,((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64)[],
    /// (uint256,uint256),bytes[]))` and selector `0xd95e4fbb`
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
        name = "handlePostTimeouts",
        abi = "handlePostTimeouts(address,((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64)[],(uint256,uint256),bytes[]))"
    )]
    pub struct HandlePostTimeoutsCall {
        pub host: ::ethers::core::types::Address,
        pub message: PostTimeoutMessage,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum IsmpHandlerCalls {
        HandleConsensus(HandleConsensusCall),
        HandleGetResponses(HandleGetResponsesCall),
        HandleGetTimeouts(HandleGetTimeoutsCall),
        HandlePostRequests(HandlePostRequestsCall),
        HandlePostResponses(HandlePostResponsesCall),
        HandlePostTimeouts(HandlePostTimeoutsCall),
    }
    impl ::ethers::core::abi::AbiDecode for IsmpHandlerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <HandleConsensusCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleConsensus(decoded))
            }
            if let Ok(decoded) =
                <HandleGetResponsesCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleGetResponses(decoded))
            }
            if let Ok(decoded) =
                <HandleGetTimeoutsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleGetTimeouts(decoded))
            }
            if let Ok(decoded) =
                <HandlePostRequestsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostRequests(decoded))
            }
            if let Ok(decoded) =
                <HandlePostResponsesCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostResponses(decoded))
            }
            if let Ok(decoded) =
                <HandlePostTimeoutsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostTimeouts(decoded))
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for IsmpHandlerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::HandleConsensus(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandleGetResponses(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::HandleGetTimeouts(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandlePostRequests(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::HandlePostResponses(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::HandlePostTimeouts(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for IsmpHandlerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::HandleConsensus(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandleGetResponses(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandleGetTimeouts(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostRequests(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostResponses(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostTimeouts(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<HandleConsensusCall> for IsmpHandlerCalls {
        fn from(value: HandleConsensusCall) -> Self {
            Self::HandleConsensus(value)
        }
    }
    impl ::core::convert::From<HandleGetResponsesCall> for IsmpHandlerCalls {
        fn from(value: HandleGetResponsesCall) -> Self {
            Self::HandleGetResponses(value)
        }
    }
    impl ::core::convert::From<HandleGetTimeoutsCall> for IsmpHandlerCalls {
        fn from(value: HandleGetTimeoutsCall) -> Self {
            Self::HandleGetTimeouts(value)
        }
    }
    impl ::core::convert::From<HandlePostRequestsCall> for IsmpHandlerCalls {
        fn from(value: HandlePostRequestsCall) -> Self {
            Self::HandlePostRequests(value)
        }
    }
    impl ::core::convert::From<HandlePostResponsesCall> for IsmpHandlerCalls {
        fn from(value: HandlePostResponsesCall) -> Self {
            Self::HandlePostResponses(value)
        }
    }
    impl ::core::convert::From<HandlePostTimeoutsCall> for IsmpHandlerCalls {
        fn from(value: HandlePostTimeoutsCall) -> Self {
            Self::HandlePostTimeouts(value)
        }
    }
    ///`GetResponseMessage(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,bytes[],
    /// uint64,uint64)[])`
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
    pub struct GetResponseMessage {
        pub proof: ::std::vec::Vec<::ethers::core::types::Bytes>,
        pub height: StateMachineHeight,
        pub requests: ::std::vec::Vec<GetRequest>,
    }
    ///`GetTimeoutMessage((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64)[])`
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
    pub struct GetTimeoutMessage {
        pub timeouts: ::std::vec::Vec<GetRequest>,
    }
    ///`PostRequestLeaf((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),uint256,uint256)`
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
    pub struct PostRequestLeaf {
        pub request: PostRequest,
        pub index: ::ethers::core::types::U256,
        pub k_index: ::ethers::core::types::U256,
    }
    ///`PostRequestMessage(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,
    /// uint64,bytes,uint64),uint256,uint256)[])`
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
    pub struct PostRequestMessage {
        pub proof: Proof,
        pub requests: ::std::vec::Vec<PostRequestLeaf>,
    }
    ///`PostResponseLeaf(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes),uint256,
    /// uint256)`
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
    pub struct PostResponseLeaf {
        pub response: PostResponse,
        pub index: ::ethers::core::types::U256,
        pub k_index: ::ethers::core::types::U256,
    }
    ///`PostResponseMessage(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,
    /// bytes,uint64,bytes,uint64),bytes),uint256,uint256)[])`
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
    pub struct PostResponseMessage {
        pub proof: Proof,
        pub responses: ::std::vec::Vec<PostResponseLeaf>,
    }
    ///`PostTimeoutMessage((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64)[],(uint256,
    /// uint256),bytes[])`
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
    pub struct PostTimeoutMessage {
        pub timeouts: ::std::vec::Vec<PostRequest>,
        pub height: StateMachineHeight,
        pub proof: ::std::vec::Vec<::ethers::core::types::Bytes>,
    }
    ///`Proof((uint256,uint256),bytes32[],uint256)`
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
    pub struct Proof {
        pub height: StateMachineHeight,
        pub multiproof: ::std::vec::Vec<[u8; 32]>,
        pub leaf_count: ::ethers::core::types::U256,
    }
}
