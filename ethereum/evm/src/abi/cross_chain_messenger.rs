pub use cross_chain_messenger::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types,
)]
pub mod cross_chain_messenger {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_admin"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("onAccept"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onAccept"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostRequest"),
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
                    ::std::borrow::ToOwned::to_owned("onGetResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onGetResponse"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
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
                                            ::ethers::core::abi::ethabi::ParamType::Array(
                                                ::std::boxed::Box::new(
                                                    ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                        ::std::vec![
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                                        ],
                                                    ),
                                                ),
                                            ),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetResponse"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onGetTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onGetTimeout"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetRequest"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onPostResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onPostResponse"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("response"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostResponse"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("onPostTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onPostTimeout"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("request"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostRequest"),
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
                    ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_host"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
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
                    ::std::borrow::ToOwned::to_owned("teleport"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("teleport"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("params"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct CrossChainMessage"),
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
                    ::std::borrow::ToOwned::to_owned("PostReceived"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("PostReceived"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("nonce"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("source"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                                    indexed: false,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("message"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("NotAuthorized"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("NotAuthorized"),
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
    pub static CROSSCHAINMESSENGER_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x0C\xDE8\x03\x80a\x0C\xDE\x839\x81\x01`@\x81\x90Ra\0/\x91a\0TV[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90Ua\0\x84V[`\0` \x82\x84\x03\x12\x15a\0fW`\0\x80\xFD[\x81Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0}W`\0\x80\xFD[\x93\x92PPPV[a\x0CK\x80a\0\x93`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0}W`\x005`\xE0\x1C\x80cT\xCEFM\x11a\0[W\x80cT\xCEFM\x14a\0\xE1W\x80c\xC5,(\xAF\x14a\0\xF4W\x80c\xC7\x15\xF5+\x14a\x01\x07W\x80c\xF3p\xFD\xBB\x14a\x01\x1AW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x82W\x80cLF\xC05\x14a\0\xBBW\x80cN\x87\xBA\x19\x14a\0\xCEW[`\0\x80\xFD[a\0\xB9a\0\x906`\x04a\x03\xF4V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\x01\x80T\x90\x91\x16\x90UV[\0[a\0\xB9a\0\xC96`\x04a\x06\xEFV[a\x01(V[a\0\xB9a\0\xDC6`\x04a\x081V[a\x01\xB6V[a\0\xB9a\0\xEF6`\x04a\x08eV[a\x02(V[a\0\xB9a\x01\x026`\x04a\t\x1CV[a\x03\x05V[a\0\xB9a\x01\x156`\x04a\x081V[a\x03\x8BV[a\0\xB9a\0\xC96`\x04a\t\xACV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01SW`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`-`$\x82\x01R\x7FCrossChainMessenger doesn't emit`D\x82\x01Rl Get Requests`\x98\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\xE1W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80\x82\x01Q\x82Q`\xC0\x84\x01Q\x92Q\x7F\xF1q\xF8\xE6\x88\xD7*\x92\xC6\xD65\xD7\xF6\x8B\xF0\xDD\xF4\xBD\xA0\xD0#i{\x06\xEA\xABO\x16\x8F\xD6\x82\xBD\x93a\x02\x1D\x93\x92\x91a\x0BOV[`@Q\x80\x91\x03\x90\xA1PV[`\0`@Q\x80`\xA0\x01`@R\x80\x83`\0\x01Q\x81R` \x010`@Q` \x01a\x02h\x91\x90``\x91\x90\x91\x1Bk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16\x81R`\x14\x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R` \x85\x81\x01Q\x90\x83\x01R\x84\x81\x01Q`\x01`\x01`@\x1B\x03\x16\x82\x82\x01R`\0``\x90\x92\x01\x82\x90R\x90T\x90Qc\xD2[\xCD=`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD2[\xCD=\x90a\x02\xCF\x90\x84\x90`\x04\x01a\x0B\x8DV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x02\xE9W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x02\xFDW=`\0\x80>=`\0\xFD[PPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x030W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FCrossChainMessenger doesn't emit`D\x82\x01Ri responses`\xB0\x1B`d\x82\x01R`\x84\x01a\x01\xADV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03\xB6W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x13`$\x82\x01RrNo timeouts for now`h\x1B`D\x82\x01R`d\x01a\x01\xADV[`\0` \x82\x84\x03\x12\x15a\x04\x06W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x04\x1DW`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04]Wa\x04]a\x04$V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04]Wa\x04]a\x04$V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04\xADWa\x04\xADa\x04$V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x04\xC6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x04\xDFWa\x04\xDFa\x04$V[a\x04\xF2`\x1F\x82\x01`\x1F\x19\x16` \x01a\x04\x85V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x05\x07W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x05;W`\0\x80\xFD[\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x05YWa\x05Ya\x04$V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x05tW`\0\x80\xFD[\x815` a\x05\x89a\x05\x84\x83a\x05@V[a\x04\x85V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x05\xA8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x05\xE7W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x05\xCBW`\0\x80\x81\xFD[a\x05\xD9\x89\x86\x83\x8B\x01\x01a\x04\xB5V[\x84RP\x91\x83\x01\x91\x83\x01a\x05\xACV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x06\x05W`\0\x80\xFD[a\x06\ra\x04:V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x06&W`\0\x80\xFD[a\x062\x85\x83\x86\x01a\x04\xB5V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x06HW`\0\x80\xFD[a\x06T\x85\x83\x86\x01a\x04\xB5V[` \x84\x01Ra\x06e`@\x85\x01a\x05$V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x06~W`\0\x80\xFD[a\x06\x8A\x85\x83\x86\x01a\x04\xB5V[``\x84\x01Ra\x06\x9B`\x80\x85\x01a\x05$V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x06\xB4W`\0\x80\xFD[Pa\x06\xC1\x84\x82\x85\x01a\x05cV[`\xA0\x83\x01RPa\x06\xD3`\xC0\x83\x01a\x05$V[`\xC0\x82\x01Ra\x06\xE4`\xE0\x83\x01a\x05$V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x07\x01W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x07\x17W`\0\x80\xFD[a\x07#\x84\x82\x85\x01a\x05\xF2V[\x94\x93PPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x07>W`\0\x80\xFD[a\x07Fa\x04:V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07_W`\0\x80\xFD[a\x07k\x85\x83\x86\x01a\x04\xB5V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\x81W`\0\x80\xFD[a\x07\x8D\x85\x83\x86\x01a\x04\xB5V[` \x84\x01Ra\x07\x9E`@\x85\x01a\x05$V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xB7W`\0\x80\xFD[a\x07\xC3\x85\x83\x86\x01a\x04\xB5V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x07\xDCW`\0\x80\xFD[a\x07\xE8\x85\x83\x86\x01a\x04\xB5V[`\x80\x84\x01Ra\x07\xF9`\xA0\x85\x01a\x05$V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08\x12W`\0\x80\xFD[Pa\x08\x1F\x84\x82\x85\x01a\x04\xB5V[`\xC0\x83\x01RPa\x06\xE4`\xE0\x83\x01a\x05$V[`\0` \x82\x84\x03\x12\x15a\x08CW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08YW`\0\x80\xFD[a\x07#\x84\x82\x85\x01a\x07+V[`\0` \x82\x84\x03\x12\x15a\x08wW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x8EW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\x08\xA2W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\x08\xBDWa\x08\xBDa\x04$V[`@R\x825\x82\x81\x11\x15a\x08\xCFW`\0\x80\xFD[a\x08\xDB\x87\x82\x86\x01a\x04\xB5V[\x82RP` \x83\x015\x82\x81\x11\x15a\x08\xF0W`\0\x80\xFD[a\x08\xFC\x87\x82\x86\x01a\x04\xB5V[` \x83\x01RPa\t\x0E`@\x84\x01a\x05$V[`@\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\t.W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\tEW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\tYW`\0\x80\xFD[a\taa\x04cV[\x825\x82\x81\x11\x15a\tpW`\0\x80\xFD[a\t|\x87\x82\x86\x01a\x07+V[\x82RP` \x83\x015\x82\x81\x11\x15a\t\x91W`\0\x80\xFD[a\t\x9D\x87\x82\x86\x01a\x04\xB5V[` \x83\x01RP\x95\x94PPPPPV[`\0` \x80\x83\x85\x03\x12\x15a\t\xBFW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xD6W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\t\xECW`\0\x80\xFD[a\t\xF4a\x04cV[\x835\x83\x81\x11\x15a\n\x03W`\0\x80\xFD[a\n\x0F\x89\x82\x87\x01a\x05\xF2V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\n#W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\n8W`\0\x80\xFD[\x835a\nFa\x05\x84\x82a\x05@V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\neW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\n\xF5W\x805\x87\x81\x11\x15a\n\x81W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\n\x97W`\0\x80\x81\xFD[a\n\x9Fa\x04cV[\x8A\x82\x015\x89\x81\x11\x15a\n\xB1W`\0\x80\x81\xFD[a\n\xBF\x8F\x8D\x83\x86\x01\x01a\x04\xB5V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\n\xD4W`\0\x80\x81\xFD[a\n\xE2\x8F\x8D\x83\x86\x01\x01a\x04\xB5V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\niV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\x0B/W` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\x0B\x13V[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x84\x16\x81R``` \x82\x01R`\0a\x0Bq``\x83\x01\x85a\x0B\tV[\x82\x81\x03`@\x84\x01Ra\x0B\x83\x81\x85a\x0B\tV[\x96\x95PPPPPPV[` \x81R`\0\x82Q`\xA0` \x84\x01Ra\x0B\xA9`\xC0\x84\x01\x82a\x0B\tV[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x0B\xC7\x83\x83a\x0B\tV[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x0B\xE5\x82\x82a\x0B\tV[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP\x80\x91PP\x92\x91PPV\xFE\xA2dipfsX\"\x12 @\xA84\xE2\xCCJ{B\xAFf\x12%wr]\x1C\"\xDE\xB5\xA8\xF1D\xC7=\x07\x11\xEDHx)\x9A\x91dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static CROSSCHAINMESSENGER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0}W`\x005`\xE0\x1C\x80cT\xCEFM\x11a\0[W\x80cT\xCEFM\x14a\0\xE1W\x80c\xC5,(\xAF\x14a\0\xF4W\x80c\xC7\x15\xF5+\x14a\x01\x07W\x80c\xF3p\xFD\xBB\x14a\x01\x1AW`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x82W\x80cLF\xC05\x14a\0\xBBW\x80cN\x87\xBA\x19\x14a\0\xCEW[`\0\x80\xFD[a\0\xB9a\0\x906`\x04a\x03\xF4V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\x01\x80T\x90\x91\x16\x90UV[\0[a\0\xB9a\0\xC96`\x04a\x06\xEFV[a\x01(V[a\0\xB9a\0\xDC6`\x04a\x081V[a\x01\xB6V[a\0\xB9a\0\xEF6`\x04a\x08eV[a\x02(V[a\0\xB9a\x01\x026`\x04a\t\x1CV[a\x03\x05V[a\0\xB9a\x01\x156`\x04a\x081V[a\x03\x8BV[a\0\xB9a\0\xC96`\x04a\t\xACV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01SW`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`-`$\x82\x01R\x7FCrossChainMessenger doesn't emit`D\x82\x01Rl Get Requests`\x98\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\xE1W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80\x82\x01Q\x82Q`\xC0\x84\x01Q\x92Q\x7F\xF1q\xF8\xE6\x88\xD7*\x92\xC6\xD65\xD7\xF6\x8B\xF0\xDD\xF4\xBD\xA0\xD0#i{\x06\xEA\xABO\x16\x8F\xD6\x82\xBD\x93a\x02\x1D\x93\x92\x91a\x0BOV[`@Q\x80\x91\x03\x90\xA1PV[`\0`@Q\x80`\xA0\x01`@R\x80\x83`\0\x01Q\x81R` \x010`@Q` \x01a\x02h\x91\x90``\x91\x90\x91\x1Bk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16\x81R`\x14\x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R` \x85\x81\x01Q\x90\x83\x01R\x84\x81\x01Q`\x01`\x01`@\x1B\x03\x16\x82\x82\x01R`\0``\x90\x92\x01\x82\x90R\x90T\x90Qc\xD2[\xCD=`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD2[\xCD=\x90a\x02\xCF\x90\x84\x90`\x04\x01a\x0B\x8DV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x02\xE9W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x02\xFDW=`\0\x80>=`\0\xFD[PPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x030W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FCrossChainMessenger doesn't emit`D\x82\x01Ri responses`\xB0\x1B`d\x82\x01R`\x84\x01a\x01\xADV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03\xB6W`@Qc\xEA\x8EN\xB5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x13`$\x82\x01RrNo timeouts for now`h\x1B`D\x82\x01R`d\x01a\x01\xADV[`\0` \x82\x84\x03\x12\x15a\x04\x06W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x04\x1DW`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04]Wa\x04]a\x04$V[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04]Wa\x04]a\x04$V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x04\xADWa\x04\xADa\x04$V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x04\xC6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x04\xDFWa\x04\xDFa\x04$V[a\x04\xF2`\x1F\x82\x01`\x1F\x19\x16` \x01a\x04\x85V[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x05\x07W`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x05;W`\0\x80\xFD[\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x05YWa\x05Ya\x04$V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x05tW`\0\x80\xFD[\x815` a\x05\x89a\x05\x84\x83a\x05@V[a\x04\x85V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x05\xA8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x05\xE7W\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x05\xCBW`\0\x80\x81\xFD[a\x05\xD9\x89\x86\x83\x8B\x01\x01a\x04\xB5V[\x84RP\x91\x83\x01\x91\x83\x01a\x05\xACV[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x06\x05W`\0\x80\xFD[a\x06\ra\x04:V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x06&W`\0\x80\xFD[a\x062\x85\x83\x86\x01a\x04\xB5V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x06HW`\0\x80\xFD[a\x06T\x85\x83\x86\x01a\x04\xB5V[` \x84\x01Ra\x06e`@\x85\x01a\x05$V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x06~W`\0\x80\xFD[a\x06\x8A\x85\x83\x86\x01a\x04\xB5V[``\x84\x01Ra\x06\x9B`\x80\x85\x01a\x05$V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x06\xB4W`\0\x80\xFD[Pa\x06\xC1\x84\x82\x85\x01a\x05cV[`\xA0\x83\x01RPa\x06\xD3`\xC0\x83\x01a\x05$V[`\xC0\x82\x01Ra\x06\xE4`\xE0\x83\x01a\x05$V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x07\x01W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x07\x17W`\0\x80\xFD[a\x07#\x84\x82\x85\x01a\x05\xF2V[\x94\x93PPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x07>W`\0\x80\xFD[a\x07Fa\x04:V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07_W`\0\x80\xFD[a\x07k\x85\x83\x86\x01a\x04\xB5V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x07\x81W`\0\x80\xFD[a\x07\x8D\x85\x83\x86\x01a\x04\xB5V[` \x84\x01Ra\x07\x9E`@\x85\x01a\x05$V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x07\xB7W`\0\x80\xFD[a\x07\xC3\x85\x83\x86\x01a\x04\xB5V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\x07\xDCW`\0\x80\xFD[a\x07\xE8\x85\x83\x86\x01a\x04\xB5V[`\x80\x84\x01Ra\x07\xF9`\xA0\x85\x01a\x05$V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\x08\x12W`\0\x80\xFD[Pa\x08\x1F\x84\x82\x85\x01a\x04\xB5V[`\xC0\x83\x01RPa\x06\xE4`\xE0\x83\x01a\x05$V[`\0` \x82\x84\x03\x12\x15a\x08CW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08YW`\0\x80\xFD[a\x07#\x84\x82\x85\x01a\x07+V[`\0` \x82\x84\x03\x12\x15a\x08wW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\x8EW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\x08\xA2W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\x08\xBDWa\x08\xBDa\x04$V[`@R\x825\x82\x81\x11\x15a\x08\xCFW`\0\x80\xFD[a\x08\xDB\x87\x82\x86\x01a\x04\xB5V[\x82RP` \x83\x015\x82\x81\x11\x15a\x08\xF0W`\0\x80\xFD[a\x08\xFC\x87\x82\x86\x01a\x04\xB5V[` \x83\x01RPa\t\x0E`@\x84\x01a\x05$V[`@\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\t.W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\tEW`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\tYW`\0\x80\xFD[a\taa\x04cV[\x825\x82\x81\x11\x15a\tpW`\0\x80\xFD[a\t|\x87\x82\x86\x01a\x07+V[\x82RP` \x83\x015\x82\x81\x11\x15a\t\x91W`\0\x80\xFD[a\t\x9D\x87\x82\x86\x01a\x04\xB5V[` \x83\x01RP\x95\x94PPPPPV[`\0` \x80\x83\x85\x03\x12\x15a\t\xBFW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xD6W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\t\xECW`\0\x80\xFD[a\t\xF4a\x04cV[\x835\x83\x81\x11\x15a\n\x03W`\0\x80\xFD[a\n\x0F\x89\x82\x87\x01a\x05\xF2V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\n#W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\n8W`\0\x80\xFD[\x835a\nFa\x05\x84\x82a\x05@V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\neW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\n\xF5W\x805\x87\x81\x11\x15a\n\x81W`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\n\x97W`\0\x80\x81\xFD[a\n\x9Fa\x04cV[\x8A\x82\x015\x89\x81\x11\x15a\n\xB1W`\0\x80\x81\xFD[a\n\xBF\x8F\x8D\x83\x86\x01\x01a\x04\xB5V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\n\xD4W`\0\x80\x81\xFD[a\n\xE2\x8F\x8D\x83\x86\x01\x01a\x04\xB5V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\niV[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\x0B/W` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\x0B\x13V[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x84\x16\x81R``` \x82\x01R`\0a\x0Bq``\x83\x01\x85a\x0B\tV[\x82\x81\x03`@\x84\x01Ra\x0B\x83\x81\x85a\x0B\tV[\x96\x95PPPPPPV[` \x81R`\0\x82Q`\xA0` \x84\x01Ra\x0B\xA9`\xC0\x84\x01\x82a\x0B\tV[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x0B\xC7\x83\x83a\x0B\tV[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x0B\xE5\x82\x82a\x0B\tV[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP\x80\x91PP\x92\x91PPV\xFE\xA2dipfsX\"\x12 @\xA84\xE2\xCCJ{B\xAFf\x12%wr]\x1C\"\xDE\xB5\xA8\xF1D\xC7=\x07\x11\xEDHx)\x9A\x91dsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static CROSSCHAINMESSENGER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
    pub struct CrossChainMessenger<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for CrossChainMessenger<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for CrossChainMessenger<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for CrossChainMessenger<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for CrossChainMessenger<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(CrossChainMessenger))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> CrossChainMessenger<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    CROSSCHAINMESSENGER_ABI.clone(),
                    client,
                ),
            )
        }
        /// Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it.
        /// Returns a new instance of a deployer that returns an instance of this contract after sending the transaction
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
                CROSSCHAINMESSENGER_ABI.clone(),
                CROSSCHAINMESSENGER_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
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
        ///Calls the contract's `setIsmpHost` (0x0e8324a2) function
        pub fn set_ismp_host(
            &self,
            host: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([14, 131, 36, 162], host)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `teleport` (0x54ce464d) function
        pub fn teleport(
            &self,
            params: CrossChainMessage,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([84, 206, 70, 77], (params,))
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `PostReceived` event
        pub fn post_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostReceivedFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            PostReceivedFilter,
        > {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for CrossChainMessenger<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `NotAuthorized` with signature `NotAuthorized()` and selector `0xea8e4eb5`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "NotAuthorized", abi = "NotAuthorized()")]
    pub struct NotAuthorized;
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethevent(name = "PostReceived", abi = "PostReceived(uint256,bytes,string)")]
    pub struct PostReceivedFilter {
        pub nonce: ::ethers::core::types::U256,
        pub source: ::ethers::core::types::Bytes,
        pub message: ::std::string::String,
    }
    ///Container type for all input parameters for the `onAccept` function with signature `onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0x4e87ba19`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onAccept",
        abi = "onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnAcceptCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature `onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))` and selector `0xf370fdbb`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onGetResponse",
        abi = "onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64),(bytes,bytes)[]))"
    )]
    pub struct OnGetResponseCall {
        pub response: GetResponse,
    }
    ///Container type for all input parameters for the `onGetTimeout` function with signature `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))` and selector `0x4c46c035`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onGetTimeout",
        abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64,uint64))"
    )]
    pub struct OnGetTimeoutCall {
        pub request: GetRequest,
    }
    ///Container type for all input parameters for the `onPostResponse` function with signature `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))` and selector `0xc52c28af`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onPostResponse",
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64),bytes))"
    )]
    pub struct OnPostResponseCall {
        pub response: PostResponse,
    }
    ///Container type for all input parameters for the `onPostTimeout` function with signature `onPostTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))` and selector `0xc715f52b`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "onPostTimeout",
        abi = "onPostTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes,uint64))"
    )]
    pub struct OnPostTimeoutCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `setIsmpHost` function with signature `setIsmpHost(address)` and selector `0x0e8324a2`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "setIsmpHost", abi = "setIsmpHost(address)")]
    pub struct SetIsmpHostCall {
        pub host: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `teleport` function with signature `teleport((bytes,bytes,uint64))` and selector `0x54ce464d`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "teleport", abi = "teleport((bytes,bytes,uint64))")]
    pub struct TeleportCall {
        pub params: CrossChainMessage,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum CrossChainMessengerCalls {
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostTimeout(OnPostTimeoutCall),
        SetIsmpHost(SetIsmpHostCall),
        Teleport(TeleportCall),
    }
    impl ::ethers::core::abi::AbiDecode for CrossChainMessengerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <OnAcceptCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnAccept(decoded));
            }
            if let Ok(decoded) = <OnGetResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnGetResponse(decoded));
            }
            if let Ok(decoded) = <OnGetTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnGetTimeout(decoded));
            }
            if let Ok(decoded) = <OnPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostResponse(decoded));
            }
            if let Ok(decoded) = <OnPostTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostTimeout(decoded));
            }
            if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetIsmpHost(decoded));
            }
            if let Ok(decoded) = <TeleportCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Teleport(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for CrossChainMessengerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::OnAccept(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnGetResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnGetTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetIsmpHost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Teleport(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for CrossChainMessengerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
                Self::Teleport(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<OnAcceptCall> for CrossChainMessengerCalls {
        fn from(value: OnAcceptCall) -> Self {
            Self::OnAccept(value)
        }
    }
    impl ::core::convert::From<OnGetResponseCall> for CrossChainMessengerCalls {
        fn from(value: OnGetResponseCall) -> Self {
            Self::OnGetResponse(value)
        }
    }
    impl ::core::convert::From<OnGetTimeoutCall> for CrossChainMessengerCalls {
        fn from(value: OnGetTimeoutCall) -> Self {
            Self::OnGetTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostResponseCall> for CrossChainMessengerCalls {
        fn from(value: OnPostResponseCall) -> Self {
            Self::OnPostResponse(value)
        }
    }
    impl ::core::convert::From<OnPostTimeoutCall> for CrossChainMessengerCalls {
        fn from(value: OnPostTimeoutCall) -> Self {
            Self::OnPostTimeout(value)
        }
    }
    impl ::core::convert::From<SetIsmpHostCall> for CrossChainMessengerCalls {
        fn from(value: SetIsmpHostCall) -> Self {
            Self::SetIsmpHost(value)
        }
    }
    impl ::core::convert::From<TeleportCall> for CrossChainMessengerCalls {
        fn from(value: TeleportCall) -> Self {
            Self::Teleport(value)
        }
    }
    ///`CrossChainMessage(bytes,bytes,uint64)`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct CrossChainMessage {
        pub dest: ::ethers::core::types::Bytes,
        pub message: ::ethers::core::types::Bytes,
        pub timeout: u64,
    }
}
