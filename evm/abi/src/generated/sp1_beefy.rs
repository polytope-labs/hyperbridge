pub use sp1_beefy::*;
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
pub mod sp1_beefy {
	pub use super::super::shared_types::*;
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("verifier"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("contract ISP1Verifier"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
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
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
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
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                        ],
                                                    ),
                                                ),
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct SP1BeefyProof"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("p"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize),
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PublicInputs"),
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
                    ::std::borrow::ToOwned::to_owned("verificationKey"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("verificationKey"),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
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
                    ::std::borrow::ToOwned::to_owned("StaleHeight"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("StaleHeight"),
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
	pub static SP1BEEFY_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct SP1Beefy<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for SP1Beefy<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for SP1Beefy<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for SP1Beefy<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for SP1Beefy<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(SP1Beefy)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> SP1Beefy<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), SP1BEEFY_ABI.clone(), client))
		}
		///Calls the contract's `noOp` (0x09a07dd3) function
		pub fn no_op(
			&self,
			s: Sp1BeefyProof,
			p: PublicInputs,
		) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([9, 160, 125, 211], (s, p))
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
		///Calls the contract's `verificationKey` (0x7ddc907d) function
		pub fn verification_key(&self) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
			self.0
				.method_hash([125, 220, 144, 125], ())
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
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for SP1Beefy<M> {
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
	pub enum SP1BeefyErrors {
		IllegalGenesisBlock(IllegalGenesisBlock),
		StaleHeight(StaleHeight),
		UnknownAuthoritySet(UnknownAuthoritySet),
		/// The standard solidity revert string, with selector
		/// Error(string) -- 0x08c379a0
		RevertString(::std::string::String),
	}
	impl ::ethers::core::abi::AbiDecode for SP1BeefyErrors {
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
			if let Ok(decoded) = <StaleHeight as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::StaleHeight(decoded));
			}
			if let Ok(decoded) =
				<UnknownAuthoritySet as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::UnknownAuthoritySet(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for SP1BeefyErrors {
		fn encode(self) -> ::std::vec::Vec<u8> {
			match self {
				Self::IllegalGenesisBlock(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::StaleHeight(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::UnknownAuthoritySet(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
			}
		}
	}
	impl ::ethers::contract::ContractRevert for SP1BeefyErrors {
		fn valid_selector(selector: [u8; 4]) -> bool {
			match selector {
				[0x08, 0xc3, 0x79, 0xa0] => true,
				_ if selector ==
					<IllegalGenesisBlock as ::ethers::contract::EthError>::selector() =>
					true,
				_ if selector == <StaleHeight as ::ethers::contract::EthError>::selector() => true,
				_ if selector ==
					<UnknownAuthoritySet as ::ethers::contract::EthError>::selector() =>
					true,
				_ => false,
			}
		}
	}
	impl ::core::fmt::Display for SP1BeefyErrors {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::IllegalGenesisBlock(element) => ::core::fmt::Display::fmt(element, f),
				Self::StaleHeight(element) => ::core::fmt::Display::fmt(element, f),
				Self::UnknownAuthoritySet(element) => ::core::fmt::Display::fmt(element, f),
				Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
			}
		}
	}
	impl ::core::convert::From<::std::string::String> for SP1BeefyErrors {
		fn from(value: String) -> Self {
			Self::RevertString(value)
		}
	}
	impl ::core::convert::From<IllegalGenesisBlock> for SP1BeefyErrors {
		fn from(value: IllegalGenesisBlock) -> Self {
			Self::IllegalGenesisBlock(value)
		}
	}
	impl ::core::convert::From<StaleHeight> for SP1BeefyErrors {
		fn from(value: StaleHeight) -> Self {
			Self::StaleHeight(value)
		}
	}
	impl ::core::convert::From<UnknownAuthoritySet> for SP1BeefyErrors {
		fn from(value: UnknownAuthoritySet) -> Self {
			Self::UnknownAuthoritySet(value)
		}
	}
	///Container type for all input parameters for the `noOp` function with signature
	/// `noOp(((uint256,uint256),(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32),
	/// (uint256,bytes)[],bytes),(bytes32,uint256,bytes32,bytes32[]))` and selector `0x09a07dd3`
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
		abi = "noOp(((uint256,uint256),(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32),(uint256,bytes)[],bytes),(bytes32,uint256,bytes32,bytes32[]))"
	)]
	pub struct NoOpCall {
		pub s: Sp1BeefyProof,
		pub p: PublicInputs,
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
	///Container type for all input parameters for the `verificationKey` function with signature
	/// `verificationKey()` and selector `0x7ddc907d`
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
	#[ethcall(name = "verificationKey", abi = "verificationKey()")]
	pub struct VerificationKeyCall;
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
	pub enum SP1BeefyCalls {
		NoOp(NoOpCall),
		SupportsInterface(SupportsInterfaceCall),
		VerificationKey(VerificationKeyCall),
		VerifyConsensus(VerifyConsensusCall),
	}
	impl ::ethers::core::abi::AbiDecode for SP1BeefyCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <NoOpCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::NoOp(decoded));
			}
			if let Ok(decoded) =
				<SupportsInterfaceCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::SupportsInterface(decoded));
			}
			if let Ok(decoded) =
				<VerificationKeyCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::VerificationKey(decoded));
			}
			if let Ok(decoded) =
				<VerifyConsensusCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::VerifyConsensus(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for SP1BeefyCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::NoOp(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SupportsInterface(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::VerificationKey(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::VerifyConsensus(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for SP1BeefyCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::NoOp(element) => ::core::fmt::Display::fmt(element, f),
				Self::SupportsInterface(element) => ::core::fmt::Display::fmt(element, f),
				Self::VerificationKey(element) => ::core::fmt::Display::fmt(element, f),
				Self::VerifyConsensus(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<NoOpCall> for SP1BeefyCalls {
		fn from(value: NoOpCall) -> Self {
			Self::NoOp(value)
		}
	}
	impl ::core::convert::From<SupportsInterfaceCall> for SP1BeefyCalls {
		fn from(value: SupportsInterfaceCall) -> Self {
			Self::SupportsInterface(value)
		}
	}
	impl ::core::convert::From<VerificationKeyCall> for SP1BeefyCalls {
		fn from(value: VerificationKeyCall) -> Self {
			Self::VerificationKey(value)
		}
	}
	impl ::core::convert::From<VerifyConsensusCall> for SP1BeefyCalls {
		fn from(value: VerifyConsensusCall) -> Self {
			Self::VerifyConsensus(value)
		}
	}
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
	///Container type for all return fields from the `verificationKey` function with signature
	/// `verificationKey()` and selector `0x7ddc907d`
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
	pub struct VerificationKeyReturn(pub [u8; 32]);
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
	///`MiniCommitment(uint256,uint256)`
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
	pub struct MiniCommitment {
		pub block_number: ::ethers::core::types::U256,
		pub validator_set_id: ::ethers::core::types::U256,
	}
	///`ParachainHeader(uint256,bytes)`
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
	pub struct ParachainHeader {
		pub id: ::ethers::core::types::U256,
		pub header: ::ethers::core::types::Bytes,
	}
	///`PartialBeefyMmrLeaf(uint256,uint256,bytes32,(uint256,uint256,bytes32),bytes32)`
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
	pub struct PartialBeefyMmrLeaf {
		pub version: ::ethers::core::types::U256,
		pub parent_number: ::ethers::core::types::U256,
		pub parent_hash: [u8; 32],
		pub next_authority_set: AuthoritySetCommitment,
		pub extra: [u8; 32],
	}
	///`PublicInputs(bytes32,uint256,bytes32,bytes32[])`
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
	pub struct PublicInputs {
		pub authorities_root: [u8; 32],
		pub authorities_len: ::ethers::core::types::U256,
		pub leaf_hash: [u8; 32],
		pub headers: ::std::vec::Vec<[u8; 32]>,
	}
	///`Sp1BeefyProof((uint256,uint256),(uint256,uint256,bytes32,(uint256,uint256,bytes32),
	/// bytes32),(uint256,bytes)[],bytes)`
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
	pub struct Sp1BeefyProof {
		pub commitment: MiniCommitment,
		pub mmr_leaf: PartialBeefyMmrLeaf,
		pub headers: ::std::vec::Vec<ParachainHeader>,
		pub proof: ::ethers::core::types::Bytes,
	}
}
