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
            constructor: ::core::option::Option::None,
            functions: ::core::convert::From::from([
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
                    ::std::borrow::ToOwned::to_owned("noOp"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("noOp"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("s"),
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
                                    name: ::std::borrow::ToOwned::to_owned("p"),
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
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
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
                (
                    ::std::borrow::ToOwned::to_owned("verifyConsensus"),
                    ::std::vec![
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
                                    kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                        ::std::boxed::Box::new(
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                        ),
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned(
                                            "struct IntermediateState[]",
                                        ),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
                        },
                    ],
                ),
            ]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("IllegalGenesisBlock"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "IllegalGenesisBlock",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidAuthoritiesProof"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidAuthoritiesProof",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidMmrProof"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("InvalidMmrProof"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidUltraPlonkProof"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "InvalidUltraPlonkProof",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("MmrRootHashMissing"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("MmrRootHashMissing"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("StaleHeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("StaleHeight"),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SuperMajorityRequired"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "SuperMajorityRequired",
                            ),
                            inputs: ::std::vec![],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnknownAuthoritySet"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "UnknownAuthoritySet",
                            ),
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
		///Calls the contract's `MMR_ROOT_PAYLOAD_ID` (0xaf8b91d6) function
		pub fn mmr_root_payload_id(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, [u8; 2]> {
			self.0
				.method_hash([175, 139, 145, 214], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `noOp` (0x756087e2) function
		pub fn no_op(
			&self,
			s: BeefyConsensusState,
			p: BeefyConsensusProof,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([117, 96, 135, 226], (s, p))
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
		///Calls the contract's `verifyConsensus` (0x7d755598) function
		pub fn verify_consensus(
			&self,
			encoded_state: ::ethers::core::types::Bytes,
			encoded_proof: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(::ethers::core::types::Bytes, ::std::vec::Vec<IntermediateState>),
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
	///Custom Error type `IllegalGenesisBlock` with signature `IllegalGenesisBlock()` and selector
	/// `0xb4eb9e51`
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
	#[etherror(name = "IllegalGenesisBlock", abi = "IllegalGenesisBlock()")]
	pub struct IllegalGenesisBlock;
	///Custom Error type `InvalidAuthoritiesProof` with signature `InvalidAuthoritiesProof()` and
	/// selector `0x528bd3ef`
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
	#[etherror(name = "InvalidAuthoritiesProof", abi = "InvalidAuthoritiesProof()")]
	pub struct InvalidAuthoritiesProof;
	///Custom Error type `InvalidMmrProof` with signature `InvalidMmrProof()` and selector
	/// `0x5c90c348`
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
	#[etherror(name = "InvalidMmrProof", abi = "InvalidMmrProof()")]
	pub struct InvalidMmrProof;
	///Custom Error type `InvalidUltraPlonkProof` with signature `InvalidUltraPlonkProof()` and
	/// selector `0x866dc22c`
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
	#[etherror(name = "InvalidUltraPlonkProof", abi = "InvalidUltraPlonkProof()")]
	pub struct InvalidUltraPlonkProof;
	///Custom Error type `MmrRootHashMissing` with signature `MmrRootHashMissing()` and selector
	/// `0x8c6238e4`
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
	#[etherror(name = "MmrRootHashMissing", abi = "MmrRootHashMissing()")]
	pub struct MmrRootHashMissing;
	///Custom Error type `StaleHeight` with signature `StaleHeight()` and selector `0xbeda4fc3`
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
	#[etherror(name = "StaleHeight", abi = "StaleHeight()")]
	pub struct StaleHeight;
	///Custom Error type `SuperMajorityRequired` with signature `SuperMajorityRequired()` and
	/// selector `0xeaa43dfc`
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
	#[etherror(name = "SuperMajorityRequired", abi = "SuperMajorityRequired()")]
	pub struct SuperMajorityRequired;
	///Custom Error type `UnknownAuthoritySet` with signature `UnknownAuthoritySet()` and selector
	/// `0xe405cd0a`
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
	#[etherror(name = "UnknownAuthoritySet", abi = "UnknownAuthoritySet()")]
	pub struct UnknownAuthoritySet;
	///Container type for all of the contract's custom errors
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum BeefyErrors {
		IllegalGenesisBlock(IllegalGenesisBlock),
		InvalidAuthoritiesProof(InvalidAuthoritiesProof),
		InvalidMmrProof(InvalidMmrProof),
		InvalidUltraPlonkProof(InvalidUltraPlonkProof),
		MmrRootHashMissing(MmrRootHashMissing),
		StaleHeight(StaleHeight),
		SuperMajorityRequired(SuperMajorityRequired),
		UnknownAuthoritySet(UnknownAuthoritySet),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for BeefyErrors {
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
				<IllegalGenesisBlock as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::IllegalGenesisBlock(decoded));
			}
			if let Ok(decoded) =
				<InvalidAuthoritiesProof as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidAuthoritiesProof(decoded));
			}
			if let Ok(decoded) = <InvalidMmrProof as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::InvalidMmrProof(decoded));
			}
			if let Ok(decoded) =
				<InvalidUltraPlonkProof as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::InvalidUltraPlonkProof(decoded));
			}
			if let Ok(decoded) =
				<MmrRootHashMissing as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MmrRootHashMissing(decoded));
			}
			if let Ok(decoded) = <StaleHeight as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::StaleHeight(decoded));
			}
			if let Ok(decoded) =
				<SuperMajorityRequired as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SuperMajorityRequired(decoded));
			}
			if let Ok(decoded) =
				<UnknownAuthoritySet as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnknownAuthoritySet(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for BeefyErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::IllegalGenesisBlock(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidAuthoritiesProof(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidMmrProof(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::InvalidUltraPlonkProof(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::MmrRootHashMissing(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StaleHeight(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SuperMajorityRequired(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::UnknownAuthoritySet(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for BeefyErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<IllegalGenesisBlock as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidAuthoritiesProof as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <InvalidMmrProof as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<InvalidUltraPlonkProof as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<MmrRootHashMissing as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <StaleHeight as ::ethers::contract::EthError>::selector() => true,
				_ if selector ==
					<SuperMajorityRequired as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector ==
					<UnknownAuthoritySet as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for BeefyErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::IllegalGenesisBlock(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidAuthoritiesProof(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidMmrProof(element) => ::core::fmt::Display::fmt(element, f),
				Self::InvalidUltraPlonkProof(element) => ::core::fmt::Display::fmt(element, f),
				Self::MmrRootHashMissing(element) => ::core::fmt::Display::fmt(element, f),
				Self::StaleHeight(element) => ::core::fmt::Display::fmt(element, f),
				Self::SuperMajorityRequired(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnknownAuthoritySet(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for BeefyErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<IllegalGenesisBlock> for BeefyErrors {
		fn from(value: IllegalGenesisBlock) -> Self {
			Self::IllegalGenesisBlock(value)
		}
	}
	impl ::core::convert::From<InvalidAuthoritiesProof> for BeefyErrors {
		fn from(value: InvalidAuthoritiesProof) -> Self {
			Self::InvalidAuthoritiesProof(value)
		}
	}
	impl ::core::convert::From<InvalidMmrProof> for BeefyErrors {
		fn from(value: InvalidMmrProof) -> Self {
			Self::InvalidMmrProof(value)
		}
	}
	impl ::core::convert::From<InvalidUltraPlonkProof> for BeefyErrors {
		fn from(value: InvalidUltraPlonkProof) -> Self {
			Self::InvalidUltraPlonkProof(value)
		}
	}
	impl ::core::convert::From<MmrRootHashMissing> for BeefyErrors {
		fn from(value: MmrRootHashMissing) -> Self {
			Self::MmrRootHashMissing(value)
		}
	}
	impl ::core::convert::From<StaleHeight> for BeefyErrors {
		fn from(value: StaleHeight) -> Self {
			Self::StaleHeight(value)
		}
	}
	impl ::core::convert::From<SuperMajorityRequired> for BeefyErrors {
		fn from(value: SuperMajorityRequired) -> Self {
			Self::SuperMajorityRequired(value)
		}
	}
	impl ::core::convert::From<UnknownAuthoritySet> for BeefyErrors {
		fn from(value: UnknownAuthoritySet) -> Self {
			Self::UnknownAuthoritySet(value)
		}
	}
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
	///Container type for all input parameters for the `noOp` function with signature
	/// `noOp((uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32)),(((((bytes2,
	/// bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,bytes32,(uint256,uint256,
	/// bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[][]),((uint256,uint256,bytes),
	/// (uint256,bytes32)[][])))` and selector `0x756087e2`
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
		name = "noOp",
		abi = "noOp((uint256,uint256,(uint256,uint256,bytes32),(uint256,uint256,bytes32)),(((((bytes2,bytes)[],uint256,uint256),(bytes,uint256)[]),(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32,uint256,uint256),bytes32[],(uint256,bytes32)[][]),((uint256,uint256,bytes),(uint256,bytes32)[][])))"
	)]
	pub struct NoOpCall {
		pub s: BeefyConsensusState,
		pub p: BeefyConsensusProof,
	}
	///Container type for all input parameters for the `supportsInterface` function with signature
	/// `supportsInterface(bytes4)` and selector `0x01ffc9a7`
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
	#[ethcall(name = "supportsInterface", abi = "supportsInterface(bytes4)")]
	pub struct SupportsInterfaceCall {
		pub interface_id: [u8; 4],
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
	pub struct VerifyConsensusCall {
		pub encoded_state: ::ethers::core::types::Bytes,
		pub encoded_proof: ::ethers::core::types::Bytes,
	}
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum BeefyCalls {
		MmrRootPayloadId(MmrRootPayloadIdCall),
		NoOp(NoOpCall),
		SupportsInterface(SupportsInterfaceCall),
		VerifyConsensus(VerifyConsensusCall),
	}
	impl ::ethers::core::abi::AbiDecode for BeefyCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) =
				<MmrRootPayloadIdCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::MmrRootPayloadId(decoded));
			}
			if let Ok(decoded) = <NoOpCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::NoOp(decoded));
			}
			if let Ok(decoded) =
				<SupportsInterfaceCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SupportsInterface(decoded));
			}
			if let Ok(decoded) =
				<VerifyConsensusCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::VerifyConsensus(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for BeefyCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::MmrRootPayloadId(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::NoOp(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SupportsInterface(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::VerifyConsensus(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for BeefyCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::MmrRootPayloadId(element) => ::core::fmt::Display::fmt(element, f),
				Self::NoOp(element) => ::core::fmt::Display::fmt(element, f),
				Self::SupportsInterface(element) => ::core::fmt::Display::fmt(element, f),
				Self::VerifyConsensus(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<MmrRootPayloadIdCall> for BeefyCalls {
		fn from(value: MmrRootPayloadIdCall) -> Self {
			Self::MmrRootPayloadId(value)
		}
	}
	impl ::core::convert::From<NoOpCall> for BeefyCalls {
		fn from(value: NoOpCall) -> Self {
			Self::NoOp(value)
		}
	}
	impl ::core::convert::From<SupportsInterfaceCall> for BeefyCalls {
		fn from(value: SupportsInterfaceCall) -> Self {
			Self::SupportsInterface(value)
		}
	}
	impl ::core::convert::From<VerifyConsensusCall> for BeefyCalls {
		fn from(value: VerifyConsensusCall) -> Self {
			Self::VerifyConsensus(value)
		}
	}
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
	///Container type for all return fields from the `supportsInterface` function with signature
	/// `supportsInterface(bytes4)` and selector `0x01ffc9a7`
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
	pub struct SupportsInterfaceReturn(pub bool);
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
	pub struct VerifyConsensusReturn(
		pub ::ethers::core::types::Bytes,
		pub ::std::vec::Vec<IntermediateState>,
	);
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
