pub use mock_module::*;
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
pub mod mock_module {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("host"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("paraId"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint256"),
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
                            outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                                name: ::std::string::String::new(),
                                kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("bytes32"),
                                ),
                            },],
                            constant: ::core::option::Option::None,
                            state_mutability:
                                ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("dispatch"),
                            inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("request"),
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
                            outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                                name: ::std::string::String::new(),
                                kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("bytes32"),
                                ),
                            },],
                            constant: ::core::option::Option::None,
                            state_mutability:
                                ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("dispatchToParachain"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("dispatchToParachain",),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
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
                            name: ::std::borrow::ToOwned::to_owned("response"),
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
                            name: ::std::borrow::ToOwned::to_owned("request"),
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
                    ::std::borrow::ToOwned::to_owned("onPostResponse"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("onPostResponse"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("response"),
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
                    ::std::borrow::ToOwned::to_owned("onPostTimeout"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("onPostTimeout"),
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
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("GetResponseReceived"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("GetResponseReceived",),
                        inputs: ::std::vec![],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("GetTimeoutReceived"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("GetTimeoutReceived"),
                        inputs: ::std::vec![],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("MessageDispatched"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("MessageDispatched"),
                        inputs: ::std::vec![],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostReceived"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("PostReceived"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
                            name: ::std::borrow::ToOwned::to_owned("message"),
                            kind: ::ethers::core::abi::ethabi::ParamType::String,
                            indexed: false,
                        },],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostResponseReceived"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("PostResponseReceived",),
                        inputs: ::std::vec![],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PostTimeoutReceived"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("PostTimeoutReceived",),
                        inputs: ::std::vec![],
                        anonymous: false,
                    },],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("ExecutionFailed"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ExecutionFailed"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotIsmpHost"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("NotIsmpHost"),
                        inputs: ::std::vec![],
                    },],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static MOCKMODULE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    pub struct MockModule<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for MockModule<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for MockModule<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for MockModule<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for MockModule<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(MockModule)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> MockModule<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(address.into(), MOCKMODULE_ABI.clone(), client))
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
        ///Calls the contract's `dispatchToParachain` (0x667f320a) function
        pub fn dispatch_to_parachain(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([102, 127, 50, 10], ())
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
            response: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([243, 112, 253, 187], (response,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetTimeout` (0x4c46c035) function
        pub fn on_get_timeout(
            &self,
            request: GetRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([76, 70, 192, 53], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostResponse` (0xc52c28af) function
        pub fn on_post_response(
            &self,
            response: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([197, 44, 40, 175], (response,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostTimeout` (0xc715f52b) function
        pub fn on_post_timeout(
            &self,
            request: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([199, 21, 245, 43], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `GetResponseReceived` event
        pub fn get_response_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GetResponseReceivedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `GetTimeoutReceived` event
        pub fn get_timeout_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, GetTimeoutReceivedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `MessageDispatched` event
        pub fn message_dispatched_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, MessageDispatchedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `PostReceived` event
        pub fn post_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostReceivedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `PostResponseReceived` event
        pub fn post_response_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostResponseReceivedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `PostTimeoutReceived` event
        pub fn post_timeout_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PostTimeoutReceivedFilter>
        {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, MockModuleEvents> {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for MockModule<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `ExecutionFailed` with signature `ExecutionFailed()` and selector
    /// `0xacfdb444`
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
        Hash,
    )]
    #[etherror(name = "NotIsmpHost", abi = "NotIsmpHost()")]
    pub struct NotIsmpHost;
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum MockModuleErrors {
        ExecutionFailed(ExecutionFailed),
        NotIsmpHost(NotIsmpHost),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for MockModuleErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RevertString(decoded))
            }
            if let Ok(decoded) = <ExecutionFailed as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::ExecutionFailed(decoded))
            }
            if let Ok(decoded) = <NotIsmpHost as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotIsmpHost(decoded))
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for MockModuleErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::ExecutionFailed(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::NotIsmpHost(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for MockModuleErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector == <ExecutionFailed as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector == <NotIsmpHost as ::ethers::contract::EthError>::selector() => true,
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for MockModuleErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ExecutionFailed(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for MockModuleErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<ExecutionFailed> for MockModuleErrors {
        fn from(value: ExecutionFailed) -> Self {
            Self::ExecutionFailed(value)
        }
    }
    impl ::core::convert::From<NotIsmpHost> for MockModuleErrors {
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
        Hash,
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
        Hash,
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
        Hash,
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
        Hash,
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
        Hash,
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
        Hash,
    )]
    #[ethevent(name = "PostTimeoutReceived", abi = "PostTimeoutReceived()")]
    pub struct PostTimeoutReceivedFilter;
    ///Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum MockModuleEvents {
        GetResponseReceivedFilter(GetResponseReceivedFilter),
        GetTimeoutReceivedFilter(GetTimeoutReceivedFilter),
        MessageDispatchedFilter(MessageDispatchedFilter),
        PostReceivedFilter(PostReceivedFilter),
        PostResponseReceivedFilter(PostResponseReceivedFilter),
        PostTimeoutReceivedFilter(PostTimeoutReceivedFilter),
    }
    impl ::ethers::contract::EthLogDecode for MockModuleEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = GetResponseReceivedFilter::decode_log(log) {
                return Ok(MockModuleEvents::GetResponseReceivedFilter(decoded))
            }
            if let Ok(decoded) = GetTimeoutReceivedFilter::decode_log(log) {
                return Ok(MockModuleEvents::GetTimeoutReceivedFilter(decoded))
            }
            if let Ok(decoded) = MessageDispatchedFilter::decode_log(log) {
                return Ok(MockModuleEvents::MessageDispatchedFilter(decoded))
            }
            if let Ok(decoded) = PostReceivedFilter::decode_log(log) {
                return Ok(MockModuleEvents::PostReceivedFilter(decoded))
            }
            if let Ok(decoded) = PostResponseReceivedFilter::decode_log(log) {
                return Ok(MockModuleEvents::PostResponseReceivedFilter(decoded))
            }
            if let Ok(decoded) = PostTimeoutReceivedFilter::decode_log(log) {
                return Ok(MockModuleEvents::PostTimeoutReceivedFilter(decoded))
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for MockModuleEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::GetResponseReceivedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::GetTimeoutReceivedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::MessageDispatchedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::PostReceivedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::PostResponseReceivedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::PostTimeoutReceivedFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<GetResponseReceivedFilter> for MockModuleEvents {
        fn from(value: GetResponseReceivedFilter) -> Self {
            Self::GetResponseReceivedFilter(value)
        }
    }
    impl ::core::convert::From<GetTimeoutReceivedFilter> for MockModuleEvents {
        fn from(value: GetTimeoutReceivedFilter) -> Self {
            Self::GetTimeoutReceivedFilter(value)
        }
    }
    impl ::core::convert::From<MessageDispatchedFilter> for MockModuleEvents {
        fn from(value: MessageDispatchedFilter) -> Self {
            Self::MessageDispatchedFilter(value)
        }
    }
    impl ::core::convert::From<PostReceivedFilter> for MockModuleEvents {
        fn from(value: PostReceivedFilter) -> Self {
            Self::PostReceivedFilter(value)
        }
    }
    impl ::core::convert::From<PostResponseReceivedFilter> for MockModuleEvents {
        fn from(value: PostResponseReceivedFilter) -> Self {
            Self::PostResponseReceivedFilter(value)
        }
    }
    impl ::core::convert::From<PostTimeoutReceivedFilter> for MockModuleEvents {
        fn from(value: PostTimeoutReceivedFilter) -> Self {
            Self::PostTimeoutReceivedFilter(value)
        }
    }
    ///Container type for all input parameters for the `dispatch` function with signature
    /// `dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x31267dee`
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
        abi = "dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct DispatchCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `dispatch` function with signature
    /// `dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector
    /// `0xd1ab46cf`
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
        abi = "dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
    )]
    pub struct DispatchWithRequestCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `dispatchToParachain` function with
    /// signature `dispatchToParachain()` and selector `0x667f320a`
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
    #[ethcall(name = "dispatchToParachain", abi = "dispatchToParachain()")]
    pub struct DispatchToParachainCall;
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
    pub struct OnGetResponseCall {
        pub response: GetResponse,
    }
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
    pub struct OnGetTimeoutCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `onPostResponse` function with signature
    /// `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))` and selector
    /// `0xc52c28af`
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
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))"
    )]
    pub struct OnPostResponseCall {
        pub response: PostResponse,
    }
    ///Container type for all input parameters for the `onPostTimeout` function with signature
    /// `onPostTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector
    /// `0xc715f52b`
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
        name = "onPostTimeout",
        abi = "onPostTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnPostTimeoutCall {
        pub request: PostRequest,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum MockModuleCalls {
        Dispatch(DispatchCall),
        DispatchWithRequest(DispatchWithRequestCall),
        DispatchToParachain(DispatchToParachainCall),
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostTimeout(OnPostTimeoutCall),
    }
    impl ::ethers::core::abi::AbiDecode for MockModuleCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <DispatchCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Dispatch(decoded))
            }
            if let Ok(decoded) =
                <DispatchWithRequestCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::DispatchWithRequest(decoded))
            }
            if let Ok(decoded) =
                <DispatchToParachainCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::DispatchToParachain(decoded))
            }
            if let Ok(decoded) = <OnAcceptCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::OnAccept(decoded))
            }
            if let Ok(decoded) = <OnGetResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OnGetResponse(decoded))
            }
            if let Ok(decoded) = <OnGetTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OnGetTimeout(decoded))
            }
            if let Ok(decoded) =
                <OnPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OnPostResponse(decoded))
            }
            if let Ok(decoded) = <OnPostTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OnPostTimeout(decoded))
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for MockModuleCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::Dispatch(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::DispatchWithRequest(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::DispatchToParachain(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnAccept(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnGetResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnGetTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnPostResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnPostTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for MockModuleCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::Dispatch(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchWithRequest(element) => ::core::fmt::Display::fmt(element, f),
                Self::DispatchToParachain(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostTimeout(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<DispatchCall> for MockModuleCalls {
        fn from(value: DispatchCall) -> Self {
            Self::Dispatch(value)
        }
    }
    impl ::core::convert::From<DispatchWithRequestCall> for MockModuleCalls {
        fn from(value: DispatchWithRequestCall) -> Self {
            Self::DispatchWithRequest(value)
        }
    }
    impl ::core::convert::From<DispatchToParachainCall> for MockModuleCalls {
        fn from(value: DispatchToParachainCall) -> Self {
            Self::DispatchToParachain(value)
        }
    }
    impl ::core::convert::From<OnAcceptCall> for MockModuleCalls {
        fn from(value: OnAcceptCall) -> Self {
            Self::OnAccept(value)
        }
    }
    impl ::core::convert::From<OnGetResponseCall> for MockModuleCalls {
        fn from(value: OnGetResponseCall) -> Self {
            Self::OnGetResponse(value)
        }
    }
    impl ::core::convert::From<OnGetTimeoutCall> for MockModuleCalls {
        fn from(value: OnGetTimeoutCall) -> Self {
            Self::OnGetTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostResponseCall> for MockModuleCalls {
        fn from(value: OnPostResponseCall) -> Self {
            Self::OnPostResponse(value)
        }
    }
    impl ::core::convert::From<OnPostTimeoutCall> for MockModuleCalls {
        fn from(value: OnPostTimeoutCall) -> Self {
            Self::OnPostTimeout(value)
        }
    }
    ///Container type for all return fields from the `dispatch` function with signature
    /// `dispatch((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x31267dee`
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
    pub struct DispatchReturn(pub [u8; 32]);
    ///Container type for all return fields from the `dispatch` function with signature
    /// `dispatch((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector
    /// `0xd1ab46cf`
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
    pub struct DispatchWithRequestReturn(pub [u8; 32]);
}
