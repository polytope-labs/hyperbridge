pub use beefy::*;
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
pub mod beefy {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
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
                    ::std::borrow::ToOwned::to_owned("AURA_CONSENSUS_ID"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("AURA_CONSENSUS_ID"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        4usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes4"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ISMP_CONSENSUS_ID"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("ISMP_CONSENSUS_ID"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        4usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes4"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("MMR_ROOT_PAYLOAD_ID"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "MMR_ROOT_PAYLOAD_ID",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(
                                        2usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes2"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SLOT_DURATION"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("SLOT_DURATION"),
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
                    ::std::borrow::ToOwned::to_owned("verifyConsensus"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("verifyConsensus"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("trustedState"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct BeefyConsensusState",
                                        ),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("proof"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                                        ::std::boxed::Box::new(
                                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                                ::std::vec![
                                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(2usize),
                                                                                    ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                                ],
                                                                            ),
                                                                        ),
                                                                    ),
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                        ],
                                                                    ),
                                                                ),
                                                            ),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                ::std::vec![
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                ],
                                                            ),
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                        ),
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                        ],
                                                                    ),
                                                                ),
                                                            ),
                                                        ),
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                        ],
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Array(
                                                        ::std::boxed::Box::new(
                                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                                ::std::boxed::Box::new(
                                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                                        ::std::vec![
                                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                                        ],
                                                                    ),
                                                                ),
                                                            ),
                                                        ),
                                                    ),
                                                ],
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct BeefyConsensusProof",
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
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct BeefyConsensusState",
                                        ),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct IntermediateState"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("verifyConsensus"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("encodedState"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("encodedProof"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bytes"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ],
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct IntermediateState"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
            ]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static BEEFY_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    pub struct Beefy<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for Beefy<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for Beefy<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for Beefy<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for Beefy<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(Beefy)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> Beefy<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(address.into(), BEEFY_ABI.clone(), client))
        }
        ///Calls the contract's `AURA_CONSENSUS_ID` (0x4e9fdbec) function
        pub fn aura_consensus_id(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 4]> {
            self.0
                .method_hash([78, 159, 219, 236], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `ISMP_CONSENSUS_ID` (0xbabb3118) function
        pub fn ismp_consensus_id(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 4]> {
            self.0
                .method_hash([186, 187, 49, 24], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `MMR_ROOT_PAYLOAD_ID` (0xaf8b91d6) function
        pub fn mmr_root_payload_id(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, [u8; 2]> {
            self.0
                .method_hash([175, 139, 145, 214], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `SLOT_DURATION` (0x905c0511) function
        pub fn slot_duration(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([144, 92, 5, 17], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `verifyConsensus` (0x5e399aea) function
        pub fn verify_consensus(
            &self,
            trusted_state: BeefyConsensusState,
            proof: BeefyConsensusProof,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            (
                (
                    ::ethers::core::types::U256,
                    ::ethers::core::types::U256,
                    (::ethers::core::types::U256, ::ethers::core::types::U256, [u8; 32]),
                    (::ethers::core::types::U256, ::ethers::core::types::U256, [u8; 32]),
                ),
                IntermediateState,
            ),
        > {
            self.0
                .method_hash([94, 57, 154, 234], (trusted_state, proof))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `verifyConsensus` (0x7d755598) function
        pub fn verify_consensus_with_encoded_state_and_encoded_proof(
            &self,
            encoded_state: ::ethers::core::types::Bytes,
            encoded_proof: ::ethers::core::types::Bytes,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            (::ethers::core::types::Bytes, IntermediateState),
        > {
            self.0
                .method_hash([125, 117, 85, 152], (encoded_state, encoded_proof))
                .expect("method not found (this should never happen)")
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for Beefy<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Container type for all input parameters for the `AURA_CONSENSUS_ID` function with signature
    /// `AURA_CONSENSUS_ID()` and selector `0x4e9fdbec`
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
    #[ethcall(name = "AURA_CONSENSUS_ID", abi = "AURA_CONSENSUS_ID()")]
    pub struct AuraConsensusIdCall;
    ///Container type for all input parameters for the `ISMP_CONSENSUS_ID` function with signature
    /// `ISMP_CONSENSUS_ID()` and selector `0xbabb3118`
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
    #[ethcall(name = "ISMP_CONSENSUS_ID", abi = "ISMP_CONSENSUS_ID()")]
    pub struct IsmpConsensusIdCall;
    ///Container type for all input parameters for the `MMR_ROOT_PAYLOAD_ID` function with
    /// signature `MMR_ROOT_PAYLOAD_ID()` and selector `0xaf8b91d6`
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
    #[ethcall(name = "MMR_ROOT_PAYLOAD_ID", abi = "MMR_ROOT_PAYLOAD_ID()")]
    pub struct MmrRootPayloadIdCall;
    ///Container type for all input parameters for the `SLOT_DURATION` function with signature
    /// `SLOT_DURATION()` and selector `0x905c0511`
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
    #[ethcall(name = "SLOT_DURATION", abi = "SLOT_DURATION()")]
    pub struct SlotDurationCall;
    ///Container type for all input parameters for the `verifyConsensus` function with signature
    /// `verifyConsensus((uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32)),
    /// (((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,bytes32,(uint256,
    /// uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[][]),((uint256,
    /// uint256,bytes),(uint256,bytes32)[][])))` and selector `0x5e399aea`
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
        name = "verifyConsensus",
        abi = "verifyConsensus((uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32)),(((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[][]),((uint256,uint256,bytes),(uint256,bytes32)[][])))"
    )]
    pub struct VerifyConsensusCall {
        pub trusted_state: BeefyConsensusState,
        pub proof: BeefyConsensusProof,
    }
    ///Container type for all input parameters for the `verifyConsensus` function with signature
    /// `verifyConsensus(bytes,bytes)` and selector `0x7d755598`
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
    #[ethcall(name = "verifyConsensus", abi = "verifyConsensus(bytes,bytes)")]
    pub struct VerifyConsensusWithEncodedStateAndEncodedProofCall {
        pub encoded_state: ::ethers::core::types::Bytes,
        pub encoded_proof: ::ethers::core::types::Bytes,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum BeefyCalls {
        AuraConsensusId(AuraConsensusIdCall),
        IsmpConsensusId(IsmpConsensusIdCall),
        MmrRootPayloadId(MmrRootPayloadIdCall),
        SlotDuration(SlotDurationCall),
        VerifyConsensus(VerifyConsensusCall),
        VerifyConsensusWithEncodedStateAndEncodedProof(
            VerifyConsensusWithEncodedStateAndEncodedProofCall,
        ),
    }
    impl ::ethers::core::abi::AbiDecode for BeefyCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <AuraConsensusIdCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::AuraConsensusId(decoded));
            }
            if let Ok(decoded) =
                <IsmpConsensusIdCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::IsmpConsensusId(decoded));
            }
            if let Ok(decoded) =
                <MmrRootPayloadIdCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::MmrRootPayloadId(decoded));
            }
            if let Ok(decoded) = <SlotDurationCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::SlotDuration(decoded));
            }
            if let Ok(decoded) =
                <VerifyConsensusCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::VerifyConsensus(decoded));
            }
            if let Ok(decoded) = <VerifyConsensusWithEncodedStateAndEncodedProofCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::VerifyConsensusWithEncodedStateAndEncodedProof(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for BeefyCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::AuraConsensusId(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::IsmpConsensusId(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::MmrRootPayloadId(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::SlotDuration(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::VerifyConsensus(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::VerifyConsensusWithEncodedStateAndEncodedProof(element) =>
                    ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for BeefyCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AuraConsensusId(element) => ::core::fmt::Display::fmt(element, f),
                Self::IsmpConsensusId(element) => ::core::fmt::Display::fmt(element, f),
                Self::MmrRootPayloadId(element) => ::core::fmt::Display::fmt(element, f),
                Self::SlotDuration(element) => ::core::fmt::Display::fmt(element, f),
                Self::VerifyConsensus(element) => ::core::fmt::Display::fmt(element, f),
                Self::VerifyConsensusWithEncodedStateAndEncodedProof(element) =>
                    ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<AuraConsensusIdCall> for BeefyCalls {
        fn from(value: AuraConsensusIdCall) -> Self {
            Self::AuraConsensusId(value)
        }
    }
    impl ::core::convert::From<IsmpConsensusIdCall> for BeefyCalls {
        fn from(value: IsmpConsensusIdCall) -> Self {
            Self::IsmpConsensusId(value)
        }
    }
    impl ::core::convert::From<MmrRootPayloadIdCall> for BeefyCalls {
        fn from(value: MmrRootPayloadIdCall) -> Self {
            Self::MmrRootPayloadId(value)
        }
    }
    impl ::core::convert::From<SlotDurationCall> for BeefyCalls {
        fn from(value: SlotDurationCall) -> Self {
            Self::SlotDuration(value)
        }
    }
    impl ::core::convert::From<VerifyConsensusCall> for BeefyCalls {
        fn from(value: VerifyConsensusCall) -> Self {
            Self::VerifyConsensus(value)
        }
    }
    impl ::core::convert::From<VerifyConsensusWithEncodedStateAndEncodedProofCall> for BeefyCalls {
        fn from(value: VerifyConsensusWithEncodedStateAndEncodedProofCall) -> Self {
            Self::VerifyConsensusWithEncodedStateAndEncodedProof(value)
        }
    }
    ///Container type for all return fields from the `AURA_CONSENSUS_ID` function with signature
    /// `AURA_CONSENSUS_ID()` and selector `0x4e9fdbec`
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
    pub struct AuraConsensusIdReturn(pub [u8; 4]);
    ///Container type for all return fields from the `ISMP_CONSENSUS_ID` function with signature
    /// `ISMP_CONSENSUS_ID()` and selector `0xbabb3118`
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
    pub struct IsmpConsensusIdReturn(pub [u8; 4]);
    ///Container type for all return fields from the `MMR_ROOT_PAYLOAD_ID` function with signature
    /// `MMR_ROOT_PAYLOAD_ID()` and selector `0xaf8b91d6`
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
    pub struct MmrRootPayloadIdReturn(pub [u8; 2]);
    ///Container type for all return fields from the `SLOT_DURATION` function with signature
    /// `SLOT_DURATION()` and selector `0x905c0511`
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
    pub struct SlotDurationReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `verifyConsensus` function with signature
    /// `verifyConsensus((uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32)),
    /// (((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,bytes32,(uint256,
    /// uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[][]),((uint256,
    /// uint256,bytes),(uint256,bytes32)[][])))` and selector `0x5e399aea`
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
    pub struct VerifyConsensusReturn(
        pub  (
            ::ethers::core::types::U256,
            ::ethers::core::types::U256,
            (::ethers::core::types::U256, ::ethers::core::types::U256, [u8; 32]),
            (::ethers::core::types::U256, ::ethers::core::types::U256, [u8; 32]),
        ),
        pub IntermediateState,
    );
    ///Container type for all return fields from the `verifyConsensus` function with signature
    /// `verifyConsensus(bytes,bytes)` and selector `0x7d755598`
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
    pub struct VerifyConsensusWithEncodedStateAndEncodedProofReturn(
        pub ::ethers::core::types::Bytes,
        pub IntermediateState,
    );
    ///`AuthoritySetCommitment(uint256,uint256,bytes32)`
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
    pub struct AuthoritySetCommitment {
        pub id: ::ethers::core::types::U256,
        pub len: ::ethers::core::types::U256,
        pub root: [u8; 32],
    }
    ///`BeefyConsensusProof(((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,
    /// uint256,bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,
    /// bytes32)[]),((uint256,uint256,bytes),(uint256,bytes32)[]))`
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
    pub struct BeefyConsensusProof {
        pub relay: RelayChainProof,
        pub parachain: ParachainProof,
    }
    ///`BeefyConsensusState(uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32))`
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
    pub struct BeefyConsensusState {
        pub latest_height: ::ethers::core::types::U256,
        pub beefy_activation_block: ::ethers::core::types::U256,
        pub current_authority_set: AuthoritySetCommitment,
        pub next_authority_set: AuthoritySetCommitment,
    }
    ///`BeefyMmrLeaf(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256)`
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
    pub struct BeefyMmrLeaf {
        pub version: ::ethers::core::types::U256,
        pub parent_number: ::ethers::core::types::U256,
        pub parent_hash: [u8; 32],
        pub next_authority_set: AuthoritySetCommitment,
        pub extra: [u8; 32],
        pub k_index: ::ethers::core::types::U256,
        pub leaf_index: ::ethers::core::types::U256,
    }
    ///`Commitment((bytes2,bytes)[],uint256,uint256)`
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
    pub struct Commitment {
        pub payload: ::std::vec::Vec<Payload>,
        pub block_number: ::ethers::core::types::U256,
        pub validator_set_id: ::ethers::core::types::U256,
    }
    ///`IntermediateState(uint256,uint256,(uint256,bytes32,bytes32))`
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
    pub struct IntermediateState {
        pub state_machine_id: ::ethers::core::types::U256,
        pub height: ::ethers::core::types::U256,
        pub commitment: StateCommitment,
    }
    ///`Node(uint256,bytes32)`
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
        pub k_index: ::ethers::core::types::U256,
        pub node: [u8; 32],
    }
    ///`Parachain(uint256,uint256,bytes)`
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
    pub struct Parachain {
        pub index: ::ethers::core::types::U256,
        pub id: ::ethers::core::types::U256,
        pub header: ::ethers::core::types::Bytes,
    }
    ///`ParachainProof((uint256,uint256,bytes),(uint256,bytes32)[])`
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
    pub struct ParachainProof {
        pub parachain: Parachain,
        pub proof: ::std::vec::Vec<::std::vec::Vec<Node>>,
    }
    ///`Payload(bytes2,bytes)`
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
    pub struct Payload {
        pub id: [u8; 2],
        pub data: ::ethers::core::types::Bytes,
    }
    ///`RelayChainProof((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,
    /// bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[])`
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
    pub struct RelayChainProof {
        pub signed_commitment: SignedCommitment,
        pub latest_mmr_leaf: BeefyMmrLeaf,
        pub mmr_proof: ::std::vec::Vec<[u8; 32]>,
        pub proof: ::std::vec::Vec<::std::vec::Vec<Node>>,
    }
    ///`SignedCommitment(((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[])`
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
    pub struct SignedCommitment {
        pub commitment: Commitment,
        pub votes: ::std::vec::Vec<Vote>,
    }
    ///`Vote(bytes,uint256)`
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
    pub struct Vote {
        pub signature: ::ethers::core::types::Bytes,
        pub authority_index: ::ethers::core::types::U256,
    }
}
