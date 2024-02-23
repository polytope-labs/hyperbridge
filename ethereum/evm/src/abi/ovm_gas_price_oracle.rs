pub use ovm_gas_price_oracle::*;
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
pub mod ovm_gas_price_oracle {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::None,
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("DECIMALS"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("DECIMALS"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("baseFee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("baseFee"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("baseFeeScalar"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("baseFeeScalar"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint32"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("blobBaseFee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("blobBaseFee"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("blobBaseFeeScalar"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("blobBaseFeeScalar"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint32"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("decimals"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("decimals"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("gasPrice"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("gasPrice"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1Fee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1Fee"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_data"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1GasUsed"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1GasUsed"),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("_data"),
							kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bytes"),
							),
						},],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("isEcotone"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("isEcotone"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Bool,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("bool"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("l1BaseFee"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("l1BaseFee"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("overhead"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("overhead"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("scalar"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("scalar"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("setEcotone"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("setEcotone"),
						inputs: ::std::vec![],
						outputs: ::std::vec![],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("version"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("version"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::String,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("string"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
			]),
			events: ::std::collections::BTreeMap::new(),
			errors: ::std::collections::BTreeMap::new(),
			receive: false,
			fallback: false,
		}
	}
	///The parsed JSON ABI of the contract.
	pub static OVM_GASPRICEORACLE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct OVM_gasPriceOracle<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for OVM_gasPriceOracle<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for OVM_gasPriceOracle<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for OVM_gasPriceOracle<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for OVM_gasPriceOracle<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(OVM_gasPriceOracle))
				.field(&self.address())
				.finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> OVM_gasPriceOracle<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(
				address.into(),
				OVM_GASPRICEORACLE_ABI.clone(),
				client,
			))
		}
		///Calls the contract's `DECIMALS` (0x2e0f2625) function
		pub fn DECIMALS(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([46, 15, 38, 37], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `baseFee` (0x6ef25c3a) function
		pub fn base_fee(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([110, 242, 92, 58], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `baseFeeScalar` (0xc5985918) function
		pub fn base_fee_scalar(&self) -> ::ethers::contract::builders::ContractCall<M, u32> {
			self.0
				.method_hash([197, 152, 89, 24], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `blobBaseFee` (0xf8206140) function
		pub fn blob_base_fee(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([248, 32, 97, 64], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `blobBaseFeeScalar` (0x68d5dca6) function
		pub fn blob_base_fee_scalar(&self) -> ::ethers::contract::builders::ContractCall<M, u32> {
			self.0
				.method_hash([104, 213, 220, 166], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `decimals` (0x313ce567) function
		pub fn decimals(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([49, 60, 229, 103], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `gasPrice` (0xfe173b97) function
		pub fn gas_price(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([254, 23, 59, 151], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1Fee` (0x49948e0e) function
		pub fn get_l1_fee(
			&self,
			data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([73, 148, 142, 14], data)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1GasUsed` (0xde26c4a1) function
		pub fn get_l1_gas_used(
			&self,
			data: ::ethers::core::types::Bytes,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([222, 38, 196, 161], data)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `isEcotone` (0x4ef6e224) function
		pub fn is_ecotone(&self) -> ::ethers::contract::builders::ContractCall<M, bool> {
			self.0
				.method_hash([78, 246, 226, 36], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `l1BaseFee` (0x519b4bd3) function
		pub fn l_1_base_fee(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([81, 155, 75, 211], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `overhead` (0x0c18c162) function
		pub fn overhead(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([12, 24, 193, 98], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `scalar` (0xf45e65d8) function
		pub fn scalar(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([244, 94, 101, 216], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `setEcotone` (0x22b90ab3) function
		pub fn set_ecotone(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
			self.0
				.method_hash([34, 185, 10, 179], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `version` (0x54fd4d50) function
		pub fn version(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
			self.0
				.method_hash([84, 253, 77, 80], ())
				.expect("method not found (this should never happen)")
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
		for OVM_gasPriceOracle<M>
	{
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Container type for all input parameters for the `DECIMALS` function with signature
	/// `DECIMALS()` and selector `0x2e0f2625`
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
	#[ethcall(name = "DECIMALS", abi = "DECIMALS()")]
	pub struct DECIMALSCall;
	///Container type for all input parameters for the `baseFee` function with signature
	/// `baseFee()` and selector `0x6ef25c3a`
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
	#[ethcall(name = "baseFee", abi = "baseFee()")]
	pub struct BaseFeeCall;
	///Container type for all input parameters for the `baseFeeScalar` function with signature
	/// `baseFeeScalar()` and selector `0xc5985918`
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
	#[ethcall(name = "baseFeeScalar", abi = "baseFeeScalar()")]
	pub struct BaseFeeScalarCall;
	///Container type for all input parameters for the `blobBaseFee` function with signature
	/// `blobBaseFee()` and selector `0xf8206140`
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
	#[ethcall(name = "blobBaseFee", abi = "blobBaseFee()")]
	pub struct BlobBaseFeeCall;
	///Container type for all input parameters for the `blobBaseFeeScalar` function with signature
	/// `blobBaseFeeScalar()` and selector `0x68d5dca6`
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
	#[ethcall(name = "blobBaseFeeScalar", abi = "blobBaseFeeScalar()")]
	pub struct BlobBaseFeeScalarCall;
	///Container type for all input parameters for the `decimals` function with signature
	/// `decimals()` and selector `0x313ce567`
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
	#[ethcall(name = "decimals", abi = "decimals()")]
	pub struct decimalsCall;
	///Container type for all input parameters for the `gasPrice` function with signature
	/// `gasPrice()` and selector `0xfe173b97`
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
	#[ethcall(name = "gasPrice", abi = "gasPrice()")]
	pub struct GasPriceCall;
	///Container type for all input parameters for the `getL1Fee` function with signature
	/// `getL1Fee(bytes)` and selector `0x49948e0e`
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
	#[ethcall(name = "getL1Fee", abi = "getL1Fee(bytes)")]
	pub struct GetL1FeeCall {
		pub data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `getL1GasUsed` function with signature
	/// `getL1GasUsed(bytes)` and selector `0xde26c4a1`
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
	#[ethcall(name = "getL1GasUsed", abi = "getL1GasUsed(bytes)")]
	pub struct GetL1GasUsedCall {
		pub data: ::ethers::core::types::Bytes,
	}
	///Container type for all input parameters for the `isEcotone` function with signature
	/// `isEcotone()` and selector `0x4ef6e224`
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
	#[ethcall(name = "isEcotone", abi = "isEcotone()")]
	pub struct IsEcotoneCall;
	///Container type for all input parameters for the `l1BaseFee` function with signature
	/// `l1BaseFee()` and selector `0x519b4bd3`
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
	#[ethcall(name = "l1BaseFee", abi = "l1BaseFee()")]
	pub struct L1BaseFeeCall;
	///Container type for all input parameters for the `overhead` function with signature
	/// `overhead()` and selector `0x0c18c162`
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
	#[ethcall(name = "overhead", abi = "overhead()")]
	pub struct OverheadCall;
	///Container type for all input parameters for the `scalar` function with signature `scalar()`
	/// and selector `0xf45e65d8`
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
	#[ethcall(name = "scalar", abi = "scalar()")]
	pub struct ScalarCall;
	///Container type for all input parameters for the `setEcotone` function with signature
	/// `setEcotone()` and selector `0x22b90ab3`
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
	#[ethcall(name = "setEcotone", abi = "setEcotone()")]
	pub struct SetEcotoneCall;
	///Container type for all input parameters for the `version` function with signature
	/// `version()` and selector `0x54fd4d50`
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
	#[ethcall(name = "version", abi = "version()")]
	pub struct VersionCall;
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum OVM_gasPriceOracleCalls {
		DECIMALS(DECIMALSCall),
		BaseFee(BaseFeeCall),
		BaseFeeScalar(BaseFeeScalarCall),
		BlobBaseFee(BlobBaseFeeCall),
		BlobBaseFeeScalar(BlobBaseFeeScalarCall),
		decimals(decimalsCall),
		GasPrice(GasPriceCall),
		GetL1Fee(GetL1FeeCall),
		GetL1GasUsed(GetL1GasUsedCall),
		IsEcotone(IsEcotoneCall),
		L1BaseFee(L1BaseFeeCall),
		Overhead(OverheadCall),
		Scalar(ScalarCall),
		SetEcotone(SetEcotoneCall),
		Version(VersionCall),
	}
	impl ::ethers::core::abi::AbiDecode for OVM_gasPriceOracleCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) = <DECIMALSCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::DECIMALS(decoded));
			}
			if let Ok(decoded) = <BaseFeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::BaseFee(decoded));
			}
			if let Ok(decoded) = <BaseFeeScalarCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::BaseFeeScalar(decoded));
			}
			if let Ok(decoded) = <BlobBaseFeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::BlobBaseFee(decoded));
			}
			if let Ok(decoded) =
				<BlobBaseFeeScalarCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::BlobBaseFeeScalar(decoded));
			}
			if let Ok(decoded) = <decimalsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::decimals(decoded));
			}
			if let Ok(decoded) = <GasPriceCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GasPrice(decoded));
			}
			if let Ok(decoded) = <GetL1FeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::GetL1Fee(decoded));
			}
			if let Ok(decoded) = <GetL1GasUsedCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1GasUsed(decoded));
			}
			if let Ok(decoded) = <IsEcotoneCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::IsEcotone(decoded));
			}
			if let Ok(decoded) = <L1BaseFeeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::L1BaseFee(decoded));
			}
			if let Ok(decoded) = <OverheadCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Overhead(decoded));
			}
			if let Ok(decoded) = <ScalarCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Scalar(decoded));
			}
			if let Ok(decoded) = <SetEcotoneCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::SetEcotone(decoded));
			}
			if let Ok(decoded) = <VersionCall as ::ethers::core::abi::AbiDecode>::decode(data) {
				return Ok(Self::Version(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for OVM_gasPriceOracleCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::DECIMALS(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BaseFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BaseFeeScalar(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BlobBaseFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::BlobBaseFeeScalar(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::decimals(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GasPrice(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1Fee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1GasUsed(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::IsEcotone(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::L1BaseFee(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Overhead(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Scalar(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::SetEcotone(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::Version(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for OVM_gasPriceOracleCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::DECIMALS(element) => ::core::fmt::Display::fmt(element, f),
				Self::BaseFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::BaseFeeScalar(element) => ::core::fmt::Display::fmt(element, f),
				Self::BlobBaseFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::BlobBaseFeeScalar(element) => ::core::fmt::Display::fmt(element, f),
				Self::decimals(element) => ::core::fmt::Display::fmt(element, f),
				Self::GasPrice(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1Fee(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1GasUsed(element) => ::core::fmt::Display::fmt(element, f),
				Self::IsEcotone(element) => ::core::fmt::Display::fmt(element, f),
				Self::L1BaseFee(element) => ::core::fmt::Display::fmt(element, f),
				Self::Overhead(element) => ::core::fmt::Display::fmt(element, f),
				Self::Scalar(element) => ::core::fmt::Display::fmt(element, f),
				Self::SetEcotone(element) => ::core::fmt::Display::fmt(element, f),
				Self::Version(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<DECIMALSCall> for OVM_gasPriceOracleCalls {
		fn from(value: DECIMALSCall) -> Self {
			Self::DECIMALS(value)
		}
	}
	impl ::core::convert::From<BaseFeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: BaseFeeCall) -> Self {
			Self::BaseFee(value)
		}
	}
	impl ::core::convert::From<BaseFeeScalarCall> for OVM_gasPriceOracleCalls {
		fn from(value: BaseFeeScalarCall) -> Self {
			Self::BaseFeeScalar(value)
		}
	}
	impl ::core::convert::From<BlobBaseFeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: BlobBaseFeeCall) -> Self {
			Self::BlobBaseFee(value)
		}
	}
	impl ::core::convert::From<BlobBaseFeeScalarCall> for OVM_gasPriceOracleCalls {
		fn from(value: BlobBaseFeeScalarCall) -> Self {
			Self::BlobBaseFeeScalar(value)
		}
	}
	impl ::core::convert::From<decimalsCall> for OVM_gasPriceOracleCalls {
		fn from(value: decimalsCall) -> Self {
			Self::decimals(value)
		}
	}
	impl ::core::convert::From<GasPriceCall> for OVM_gasPriceOracleCalls {
		fn from(value: GasPriceCall) -> Self {
			Self::GasPrice(value)
		}
	}
	impl ::core::convert::From<GetL1FeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: GetL1FeeCall) -> Self {
			Self::GetL1Fee(value)
		}
	}
	impl ::core::convert::From<GetL1GasUsedCall> for OVM_gasPriceOracleCalls {
		fn from(value: GetL1GasUsedCall) -> Self {
			Self::GetL1GasUsed(value)
		}
	}
	impl ::core::convert::From<IsEcotoneCall> for OVM_gasPriceOracleCalls {
		fn from(value: IsEcotoneCall) -> Self {
			Self::IsEcotone(value)
		}
	}
	impl ::core::convert::From<L1BaseFeeCall> for OVM_gasPriceOracleCalls {
		fn from(value: L1BaseFeeCall) -> Self {
			Self::L1BaseFee(value)
		}
	}
	impl ::core::convert::From<OverheadCall> for OVM_gasPriceOracleCalls {
		fn from(value: OverheadCall) -> Self {
			Self::Overhead(value)
		}
	}
	impl ::core::convert::From<ScalarCall> for OVM_gasPriceOracleCalls {
		fn from(value: ScalarCall) -> Self {
			Self::Scalar(value)
		}
	}
	impl ::core::convert::From<SetEcotoneCall> for OVM_gasPriceOracleCalls {
		fn from(value: SetEcotoneCall) -> Self {
			Self::SetEcotone(value)
		}
	}
	impl ::core::convert::From<VersionCall> for OVM_gasPriceOracleCalls {
		fn from(value: VersionCall) -> Self {
			Self::Version(value)
		}
	}
	///Container type for all return fields from the `DECIMALS` function with signature
	/// `DECIMALS()` and selector `0x2e0f2625`
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
	pub struct DECIMALSReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `baseFee` function with signature `baseFee()`
	/// and selector `0x6ef25c3a`
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
	pub struct BaseFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `baseFeeScalar` function with signature
	/// `baseFeeScalar()` and selector `0xc5985918`
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
	pub struct BaseFeeScalarReturn(pub u32);
	///Container type for all return fields from the `blobBaseFee` function with signature
	/// `blobBaseFee()` and selector `0xf8206140`
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
	pub struct BlobBaseFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `blobBaseFeeScalar` function with signature
	/// `blobBaseFeeScalar()` and selector `0x68d5dca6`
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
	pub struct BlobBaseFeeScalarReturn(pub u32);
	///Container type for all return fields from the `decimals` function with signature
	/// `decimals()` and selector `0x313ce567`
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
	pub struct decimalsReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `gasPrice` function with signature
	/// `gasPrice()` and selector `0xfe173b97`
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
	pub struct GasPriceReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1Fee` function with signature
	/// `getL1Fee(bytes)` and selector `0x49948e0e`
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
	pub struct GetL1FeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1GasUsed` function with signature
	/// `getL1GasUsed(bytes)` and selector `0xde26c4a1`
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
	pub struct GetL1GasUsedReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `isEcotone` function with signature
	/// `isEcotone()` and selector `0x4ef6e224`
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
	pub struct IsEcotoneReturn(pub bool);
	///Container type for all return fields from the `l1BaseFee` function with signature
	/// `l1BaseFee()` and selector `0x519b4bd3`
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
	pub struct L1BaseFeeReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `overhead` function with signature
	/// `overhead()` and selector `0x0c18c162`
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
	pub struct OverheadReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `scalar` function with signature `scalar()`
	/// and selector `0xf45e65d8`
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
	pub struct ScalarReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `version` function with signature `version()`
	/// and selector `0x54fd4d50`
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
	pub struct VersionReturn(pub ::std::string::String);
}
