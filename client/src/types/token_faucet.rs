pub use token_faucet::*;
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
pub mod token_faucet {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                    name: ::std::borrow::ToOwned::to_owned("_token"),
                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                    internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
                        "address"
                    ),),
                },],
            }),
            functions: ::core::convert::From::from([(
                ::std::borrow::ToOwned::to_owned("drip"),
                ::std::vec![::ethers::core::abi::ethabi::Function {
                    name: ::std::borrow::ToOwned::to_owned("drip"),
                    inputs: ::std::vec![],
                    outputs: ::std::vec![],
                    constant: ::core::option::Option::None,
                    state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                },],
            )]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static TOKENFAUCET_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x02:8\x03\x80a\x02:\x839\x81\x01`@\x81\x90Ra\0/\x91a\0TV[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90Ua\0\x84V[`\0` \x82\x84\x03\x12\x15a\0fW`\0\x80\xFD[\x81Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0}W`\0\x80\xFD[\x93\x92PPPV[a\x01\xA7\x80a\0\x93`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0+W`\x005`\xE0\x1C\x80c\x9Fg\x8C\xCA\x14a\x000W[`\0\x80\xFD[a\08a\0:V[\0[3`\0\x90\x81R` \x81\x90R`@\x81 T\x90a\0U\x82Ba\x01JV[\x90Pb\x01Q\x80\x81\x10\x15a\0\xB9W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\"`$\x82\x01R\x7FCan only request tokens once dai`D\x82\x01Raly`\xF0\x1B`d\x82\x01R`\x84\x01`@Q\x80\x91\x03\x90\xFD[3`\0\x81\x81R` \x81\x90R`@\x80\x82 B\x90U`\x01T\x90Qc\x94\xD0\x08\xEF`\xE0\x1B\x81R`\x04\x81\x01\x93\x90\x93Rh65\xC9\xAD\xC5\xDE\xA0\0\0`$\x84\x01R```D\x84\x01R`d\x83\x01\x91\x90\x91R`\x01`\x01`\xA0\x1B\x03\x16\x90c\x94\xD0\x08\xEF\x90`\x84\x01`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x01.W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x01BW=`\0\x80>=`\0\xFD[PPPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x01kWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x92\x91PPV\xFE\xA2dipfsX\"\x12 `\xD3\xE6\xBA\xA0\xCF\xEC\x07I\xF4\xBE\x1A7\xD4U5\x10?U\xE1\x19\xC1\xBB\xDEvx\xD6-l\xEA._dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static TOKENFAUCET_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0+W`\x005`\xE0\x1C\x80c\x9Fg\x8C\xCA\x14a\x000W[`\0\x80\xFD[a\08a\0:V[\0[3`\0\x90\x81R` \x81\x90R`@\x81 T\x90a\0U\x82Ba\x01JV[\x90Pb\x01Q\x80\x81\x10\x15a\0\xB9W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\"`$\x82\x01R\x7FCan only request tokens once dai`D\x82\x01Raly`\xF0\x1B`d\x82\x01R`\x84\x01`@Q\x80\x91\x03\x90\xFD[3`\0\x81\x81R` \x81\x90R`@\x80\x82 B\x90U`\x01T\x90Qc\x94\xD0\x08\xEF`\xE0\x1B\x81R`\x04\x81\x01\x93\x90\x93Rh65\xC9\xAD\xC5\xDE\xA0\0\0`$\x84\x01R```D\x84\x01R`d\x83\x01\x91\x90\x91R`\x01`\x01`\xA0\x1B\x03\x16\x90c\x94\xD0\x08\xEF\x90`\x84\x01`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x01.W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x01BW=`\0\x80>=`\0\xFD[PPPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x01kWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x92\x91PPV\xFE\xA2dipfsX\"\x12 `\xD3\xE6\xBA\xA0\xCF\xEC\x07I\xF4\xBE\x1A7\xD4U5\x10?U\xE1\x19\xC1\xBB\xDEvx\xD6-l\xEA._dsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static TOKENFAUCET_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct TokenFaucet<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for TokenFaucet<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for TokenFaucet<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for TokenFaucet<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for TokenFaucet<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(TokenFaucet)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> TokenFaucet<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(address.into(), TOKENFAUCET_ABI.clone(), client))
        }
        /// Constructs the general purpose `Deployer` instance based on the provided constructor
        /// arguments and sends it. Returns a new instance of a deployer that returns an
        /// instance of this contract after sending the transaction
        ///
        /// Notes:
        /// - If there are no constructor arguments, you should pass `()` as the argument.
        /// - The default poll duration is 7 seconds.
        /// - The default number of confirmations is 1 block.
        ///
        ///
        /// # Example
        ///
        /// Generate contract bindings with `abigen!` and deploy a new contract instance.
        ///
        /// *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact.
        ///
        /// ```ignore
        /// # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {
        ///     abigen!(Greeter, "../greeter.json");
        ///
        ///    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();
        ///    let msg = greeter_contract.greet().call().await.unwrap();
        /// # }
        /// ```
        pub fn deploy<T: ::ethers::core::abi::Tokenize>(
            client: ::std::sync::Arc<M>,
            constructor_args: T,
        ) -> ::core::result::Result<
            ::ethers::contract::builders::ContractDeployer<M, Self>,
            ::ethers::contract::ContractError<M>,
        > {
            let factory = ::ethers::contract::ContractFactory::new(
                TOKENFAUCET_ABI.clone(),
                TOKENFAUCET_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        ///Calls the contract's `drip` (0x9f678cca) function
        pub fn drip(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([159, 103, 140, 202], ())
                .expect("method not found (this should never happen)")
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for TokenFaucet<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Container type for all input parameters for the `drip` function with signature `drip()` and
    /// selector `0x9f678cca`
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
    #[ethcall(name = "drip", abi = "drip()")]
    pub struct DripCall;
}
