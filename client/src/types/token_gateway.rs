pub use token_gateway::*;
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
pub mod token_gateway {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                    name: ::std::borrow::ToOwned::to_owned("_admin"),
                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                    internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
                        "address"
                    ),),
                },],
            }),
            functions: ::core::convert::From::from([
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
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
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
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
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
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
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
                (
                    ::std::borrow::ToOwned::to_owned("send"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("send"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("params"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Address,
                                ::ethers::core::abi::ethabi::ParamType::Bytes,
                                ::ethers::core::abi::ethabi::ParamType::Address,
                                ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                            ],),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("struct SendParams"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("setIsmpHost"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("_host"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
            ]),
            events: ::core::convert::From::from([(
                ::std::borrow::ToOwned::to_owned("AssetReceived"),
                ::std::vec![::ethers::core::abi::ethabi::Event {
                    name: ::std::borrow::ToOwned::to_owned("AssetReceived"),
                    inputs: ::std::vec![
                        ::ethers::core::abi::ethabi::EventParam {
                            name: ::std::borrow::ToOwned::to_owned("source"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Bytes,
                            indexed: false,
                        },
                        ::ethers::core::abi::ethabi::EventParam {
                            name: ::std::borrow::ToOwned::to_owned("nonce"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            indexed: false,
                        },
                    ],
                    anonymous: false,
                },],
            )]),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static TOKENGATEWAY_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x0F@8\x03\x80a\x0F@\x839\x81\x01`@\x81\x90Ra\0/\x91a\0TV[`\x01\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90Ua\0\x84V[`\0` \x82\x84\x03\x12\x15a\0fW`\0\x80\xFD[\x81Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0}W`\0\x80\xFD[\x93\x92PPPV[a\x0E\xAD\x80a\0\x93`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0}W`\x005`\xE0\x1C\x80cn[j%\x11a\0[W\x80cn[j%\x14a\0\xBDW\x80c\xC5,(\xAF\x14a\0\xD0W\x80c\xC7\x15\xF5+\x14a\0\xE3W\x80c\xF3p\xFD\xBB\x14a\0\xF6W`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x82W\x80cLF\xC05\x14a\0\x97W\x80cN\x87\xBA\x19\x14a\0\xAAW[`\0\x80\xFD[a\0\x95a\0\x906`\x04a\x05\xB2V[a\x01\x04V[\0[a\0\x95a\0\xA56`\x04a\x08\xC3V[a\x01`V[a\0\x95a\0\xB86`\x04a\n\x05V[a\x01\xE2V[a\0\x95a\0\xCB6`\x04a\n9V[a\x02\xD6V[a\0\x95a\0\xDE6`\x04a\n\xE9V[a\x04eV[a\0\x95a\0\xF16`\x04a\n\x05V[a\x04\xE3V[a\0\x95a\0\xA56`\x04a\x0ByV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x017W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@Q\x80\x91\x03\x90\xFD[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\x01\x80T\x90\x91\x16\x90UV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\x8AW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`'`$\x82\x01R\x7FToken gateway doesn't emit Get R`D\x82\x01Rfequests`\xC8\x1B`d\x82\x01R`\x84\x01a\x01.V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x0CW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`\0\x80`\0\x80\x84`\xC0\x01Q\x80` \x01\x90Q\x81\x01\x90a\x02*\x91\x90a\r\x01V[\x93P\x93P\x93P\x93P\x80`\x01`\x01`\xA0\x1B\x03\x16c\x94\xD0\x08\xEF\x84\x84`@Q\x83c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x02`\x92\x91\x90a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x02zW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x02\x8EW=`\0\x80>=`\0\xFD[PP\x86Q`@\x80\x89\x01Q\x90Q\x7F\xBF\x1D\x85n\xC8\x85\xD8\xE7}\x92`x\xE6cFe\x04s\xCF\x97-5\x81?{>\\_>\xDC\xB5\xD3\x94Pa\x02\xC7\x93Pa\r\xC4V[`@Q\x80\x91\x03\x90\xA1PPPPPV[``\x81\x01Q\x81Q`@QcD\xD1q\x87`\xE0\x1B\x81R3\x92`\x01`\x01`\xA0\x1B\x03\x16\x91cD\xD1q\x87\x91a\x03\n\x91\x85\x91`\x04\x01a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x03$W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x038W=`\0\x80>=`\0\xFD[PPP` \x80\x84\x01Q\x84Q``\x86\x01Q`@Q`\0\x95Pa\x03\x84\x94\x87\x94\x93\x92\x91\x01`\x01`\x01`\xA0\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R`@\x83\x01\x91\x90\x91R\x90\x91\x16``\x82\x01R`\x80\x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R`\xA0\x83\x01\x82R\x85\x82\x01Q\x83R\x90Qk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x190``\x1B\x16` \x82\x81\x01\x91\x90\x91R\x91\x93P`\0\x92\x91\x82\x01\x90`4\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R` \x82\x01\x85\x90R`\x80\x87\x01Q`\x01`\x01`@\x1B\x03\x16\x82\x82\x01R`\0``\x90\x92\x01\x82\x90R\x90T\x90Qc\xD2[\xCD=`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD2[\xCD=\x90a\x04-\x90\x84\x90`\x04\x01a\r\xEFV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04GW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04[W=`\0\x80>=`\0\xFD[PPPPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x04\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FToken gateway doesn't emit respo`D\x82\x01Rcnses`\xE0\x1B`d\x82\x01R`\x84\x01a\x01.V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x05\rW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`\0\x80`\0\x80\x84`\xC0\x01Q\x80` \x01\x90Q\x81\x01\x90a\x05+\x91\x90a\r\x01V[\x93P\x93P\x93P\x93P\x80`\x01`\x01`\xA0\x1B\x03\x16c\x94\xD0\x08\xEF\x85\x84`@Q\x83c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x05a\x92\x91\x90a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05{W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x8FW=`\0\x80>=`\0\xFD[PPPPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05\xAFW`\0\x80\xFD[PV[`\0` \x82\x84\x03\x12\x15a\x05\xC4W`\0\x80\xFD[\x815a\x05\xCF\x81a\x05\x9AV[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@R\x90V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x81Wa\x06\x81a\x05\xD6V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x06\x9AW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06\xB3Wa\x06\xB3a\x05\xD6V[a\x06\xC6`\x1F\x82\x01`\x1F\x19\x16` \x01a\x06YV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x06\xDBW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07\x0FW`\0\x80\xFD[\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07-Wa\x07-a\x05\xD6V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07HW`\0\x80\xFD[\x815` a\x07]a\x07X\x83a\x07\x14V[a\x06YV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x07|W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x07\xBBW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x07\x9FW`\0\x80\x81\xFD[a\x07\xAD\x89\x86\x83\x8B\x01\x01a\x06\x89V[\x84RP\x91\x83\x01\x91\x83\x01a\x07\x80V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x07\xD9W`\0\x80\xFD[a\x07\xE1a\x05\xECV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xFAW`\0\x80\xFD[a\x08\x06\x85\x83\x86\x01a\x06\x89V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\x1CW`\0\x80\xFD[a\x08(\x85\x83\x86\x01a\x06\x89V[` \x84\x01Ra\x089`@\x85\x01a\x06\xF8V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08RW`\0\x80\xFD[a\x08^\x85\x83\x86\x01a\x06\x89V[``\x84\x01Ra\x08o`\x80\x85\x01a\x06\xF8V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x08\x88W`\0\x80\xFD[Pa\x08\x95\x84\x82\x85\x01a\x077V[`\xA0\x83\x01RPa\x08\xA7`\xC0\x83\x01a\x06\xF8V[`\xC0\x82\x01Ra\x08\xB8`\xE0\x83\x01a\x06\xF8V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08\xD5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08\xEBW`\0\x80\xFD[a\x08\xF7\x84\x82\x85\x01a\x07\xC6V[\x94\x93PPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\t\x12W`\0\x80\xFD[a\t\x1Aa\x05\xECV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t3W`\0\x80\xFD[a\t?\x85\x83\x86\x01a\x06\x89V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\tUW`\0\x80\xFD[a\ta\x85\x83\x86\x01a\x06\x89V[` \x84\x01Ra\tr`@\x85\x01a\x06\xF8V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t\x8BW`\0\x80\xFD[a\t\x97\x85\x83\x86\x01a\x06\x89V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\xB0W`\0\x80\xFD[a\t\xBC\x85\x83\x86\x01a\x06\x89V[`\x80\x84\x01Ra\t\xCD`\xA0\x85\x01a\x06\xF8V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\xE6W`\0\x80\xFD[Pa\t\xF3\x84\x82\x85\x01a\x06\x89V[`\xC0\x83\x01RPa\x08\xB8`\xE0\x83\x01a\x06\xF8V[`\0` \x82\x84\x03\x12\x15a\n\x17W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n-W`\0\x80\xFD[a\x08\xF7\x84\x82\x85\x01a\x08\xFFV[`\0` \x82\x84\x03\x12\x15a\nKW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nbW`\0\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\nvW`\0\x80\xFD[a\n~a\x06\x15V[\x825\x81R` \x83\x015a\n\x90\x81a\x05\x9AV[` \x82\x01R`@\x83\x015\x82\x81\x11\x15a\n\xA7W`\0\x80\xFD[a\n\xB3\x87\x82\x86\x01a\x06\x89V[`@\x83\x01RP``\x83\x015\x91Pa\n\xC9\x82a\x05\x9AV[\x81``\x82\x01Ra\n\xDB`\x80\x84\x01a\x06\xF8V[`\x80\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\n\xFBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x12W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0B&W`\0\x80\xFD[a\x0B.a\x067V[\x825\x82\x81\x11\x15a\x0B=W`\0\x80\xFD[a\x0BI\x87\x82\x86\x01a\x08\xFFV[\x82RP` \x83\x015\x82\x81\x11\x15a\x0B^W`\0\x80\xFD[a\x0Bj\x87\x82\x86\x01a\x06\x89V[` \x83\x01RP\x95\x94PPPPPV[`\0` \x80\x83\x85\x03\x12\x15a\x0B\x8CW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xA3W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B\xB9W`\0\x80\xFD[a\x0B\xC1a\x067V[\x835\x83\x81\x11\x15a\x0B\xD0W`\0\x80\xFD[a\x0B\xDC\x89\x82\x87\x01a\x07\xC6V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0B\xF0W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0C\x05W`\0\x80\xFD[\x835a\x0C\x13a\x07X\x82a\x07\x14V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0C2W`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\xC2W\x805\x87\x81\x11\x15a\x0CNW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0CdW`\0\x80\x81\xFD[a\x0Cla\x067V[\x8A\x82\x015\x89\x81\x11\x15a\x0C~W`\0\x80\x81\xFD[a\x0C\x8C\x8F\x8D\x83\x86\x01\x01a\x06\x89V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0C\xA1W`\0\x80\x81\xFD[a\x0C\xAF\x8F\x8D\x83\x86\x01\x01a\x06\x89V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0C6V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[` \x80\x82R`\x11\x90\x82\x01Rp\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x18\xD8[\x1B`z\x1B`@\x82\x01R``\x01\x90V[`\0\x80`\0\x80`\x80\x85\x87\x03\x12\x15a\r\x17W`\0\x80\xFD[\x84Qa\r\"\x81a\x05\x9AV[` \x86\x01Q\x90\x94Pa\r3\x81a\x05\x9AV[`@\x86\x01Q``\x87\x01Q\x91\x94P\x92Pa\rK\x81a\x05\x9AV[\x93\x96\x92\x95P\x90\x93PPV[`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x82R` \x82\x01R```@\x82\x01\x81\x90R`\0\x90\x82\x01R`\x80\x01\x90V[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\r\xA4W` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\r\x88V[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[`@\x81R`\0a\r\xD7`@\x83\x01\x85a\r~V[\x90P`\x01`\x01`@\x1B\x03\x83\x16` \x83\x01R\x93\x92PPPV[` \x81R`\0\x82Q`\xA0` \x84\x01Ra\x0E\x0B`\xC0\x84\x01\x82a\r~V[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x0E)\x83\x83a\r~V[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x0EG\x82\x82a\r~V[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP\x80\x91PP\x92\x91PPV\xFE\xA2dipfsX\"\x12 l\xDC\xBE\xE8\x18\xE23Tg\"\xB5\x08\xB7db\xB9m\xA0C-T\x84\xD4y\xD24\x1B\x98A.\xE5\xCBdsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static TOKENGATEWAY_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0}W`\x005`\xE0\x1C\x80cn[j%\x11a\0[W\x80cn[j%\x14a\0\xBDW\x80c\xC5,(\xAF\x14a\0\xD0W\x80c\xC7\x15\xF5+\x14a\0\xE3W\x80c\xF3p\xFD\xBB\x14a\0\xF6W`\0\x80\xFD[\x80c\x0E\x83$\xA2\x14a\0\x82W\x80cLF\xC05\x14a\0\x97W\x80cN\x87\xBA\x19\x14a\0\xAAW[`\0\x80\xFD[a\0\x95a\0\x906`\x04a\x05\xB2V[a\x01\x04V[\0[a\0\x95a\0\xA56`\x04a\x08\xC3V[a\x01`V[a\0\x95a\0\xB86`\x04a\n\x05V[a\x01\xE2V[a\0\x95a\0\xCB6`\x04a\n9V[a\x02\xD6V[a\0\x95a\0\xDE6`\x04a\n\xE9V[a\x04eV[a\0\x95a\0\xF16`\x04a\n\x05V[a\x04\xE3V[a\0\x95a\0\xA56`\x04a\x0ByV[`\x01T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x017W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@Q\x80\x91\x03\x90\xFD[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\x01\x80T\x90\x91\x16\x90UV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01\x8AW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`'`$\x82\x01R\x7FToken gateway doesn't emit Get R`D\x82\x01Rfequests`\xC8\x1B`d\x82\x01R`\x84\x01a\x01.V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x02\x0CW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`\0\x80`\0\x80\x84`\xC0\x01Q\x80` \x01\x90Q\x81\x01\x90a\x02*\x91\x90a\r\x01V[\x93P\x93P\x93P\x93P\x80`\x01`\x01`\xA0\x1B\x03\x16c\x94\xD0\x08\xEF\x84\x84`@Q\x83c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x02`\x92\x91\x90a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x02zW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x02\x8EW=`\0\x80>=`\0\xFD[PP\x86Q`@\x80\x89\x01Q\x90Q\x7F\xBF\x1D\x85n\xC8\x85\xD8\xE7}\x92`x\xE6cFe\x04s\xCF\x97-5\x81?{>\\_>\xDC\xB5\xD3\x94Pa\x02\xC7\x93Pa\r\xC4V[`@Q\x80\x91\x03\x90\xA1PPPPPV[``\x81\x01Q\x81Q`@QcD\xD1q\x87`\xE0\x1B\x81R3\x92`\x01`\x01`\xA0\x1B\x03\x16\x91cD\xD1q\x87\x91a\x03\n\x91\x85\x91`\x04\x01a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x03$W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x038W=`\0\x80>=`\0\xFD[PPP` \x80\x84\x01Q\x84Q``\x86\x01Q`@Q`\0\x95Pa\x03\x84\x94\x87\x94\x93\x92\x91\x01`\x01`\x01`\xA0\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R`@\x83\x01\x91\x90\x91R\x90\x91\x16``\x82\x01R`\x80\x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R`\xA0\x83\x01\x82R\x85\x82\x01Q\x83R\x90Qk\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x190``\x1B\x16` \x82\x81\x01\x91\x90\x91R\x91\x93P`\0\x92\x91\x82\x01\x90`4\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x81R\x90\x82R` \x82\x01\x85\x90R`\x80\x87\x01Q`\x01`\x01`@\x1B\x03\x16\x82\x82\x01R`\0``\x90\x92\x01\x82\x90R\x90T\x90Qc\xD2[\xCD=`\xE0\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD2[\xCD=\x90a\x04-\x90\x84\x90`\x04\x01a\r\xEFV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04GW`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04[W=`\0\x80>=`\0\xFD[PPPPPPPPV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x04\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FToken gateway doesn't emit respo`D\x82\x01Rcnses`\xE0\x1B`d\x82\x01R`\x84\x01a\x01.V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x05\rW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01.\x90a\x0C\xD6V[`\0\x80`\0\x80\x84`\xC0\x01Q\x80` \x01\x90Q\x81\x01\x90a\x05+\x91\x90a\r\x01V[\x93P\x93P\x93P\x93P\x80`\x01`\x01`\xA0\x1B\x03\x16c\x94\xD0\x08\xEF\x85\x84`@Q\x83c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01a\x05a\x92\x91\x90a\rVV[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x05{W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x05\x8FW=`\0\x80>=`\0\xFD[PPPPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05\xAFW`\0\x80\xFD[PV[`\0` \x82\x84\x03\x12\x15a\x05\xC4W`\0\x80\xFD[\x815a\x05\xCF\x81a\x05\x9AV[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Qa\x01\0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@R\x90V[`@Q`\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x0FWa\x06\x0Fa\x05\xD6V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x06\x81Wa\x06\x81a\x05\xD6V[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x06\x9AW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06\xB3Wa\x06\xB3a\x05\xD6V[a\x06\xC6`\x1F\x82\x01`\x1F\x19\x16` \x01a\x06YV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x06\xDBW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x07\x0FW`\0\x80\xFD[\x91\x90PV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\x07-Wa\x07-a\x05\xD6V[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x07HW`\0\x80\xFD[\x815` a\x07]a\x07X\x83a\x07\x14V[a\x06YV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x07|W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x07\xBBW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x07\x9FW`\0\x80\x81\xFD[a\x07\xAD\x89\x86\x83\x8B\x01\x01a\x06\x89V[\x84RP\x91\x83\x01\x91\x83\x01a\x07\x80V[P\x96\x95PPPPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\x07\xD9W`\0\x80\xFD[a\x07\xE1a\x05\xECV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x07\xFAW`\0\x80\xFD[a\x08\x06\x85\x83\x86\x01a\x06\x89V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\x1CW`\0\x80\xFD[a\x08(\x85\x83\x86\x01a\x06\x89V[` \x84\x01Ra\x089`@\x85\x01a\x06\xF8V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x08RW`\0\x80\xFD[a\x08^\x85\x83\x86\x01a\x06\x89V[``\x84\x01Ra\x08o`\x80\x85\x01a\x06\xF8V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x08\x88W`\0\x80\xFD[Pa\x08\x95\x84\x82\x85\x01a\x077V[`\xA0\x83\x01RPa\x08\xA7`\xC0\x83\x01a\x06\xF8V[`\xC0\x82\x01Ra\x08\xB8`\xE0\x83\x01a\x06\xF8V[`\xE0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x08\xD5W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08\xEBW`\0\x80\xFD[a\x08\xF7\x84\x82\x85\x01a\x07\xC6V[\x94\x93PPPPV[`\0a\x01\0\x82\x84\x03\x12\x15a\t\x12W`\0\x80\xFD[a\t\x1Aa\x05\xECV[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t3W`\0\x80\xFD[a\t?\x85\x83\x86\x01a\x06\x89V[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\tUW`\0\x80\xFD[a\ta\x85\x83\x86\x01a\x06\x89V[` \x84\x01Ra\tr`@\x85\x01a\x06\xF8V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t\x8BW`\0\x80\xFD[a\t\x97\x85\x83\x86\x01a\x06\x89V[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\t\xB0W`\0\x80\xFD[a\t\xBC\x85\x83\x86\x01a\x06\x89V[`\x80\x84\x01Ra\t\xCD`\xA0\x85\x01a\x06\xF8V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\xE6W`\0\x80\xFD[Pa\t\xF3\x84\x82\x85\x01a\x06\x89V[`\xC0\x83\x01RPa\x08\xB8`\xE0\x83\x01a\x06\xF8V[`\0` \x82\x84\x03\x12\x15a\n\x17W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n-W`\0\x80\xFD[a\x08\xF7\x84\x82\x85\x01a\x08\xFFV[`\0` \x82\x84\x03\x12\x15a\nKW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\nbW`\0\x80\xFD[\x90\x83\x01\x90`\xA0\x82\x86\x03\x12\x15a\nvW`\0\x80\xFD[a\n~a\x06\x15V[\x825\x81R` \x83\x015a\n\x90\x81a\x05\x9AV[` \x82\x01R`@\x83\x015\x82\x81\x11\x15a\n\xA7W`\0\x80\xFD[a\n\xB3\x87\x82\x86\x01a\x06\x89V[`@\x83\x01RP``\x83\x015\x91Pa\n\xC9\x82a\x05\x9AV[\x81``\x82\x01Ra\n\xDB`\x80\x84\x01a\x06\xF8V[`\x80\x82\x01R\x95\x94PPPPPV[`\0` \x82\x84\x03\x12\x15a\n\xFBW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\x12W`\0\x80\xFD[\x90\x83\x01\x90`@\x82\x86\x03\x12\x15a\x0B&W`\0\x80\xFD[a\x0B.a\x067V[\x825\x82\x81\x11\x15a\x0B=W`\0\x80\xFD[a\x0BI\x87\x82\x86\x01a\x08\xFFV[\x82RP` \x83\x015\x82\x81\x11\x15a\x0B^W`\0\x80\xFD[a\x0Bj\x87\x82\x86\x01a\x06\x89V[` \x83\x01RP\x95\x94PPPPPV[`\0` \x80\x83\x85\x03\x12\x15a\x0B\x8CW`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xA3W`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0B\xB9W`\0\x80\xFD[a\x0B\xC1a\x067V[\x835\x83\x81\x11\x15a\x0B\xD0W`\0\x80\xFD[a\x0B\xDC\x89\x82\x87\x01a\x07\xC6V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\x0B\xF0W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\x0C\x05W`\0\x80\xFD[\x835a\x0C\x13a\x07X\x82a\x07\x14V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\x0C2W`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\x0C\xC2W\x805\x87\x81\x11\x15a\x0CNW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\x0CdW`\0\x80\x81\xFD[a\x0Cla\x067V[\x8A\x82\x015\x89\x81\x11\x15a\x0C~W`\0\x80\x81\xFD[a\x0C\x8C\x8F\x8D\x83\x86\x01\x01a\x06\x89V[\x82RP\x87\x82\x015\x89\x81\x11\x15a\x0C\xA1W`\0\x80\x81\xFD[a\x0C\xAF\x8F\x8D\x83\x86\x01\x01a\x06\x89V[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\x0C6V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[` \x80\x82R`\x11\x90\x82\x01Rp\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x18\xD8[\x1B`z\x1B`@\x82\x01R``\x01\x90V[`\0\x80`\0\x80`\x80\x85\x87\x03\x12\x15a\r\x17W`\0\x80\xFD[\x84Qa\r\"\x81a\x05\x9AV[` \x86\x01Q\x90\x94Pa\r3\x81a\x05\x9AV[`@\x86\x01Q``\x87\x01Q\x91\x94P\x92Pa\rK\x81a\x05\x9AV[\x93\x96\x92\x95P\x90\x93PPV[`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x82R` \x82\x01R```@\x82\x01\x81\x90R`\0\x90\x82\x01R`\x80\x01\x90V[`\0\x81Q\x80\x84R`\0[\x81\x81\x10\x15a\r\xA4W` \x81\x85\x01\x81\x01Q\x86\x83\x01\x82\x01R\x01a\r\x88V[P`\0` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[`@\x81R`\0a\r\xD7`@\x83\x01\x85a\r~V[\x90P`\x01`\x01`@\x1B\x03\x83\x16` \x83\x01R\x93\x92PPPV[` \x81R`\0\x82Q`\xA0` \x84\x01Ra\x0E\x0B`\xC0\x84\x01\x82a\r~V[\x90P` \x84\x01Q`\x1F\x19\x80\x85\x84\x03\x01`@\x86\x01Ra\x0E)\x83\x83a\r~V[\x92P`@\x86\x01Q\x91P\x80\x85\x84\x03\x01``\x86\x01RPa\x0EG\x82\x82a\r~V[\x91PP``\x84\x01Q`\x01`\x01`@\x1B\x03\x80\x82\x16`\x80\x86\x01R\x80`\x80\x87\x01Q\x16`\xA0\x86\x01RPP\x80\x91PP\x92\x91PPV\xFE\xA2dipfsX\"\x12 l\xDC\xBE\xE8\x18\xE23Tg\"\xB5\x08\xB7db\xB9m\xA0C-T\x84\xD4y\xD24\x1B\x98A.\xE5\xCBdsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static TOKENGATEWAY_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct TokenGateway<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for TokenGateway<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for TokenGateway<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for TokenGateway<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for TokenGateway<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(TokenGateway)).field(&self.address()).finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> TokenGateway<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(
                address.into(),
                TOKENGATEWAY_ABI.clone(),
                client,
            ))
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
                TOKENGATEWAY_ABI.clone(),
                TOKENGATEWAY_BYTECODE.clone().into(),
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
        ///Calls the contract's `send` (0x6e5b6a25) function
        pub fn send(
            &self,
            params: SendParams,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([110, 91, 106, 37], (params,))
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
        ///Gets the contract's `AssetReceived` event
        pub fn asset_received_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AssetReceivedFilter>
        {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AssetReceivedFilter>
        {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for TokenGateway<M> {
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
    #[ethevent(name = "AssetReceived", abi = "AssetReceived(bytes,uint256)")]
    pub struct AssetReceivedFilter {
        pub source: ::ethers::core::types::Bytes,
        pub nonce: ::ethers::core::types::U256,
    }
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
    ///Container type for all input parameters for the `send` function with signature
    /// `send((uint256,address,bytes,address,uint64))` and selector `0x6e5b6a25`
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
    #[ethcall(name = "send", abi = "send((uint256,address,bytes,address,uint64))")]
    pub struct SendCall {
        pub params: SendParams,
    }
    ///Container type for all input parameters for the `setIsmpHost` function with signature
    /// `setIsmpHost(address)` and selector `0x0e8324a2`
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
    #[ethcall(name = "setIsmpHost", abi = "setIsmpHost(address)")]
    pub struct SetIsmpHostCall {
        pub host: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum TokenGatewayCalls {
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostTimeout(OnPostTimeoutCall),
        Send(SendCall),
        SetIsmpHost(SetIsmpHostCall),
    }
    impl ::ethers::core::abi::AbiDecode for TokenGatewayCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
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
            if let Ok(decoded) = <SendCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Send(decoded))
            }
            if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::SetIsmpHost(decoded))
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for TokenGatewayCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::OnAccept(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnGetResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnGetTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnPostResponse(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OnPostTimeout(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Send(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::SetIsmpHost(element) => ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for TokenGatewayCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::Send(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<OnAcceptCall> for TokenGatewayCalls {
        fn from(value: OnAcceptCall) -> Self {
            Self::OnAccept(value)
        }
    }
    impl ::core::convert::From<OnGetResponseCall> for TokenGatewayCalls {
        fn from(value: OnGetResponseCall) -> Self {
            Self::OnGetResponse(value)
        }
    }
    impl ::core::convert::From<OnGetTimeoutCall> for TokenGatewayCalls {
        fn from(value: OnGetTimeoutCall) -> Self {
            Self::OnGetTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostResponseCall> for TokenGatewayCalls {
        fn from(value: OnPostResponseCall) -> Self {
            Self::OnPostResponse(value)
        }
    }
    impl ::core::convert::From<OnPostTimeoutCall> for TokenGatewayCalls {
        fn from(value: OnPostTimeoutCall) -> Self {
            Self::OnPostTimeout(value)
        }
    }
    impl ::core::convert::From<SendCall> for TokenGatewayCalls {
        fn from(value: SendCall) -> Self {
            Self::Send(value)
        }
    }
    impl ::core::convert::From<SetIsmpHostCall> for TokenGatewayCalls {
        fn from(value: SetIsmpHostCall) -> Self {
            Self::SetIsmpHost(value)
        }
    }
    ///`SendParams(uint256,address,bytes,address,uint64)`
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
    pub struct SendParams {
        pub amount: ::ethers::core::types::U256,
        pub to: ::ethers::core::types::Address,
        pub dest: ::ethers::core::types::Bytes,
        pub token_contract: ::ethers::core::types::Address,
        pub timeout: u64,
    }
}
