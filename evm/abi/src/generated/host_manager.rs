pub use host_manager::*;
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
pub mod host_manager {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("managerParams"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                            ::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Address,
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Address,
                            ],
                        ),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("struct HostManagerParams"),
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
                                    name: ::std::string::String::new(),
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
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
                                    name: ::std::string::String::new(),
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
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct GetRequest"),
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
                    ::std::borrow::ToOwned::to_owned("onPostRequestTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "onPostRequestTimeout",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
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
                    ::std::borrow::ToOwned::to_owned("onPostResponse"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("onPostResponse"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
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
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostResponse"),
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
                    ::std::borrow::ToOwned::to_owned("onPostResponseTimeout"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "onPostResponseTimeout",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
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
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bytes,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct PostResponse"),
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
                    ::std::borrow::ToOwned::to_owned("params"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("params"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Tuple(
                                        ::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Address,
                                        ],
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("struct HostManagerParams"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
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
                                    name: ::std::borrow::ToOwned::to_owned("host"),
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
            ]),
            events: ::std::collections::BTreeMap::new(),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static HOSTMANAGER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> = ::ethers::contract::Lazy::new(
        __abi,
    );
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x13\xBD8\x03\x80a\x13\xBD\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\x8BV[\x80Q`\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x90\x81\x16`\x01`\x01`\xA0\x1B\x03\x93\x84\x16\x17\x90\x91U` \x83\x01Q`\x01U`@\x90\x92\x01Q`\x02\x80T\x90\x93\x16\x91\x16\x17\x90Ua\0\xFDV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\0\x86W`\0\x80\xFD[\x91\x90PV[`\0``\x82\x84\x03\x12\x15a\0\x9DW`\0\x80\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\0\xCDWcNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Ra\0\xD9\x83a\0oV[\x81R` \x83\x01Q` \x82\x01Ra\0\xF1`@\x84\x01a\0oV[`@\x82\x01R\x93\x92PPPV[a\x12\xB1\x80a\x01\x0C`\09`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xCF\xF0\xAB\x96\x11a\0[W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xDE\xAET\xF5\x14a\x01`W\x80c\xEA\xEE\x1C\xAA\x14a\x01nW\x80c\xFE\xFF\x7F\xA8\x14a\0\x8DW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\xBC\r\xD4G\x14a\0\xB5W\x80c\xC4\x92\xE4&\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\t\xA4V[a\x01\x81V[\0[a\0\xA0a\0\xB06`\x04a\nrV[a\x01\xDDV[a\0\xA0a\0\xC36`\x04a\n\x94V[a\x02`V[a\0\xA0a\0\xD66`\x04a\x0CmV[a\x02\xB6V[a\x01,`@\x80Q``\x81\x01\x82R`\0\x80\x82R` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91RP`@\x80Q``\x81\x01\x82R`\0T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R`\x01T` \x83\x01R`\x02T\x16\x91\x81\x01\x91\x90\x91R\x90V[`@\x80Q\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R` \x80\x85\x01Q\x90\x83\x01R\x92\x82\x01Q\x90\x92\x16\x90\x82\x01R``\x01`@Q\x80\x91\x03\x90\xF3[a\0\xA0a\0\xD66`\x04a\x0C\xA1V[a\0\xA0a\x01|6`\x04a\r\xFEV[a\x03\nV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x027W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xD4V[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xD4V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xD4V[`\x02T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03dW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xD4V[a\x03\xBCa\x03u`\0`\x01\x01Ta\x05\x9FV[a\x03\x7F\x83\x80a\x0E8V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xD0\x90PV[a\x03\xFFW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xD4V[`\0a\x04\x0E`\xC0\x83\x01\x83a\x0E8V[`\0\x81\x81\x10a\x04\x1FWa\x04\x1Fa\x0E\x85V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04:Wa\x04:a\x0E\x9BV[\x90P`\0\x81`\x01\x81\x11\x15a\x04PWa\x04Pa\x0E\x9BV[\x03a\x04\xF2W`\0a\x04d`\xC0\x84\x01\x84a\x0E8V[a\x04r\x91`\x01\x90\x82\x90a\x0E\xB1V[\x81\x01\x90a\x04\x7F\x91\x90a\x0E\xDBV[`\x02T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xD5W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xE9W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x05\x06Wa\x05\x06a\x0E\x9BV[\x03a\x05fW`\0a\x05\x1A`\xC0\x84\x01\x84a\x0E8V[a\x05(\x91`\x01\x90\x82\x90a\x0E\xB1V[\x81\x01\x90a\x055\x91\x90a\x0F\x8CV[`\x02T`@Qc\x03G\x98{`\xE6\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD1\xE6\x1E\xC0\x90a\x04\xBB\x90\x84\x90`\x04\x01a\x11DV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xD4V[``a\x05\xAA\x82a\x05\xFDV[`@Q` \x01a\x05\xBA\x91\x90a\x12JV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0\x81Q\x83Q\x14a\x05\xE3WP`\0a\x05\xF7V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[```\0a\x06\n\x83a\x06\x8FV[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06)Wa\x06)a\x07gV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x06SW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x06]WP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x06\xCEWr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x06\xFAWm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x07\x18Wf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x070Wc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x07DWa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x07VW`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x05\xF7W`\x01\x01\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Qa\x01\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\x12Wa\x08\x12a\x07gV[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x08+W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08DWa\x08Da\x07gV[a\x08W`\x1F\x82\x01`\x1F\x19\x16` \x01a\x07\xEAV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08lW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xA0W`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xB7W`\0\x80\xFD[a\x08\xBFa\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xD8W`\0\x80\xFD[a\x08\xE4\x85\x83\x86\x01a\x08\x1AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xFAW`\0\x80\xFD[a\t\x06\x85\x83\x86\x01a\x08\x1AV[` \x84\x01Ra\t\x17`@\x85\x01a\x08\x89V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t0W`\0\x80\xFD[a\t<\x85\x83\x86\x01a\x08\x1AV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tUW`\0\x80\xFD[a\ta\x85\x83\x86\x01a\x08\x1AV[`\x80\x84\x01Ra\tr`\xA0\x85\x01a\x08\x89V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\x8BW`\0\x80\xFD[Pa\t\x98\x84\x82\x85\x01a\x08\x1AV[`\xC0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\xB6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xCDW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\t\xE1W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\t\xFCWa\t\xFCa\x07gV[`@R\x825\x82\x81\x11\x15a\n\x0EW`\0\x80\xFD[a\n\x1A\x87\x82\x86\x01a\x08\xA5V[\x82RP` \x83\x015\x82\x81\x11\x15a\n/W`\0\x80\xFD[a\n;\x87\x82\x86\x01a\x08\x1AV[` \x83\x01RPa\nM`@\x84\x01a\x08\x89V[`@\x82\x01R\x95\x94PPPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xA0W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\x84W`\0\x80\xFD[a\n\x8D\x82a\n[V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\xA6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xBCW`\0\x80\xFD[a\n\xC8\x84\x82\x85\x01a\x08\xA5V[\x94\x93PPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n\xE9Wa\n\xE9a\x07gV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\x04W`\0\x80\xFD[\x815` a\x0B\x19a\x0B\x14\x83a\n\xD0V[a\x07\xEAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0BwW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B[W`\0\x80\x81\xFD[a\x0Bi\x89\x86\x83\x8B\x01\x01a\x08\x1AV[\x84RP\x91\x83\x01\x91\x83\x01a\x0B<V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0B\x94W`\0\x80\xFD[a\x0B\x9Ca\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xB5W`\0\x80\xFD[a\x0B\xC1\x85\x83\x86\x01a\x08\x1AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B\xD7W`\0\x80\xFD[a\x0B\xE3\x85\x83\x86\x01a\x08\x1AV[` \x84\x01Ra\x0B\xF4`@\x85\x01a\x08\x89V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\rW`\0\x80\xFD[a\x0C\x19\x85\x83\x86\x01a\x08\x1AV[``\x84\x01Ra\x0C*`\x80\x85\x01a\x08\x89V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0CCW`\0\x80\xFD[Pa\x0CP\x84\x82\x85\x01a\n\xF3V[`\xA0\x83\x01RPa\x0Cb`\xC0\x83\x01a\x08\x89V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0C\x7FW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x95W`\0\x80\xFD[a\n\xC8\x84\x82\x85\x01a\x0B\x82V[`\0` \x80\x83\x85\x03\x12\x15a\x0C\xB4W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0C\xCBW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0C\xE1W`\0\x80\xFD[a\x0C\xE9a\x07\xA5V[\x835\x83\x81\x11\x15a\x0C\xF8W`\0\x80\xFD[a\r\x04\x89\x82\x87\x01a\x0B\x82V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\r\x18W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\r-W`\0\x80\xFD[\x835a\r;a\x0B\x14\x82a\n\xD0V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\rZW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\r\xEAW\x805\x87\x81\x11\x15a\rvW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\r\x8CW`\0\x80\x81\xFD[a\r\x94a\x07\xA5V[\x8A\x82\x015\x89\x81\x11\x15a\r\xA6W`\0\x80\x81\xFD[a\r\xB4\x8F\x8D\x83\x86\x01\x01a\x08\x1AV[\x82RP\x87\x82\x015\x89\x81\x11\x15a\r\xC9W`\0\x80\x81\xFD[a\r\xD7\x8F\x8D\x83\x86\x01\x01a\x08\x1AV[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\r^V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x0E\x10W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E&W`\0\x80\xFD[\x82\x01`\xE0\x81\x85\x03\x12\x15a\n\x8DW`\0\x80\xFD[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0EOW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0EiW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E~W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0E\xC1W`\0\x80\xFD[\x83\x86\x11\x15a\x0E\xCEW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E\xEDW`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0F\x0FWa\x0F\x0Fa\x07gV[`@Ra\x0F\x1B\x83a\n[V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x0FBW`\0\x80\xFD[\x815` a\x0FRa\x0B\x14\x83a\n\xD0V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0FqW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0BwW\x805\x83R\x91\x83\x01\x91\x83\x01a\x0FuV[`\0` \x82\x84\x03\x12\x15a\x0F\x9EW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0F\xB5W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xA0\x82\x86\x03\x12\x15a\x0F\xCAW`\0\x80\xFD[a\x0F\xD2a\x07\xC7V[\x825\x81R` \x83\x015` \x82\x01Ra\x0F\xEC`@\x84\x01a\n[V[`@\x82\x01Ra\x0F\xFD``\x84\x01a\n[V[``\x82\x01Ra\x10\x0E`\x80\x84\x01a\n[V[`\x80\x82\x01Ra\x10\x1F`\xA0\x84\x01a\n[V[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x10F\x81\x85\x01a\n[V[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x10^W`\0\x80\xFD[a\x10j\x88\x82\x87\x01a\x08\x1AV[\x82\x84\x01RPPa\x01@\x80\x84\x015\x81\x83\x01RPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x10\x9CW`\0\x80\xFD[a\x10\xA8\x88\x82\x87\x01a\x0F1V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0[\x83\x81\x10\x15a\x10\xD4W\x81\x81\x01Q\x83\x82\x01R` \x01a\x10\xBCV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x10\xF5\x81` \x86\x01` \x86\x01a\x10\xB9V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x119W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\x1DV[P\x94\x95\x94PPPPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x11u``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x11\xEA\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xA0a\x01@\x81\x81\x86\x01Ra\x12\na\x01\xC0\x86\x01\x84a\x10\xDDV[\x90\x86\x01Qa\x01`\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\x80\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x12@\x83\x82a\x11\tV[\x96\x95PPPPPPV[hPOLKADOT-`\xB8\x1B\x81R`\0\x82Qa\x12n\x81`\t\x85\x01` \x87\x01a\x10\xB9V[\x91\x90\x91\x01`\t\x01\x92\x91PPV\xFE\xA2dipfsX\"\x12 e\xF0\xFD\xA5\xDC7\x10\x14\xA7&\x0F>\xD3\x03\xD9\x84A@\xB4\xF6C\xDF\xB2}s\xB0\xAE\r\"\xEEY\x87dsolcC\0\x08\x11\x003";
    /// The bytecode of the contract.
    pub static HOSTMANAGER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\x88W`\x005`\xE0\x1C\x80c\xCF\xF0\xAB\x96\x11a\0[W\x80c\xCF\xF0\xAB\x96\x14a\0\xDBW\x80c\xDE\xAET\xF5\x14a\x01`W\x80c\xEA\xEE\x1C\xAA\x14a\x01nW\x80c\xFE\xFF\x7F\xA8\x14a\0\x8DW`\0\x80\xFD[\x80c\x0B\xC3{\xAB\x14a\0\x8DW\x80c\x0E\x83$\xA2\x14a\0\xA2W\x80c\xBC\r\xD4G\x14a\0\xB5W\x80c\xC4\x92\xE4&\x14a\0\xC8W[`\0\x80\xFD[a\0\xA0a\0\x9B6`\x04a\t\xA4V[a\x01\x81V[\0[a\0\xA0a\0\xB06`\x04a\nrV[a\x01\xDDV[a\0\xA0a\0\xC36`\x04a\n\x94V[a\x02`V[a\0\xA0a\0\xD66`\x04a\x0CmV[a\x02\xB6V[a\x01,`@\x80Q``\x81\x01\x82R`\0\x80\x82R` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91RP`@\x80Q``\x81\x01\x82R`\0T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R`\x01T` \x83\x01R`\x02T\x16\x91\x81\x01\x91\x90\x91R\x90V[`@\x80Q\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x82R` \x80\x85\x01Q\x90\x83\x01R\x92\x82\x01Q\x90\x92\x16\x90\x82\x01R``\x01`@Q\x80\x91\x03\x90\xF3[a\0\xA0a\0\xD66`\x04a\x0C\xA1V[a\0\xA0a\x01|6`\x04a\r\xFEV[a\x03\nV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`&`$\x82\x01R\x7FIsmpModule doesn't emit Post res`D\x82\x01Reponses`\xD0\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x027W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xD4V[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x90\x92\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90U`\0\x80T\x90\x91\x16\x90UV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`%`$\x82\x01R\x7FIsmpModule doesn't emit Post req`D\x82\x01Rduests`\xD8\x1B`d\x82\x01R`\x84\x01a\x01\xD4V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`$\x80\x82\x01R\x7FIsmpModule doesn't emit Get requ`D\x82\x01Rcests`\xE0\x1B`d\x82\x01R`\x84\x01a\x01\xD4V[`\x02T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x03dW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01\x81\x90R`$\x82\x01R\x7FHostManager: Unauthorized action`D\x82\x01R`d\x01a\x01\xD4V[a\x03\xBCa\x03u`\0`\x01\x01Ta\x05\x9FV[a\x03\x7F\x83\x80a\x0E8V[\x80\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x93\x92\x91\x90\x81\x81R` \x01\x83\x83\x80\x82\x847`\0\x92\x01\x91\x90\x91RP\x92\x93\x92PPa\x05\xD0\x90PV[a\x03\xFFW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15[\x98]]\x1A\x1B\xDC\x9A^\x99Y\x08\x1C\x99\\]Y\\\xDD`b\x1B`D\x82\x01R`d\x01a\x01\xD4V[`\0a\x04\x0E`\xC0\x83\x01\x83a\x0E8V[`\0\x81\x81\x10a\x04\x1FWa\x04\x1Fa\x0E\x85V[\x91\x90\x91\x015`\xF8\x1C\x90P`\x01\x81\x11\x15a\x04:Wa\x04:a\x0E\x9BV[\x90P`\0\x81`\x01\x81\x11\x15a\x04PWa\x04Pa\x0E\x9BV[\x03a\x04\xF2W`\0a\x04d`\xC0\x84\x01\x84a\x0E8V[a\x04r\x91`\x01\x90\x82\x90a\x0E\xB1V[\x81\x01\x90a\x04\x7F\x91\x90a\x0E\xDBV[`\x02T`@Qc<VT\x17`\xE0\x1B\x81R\x82Q`\x01`\x01`\xA0\x1B\x03\x90\x81\x16`\x04\x83\x01R` \x84\x01Q`$\x83\x01R\x92\x93P\x91\x16\x90c<VT\x17\x90`D\x01[`\0`@Q\x80\x83\x03\x81`\0\x87\x80;\x15\x80\x15a\x04\xD5W`\0\x80\xFD[PZ\xF1\x15\x80\x15a\x04\xE9W=`\0\x80>=`\0\xFD[PPPPPPPV[`\x01\x81`\x01\x81\x11\x15a\x05\x06Wa\x05\x06a\x0E\x9BV[\x03a\x05fW`\0a\x05\x1A`\xC0\x84\x01\x84a\x0E8V[a\x05(\x91`\x01\x90\x82\x90a\x0E\xB1V[\x81\x01\x90a\x055\x91\x90a\x0F\x8CV[`\x02T`@Qc\x03G\x98{`\xE6\x1B\x81R\x91\x92P`\x01`\x01`\xA0\x1B\x03\x16\x90c\xD1\xE6\x1E\xC0\x90a\x04\xBB\x90\x84\x90`\x04\x01a\x11DV[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0E`$\x82\x01Rm*\xB75\xB77\xBB\xB7\x100\xB1\xBA4\xB7\xB7`\x91\x1B`D\x82\x01R`d\x01a\x01\xD4V[``a\x05\xAA\x82a\x05\xFDV[`@Q` \x01a\x05\xBA\x91\x90a\x12JV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x91\x90PV[`\0\x81Q\x83Q\x14a\x05\xE3WP`\0a\x05\xF7V[P\x81Q` \x82\x81\x01\x82\x90 \x90\x84\x01\x91\x90\x91 \x14[\x92\x91PPV[```\0a\x06\n\x83a\x06\x8FV[`\x01\x01\x90P`\0\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a\x06)Wa\x06)a\x07gV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x06SW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P\x81\x81\x01` \x01[`\0\x19\x01o\x18\x18\x99\x19\x9A\x1A\x9B\x1B\x9C\x1C\xB0\xB11\xB22\xB3`\x81\x1B`\n\x86\x06\x1A\x81S`\n\x85\x04\x94P\x84a\x06]WP\x93\x92PPPV[`\0\x80r\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x10a\x06\xCEWr\x18O\x03\xE9?\xF9\xF4\xDA\xA7\x97\xEDn8\xEDd\xBFj\x1F\x01`@\x1B\x83\x04\x92P`@\x01[m\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x10a\x06\xFAWm\x04\xEE-mA[\x85\xAC\xEF\x81\0\0\0\0\x83\x04\x92P` \x01[f#\x86\xF2o\xC1\0\0\x83\x10a\x07\x18Wf#\x86\xF2o\xC1\0\0\x83\x04\x92P`\x10\x01[c\x05\xF5\xE1\0\x83\x10a\x070Wc\x05\xF5\xE1\0\x83\x04\x92P`\x08\x01[a'\x10\x83\x10a\x07DWa'\x10\x83\x04\x92P`\x04\x01[`d\x83\x10a\x07VW`d\x83\x04\x92P`\x02\x01[`\n\x83\x10a\x05\xF7W`\x01\x01\x92\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`@Q`\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@R\x90V[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Qa\x01\xA0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x07\x9FWa\x07\x9Fa\x07gV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x08\x12Wa\x08\x12a\x07gV[`@R\x91\x90PV[`\0\x82`\x1F\x83\x01\x12a\x08+W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x08DWa\x08Da\x07gV[a\x08W`\x1F\x82\x01`\x1F\x19\x16` \x01a\x07\xEAV[\x81\x81R\x84` \x83\x86\x01\x01\x11\x15a\x08lW`\0\x80\xFD[\x81` \x85\x01` \x83\x017`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x08\xA0W`\0\x80\xFD[\x91\x90PV[`\0`\xE0\x82\x84\x03\x12\x15a\x08\xB7W`\0\x80\xFD[a\x08\xBFa\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x08\xD8W`\0\x80\xFD[a\x08\xE4\x85\x83\x86\x01a\x08\x1AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x08\xFAW`\0\x80\xFD[a\t\x06\x85\x83\x86\x01a\x08\x1AV[` \x84\x01Ra\t\x17`@\x85\x01a\x08\x89V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\t0W`\0\x80\xFD[a\t<\x85\x83\x86\x01a\x08\x1AV[``\x84\x01R`\x80\x84\x015\x91P\x80\x82\x11\x15a\tUW`\0\x80\xFD[a\ta\x85\x83\x86\x01a\x08\x1AV[`\x80\x84\x01Ra\tr`\xA0\x85\x01a\x08\x89V[`\xA0\x84\x01R`\xC0\x84\x015\x91P\x80\x82\x11\x15a\t\x8BW`\0\x80\xFD[Pa\t\x98\x84\x82\x85\x01a\x08\x1AV[`\xC0\x83\x01RP\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\t\xB6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\t\xCDW`\0\x80\xFD[\x90\x83\x01\x90``\x82\x86\x03\x12\x15a\t\xE1W`\0\x80\xFD[`@Q``\x81\x01\x81\x81\x10\x83\x82\x11\x17\x15a\t\xFCWa\t\xFCa\x07gV[`@R\x825\x82\x81\x11\x15a\n\x0EW`\0\x80\xFD[a\n\x1A\x87\x82\x86\x01a\x08\xA5V[\x82RP` \x83\x015\x82\x81\x11\x15a\n/W`\0\x80\xFD[a\n;\x87\x82\x86\x01a\x08\x1AV[` \x83\x01RPa\nM`@\x84\x01a\x08\x89V[`@\x82\x01R\x95\x94PPPPPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xA0W`\0\x80\xFD[`\0` \x82\x84\x03\x12\x15a\n\x84W`\0\x80\xFD[a\n\x8D\x82a\n[V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\xA6W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\n\xBCW`\0\x80\xFD[a\n\xC8\x84\x82\x85\x01a\x08\xA5V[\x94\x93PPPPV[`\0`\x01`\x01`@\x1B\x03\x82\x11\x15a\n\xE9Wa\n\xE9a\x07gV[P`\x05\x1B` \x01\x90V[`\0\x82`\x1F\x83\x01\x12a\x0B\x04W`\0\x80\xFD[\x815` a\x0B\x19a\x0B\x14\x83a\n\xD0V[a\x07\xEAV[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0B8W`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0BwW\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0B[W`\0\x80\x81\xFD[a\x0Bi\x89\x86\x83\x8B\x01\x01a\x08\x1AV[\x84RP\x91\x83\x01\x91\x83\x01a\x0B<V[P\x96\x95PPPPPPV[`\0`\xE0\x82\x84\x03\x12\x15a\x0B\x94W`\0\x80\xFD[a\x0B\x9Ca\x07}V[\x90P\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0B\xB5W`\0\x80\xFD[a\x0B\xC1\x85\x83\x86\x01a\x08\x1AV[\x83R` \x84\x015\x91P\x80\x82\x11\x15a\x0B\xD7W`\0\x80\xFD[a\x0B\xE3\x85\x83\x86\x01a\x08\x1AV[` \x84\x01Ra\x0B\xF4`@\x85\x01a\x08\x89V[`@\x84\x01R``\x84\x015\x91P\x80\x82\x11\x15a\x0C\rW`\0\x80\xFD[a\x0C\x19\x85\x83\x86\x01a\x08\x1AV[``\x84\x01Ra\x0C*`\x80\x85\x01a\x08\x89V[`\x80\x84\x01R`\xA0\x84\x015\x91P\x80\x82\x11\x15a\x0CCW`\0\x80\xFD[Pa\x0CP\x84\x82\x85\x01a\n\xF3V[`\xA0\x83\x01RPa\x0Cb`\xC0\x83\x01a\x08\x89V[`\xC0\x82\x01R\x92\x91PPV[`\0` \x82\x84\x03\x12\x15a\x0C\x7FW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0C\x95W`\0\x80\xFD[a\n\xC8\x84\x82\x85\x01a\x0B\x82V[`\0` \x80\x83\x85\x03\x12\x15a\x0C\xB4W`\0\x80\xFD[\x825`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0C\xCBW`\0\x80\xFD[\x81\x85\x01\x91P`@\x80\x83\x88\x03\x12\x15a\x0C\xE1W`\0\x80\xFD[a\x0C\xE9a\x07\xA5V[\x835\x83\x81\x11\x15a\x0C\xF8W`\0\x80\xFD[a\r\x04\x89\x82\x87\x01a\x0B\x82V[\x82RP\x84\x84\x015\x83\x81\x11\x15a\r\x18W`\0\x80\xFD[\x80\x85\x01\x94PP\x87`\x1F\x85\x01\x12a\r-W`\0\x80\xFD[\x835a\r;a\x0B\x14\x82a\n\xD0V[\x81\x81R`\x05\x91\x90\x91\x1B\x85\x01\x86\x01\x90\x86\x81\x01\x90\x8A\x83\x11\x15a\rZW`\0\x80\xFD[\x87\x87\x01[\x83\x81\x10\x15a\r\xEAW\x805\x87\x81\x11\x15a\rvW`\0\x80\x81\xFD[\x88\x01\x80\x8D\x03`\x1F\x19\x01\x87\x13\x15a\r\x8CW`\0\x80\x81\xFD[a\r\x94a\x07\xA5V[\x8A\x82\x015\x89\x81\x11\x15a\r\xA6W`\0\x80\x81\xFD[a\r\xB4\x8F\x8D\x83\x86\x01\x01a\x08\x1AV[\x82RP\x87\x82\x015\x89\x81\x11\x15a\r\xC9W`\0\x80\x81\xFD[a\r\xD7\x8F\x8D\x83\x86\x01\x01a\x08\x1AV[\x82\x8D\x01RP\x84RP\x91\x88\x01\x91\x88\x01a\r^V[P\x96\x83\x01\x96\x90\x96RP\x97\x96PPPPPPPV[`\0` \x82\x84\x03\x12\x15a\x0E\x10W`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a\x0E&W`\0\x80\xFD[\x82\x01`\xE0\x81\x85\x03\x12\x15a\n\x8DW`\0\x80\xFD[`\0\x80\x835`\x1E\x19\x846\x03\x01\x81\x12a\x0EOW`\0\x80\xFD[\x83\x01\x805\x91P`\x01`\x01`@\x1B\x03\x82\x11\x15a\x0EiW`\0\x80\xFD[` \x01\x91P6\x81\x90\x03\x82\x13\x15a\x0E~W`\0\x80\xFD[\x92P\x92\x90PV[cNH{q`\xE0\x1B`\0R`2`\x04R`$`\0\xFD[cNH{q`\xE0\x1B`\0R`!`\x04R`$`\0\xFD[`\0\x80\x85\x85\x11\x15a\x0E\xC1W`\0\x80\xFD[\x83\x86\x11\x15a\x0E\xCEW`\0\x80\xFD[PP\x82\x01\x93\x91\x90\x92\x03\x91PV[`\0`@\x82\x84\x03\x12\x15a\x0E\xEDW`\0\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a\x0F\x0FWa\x0F\x0Fa\x07gV[`@Ra\x0F\x1B\x83a\n[V[\x81R` \x83\x015` \x82\x01R\x80\x91PP\x92\x91PPV[`\0\x82`\x1F\x83\x01\x12a\x0FBW`\0\x80\xFD[\x815` a\x0FRa\x0B\x14\x83a\n\xD0V[\x82\x81R`\x05\x92\x90\x92\x1B\x84\x01\x81\x01\x91\x81\x81\x01\x90\x86\x84\x11\x15a\x0FqW`\0\x80\xFD[\x82\x86\x01[\x84\x81\x10\x15a\x0BwW\x805\x83R\x91\x83\x01\x91\x83\x01a\x0FuV[`\0` \x82\x84\x03\x12\x15a\x0F\x9EW`\0\x80\xFD[\x815`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\x0F\xB5W`\0\x80\xFD[\x90\x83\x01\x90a\x01\xA0\x82\x86\x03\x12\x15a\x0F\xCAW`\0\x80\xFD[a\x0F\xD2a\x07\xC7V[\x825\x81R` \x83\x015` \x82\x01Ra\x0F\xEC`@\x84\x01a\n[V[`@\x82\x01Ra\x0F\xFD``\x84\x01a\n[V[``\x82\x01Ra\x10\x0E`\x80\x84\x01a\n[V[`\x80\x82\x01Ra\x10\x1F`\xA0\x84\x01a\n[V[`\xA0\x82\x01R`\xC0\x83\x015`\xC0\x82\x01R`\xE0\x83\x015`\xE0\x82\x01Ra\x01\0a\x10F\x81\x85\x01a\n[V[\x90\x82\x01Ra\x01 \x83\x81\x015\x83\x81\x11\x15a\x10^W`\0\x80\xFD[a\x10j\x88\x82\x87\x01a\x08\x1AV[\x82\x84\x01RPPa\x01@\x80\x84\x015\x81\x83\x01RPa\x01`\x80\x84\x015\x81\x83\x01RPa\x01\x80\x80\x84\x015\x83\x81\x11\x15a\x10\x9CW`\0\x80\xFD[a\x10\xA8\x88\x82\x87\x01a\x0F1V[\x91\x83\x01\x91\x90\x91RP\x95\x94PPPPPV[`\0[\x83\x81\x10\x15a\x10\xD4W\x81\x81\x01Q\x83\x82\x01R` \x01a\x10\xBCV[PP`\0\x91\x01RV[`\0\x81Q\x80\x84Ra\x10\xF5\x81` \x86\x01` \x86\x01a\x10\xB9V[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[`\0\x81Q\x80\x84R` \x80\x85\x01\x94P\x80\x84\x01`\0[\x83\x81\x10\x15a\x119W\x81Q\x87R\x95\x82\x01\x95\x90\x82\x01\x90`\x01\x01a\x11\x1DV[P\x94\x95\x94PPPPPV[` \x81R\x81Q` \x82\x01R` \x82\x01Q`@\x82\x01R`\0`@\x83\x01Qa\x11u``\x84\x01\x82`\x01`\x01`\xA0\x1B\x03\x16\x90RV[P``\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\x80\x84\x01RP`\x80\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xA0\x84\x01RP`\xA0\x83\x01Q`\x01`\x01`\xA0\x1B\x03\x81\x16`\xC0\x84\x01RP`\xC0\x83\x01Q`\xE0\x83\x01R`\xE0\x83\x01Qa\x01\0\x81\x81\x85\x01R\x80\x85\x01Q\x91PPa\x01 a\x11\xEA\x81\x85\x01\x83`\x01`\x01`\xA0\x1B\x03\x16\x90RV[\x80\x85\x01Q\x91PPa\x01\xA0a\x01@\x81\x81\x86\x01Ra\x12\na\x01\xC0\x86\x01\x84a\x10\xDDV[\x90\x86\x01Qa\x01`\x86\x81\x01\x91\x90\x91R\x86\x01Qa\x01\x80\x80\x87\x01\x91\x90\x91R\x86\x01Q\x85\x82\x03`\x1F\x19\x01\x83\x87\x01R\x90\x92Pa\x12@\x83\x82a\x11\tV[\x96\x95PPPPPPV[hPOLKADOT-`\xB8\x1B\x81R`\0\x82Qa\x12n\x81`\t\x85\x01` \x87\x01a\x10\xB9V[\x91\x90\x91\x01`\t\x01\x92\x91PPV\xFE\xA2dipfsX\"\x12 e\xF0\xFD\xA5\xDC7\x10\x14\xA7&\x0F>\xD3\x03\xD9\x84A@\xB4\xF6C\xDF\xB2}s\xB0\xAE\r\"\xEEY\x87dsolcC\0\x08\x11\x003";
    /// The deployed bytecode of the contract.
    pub static HOSTMANAGER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
    pub struct HostManager<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for HostManager<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for HostManager<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for HostManager<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for HostManager<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(HostManager))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> HostManager<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    HOSTMANAGER_ABI.clone(),
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
                HOSTMANAGER_ABI.clone(),
                HOSTMANAGER_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        ///Calls the contract's `onAccept` (0xeaee1caa) function
        pub fn on_accept(
            &self,
            request: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([234, 238, 28, 170], (request,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetResponse` (0xdeae54f5) function
        pub fn on_get_response(
            &self,
            p0: GetResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([222, 174, 84, 245], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onGetTimeout` (0xc492e426) function
        pub fn on_get_timeout(
            &self,
            p0: GetRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([196, 146, 228, 38], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostRequestTimeout` (0xbc0dd447) function
        pub fn on_post_request_timeout(
            &self,
            p0: PostRequest,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([188, 13, 212, 71], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostResponse` (0xfeff7fa8) function
        pub fn on_post_response(
            &self,
            p0: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([254, 255, 127, 168], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `onPostResponseTimeout` (0x0bc37bab) function
        pub fn on_post_response_timeout(
            &self,
            p0: PostResponse,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([11, 195, 123, 171], (p0,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `params` (0xcff0ab96) function
        pub fn params(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, HostManagerParams> {
            self.0
                .method_hash([207, 240, 171, 150], ())
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
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for HostManager<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Container type for all input parameters for the `onAccept` function with signature `onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes))` and selector `0xeaee1caa`
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
        abi = "onAccept((bytes,bytes,uint64,bytes,bytes,uint64,bytes))"
    )]
    pub struct OnAcceptCall {
        pub request: PostRequest,
    }
    ///Container type for all input parameters for the `onGetResponse` function with signature `onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]))` and selector `0xdeae54f5`
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
        abi = "onGetResponse(((bytes,bytes,uint64,bytes,uint64,bytes[],uint64),(bytes,bytes)[]))"
    )]
    pub struct OnGetResponseCall(pub GetResponse);
    ///Container type for all input parameters for the `onGetTimeout` function with signature `onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))` and selector `0xc492e426`
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
        abi = "onGetTimeout((bytes,bytes,uint64,bytes,uint64,bytes[],uint64))"
    )]
    pub struct OnGetTimeoutCall(pub GetRequest);
    ///Container type for all input parameters for the `onPostRequestTimeout` function with signature `onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))` and selector `0xbc0dd447`
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
        name = "onPostRequestTimeout",
        abi = "onPostRequestTimeout((bytes,bytes,uint64,bytes,bytes,uint64,bytes))"
    )]
    pub struct OnPostRequestTimeoutCall(pub PostRequest);
    ///Container type for all input parameters for the `onPostResponse` function with signature `onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))` and selector `0xfeff7fa8`
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
        abi = "onPostResponse(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))"
    )]
    pub struct OnPostResponseCall(pub PostResponse);
    ///Container type for all input parameters for the `onPostResponseTimeout` function with signature `onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))` and selector `0x0bc37bab`
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
        name = "onPostResponseTimeout",
        abi = "onPostResponseTimeout(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64))"
    )]
    pub struct OnPostResponseTimeoutCall(pub PostResponse);
    ///Container type for all input parameters for the `params` function with signature `params()` and selector `0xcff0ab96`
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
    #[ethcall(name = "params", abi = "params()")]
    pub struct ParamsCall;
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
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum HostManagerCalls {
        OnAccept(OnAcceptCall),
        OnGetResponse(OnGetResponseCall),
        OnGetTimeout(OnGetTimeoutCall),
        OnPostRequestTimeout(OnPostRequestTimeoutCall),
        OnPostResponse(OnPostResponseCall),
        OnPostResponseTimeout(OnPostResponseTimeoutCall),
        Params(ParamsCall),
        SetIsmpHost(SetIsmpHostCall),
    }
    impl ::ethers::core::abi::AbiDecode for HostManagerCalls {
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
            if let Ok(decoded) = <OnPostRequestTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostRequestTimeout(decoded));
            }
            if let Ok(decoded) = <OnPostResponseCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostResponse(decoded));
            }
            if let Ok(decoded) = <OnPostResponseTimeoutCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OnPostResponseTimeout(decoded));
            }
            if let Ok(decoded) = <ParamsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Params(decoded));
            }
            if let Ok(decoded) = <SetIsmpHostCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetIsmpHost(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for HostManagerCalls {
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
                Self::OnPostRequestTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostResponse(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OnPostResponseTimeout(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Params(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::SetIsmpHost(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for HostManagerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::OnAccept(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnGetTimeout(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostRequestTimeout(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::OnPostResponse(element) => ::core::fmt::Display::fmt(element, f),
                Self::OnPostResponseTimeout(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::Params(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIsmpHost(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<OnAcceptCall> for HostManagerCalls {
        fn from(value: OnAcceptCall) -> Self {
            Self::OnAccept(value)
        }
    }
    impl ::core::convert::From<OnGetResponseCall> for HostManagerCalls {
        fn from(value: OnGetResponseCall) -> Self {
            Self::OnGetResponse(value)
        }
    }
    impl ::core::convert::From<OnGetTimeoutCall> for HostManagerCalls {
        fn from(value: OnGetTimeoutCall) -> Self {
            Self::OnGetTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostRequestTimeoutCall> for HostManagerCalls {
        fn from(value: OnPostRequestTimeoutCall) -> Self {
            Self::OnPostRequestTimeout(value)
        }
    }
    impl ::core::convert::From<OnPostResponseCall> for HostManagerCalls {
        fn from(value: OnPostResponseCall) -> Self {
            Self::OnPostResponse(value)
        }
    }
    impl ::core::convert::From<OnPostResponseTimeoutCall> for HostManagerCalls {
        fn from(value: OnPostResponseTimeoutCall) -> Self {
            Self::OnPostResponseTimeout(value)
        }
    }
    impl ::core::convert::From<ParamsCall> for HostManagerCalls {
        fn from(value: ParamsCall) -> Self {
            Self::Params(value)
        }
    }
    impl ::core::convert::From<SetIsmpHostCall> for HostManagerCalls {
        fn from(value: SetIsmpHostCall) -> Self {
            Self::SetIsmpHost(value)
        }
    }
    ///Container type for all return fields from the `params` function with signature `params()` and selector `0xcff0ab96`
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
    pub struct ParamsReturn(pub HostManagerParams);
    ///`HostManagerParams(address,uint256,address)`
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
    pub struct HostManagerParams {
        pub admin: ::ethers::core::types::Address,
        pub governor_state_machine_id: ::ethers::core::types::U256,
        pub host: ::ethers::core::types::Address,
    }
}
