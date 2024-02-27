pub use arb_gas_info::*;
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
pub mod arb_gas_info {
	#[allow(deprecated)]
	fn __abi() -> ::ethers::core::abi::Abi {
		::ethers::core::abi::ethabi::Contract {
			constructor: ::core::option::Option::None,
			functions: ::core::convert::From::from([
				(
					::std::borrow::ToOwned::to_owned("getAmortizedCostCapBips"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getAmortizedCostCapBips",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getCurrentTxL1GasFees"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getCurrentTxL1GasFees",),
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
					::std::borrow::ToOwned::to_owned("getGasAccountingParams"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getGasAccountingParams",),
						inputs: ::std::vec![],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getGasBacklog"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getGasBacklog"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getGasBacklogTolerance"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getGasBacklogTolerance",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1BaseFeeEstimate"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1BaseFeeEstimate",),
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
					::std::borrow::ToOwned::to_owned("getL1BaseFeeEstimateInertia"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1BaseFeeEstimateInertia",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1FeesAvailable"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1FeesAvailable"),
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
					::std::borrow::ToOwned::to_owned("getL1GasPriceEstimate"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1GasPriceEstimate",),
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
					::std::borrow::ToOwned::to_owned("getL1PricingSurplus"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1PricingSurplus",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Int(256usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("int256"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1RewardRate"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1RewardRate"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getL1RewardRecipient"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getL1RewardRecipient",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getMinimumGasPrice"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getMinimumGasPrice"),
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
					::std::borrow::ToOwned::to_owned("getPerBatchGasCharge"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPerBatchGasCharge",),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Int(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("int64"),
							),
						},],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getPricesInArbGas"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPricesInArbGas"),
						inputs: ::std::vec![],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getPricesInArbGasWithAggregator"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPricesInArbGasWithAggregator",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("aggregator"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getPricesInWei"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPricesInWei"),
						inputs: ::std::vec![],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getPricesInWeiWithAggregator"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPricesInWeiWithAggregator",),
						inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::borrow::ToOwned::to_owned("aggregator"),
							kind: ::ethers::core::abi::ethabi::ParamType::Address,
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("address"),
							),
						},],
						outputs: ::std::vec![
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
							::ethers::core::abi::ethabi::Param {
								name: ::std::string::String::new(),
								kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
								internal_type: ::core::option::Option::Some(
									::std::borrow::ToOwned::to_owned("uint256"),
								),
							},
						],
						constant: ::core::option::Option::None,
						state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
					},],
				),
				(
					::std::borrow::ToOwned::to_owned("getPricingInertia"),
					::std::vec![::ethers::core::abi::ethabi::Function {
						name: ::std::borrow::ToOwned::to_owned("getPricingInertia"),
						inputs: ::std::vec![],
						outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
							name: ::std::string::String::new(),
							kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
							internal_type: ::core::option::Option::Some(
								::std::borrow::ToOwned::to_owned("uint64"),
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
	pub static ARBGASINFO_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
		::ethers::contract::Lazy::new(__abi);
	pub struct ArbGasInfo<M>(::ethers::contract::Contract<M>);
	impl<M> ::core::clone::Clone for ArbGasInfo<M> {
		fn clone(&self) -> Self {
			Self(::core::clone::Clone::clone(&self.0))
		}
	}
	impl<M> ::core::ops::Deref for ArbGasInfo<M> {
		type Target = ::ethers::contract::Contract<M>;
		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl<M> ::core::ops::DerefMut for ArbGasInfo<M> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}
	impl<M> ::core::fmt::Debug for ArbGasInfo<M> {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			f.debug_tuple(::core::stringify!(ArbGasInfo)).field(&self.address()).finish()
		}
	}
	impl<M: ::ethers::providers::Middleware> ArbGasInfo<M> {
		/// Creates a new contract instance with the specified `ethers` client at
		/// `address`. The contract derefs to a `ethers::Contract` object.
		pub fn new<T: Into<::ethers::core::types::Address>>(
			address: T,
			client: ::std::sync::Arc<M>,
		) -> Self {
			Self(::ethers::contract::Contract::new(address.into(), ARBGASINFO_ABI.clone(), client))
		}
		///Calls the contract's `getAmortizedCostCapBips` (0x7a7d6beb) function
		pub fn get_amortized_cost_cap_bips(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([122, 125, 107, 235], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getCurrentTxL1GasFees` (0xc6f7de0e) function
		pub fn get_current_tx_l1_gas_fees(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([198, 247, 222, 14], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getGasAccountingParams` (0x612af178) function
		pub fn get_gas_accounting_params(
			&self,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(::ethers::core::types::U256, ::ethers::core::types::U256, ::ethers::core::types::U256),
		> {
			self.0
				.method_hash([97, 42, 241, 120], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getGasBacklog` (0x1d5b5c20) function
		pub fn get_gas_backlog(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([29, 91, 92, 32], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getGasBacklogTolerance` (0x25754f91) function
		pub fn get_gas_backlog_tolerance(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([37, 117, 79, 145], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1BaseFeeEstimate` (0xf5d6ded7) function
		pub fn get_l1_base_fee_estimate(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([245, 214, 222, 215], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1BaseFeeEstimateInertia` (0x29eb31ee) function
		pub fn get_l1_base_fee_estimate_inertia(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([41, 235, 49, 238], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1FeesAvailable` (0x5b39d23c) function
		pub fn get_l1_fees_available(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([91, 57, 210, 60], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1GasPriceEstimate` (0x055f362f) function
		pub fn get_l1_gas_price_estimate(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([5, 95, 54, 47], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1PricingSurplus` (0x520acdd7) function
		pub fn get_l1_pricing_surplus(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::I256> {
			self.0
				.method_hash([82, 10, 205, 215], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1RewardRate` (0x8a5b1d28) function
		pub fn get_l1_reward_rate(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([138, 91, 29, 40], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getL1RewardRecipient` (0x9e6d7e31) function
		pub fn get_l1_reward_recipient(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
			self.0
				.method_hash([158, 109, 126, 49], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getMinimumGasPrice` (0xf918379a) function
		pub fn get_minimum_gas_price(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
			self.0
				.method_hash([249, 24, 55, 154], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPerBatchGasCharge` (0x6ecca45a) function
		pub fn get_per_batch_gas_charge(
			&self,
		) -> ::ethers::contract::builders::ContractCall<M, i64> {
			self.0
				.method_hash([110, 204, 164, 90], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPricesInArbGas` (0x02199f34) function
		pub fn get_prices_in_arb_gas(
			&self,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(::ethers::core::types::U256, ::ethers::core::types::U256, ::ethers::core::types::U256),
		> {
			self.0
				.method_hash([2, 25, 159, 52], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPricesInArbGasWithAggregator` (0x7a1ea732) function
		pub fn get_prices_in_arb_gas_with_aggregator(
			&self,
			aggregator: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(::ethers::core::types::U256, ::ethers::core::types::U256, ::ethers::core::types::U256),
		> {
			self.0
				.method_hash([122, 30, 167, 50], aggregator)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPricesInWei` (0x41b247a8) function
		pub fn get_prices_in_wei(
			&self,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
			),
		> {
			self.0
				.method_hash([65, 178, 71, 168], ())
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPricesInWeiWithAggregator` (0xba9c916e) function
		pub fn get_prices_in_wei_with_aggregator(
			&self,
			aggregator: ::ethers::core::types::Address,
		) -> ::ethers::contract::builders::ContractCall<
			M,
			(
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
				::ethers::core::types::U256,
			),
		> {
			self.0
				.method_hash([186, 156, 145, 110], aggregator)
				.expect("method not found (this should never happen)")
		}
		///Calls the contract's `getPricingInertia` (0x3dfb45b9) function
		pub fn get_pricing_inertia(&self) -> ::ethers::contract::builders::ContractCall<M, u64> {
			self.0
				.method_hash([61, 251, 69, 185], ())
				.expect("method not found (this should never happen)")
		}
	}
	impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for ArbGasInfo<M> {
		fn from(contract: ::ethers::contract::Contract<M>) -> Self {
			Self::new(contract.address(), contract.client())
		}
	}
	///Container type for all input parameters for the `getAmortizedCostCapBips` function with
	/// signature `getAmortizedCostCapBips()` and selector `0x7a7d6beb`
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
	#[ethcall(name = "getAmortizedCostCapBips", abi = "getAmortizedCostCapBips()")]
	pub struct GetAmortizedCostCapBipsCall;
	///Container type for all input parameters for the `getCurrentTxL1GasFees` function with
	/// signature `getCurrentTxL1GasFees()` and selector `0xc6f7de0e`
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
	#[ethcall(name = "getCurrentTxL1GasFees", abi = "getCurrentTxL1GasFees()")]
	pub struct GetCurrentTxL1GasFeesCall;
	///Container type for all input parameters for the `getGasAccountingParams` function with
	/// signature `getGasAccountingParams()` and selector `0x612af178`
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
	#[ethcall(name = "getGasAccountingParams", abi = "getGasAccountingParams()")]
	pub struct GetGasAccountingParamsCall;
	///Container type for all input parameters for the `getGasBacklog` function with signature
	/// `getGasBacklog()` and selector `0x1d5b5c20`
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
	#[ethcall(name = "getGasBacklog", abi = "getGasBacklog()")]
	pub struct GetGasBacklogCall;
	///Container type for all input parameters for the `getGasBacklogTolerance` function with
	/// signature `getGasBacklogTolerance()` and selector `0x25754f91`
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
	#[ethcall(name = "getGasBacklogTolerance", abi = "getGasBacklogTolerance()")]
	pub struct GetGasBacklogToleranceCall;
	///Container type for all input parameters for the `getL1BaseFeeEstimate` function with
	/// signature `getL1BaseFeeEstimate()` and selector `0xf5d6ded7`
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
	#[ethcall(name = "getL1BaseFeeEstimate", abi = "getL1BaseFeeEstimate()")]
	pub struct GetL1BaseFeeEstimateCall;
	///Container type for all input parameters for the `getL1BaseFeeEstimateInertia` function with
	/// signature `getL1BaseFeeEstimateInertia()` and selector `0x29eb31ee`
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
	#[ethcall(name = "getL1BaseFeeEstimateInertia", abi = "getL1BaseFeeEstimateInertia()")]
	pub struct GetL1BaseFeeEstimateInertiaCall;
	///Container type for all input parameters for the `getL1FeesAvailable` function with signature
	/// `getL1FeesAvailable()` and selector `0x5b39d23c`
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
	#[ethcall(name = "getL1FeesAvailable", abi = "getL1FeesAvailable()")]
	pub struct GetL1FeesAvailableCall;
	///Container type for all input parameters for the `getL1GasPriceEstimate` function with
	/// signature `getL1GasPriceEstimate()` and selector `0x055f362f`
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
	#[ethcall(name = "getL1GasPriceEstimate", abi = "getL1GasPriceEstimate()")]
	pub struct GetL1GasPriceEstimateCall;
	///Container type for all input parameters for the `getL1PricingSurplus` function with
	/// signature `getL1PricingSurplus()` and selector `0x520acdd7`
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
	#[ethcall(name = "getL1PricingSurplus", abi = "getL1PricingSurplus()")]
	pub struct GetL1PricingSurplusCall;
	///Container type for all input parameters for the `getL1RewardRate` function with signature
	/// `getL1RewardRate()` and selector `0x8a5b1d28`
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
	#[ethcall(name = "getL1RewardRate", abi = "getL1RewardRate()")]
	pub struct GetL1RewardRateCall;
	///Container type for all input parameters for the `getL1RewardRecipient` function with
	/// signature `getL1RewardRecipient()` and selector `0x9e6d7e31`
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
	#[ethcall(name = "getL1RewardRecipient", abi = "getL1RewardRecipient()")]
	pub struct GetL1RewardRecipientCall;
	///Container type for all input parameters for the `getMinimumGasPrice` function with signature
	/// `getMinimumGasPrice()` and selector `0xf918379a`
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
	#[ethcall(name = "getMinimumGasPrice", abi = "getMinimumGasPrice()")]
	pub struct GetMinimumGasPriceCall;
	///Container type for all input parameters for the `getPerBatchGasCharge` function with
	/// signature `getPerBatchGasCharge()` and selector `0x6ecca45a`
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
	#[ethcall(name = "getPerBatchGasCharge", abi = "getPerBatchGasCharge()")]
	pub struct GetPerBatchGasChargeCall;
	///Container type for all input parameters for the `getPricesInArbGas` function with signature
	/// `getPricesInArbGas()` and selector `0x02199f34`
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
	#[ethcall(name = "getPricesInArbGas", abi = "getPricesInArbGas()")]
	pub struct GetPricesInArbGasCall;
	///Container type for all input parameters for the `getPricesInArbGasWithAggregator` function
	/// with signature `getPricesInArbGasWithAggregator(address)` and selector `0x7a1ea732`
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
		name = "getPricesInArbGasWithAggregator",
		abi = "getPricesInArbGasWithAggregator(address)"
	)]
	pub struct GetPricesInArbGasWithAggregatorCall {
		pub aggregator: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `getPricesInWei` function with signature
	/// `getPricesInWei()` and selector `0x41b247a8`
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
	#[ethcall(name = "getPricesInWei", abi = "getPricesInWei()")]
	pub struct GetPricesInWeiCall;
	///Container type for all input parameters for the `getPricesInWeiWithAggregator` function with
	/// signature `getPricesInWeiWithAggregator(address)` and selector `0xba9c916e`
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
	#[ethcall(name = "getPricesInWeiWithAggregator", abi = "getPricesInWeiWithAggregator(address)")]
	pub struct GetPricesInWeiWithAggregatorCall {
		pub aggregator: ::ethers::core::types::Address,
	}
	///Container type for all input parameters for the `getPricingInertia` function with signature
	/// `getPricingInertia()` and selector `0x3dfb45b9`
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
	#[ethcall(name = "getPricingInertia", abi = "getPricingInertia()")]
	pub struct GetPricingInertiaCall;
	///Container type for all of the contract's call
	#[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
	pub enum ArbGasInfoCalls {
		GetAmortizedCostCapBips(GetAmortizedCostCapBipsCall),
		GetCurrentTxL1GasFees(GetCurrentTxL1GasFeesCall),
		GetGasAccountingParams(GetGasAccountingParamsCall),
		GetGasBacklog(GetGasBacklogCall),
		GetGasBacklogTolerance(GetGasBacklogToleranceCall),
		GetL1BaseFeeEstimate(GetL1BaseFeeEstimateCall),
		GetL1BaseFeeEstimateInertia(GetL1BaseFeeEstimateInertiaCall),
		GetL1FeesAvailable(GetL1FeesAvailableCall),
		GetL1GasPriceEstimate(GetL1GasPriceEstimateCall),
		GetL1PricingSurplus(GetL1PricingSurplusCall),
		GetL1RewardRate(GetL1RewardRateCall),
		GetL1RewardRecipient(GetL1RewardRecipientCall),
		GetMinimumGasPrice(GetMinimumGasPriceCall),
		GetPerBatchGasCharge(GetPerBatchGasChargeCall),
		GetPricesInArbGas(GetPricesInArbGasCall),
		GetPricesInArbGasWithAggregator(GetPricesInArbGasWithAggregatorCall),
		GetPricesInWei(GetPricesInWeiCall),
		GetPricesInWeiWithAggregator(GetPricesInWeiWithAggregatorCall),
		GetPricingInertia(GetPricingInertiaCall),
	}
	impl ::ethers::core::abi::AbiDecode for ArbGasInfoCalls {
		fn decode(
			data: impl AsRef<[u8]>,
		) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
			let data = data.as_ref();
			if let Ok(decoded) =
				<GetAmortizedCostCapBipsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetAmortizedCostCapBips(decoded));
			}
			if let Ok(decoded) =
				<GetCurrentTxL1GasFeesCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetCurrentTxL1GasFees(decoded));
			}
			if let Ok(decoded) =
				<GetGasAccountingParamsCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetGasAccountingParams(decoded));
			}
			if let Ok(decoded) = <GetGasBacklogCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetGasBacklog(decoded));
			}
			if let Ok(decoded) =
				<GetGasBacklogToleranceCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetGasBacklogTolerance(decoded));
			}
			if let Ok(decoded) =
				<GetL1BaseFeeEstimateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1BaseFeeEstimate(decoded));
			}
			if let Ok(decoded) =
				<GetL1BaseFeeEstimateInertiaCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1BaseFeeEstimateInertia(decoded));
			}
			if let Ok(decoded) =
				<GetL1FeesAvailableCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1FeesAvailable(decoded));
			}
			if let Ok(decoded) =
				<GetL1GasPriceEstimateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1GasPriceEstimate(decoded));
			}
			if let Ok(decoded) =
				<GetL1PricingSurplusCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1PricingSurplus(decoded));
			}
			if let Ok(decoded) =
				<GetL1RewardRateCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1RewardRate(decoded));
			}
			if let Ok(decoded) =
				<GetL1RewardRecipientCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetL1RewardRecipient(decoded));
			}
			if let Ok(decoded) =
				<GetMinimumGasPriceCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetMinimumGasPrice(decoded));
			}
			if let Ok(decoded) =
				<GetPerBatchGasChargeCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetPerBatchGasCharge(decoded));
			}
			if let Ok(decoded) =
				<GetPricesInArbGasCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetPricesInArbGas(decoded));
			}
			if let Ok(decoded) =
				<GetPricesInArbGasWithAggregatorCall as ::ethers::core::abi::AbiDecode>::decode(
					data,
				) {
				return Ok(Self::GetPricesInArbGasWithAggregator(decoded));
			}
			if let Ok(decoded) =
				<GetPricesInWeiCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetPricesInWei(decoded));
			}
			if let Ok(decoded) =
				<GetPricesInWeiWithAggregatorCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetPricesInWeiWithAggregator(decoded));
			}
			if let Ok(decoded) =
				<GetPricingInertiaCall as ::ethers::core::abi::AbiDecode>::decode(data)
			{
				return Ok(Self::GetPricingInertia(decoded));
			}
			Err(::ethers::core::abi::Error::InvalidData.into())
		}
	}
	impl ::ethers::core::abi::AbiEncode for ArbGasInfoCalls {
		fn encode(self) -> Vec<u8> {
			match self {
				Self::GetAmortizedCostCapBips(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetCurrentTxL1GasFees(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetGasAccountingParams(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetGasBacklog(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetGasBacklogTolerance(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1BaseFeeEstimate(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1BaseFeeEstimateInertia(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1FeesAvailable(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1GasPriceEstimate(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1PricingSurplus(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1RewardRate(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetL1RewardRecipient(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetMinimumGasPrice(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPerBatchGasCharge(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPricesInArbGas(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPricesInArbGasWithAggregator(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPricesInWei(element) => ::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPricesInWeiWithAggregator(element) =>
					::ethers::core::abi::AbiEncode::encode(element),
				Self::GetPricingInertia(element) => ::ethers::core::abi::AbiEncode::encode(element),
			}
		}
	}
	impl ::core::fmt::Display for ArbGasInfoCalls {
		fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
			match self {
				Self::GetAmortizedCostCapBips(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetCurrentTxL1GasFees(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetGasAccountingParams(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetGasBacklog(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetGasBacklogTolerance(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1BaseFeeEstimate(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1BaseFeeEstimateInertia(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1FeesAvailable(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1GasPriceEstimate(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1PricingSurplus(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1RewardRate(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetL1RewardRecipient(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetMinimumGasPrice(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetPerBatchGasCharge(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetPricesInArbGas(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetPricesInArbGasWithAggregator(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::GetPricesInWei(element) => ::core::fmt::Display::fmt(element, f),
				Self::GetPricesInWeiWithAggregator(element) =>
					::core::fmt::Display::fmt(element, f),
				Self::GetPricingInertia(element) => ::core::fmt::Display::fmt(element, f),
			}
		}
	}
	impl ::core::convert::From<GetAmortizedCostCapBipsCall> for ArbGasInfoCalls {
		fn from(value: GetAmortizedCostCapBipsCall) -> Self {
			Self::GetAmortizedCostCapBips(value)
		}
	}
	impl ::core::convert::From<GetCurrentTxL1GasFeesCall> for ArbGasInfoCalls {
		fn from(value: GetCurrentTxL1GasFeesCall) -> Self {
			Self::GetCurrentTxL1GasFees(value)
		}
	}
	impl ::core::convert::From<GetGasAccountingParamsCall> for ArbGasInfoCalls {
		fn from(value: GetGasAccountingParamsCall) -> Self {
			Self::GetGasAccountingParams(value)
		}
	}
	impl ::core::convert::From<GetGasBacklogCall> for ArbGasInfoCalls {
		fn from(value: GetGasBacklogCall) -> Self {
			Self::GetGasBacklog(value)
		}
	}
	impl ::core::convert::From<GetGasBacklogToleranceCall> for ArbGasInfoCalls {
		fn from(value: GetGasBacklogToleranceCall) -> Self {
			Self::GetGasBacklogTolerance(value)
		}
	}
	impl ::core::convert::From<GetL1BaseFeeEstimateCall> for ArbGasInfoCalls {
		fn from(value: GetL1BaseFeeEstimateCall) -> Self {
			Self::GetL1BaseFeeEstimate(value)
		}
	}
	impl ::core::convert::From<GetL1BaseFeeEstimateInertiaCall> for ArbGasInfoCalls {
		fn from(value: GetL1BaseFeeEstimateInertiaCall) -> Self {
			Self::GetL1BaseFeeEstimateInertia(value)
		}
	}
	impl ::core::convert::From<GetL1FeesAvailableCall> for ArbGasInfoCalls {
		fn from(value: GetL1FeesAvailableCall) -> Self {
			Self::GetL1FeesAvailable(value)
		}
	}
	impl ::core::convert::From<GetL1GasPriceEstimateCall> for ArbGasInfoCalls {
		fn from(value: GetL1GasPriceEstimateCall) -> Self {
			Self::GetL1GasPriceEstimate(value)
		}
	}
	impl ::core::convert::From<GetL1PricingSurplusCall> for ArbGasInfoCalls {
		fn from(value: GetL1PricingSurplusCall) -> Self {
			Self::GetL1PricingSurplus(value)
		}
	}
	impl ::core::convert::From<GetL1RewardRateCall> for ArbGasInfoCalls {
		fn from(value: GetL1RewardRateCall) -> Self {
			Self::GetL1RewardRate(value)
		}
	}
	impl ::core::convert::From<GetL1RewardRecipientCall> for ArbGasInfoCalls {
		fn from(value: GetL1RewardRecipientCall) -> Self {
			Self::GetL1RewardRecipient(value)
		}
	}
	impl ::core::convert::From<GetMinimumGasPriceCall> for ArbGasInfoCalls {
		fn from(value: GetMinimumGasPriceCall) -> Self {
			Self::GetMinimumGasPrice(value)
		}
	}
	impl ::core::convert::From<GetPerBatchGasChargeCall> for ArbGasInfoCalls {
		fn from(value: GetPerBatchGasChargeCall) -> Self {
			Self::GetPerBatchGasCharge(value)
		}
	}
	impl ::core::convert::From<GetPricesInArbGasCall> for ArbGasInfoCalls {
		fn from(value: GetPricesInArbGasCall) -> Self {
			Self::GetPricesInArbGas(value)
		}
	}
	impl ::core::convert::From<GetPricesInArbGasWithAggregatorCall> for ArbGasInfoCalls {
		fn from(value: GetPricesInArbGasWithAggregatorCall) -> Self {
			Self::GetPricesInArbGasWithAggregator(value)
		}
	}
	impl ::core::convert::From<GetPricesInWeiCall> for ArbGasInfoCalls {
		fn from(value: GetPricesInWeiCall) -> Self {
			Self::GetPricesInWei(value)
		}
	}
	impl ::core::convert::From<GetPricesInWeiWithAggregatorCall> for ArbGasInfoCalls {
		fn from(value: GetPricesInWeiWithAggregatorCall) -> Self {
			Self::GetPricesInWeiWithAggregator(value)
		}
	}
	impl ::core::convert::From<GetPricingInertiaCall> for ArbGasInfoCalls {
		fn from(value: GetPricingInertiaCall) -> Self {
			Self::GetPricingInertia(value)
		}
	}
	///Container type for all return fields from the `getAmortizedCostCapBips` function with
	/// signature `getAmortizedCostCapBips()` and selector `0x7a7d6beb`
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
	pub struct GetAmortizedCostCapBipsReturn(pub u64);
	///Container type for all return fields from the `getCurrentTxL1GasFees` function with
	/// signature `getCurrentTxL1GasFees()` and selector `0xc6f7de0e`
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
	pub struct GetCurrentTxL1GasFeesReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getGasAccountingParams` function with
	/// signature `getGasAccountingParams()` and selector `0x612af178`
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
	pub struct GetGasAccountingParamsReturn(
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
	);
	///Container type for all return fields from the `getGasBacklog` function with signature
	/// `getGasBacklog()` and selector `0x1d5b5c20`
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
	pub struct GetGasBacklogReturn(pub u64);
	///Container type for all return fields from the `getGasBacklogTolerance` function with
	/// signature `getGasBacklogTolerance()` and selector `0x25754f91`
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
	pub struct GetGasBacklogToleranceReturn(pub u64);
	///Container type for all return fields from the `getL1BaseFeeEstimate` function with signature
	/// `getL1BaseFeeEstimate()` and selector `0xf5d6ded7`
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
	pub struct GetL1BaseFeeEstimateReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1BaseFeeEstimateInertia` function with
	/// signature `getL1BaseFeeEstimateInertia()` and selector `0x29eb31ee`
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
	pub struct GetL1BaseFeeEstimateInertiaReturn(pub u64);
	///Container type for all return fields from the `getL1FeesAvailable` function with signature
	/// `getL1FeesAvailable()` and selector `0x5b39d23c`
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
	pub struct GetL1FeesAvailableReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1GasPriceEstimate` function with
	/// signature `getL1GasPriceEstimate()` and selector `0x055f362f`
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
	pub struct GetL1GasPriceEstimateReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getL1PricingSurplus` function with signature
	/// `getL1PricingSurplus()` and selector `0x520acdd7`
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
	pub struct GetL1PricingSurplusReturn(pub ::ethers::core::types::I256);
	///Container type for all return fields from the `getL1RewardRate` function with signature
	/// `getL1RewardRate()` and selector `0x8a5b1d28`
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
	pub struct GetL1RewardRateReturn(pub u64);
	///Container type for all return fields from the `getL1RewardRecipient` function with signature
	/// `getL1RewardRecipient()` and selector `0x9e6d7e31`
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
	pub struct GetL1RewardRecipientReturn(pub ::ethers::core::types::Address);
	///Container type for all return fields from the `getMinimumGasPrice` function with signature
	/// `getMinimumGasPrice()` and selector `0xf918379a`
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
	pub struct GetMinimumGasPriceReturn(pub ::ethers::core::types::U256);
	///Container type for all return fields from the `getPerBatchGasCharge` function with signature
	/// `getPerBatchGasCharge()` and selector `0x6ecca45a`
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
	pub struct GetPerBatchGasChargeReturn(pub i64);
	///Container type for all return fields from the `getPricesInArbGas` function with signature
	/// `getPricesInArbGas()` and selector `0x02199f34`
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
	pub struct GetPricesInArbGasReturn(
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
	);
	///Container type for all return fields from the `getPricesInArbGasWithAggregator` function
	/// with signature `getPricesInArbGasWithAggregator(address)` and selector `0x7a1ea732`
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
	pub struct GetPricesInArbGasWithAggregatorReturn(
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
	);
	///Container type for all return fields from the `getPricesInWei` function with signature
	/// `getPricesInWei()` and selector `0x41b247a8`
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
	pub struct GetPricesInWeiReturn(
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
	);
	///Container type for all return fields from the `getPricesInWeiWithAggregator` function with
	/// signature `getPricesInWeiWithAggregator(address)` and selector `0xba9c916e`
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
	pub struct GetPricesInWeiWithAggregatorReturn(
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
		pub ::ethers::core::types::U256,
	);
	///Container type for all return fields from the `getPricingInertia` function with signature
	/// `getPricingInertia()` and selector `0x3dfb45b9`
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
	pub struct GetPricingInertiaReturn(pub u64);
}
