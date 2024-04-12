pub use handler::*;
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
pub mod handler {
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
                    ::std::borrow::ToOwned::to_owned("handleGetRequestTimeouts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "handleGetRequestTimeouts",
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
                    ::std::borrow::ToOwned::to_owned("handlePostRequestTimeouts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "handlePostRequestTimeouts",
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
                                            "struct PostRequestTimeoutMessage",
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
                    ::std::borrow::ToOwned::to_owned("handlePostResponseTimeouts"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "handlePostResponseTimeouts",
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
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
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
                                                                ],
                                                            ),
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
                                            "struct PostResponseTimeoutMessage",
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
                                                                        ],
                                                                    ),
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
    pub static HANDLER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    pub struct Handler<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for Handler<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for Handler<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for Handler<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for Handler<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(Handler)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> Handler<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(address.into(), HANDLER_ABI.clone(), client))
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
        ///Calls the contract's `handleGetRequestTimeouts` (0x8fcfc947) function
        pub fn handle_get_request_timeouts(
            &self,
            host: ::ethers::core::types::Address,
            message: GetTimeoutMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([143, 207, 201, 71], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handleGetResponses` (0x4e2def1e) function
        pub fn handle_get_responses(
            &self,
            host: ::ethers::core::types::Address,
            message: GetResponseMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([78, 45, 239, 30], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostRequestTimeouts` (0x089b174c) function
        pub fn handle_post_request_timeouts(
            &self,
            host: ::ethers::core::types::Address,
            message: PostRequestTimeoutMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([8, 155, 23, 76], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostRequests` (0x9d38eb35) function
        pub fn handle_post_requests(
            &self,
            host: ::ethers::core::types::Address,
            request: PostRequestMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([157, 56, 235, 53], (host, request))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostResponseTimeouts` (0xe407f86b) function
        pub fn handle_post_response_timeouts(
            &self,
            host: ::ethers::core::types::Address,
            message: PostResponseTimeoutMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([228, 7, 248, 107], (host, message))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `handlePostResponses` (0x72becccd) function
        pub fn handle_post_responses(
            &self,
            host: ::ethers::core::types::Address,
            response: PostResponseMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([114, 190, 204, 205], (host, response))
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
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for Handler<M> {
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
    ///Container type for all input parameters for the `handleGetRequestTimeouts` function with
    /// signature `handleGetRequestTimeouts(address,((bytes,bytes,uint64,bytes,uint64,bytes[],
    /// uint64)[]))` and selector `0x8fcfc947`
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
        name = "handleGetRequestTimeouts",
        abi = "handleGetRequestTimeouts(address,((bytes,bytes,uint64,bytes,uint64,bytes[],uint64)[]))"
    )]
    pub struct HandleGetRequestTimeoutsCall {
        pub host: ::ethers::core::types::Address,
        pub message: GetTimeoutMessage,
    }
    ///Container type for all input parameters for the `handleGetResponses` function with signature
    /// `handleGetResponses(address,(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,
    /// bytes[],uint64)[]))` and selector `0x4e2def1e`
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
        abi = "handleGetResponses(address,(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,bytes[],uint64)[]))"
    )]
    pub struct HandleGetResponsesCall {
        pub host: ::ethers::core::types::Address,
        pub message: GetResponseMessage,
    }
    ///Container type for all input parameters for the `handlePostRequestTimeouts` function with
    /// signature `handlePostRequestTimeouts(address,((bytes,bytes,uint64,bytes,bytes,uint64,
    /// bytes)[],(uint256,uint256),bytes[]))` and selector `0x089b174c`
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
        name = "handlePostRequestTimeouts",
        abi = "handlePostRequestTimeouts(address,((bytes,bytes,uint64,bytes,bytes,uint64,bytes)[],(uint256,uint256),bytes[]))"
    )]
    pub struct HandlePostRequestTimeoutsCall {
        pub host: ::ethers::core::types::Address,
        pub message: PostRequestTimeoutMessage,
    }
    ///Container type for all input parameters for the `handlePostRequests` function with signature
    /// `handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,
    /// bytes,bytes,uint64,bytes),uint256,uint256)[]))` and selector `0x9d38eb35`
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
        abi = "handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,uint64,bytes),uint256,uint256)[]))"
    )]
    pub struct HandlePostRequestsCall {
        pub host: ::ethers::core::types::Address,
        pub request: PostRequestMessage,
    }
    ///Container type for all input parameters for the `handlePostResponseTimeouts` function with
    /// signature `handlePostResponseTimeouts(address,(((bytes,bytes,uint64,bytes,bytes,uint64,
    /// bytes),bytes,uint64)[],(uint256,uint256),bytes[]))` and selector `0xe407f86b`
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
        name = "handlePostResponseTimeouts",
        abi = "handlePostResponseTimeouts(address,(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64)[],(uint256,uint256),bytes[]))"
    )]
    pub struct HandlePostResponseTimeoutsCall {
        pub host: ::ethers::core::types::Address,
        pub message: PostResponseTimeoutMessage,
    }
    ///Container type for all input parameters for the `handlePostResponses` function with
    /// signature `handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,
    /// bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,uint256)[]))` and selector
    /// `0x72becccd`
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
        abi = "handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,uint256)[]))"
    )]
    pub struct HandlePostResponsesCall {
        pub host: ::ethers::core::types::Address,
        pub response: PostResponseMessage,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum HandlerCalls {
        HandleConsensus(HandleConsensusCall),
        HandleGetRequestTimeouts(HandleGetRequestTimeoutsCall),
        HandleGetResponses(HandleGetResponsesCall),
        HandlePostRequestTimeouts(HandlePostRequestTimeoutsCall),
        HandlePostRequests(HandlePostRequestsCall),
        HandlePostResponseTimeouts(HandlePostResponseTimeoutsCall),
        HandlePostResponses(HandlePostResponsesCall),
    }
    impl ::ethers::core::abi::AbiDecode for HandlerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <HandleConsensusCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleConsensus(decoded));
            }
            if let Ok(decoded) =
                <HandleGetRequestTimeoutsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleGetRequestTimeouts(decoded));
            }
            if let Ok(decoded) =
                <HandleGetResponsesCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandleGetResponses(decoded));
            }
            if let Ok(decoded) =
                <HandlePostRequestTimeoutsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostRequestTimeouts(decoded));
            }
            if let Ok(decoded) =
                <HandlePostRequestsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostRequests(decoded));
            }
            if let Ok(decoded) =
                <HandlePostResponseTimeoutsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostResponseTimeouts(decoded));
            }
            if let Ok(decoded) =
                <HandlePostResponsesCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::HandlePostResponses(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for HandlerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::HandleConsensus(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandleGetRequestTimeouts(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandleGetResponses(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandlePostRequestTimeouts(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandlePostRequests(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandlePostResponseTimeouts(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
                Self::HandlePostResponses(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for HandlerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::HandleConsensus(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandleGetRequestTimeouts(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandleGetResponses(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostRequestTimeouts(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostRequests(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostResponseTimeouts(element) => ::core::fmt::Display::fmt(element, f),
                Self::HandlePostResponses(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<HandleConsensusCall> for HandlerCalls {
        fn from(value: HandleConsensusCall) -> Self {
            Self::HandleConsensus(value)
        }
    }
    impl ::core::convert::From<HandleGetRequestTimeoutsCall> for HandlerCalls {
        fn from(value: HandleGetRequestTimeoutsCall) -> Self {
            Self::HandleGetRequestTimeouts(value)
        }
    }
    impl ::core::convert::From<HandleGetResponsesCall> for HandlerCalls {
        fn from(value: HandleGetResponsesCall) -> Self {
            Self::HandleGetResponses(value)
        }
    }
    impl ::core::convert::From<HandlePostRequestTimeoutsCall> for HandlerCalls {
        fn from(value: HandlePostRequestTimeoutsCall) -> Self {
            Self::HandlePostRequestTimeouts(value)
        }
    }
    impl ::core::convert::From<HandlePostRequestsCall> for HandlerCalls {
        fn from(value: HandlePostRequestsCall) -> Self {
            Self::HandlePostRequests(value)
        }
    }
    impl ::core::convert::From<HandlePostResponseTimeoutsCall> for HandlerCalls {
        fn from(value: HandlePostResponseTimeoutsCall) -> Self {
            Self::HandlePostResponseTimeouts(value)
        }
    }
    impl ::core::convert::From<HandlePostResponsesCall> for HandlerCalls {
        fn from(value: HandlePostResponsesCall) -> Self {
            Self::HandlePostResponses(value)
        }
    }
    ///`GetResponseMessage(bytes[],(uint256,uint256),(bytes,bytes,uint64,bytes,uint64,bytes[],
    /// uint64)[])`
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
    ///`GetTimeoutMessage((bytes,bytes,uint64,bytes,uint64,bytes[],uint64)[])`
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
    ///`PostRequestLeaf((bytes,bytes,uint64,bytes,bytes,uint64,bytes),uint256,uint256)`
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
    /// uint64,bytes),uint256,uint256)[])`
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
    ///`PostRequestTimeoutMessage((bytes,bytes,uint64,bytes,bytes,uint64,bytes)[],(uint256,
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
    pub struct PostRequestTimeoutMessage {
        pub timeouts: ::std::vec::Vec<PostRequest>,
        pub height: StateMachineHeight,
        pub proof: ::std::vec::Vec<::ethers::core::types::Bytes>,
    }
    ///`PostResponseLeaf(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,
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
    /// bytes,uint64,bytes),bytes,uint64),uint256,uint256)[])`
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
    ///`PostResponseTimeoutMessage(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64)[],
    /// (uint256,uint256),bytes[])`
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
    pub struct PostResponseTimeoutMessage {
        pub timeouts: ::std::vec::Vec<PostResponse>,
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
