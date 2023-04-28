#[allow(dead_code, unused_imports, non_camel_case_types)]
#[allow(clippy::all)]
pub mod api {
    #[allow(unused_imports)]
    mod root_mod {
        pub use super::*;
    }
    pub static PALLETS: [&str; 19usize] = [
        "System",
        "Timestamp",
        "ParachainSystem",
        "ParachainInfo",
        "Balances",
        "TransactionPayment",
        "Authorship",
        "CollatorSelection",
        "Session",
        "Aura",
        "AuraExt",
        "Sudo",
        "XcmpQueue",
        "PolkadotXcm",
        "CumulusXcm",
        "DmpQueue",
        "Ismp",
        "IsmpParachain",
        "IsmpAssets",
    ];
    /// The error type returned when there is a runtime issue.
    pub type DispatchError = runtime_types::sp_runtime::DispatchError;
    #[derive(
        ::subxt::ext::codec::Decode,
        ::subxt::ext::codec::Encode,
        ::subxt::ext::scale_decode::DecodeAsType,
        ::subxt::ext::scale_encode::EncodeAsType,
        Debug,
    )]
    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
    pub enum Event {
        #[codec(index = 0)]
        System(system::Event),
        #[codec(index = 2)]
        ParachainSystem(parachain_system::Event),
        #[codec(index = 10)]
        Balances(balances::Event),
        #[codec(index = 11)]
        TransactionPayment(transaction_payment::Event),
        #[codec(index = 21)]
        CollatorSelection(collator_selection::Event),
        #[codec(index = 22)]
        Session(session::Event),
        #[codec(index = 25)]
        Sudo(sudo::Event),
        #[codec(index = 30)]
        XcmpQueue(xcmp_queue::Event),
        #[codec(index = 31)]
        PolkadotXcm(polkadot_xcm::Event),
        #[codec(index = 32)]
        CumulusXcm(cumulus_xcm::Event),
        #[codec(index = 33)]
        DmpQueue(dmp_queue::Event),
        #[codec(index = 40)]
        Ismp(ismp::Event),
        #[codec(index = 41)]
        IsmpParachain(ismp_parachain::Event),
        #[codec(index = 42)]
        IsmpAssets(ismp_assets::Event),
    }
    impl ::subxt::events::RootEvent for Event {
        fn root_event(
            pallet_bytes: &[u8],
            pallet_name: &str,
            pallet_ty: u32,
            metadata: &::subxt::Metadata,
        ) -> Result<Self, ::subxt::Error> {
            use ::subxt::metadata::DecodeWithMetadata;
            if pallet_name == "System" {
                return Ok(Event::System(system::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "ParachainSystem" {
                return Ok(Event::ParachainSystem(parachain_system::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "Balances" {
                return Ok(Event::Balances(balances::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "TransactionPayment" {
                return Ok(Event::TransactionPayment(
                    transaction_payment::Event::decode_with_metadata(
                        &mut &*pallet_bytes,
                        pallet_ty,
                        metadata,
                    )?,
                ))
            }
            if pallet_name == "CollatorSelection" {
                return Ok(Event::CollatorSelection(
                    collator_selection::Event::decode_with_metadata(
                        &mut &*pallet_bytes,
                        pallet_ty,
                        metadata,
                    )?,
                ))
            }
            if pallet_name == "Session" {
                return Ok(Event::Session(session::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "Sudo" {
                return Ok(Event::Sudo(sudo::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "XcmpQueue" {
                return Ok(Event::XcmpQueue(xcmp_queue::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "PolkadotXcm" {
                return Ok(Event::PolkadotXcm(polkadot_xcm::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "CumulusXcm" {
                return Ok(Event::CumulusXcm(cumulus_xcm::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "DmpQueue" {
                return Ok(Event::DmpQueue(dmp_queue::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "Ismp" {
                return Ok(Event::Ismp(ismp::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "IsmpParachain" {
                return Ok(Event::IsmpParachain(ismp_parachain::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            if pallet_name == "IsmpAssets" {
                return Ok(Event::IsmpAssets(ismp_assets::Event::decode_with_metadata(
                    &mut &*pallet_bytes,
                    pallet_ty,
                    metadata,
                )?))
            }
            Err(::subxt::ext::scale_decode::Error::custom(format!(
                "Pallet name '{}' not found in root Event enum",
                pallet_name
            ))
            .into())
        }
    }
    pub fn constants() -> ConstantsApi {
        ConstantsApi
    }
    pub fn storage() -> StorageApi {
        StorageApi
    }
    pub fn tx() -> TransactionApi {
        TransactionApi
    }
    pub struct ConstantsApi;
    impl ConstantsApi {
        pub fn system(&self) -> system::constants::ConstantsApi {
            system::constants::ConstantsApi
        }
        pub fn timestamp(&self) -> timestamp::constants::ConstantsApi {
            timestamp::constants::ConstantsApi
        }
        pub fn balances(&self) -> balances::constants::ConstantsApi {
            balances::constants::ConstantsApi
        }
        pub fn transaction_payment(&self) -> transaction_payment::constants::ConstantsApi {
            transaction_payment::constants::ConstantsApi
        }
    }
    pub struct StorageApi;
    impl StorageApi {
        pub fn system(&self) -> system::storage::StorageApi {
            system::storage::StorageApi
        }
        pub fn timestamp(&self) -> timestamp::storage::StorageApi {
            timestamp::storage::StorageApi
        }
        pub fn parachain_system(&self) -> parachain_system::storage::StorageApi {
            parachain_system::storage::StorageApi
        }
        pub fn parachain_info(&self) -> parachain_info::storage::StorageApi {
            parachain_info::storage::StorageApi
        }
        pub fn balances(&self) -> balances::storage::StorageApi {
            balances::storage::StorageApi
        }
        pub fn transaction_payment(&self) -> transaction_payment::storage::StorageApi {
            transaction_payment::storage::StorageApi
        }
        pub fn authorship(&self) -> authorship::storage::StorageApi {
            authorship::storage::StorageApi
        }
        pub fn collator_selection(&self) -> collator_selection::storage::StorageApi {
            collator_selection::storage::StorageApi
        }
        pub fn session(&self) -> session::storage::StorageApi {
            session::storage::StorageApi
        }
        pub fn aura(&self) -> aura::storage::StorageApi {
            aura::storage::StorageApi
        }
        pub fn aura_ext(&self) -> aura_ext::storage::StorageApi {
            aura_ext::storage::StorageApi
        }
        pub fn sudo(&self) -> sudo::storage::StorageApi {
            sudo::storage::StorageApi
        }
        pub fn xcmp_queue(&self) -> xcmp_queue::storage::StorageApi {
            xcmp_queue::storage::StorageApi
        }
        pub fn dmp_queue(&self) -> dmp_queue::storage::StorageApi {
            dmp_queue::storage::StorageApi
        }
        pub fn ismp(&self) -> ismp::storage::StorageApi {
            ismp::storage::StorageApi
        }
        pub fn ismp_parachain(&self) -> ismp_parachain::storage::StorageApi {
            ismp_parachain::storage::StorageApi
        }
        pub fn ismp_assets(&self) -> ismp_assets::storage::StorageApi {
            ismp_assets::storage::StorageApi
        }
    }
    pub struct TransactionApi;
    impl TransactionApi {
        pub fn system(&self) -> system::calls::TransactionApi {
            system::calls::TransactionApi
        }
        pub fn timestamp(&self) -> timestamp::calls::TransactionApi {
            timestamp::calls::TransactionApi
        }
        pub fn parachain_system(&self) -> parachain_system::calls::TransactionApi {
            parachain_system::calls::TransactionApi
        }
        pub fn balances(&self) -> balances::calls::TransactionApi {
            balances::calls::TransactionApi
        }
        pub fn collator_selection(&self) -> collator_selection::calls::TransactionApi {
            collator_selection::calls::TransactionApi
        }
        pub fn session(&self) -> session::calls::TransactionApi {
            session::calls::TransactionApi
        }
        pub fn sudo(&self) -> sudo::calls::TransactionApi {
            sudo::calls::TransactionApi
        }
        pub fn xcmp_queue(&self) -> xcmp_queue::calls::TransactionApi {
            xcmp_queue::calls::TransactionApi
        }
        pub fn polkadot_xcm(&self) -> polkadot_xcm::calls::TransactionApi {
            polkadot_xcm::calls::TransactionApi
        }
        pub fn dmp_queue(&self) -> dmp_queue::calls::TransactionApi {
            dmp_queue::calls::TransactionApi
        }
        pub fn ismp(&self) -> ismp::calls::TransactionApi {
            ismp::calls::TransactionApi
        }
        pub fn ismp_assets(&self) -> ismp_assets::calls::TransactionApi {
            ismp_assets::calls::TransactionApi
        }
    }
    /// check whether the Client you are using is aligned with the statically generated codegen.
    pub fn validate_codegen<T: ::subxt::Config, C: ::subxt::client::OfflineClientT<T>>(
        client: &C,
    ) -> Result<(), ::subxt::error::MetadataError> {
        let runtime_metadata_hash = client.metadata().metadata_hash(&PALLETS);
        if runtime_metadata_hash !=
            [
                20u8, 80u8, 226u8, 103u8, 32u8, 34u8, 220u8, 4u8, 11u8, 138u8, 154u8, 34u8, 20u8,
                34u8, 32u8, 34u8, 83u8, 19u8, 27u8, 130u8, 10u8, 94u8, 159u8, 12u8, 89u8, 13u8,
                13u8, 73u8, 16u8, 191u8, 22u8, 183u8,
            ]
        {
            Err(::subxt::error::MetadataError::IncompatibleMetadata)
        } else {
            Ok(())
        }
    }
    pub mod system {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Remark {
                pub remark: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetHeapPages {
                pub pages: ::core::primitive::u64,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetCode {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetCodeWithoutChecks {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetStorage {
                pub items: ::std::vec::Vec<(
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::std::vec::Vec<::core::primitive::u8>,
                )>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct KillStorage {
                pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct KillPrefix {
                pub prefix: ::std::vec::Vec<::core::primitive::u8>,
                pub subkeys: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct RemarkWithEvent {
                pub remark: ::std::vec::Vec<::core::primitive::u8>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Make some on-chain remark.
                ///
                ///## Complexity
                /// - `O(1)`
                pub fn remark(
                    &self,
                    remark: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<Remark> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "remark",
                        Remark { remark },
                        [
                            101u8, 80u8, 195u8, 226u8, 224u8, 247u8, 60u8, 128u8, 3u8, 101u8, 51u8,
                            147u8, 96u8, 126u8, 76u8, 230u8, 194u8, 227u8, 191u8, 73u8, 160u8,
                            146u8, 87u8, 147u8, 243u8, 28u8, 228u8, 116u8, 224u8, 181u8, 129u8,
                            160u8,
                        ],
                    )
                }
                ///Set the number of pages in the WebAssembly environment's heap.
                pub fn set_heap_pages(
                    &self,
                    pages: ::core::primitive::u64,
                ) -> ::subxt::tx::Payload<SetHeapPages> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "set_heap_pages",
                        SetHeapPages { pages },
                        [
                            43u8, 103u8, 128u8, 49u8, 156u8, 136u8, 11u8, 204u8, 80u8, 6u8, 244u8,
                            86u8, 171u8, 44u8, 140u8, 225u8, 142u8, 198u8, 43u8, 87u8, 26u8, 45u8,
                            125u8, 222u8, 165u8, 254u8, 172u8, 158u8, 39u8, 178u8, 86u8, 87u8,
                        ],
                    )
                }
                ///Set the new runtime code.
                ///
                ///## Complexity
                /// - `O(C + S)` where `C` length of `code` and `S` complexity of `can_set_code`
                pub fn set_code(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<SetCode> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "set_code",
                        SetCode { code },
                        [
                            27u8, 104u8, 244u8, 205u8, 188u8, 254u8, 121u8, 13u8, 106u8, 120u8,
                            244u8, 108u8, 97u8, 84u8, 100u8, 68u8, 26u8, 69u8, 93u8, 128u8, 107u8,
                            4u8, 3u8, 142u8, 13u8, 134u8, 196u8, 62u8, 113u8, 181u8, 14u8, 40u8,
                        ],
                    )
                }
                ///Set the new runtime code without doing any checks of the given `code`.
                ///
                ///## Complexity
                /// - `O(C)` where `C` length of `code`
                pub fn set_code_without_checks(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<SetCodeWithoutChecks> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "set_code_without_checks",
                        SetCodeWithoutChecks { code },
                        [
                            102u8, 160u8, 125u8, 235u8, 30u8, 23u8, 45u8, 239u8, 112u8, 148u8,
                            159u8, 158u8, 42u8, 93u8, 206u8, 94u8, 80u8, 250u8, 66u8, 195u8, 60u8,
                            40u8, 142u8, 169u8, 183u8, 80u8, 80u8, 96u8, 3u8, 231u8, 99u8, 216u8,
                        ],
                    )
                }
                ///Set some items of storage.
                pub fn set_storage(
                    &self,
                    items: ::std::vec::Vec<(
                        ::std::vec::Vec<::core::primitive::u8>,
                        ::std::vec::Vec<::core::primitive::u8>,
                    )>,
                ) -> ::subxt::tx::Payload<SetStorage> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "set_storage",
                        SetStorage { items },
                        [
                            74u8, 43u8, 106u8, 255u8, 50u8, 151u8, 192u8, 155u8, 14u8, 90u8, 19u8,
                            45u8, 165u8, 16u8, 235u8, 242u8, 21u8, 131u8, 33u8, 172u8, 119u8, 78u8,
                            140u8, 10u8, 107u8, 202u8, 122u8, 235u8, 181u8, 191u8, 22u8, 116u8,
                        ],
                    )
                }
                ///Kill some items from storage.
                pub fn kill_storage(
                    &self,
                    keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                ) -> ::subxt::tx::Payload<KillStorage> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "kill_storage",
                        KillStorage { keys },
                        [
                            174u8, 174u8, 13u8, 174u8, 75u8, 138u8, 128u8, 235u8, 222u8, 216u8,
                            85u8, 18u8, 198u8, 1u8, 138u8, 70u8, 19u8, 108u8, 209u8, 41u8, 228u8,
                            67u8, 130u8, 230u8, 160u8, 207u8, 11u8, 180u8, 139u8, 242u8, 41u8,
                            15u8,
                        ],
                    )
                }
                ///Kill all storage items with a key that starts with the given prefix.
                ///
                ///**NOTE:** We rely on the Root origin to provide us the number of subkeys under
                ///the prefix we are removing to accurately calculate the weight of this function.
                pub fn kill_prefix(
                    &self,
                    prefix: ::std::vec::Vec<::core::primitive::u8>,
                    subkeys: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<KillPrefix> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "kill_prefix",
                        KillPrefix { prefix, subkeys },
                        [
                            203u8, 116u8, 217u8, 42u8, 154u8, 215u8, 77u8, 217u8, 13u8, 22u8,
                            193u8, 2u8, 128u8, 115u8, 179u8, 115u8, 187u8, 218u8, 129u8, 34u8,
                            80u8, 4u8, 173u8, 120u8, 92u8, 35u8, 237u8, 112u8, 201u8, 207u8, 200u8,
                            48u8,
                        ],
                    )
                }
                ///Make some on-chain remark and emit event.
                pub fn remark_with_event(
                    &self,
                    remark: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<RemarkWithEvent> {
                    ::subxt::tx::Payload::new_static(
                        "System",
                        "remark_with_event",
                        RemarkWithEvent { remark },
                        [
                            123u8, 225u8, 180u8, 179u8, 144u8, 74u8, 27u8, 85u8, 101u8, 75u8,
                            134u8, 44u8, 181u8, 25u8, 183u8, 158u8, 14u8, 213u8, 56u8, 225u8,
                            136u8, 88u8, 26u8, 114u8, 178u8, 43u8, 176u8, 43u8, 240u8, 84u8, 116u8,
                            46u8,
                        ],
                    )
                }
            }
        }
        ///Event for the System pallet.
        pub type Event = runtime_types::frame_system::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An extrinsic completed successfully.
            pub struct ExtrinsicSuccess {
                pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
            }
            impl ::subxt::events::StaticEvent for ExtrinsicSuccess {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "ExtrinsicSuccess";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An extrinsic failed.
            pub struct ExtrinsicFailed {
                pub dispatch_error: runtime_types::sp_runtime::DispatchError,
                pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
            }
            impl ::subxt::events::StaticEvent for ExtrinsicFailed {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "ExtrinsicFailed";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///`:code` was updated.
            pub struct CodeUpdated;
            impl ::subxt::events::StaticEvent for CodeUpdated {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "CodeUpdated";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A new account was created.
            pub struct NewAccount {
                pub account: ::subxt::utils::AccountId32,
            }
            impl ::subxt::events::StaticEvent for NewAccount {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "NewAccount";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An account was reaped.
            pub struct KilledAccount {
                pub account: ::subxt::utils::AccountId32,
            }
            impl ::subxt::events::StaticEvent for KilledAccount {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "KilledAccount";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///On on-chain remark happened.
            pub struct Remarked {
                pub sender: ::subxt::utils::AccountId32,
                pub hash: ::subxt::utils::H256,
            }
            impl ::subxt::events::StaticEvent for Remarked {
                const PALLET: &'static str = "System";
                const EVENT: &'static str = "Remarked";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The full account information for a particular account ID.
                pub fn account(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::frame_system::AccountInfo<
                        ::core::primitive::u32,
                        runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "Account",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            176u8, 187u8, 21u8, 220u8, 159u8, 204u8, 127u8, 14u8, 21u8, 69u8, 77u8,
                            114u8, 230u8, 141u8, 107u8, 79u8, 23u8, 16u8, 174u8, 243u8, 252u8,
                            42u8, 65u8, 120u8, 229u8, 38u8, 210u8, 255u8, 22u8, 40u8, 109u8, 223u8,
                        ],
                    )
                }
                /// The full account information for a particular account ID.
                pub fn account_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::frame_system::AccountInfo<
                        ::core::primitive::u32,
                        runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "Account",
                        Vec::new(),
                        [
                            176u8, 187u8, 21u8, 220u8, 159u8, 204u8, 127u8, 14u8, 21u8, 69u8, 77u8,
                            114u8, 230u8, 141u8, 107u8, 79u8, 23u8, 16u8, 174u8, 243u8, 252u8,
                            42u8, 65u8, 120u8, 229u8, 38u8, 210u8, 255u8, 22u8, 40u8, 109u8, 223u8,
                        ],
                    )
                }
                /// Total extrinsics count for the current block.
                pub fn extrinsic_count(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "ExtrinsicCount",
                        vec![],
                        [
                            223u8, 60u8, 201u8, 120u8, 36u8, 44u8, 180u8, 210u8, 242u8, 53u8,
                            222u8, 154u8, 123u8, 176u8, 249u8, 8u8, 225u8, 28u8, 232u8, 4u8, 136u8,
                            41u8, 151u8, 82u8, 189u8, 149u8, 49u8, 166u8, 139u8, 9u8, 163u8, 231u8,
                        ],
                    )
                }
                /// The current weight for the block.
                pub fn block_weight(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::frame_support::dispatch::PerDispatchClass<
                        runtime_types::sp_weights::weight_v2::Weight,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "BlockWeight",
                        vec![],
                        [
                            120u8, 67u8, 71u8, 163u8, 36u8, 202u8, 52u8, 106u8, 143u8, 155u8,
                            144u8, 87u8, 142u8, 241u8, 232u8, 183u8, 56u8, 235u8, 27u8, 237u8,
                            20u8, 202u8, 33u8, 85u8, 189u8, 0u8, 28u8, 52u8, 198u8, 40u8, 219u8,
                            54u8,
                        ],
                    )
                }
                /// Total length (in bytes) for all extrinsics put together, for the current block.
                pub fn all_extrinsics_len(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "AllExtrinsicsLen",
                        vec![],
                        [
                            202u8, 145u8, 209u8, 225u8, 40u8, 220u8, 174u8, 74u8, 93u8, 164u8,
                            254u8, 248u8, 254u8, 192u8, 32u8, 117u8, 96u8, 149u8, 53u8, 145u8,
                            219u8, 64u8, 234u8, 18u8, 217u8, 200u8, 203u8, 141u8, 145u8, 28u8,
                            134u8, 60u8,
                        ],
                    )
                }
                /// Map of block numbers to block hashes.
                pub fn block_hash(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "BlockHash",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            50u8, 112u8, 176u8, 239u8, 175u8, 18u8, 205u8, 20u8, 241u8, 195u8,
                            21u8, 228u8, 186u8, 57u8, 200u8, 25u8, 38u8, 44u8, 106u8, 20u8, 168u8,
                            80u8, 76u8, 235u8, 12u8, 51u8, 137u8, 149u8, 200u8, 4u8, 220u8, 237u8,
                        ],
                    )
                }
                /// Map of block numbers to block hashes.
                pub fn block_hash_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "BlockHash",
                        Vec::new(),
                        [
                            50u8, 112u8, 176u8, 239u8, 175u8, 18u8, 205u8, 20u8, 241u8, 195u8,
                            21u8, 228u8, 186u8, 57u8, 200u8, 25u8, 38u8, 44u8, 106u8, 20u8, 168u8,
                            80u8, 76u8, 235u8, 12u8, 51u8, 137u8, 149u8, 200u8, 4u8, 220u8, 237u8,
                        ],
                    )
                }
                /// Extrinsics data for the current block (maps an extrinsic's index to its data).
                pub fn extrinsic_data(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "ExtrinsicData",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            210u8, 224u8, 211u8, 186u8, 118u8, 210u8, 185u8, 194u8, 238u8, 211u8,
                            254u8, 73u8, 67u8, 184u8, 31u8, 229u8, 168u8, 125u8, 98u8, 23u8, 241u8,
                            59u8, 49u8, 86u8, 126u8, 9u8, 114u8, 163u8, 160u8, 62u8, 50u8, 67u8,
                        ],
                    )
                }
                /// Extrinsics data for the current block (maps an extrinsic's index to its data).
                pub fn extrinsic_data_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "ExtrinsicData",
                        Vec::new(),
                        [
                            210u8, 224u8, 211u8, 186u8, 118u8, 210u8, 185u8, 194u8, 238u8, 211u8,
                            254u8, 73u8, 67u8, 184u8, 31u8, 229u8, 168u8, 125u8, 98u8, 23u8, 241u8,
                            59u8, 49u8, 86u8, 126u8, 9u8, 114u8, 163u8, 160u8, 62u8, 50u8, 67u8,
                        ],
                    )
                }
                /// The current block number being processed. Set by `execute_block`.
                pub fn number(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "Number",
                        vec![],
                        [
                            228u8, 96u8, 102u8, 190u8, 252u8, 130u8, 239u8, 172u8, 126u8, 235u8,
                            246u8, 139u8, 208u8, 15u8, 88u8, 245u8, 141u8, 232u8, 43u8, 204u8,
                            36u8, 87u8, 211u8, 141u8, 187u8, 68u8, 236u8, 70u8, 193u8, 235u8,
                            164u8, 191u8,
                        ],
                    )
                }
                /// Hash of the previous block.
                pub fn parent_hash(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "ParentHash",
                        vec![],
                        [
                            232u8, 206u8, 177u8, 119u8, 38u8, 57u8, 233u8, 50u8, 225u8, 49u8,
                            169u8, 176u8, 210u8, 51u8, 231u8, 176u8, 234u8, 186u8, 188u8, 112u8,
                            15u8, 152u8, 195u8, 232u8, 201u8, 97u8, 208u8, 249u8, 9u8, 163u8, 69u8,
                            36u8,
                        ],
                    )
                }
                /// Digest of the current block, also part of the block header.
                pub fn digest(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_runtime::generic::digest::Digest,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "Digest",
                        vec![],
                        [
                            83u8, 141u8, 200u8, 132u8, 182u8, 55u8, 197u8, 122u8, 13u8, 159u8,
                            31u8, 42u8, 60u8, 191u8, 89u8, 221u8, 242u8, 47u8, 199u8, 213u8, 48u8,
                            216u8, 131u8, 168u8, 245u8, 82u8, 56u8, 190u8, 62u8, 69u8, 96u8, 37u8,
                        ],
                    )
                }
                /// Events deposited for the current block.
                ///
                /// NOTE: The item is unbound and should therefore never be read on chain.
                /// It could otherwise inflate the PoV size of a block.
                ///
                /// Events have a large in-memory size. Box the events to not go out-of-memory
                /// just in case someone still reads them from within the runtime.
                pub fn events(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<
                        runtime_types::frame_system::EventRecord<
                            runtime_types::hyperbridge_runtime::RuntimeEvent,
                            ::subxt::utils::H256,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "Events",
                        vec![],
                        [
                            126u8, 7u8, 252u8, 2u8, 148u8, 106u8, 57u8, 241u8, 159u8, 56u8, 20u8,
                            109u8, 231u8, 2u8, 8u8, 108u8, 145u8, 42u8, 14u8, 200u8, 94u8, 46u8,
                            84u8, 81u8, 237u8, 61u8, 171u8, 120u8, 78u8, 63u8, 208u8, 142u8,
                        ],
                    )
                }
                /// The number of events in the `Events<T>` list.
                pub fn event_count(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "EventCount",
                        vec![],
                        [
                            236u8, 93u8, 90u8, 177u8, 250u8, 211u8, 138u8, 187u8, 26u8, 208u8,
                            203u8, 113u8, 221u8, 233u8, 227u8, 9u8, 249u8, 25u8, 202u8, 185u8,
                            161u8, 144u8, 167u8, 104u8, 127u8, 187u8, 38u8, 18u8, 52u8, 61u8, 66u8,
                            112u8,
                        ],
                    )
                }
                /// Mapping between a topic (represented by T::Hash) and a vector of indexes
                /// of events in the `<Events<T>>` list.
                ///
                /// All topic vectors have deterministic storage locations depending on the topic.
                /// This allows light-clients to leverage the changes trie storage
                /// tracking mechanism and in case of changes fetch the list of
                /// events of interest.
                ///
                /// The value has the type `(T::BlockNumber, EventIndex)` because if we used only
                /// just the `EventIndex` then in case if the topic has the same
                /// contents on the next block no notification will be triggered
                /// thus the event might be lost.
                pub fn event_topics(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "EventTopics",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            205u8, 90u8, 142u8, 190u8, 176u8, 37u8, 94u8, 82u8, 98u8, 1u8, 129u8,
                            63u8, 246u8, 101u8, 130u8, 58u8, 216u8, 16u8, 139u8, 196u8, 154u8,
                            111u8, 110u8, 178u8, 24u8, 44u8, 183u8, 176u8, 232u8, 82u8, 223u8,
                            38u8,
                        ],
                    )
                }
                /// Mapping between a topic (represented by T::Hash) and a vector of indexes
                /// of events in the `<Events<T>>` list.
                ///
                /// All topic vectors have deterministic storage locations depending on the topic.
                /// This allows light-clients to leverage the changes trie storage
                /// tracking mechanism and in case of changes fetch the list of
                /// events of interest.
                ///
                /// The value has the type `(T::BlockNumber, EventIndex)` because if we used only
                /// just the `EventIndex` then in case if the topic has the same
                /// contents on the next block no notification will be triggered
                /// thus the event might be lost.
                pub fn event_topics_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(::core::primitive::u32, ::core::primitive::u32)>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "EventTopics",
                        Vec::new(),
                        [
                            205u8, 90u8, 142u8, 190u8, 176u8, 37u8, 94u8, 82u8, 98u8, 1u8, 129u8,
                            63u8, 246u8, 101u8, 130u8, 58u8, 216u8, 16u8, 139u8, 196u8, 154u8,
                            111u8, 110u8, 178u8, 24u8, 44u8, 183u8, 176u8, 232u8, 82u8, 223u8,
                            38u8,
                        ],
                    )
                }
                /// Stores the `spec_version` and `spec_name` of when the last runtime upgrade
                /// happened.
                pub fn last_runtime_upgrade(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::frame_system::LastRuntimeUpgradeInfo,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "LastRuntimeUpgrade",
                        vec![],
                        [
                            52u8, 37u8, 117u8, 111u8, 57u8, 130u8, 196u8, 14u8, 99u8, 77u8, 91u8,
                            126u8, 178u8, 249u8, 78u8, 34u8, 9u8, 194u8, 92u8, 105u8, 113u8, 81u8,
                            185u8, 127u8, 245u8, 184u8, 60u8, 29u8, 234u8, 182u8, 96u8, 196u8,
                        ],
                    )
                }
                /// True if we have upgraded so that `type RefCount` is `u32`. False (default) if
                /// not.
                pub fn upgraded_to_u32_ref_count(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "UpgradedToU32RefCount",
                        vec![],
                        [
                            171u8, 88u8, 244u8, 92u8, 122u8, 67u8, 27u8, 18u8, 59u8, 175u8, 175u8,
                            178u8, 20u8, 150u8, 213u8, 59u8, 222u8, 141u8, 32u8, 107u8, 3u8, 114u8,
                            83u8, 250u8, 180u8, 233u8, 152u8, 54u8, 187u8, 99u8, 131u8, 204u8,
                        ],
                    )
                }
                /// True if we have upgraded so that AccountInfo contains three types of `RefCount`.
                /// False (default) if not.
                pub fn upgraded_to_triple_ref_count(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "UpgradedToTripleRefCount",
                        vec![],
                        [
                            90u8, 33u8, 56u8, 86u8, 90u8, 101u8, 89u8, 133u8, 203u8, 56u8, 201u8,
                            210u8, 244u8, 232u8, 150u8, 18u8, 51u8, 105u8, 14u8, 230u8, 103u8,
                            155u8, 246u8, 99u8, 53u8, 207u8, 225u8, 128u8, 186u8, 76u8, 40u8,
                            185u8,
                        ],
                    )
                }
                /// The execution phase of the block.
                pub fn execution_phase(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::frame_system::Phase,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "System",
                        "ExecutionPhase",
                        vec![],
                        [
                            230u8, 183u8, 221u8, 135u8, 226u8, 223u8, 55u8, 104u8, 138u8, 224u8,
                            103u8, 156u8, 222u8, 99u8, 203u8, 199u8, 164u8, 168u8, 193u8, 133u8,
                            201u8, 155u8, 63u8, 95u8, 17u8, 206u8, 165u8, 123u8, 161u8, 33u8,
                            172u8, 93u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                /// Block & extrinsics weights: base values and limits.
                pub fn block_weights(
                    &self,
                ) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockWeights>
                {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "BlockWeights",
                        [
                            118u8, 253u8, 239u8, 217u8, 145u8, 115u8, 85u8, 86u8, 172u8, 248u8,
                            139u8, 32u8, 158u8, 126u8, 172u8, 188u8, 197u8, 105u8, 145u8, 235u8,
                            171u8, 50u8, 31u8, 225u8, 167u8, 187u8, 241u8, 87u8, 6u8, 17u8, 234u8,
                            185u8,
                        ],
                    )
                }
                /// The maximum length of a block (in bytes).
                pub fn block_length(
                    &self,
                ) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockLength>
                {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "BlockLength",
                        [
                            116u8, 184u8, 225u8, 228u8, 207u8, 203u8, 4u8, 220u8, 234u8, 198u8,
                            150u8, 108u8, 205u8, 87u8, 194u8, 131u8, 229u8, 51u8, 140u8, 4u8, 47u8,
                            12u8, 200u8, 144u8, 153u8, 62u8, 51u8, 39u8, 138u8, 205u8, 203u8,
                            236u8,
                        ],
                    )
                }
                /// Maximum number of block number to block hash mappings to keep (oldest pruned
                /// first).
                pub fn block_hash_count(
                    &self,
                ) -> ::subxt::constants::Address<::core::primitive::u32> {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "BlockHashCount",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                /// The weight of runtime database operations the runtime can invoke.
                pub fn db_weight(
                    &self,
                ) -> ::subxt::constants::Address<runtime_types::sp_weights::RuntimeDbWeight>
                {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "DbWeight",
                        [
                            124u8, 162u8, 190u8, 149u8, 49u8, 177u8, 162u8, 231u8, 62u8, 167u8,
                            199u8, 181u8, 43u8, 232u8, 185u8, 116u8, 195u8, 51u8, 233u8, 223u8,
                            20u8, 129u8, 246u8, 13u8, 65u8, 180u8, 64u8, 9u8, 157u8, 59u8, 245u8,
                            118u8,
                        ],
                    )
                }
                /// Get the chain's current version.
                pub fn version(
                    &self,
                ) -> ::subxt::constants::Address<runtime_types::sp_version::RuntimeVersion>
                {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "Version",
                        [
                            93u8, 98u8, 57u8, 243u8, 229u8, 8u8, 234u8, 231u8, 72u8, 230u8, 139u8,
                            47u8, 63u8, 181u8, 17u8, 2u8, 220u8, 231u8, 104u8, 237u8, 185u8, 143u8,
                            165u8, 253u8, 188u8, 76u8, 147u8, 12u8, 170u8, 26u8, 74u8, 200u8,
                        ],
                    )
                }
                /// The designated SS58 prefix of this chain.
                ///
                /// This replaces the "ss58Format" property declared in the chain spec. Reason is
                /// that the runtime should know about the prefix in order to make use of it as
                /// an identifier of the chain.
                pub fn ss58_prefix(&self) -> ::subxt::constants::Address<::core::primitive::u16> {
                    ::subxt::constants::Address::new_static(
                        "System",
                        "SS58Prefix",
                        [
                            116u8, 33u8, 2u8, 170u8, 181u8, 147u8, 171u8, 169u8, 167u8, 227u8,
                            41u8, 144u8, 11u8, 236u8, 82u8, 100u8, 74u8, 60u8, 184u8, 72u8, 169u8,
                            90u8, 208u8, 135u8, 15u8, 117u8, 10u8, 123u8, 128u8, 193u8, 29u8, 70u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod timestamp {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Set {
                #[codec(compact)]
                pub now: ::core::primitive::u64,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Set the current time.
                ///
                ///This call should be invoked exactly once per block. It will panic at the
                /// finalization phase, if this call hasn't been invoked by that
                /// time.
                ///
                ///The timestamp should be greater than the previous one by the amount specified by
                ///`MinimumPeriod`.
                ///
                ///The dispatch origin for this call must be `Inherent`.
                ///
                ///## Complexity
                /// - `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)
                /// - 1 storage read and 1 storage mutation (codec `O(1)`). (because of
                ///   `DidUpdate::take` in
                ///  `on_finalize`)
                /// - 1 event handler `on_timestamp_set`. Must be `O(1)`.
                pub fn set(&self, now: ::core::primitive::u64) -> ::subxt::tx::Payload<Set> {
                    ::subxt::tx::Payload::new_static(
                        "Timestamp",
                        "set",
                        Set { now },
                        [
                            6u8, 97u8, 172u8, 236u8, 118u8, 238u8, 228u8, 114u8, 15u8, 115u8,
                            102u8, 85u8, 66u8, 151u8, 16u8, 33u8, 187u8, 17u8, 166u8, 88u8, 127u8,
                            214u8, 182u8, 51u8, 168u8, 88u8, 43u8, 101u8, 185u8, 8u8, 1u8, 28u8,
                        ],
                    )
                }
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Current time for the current block.
                pub fn now(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Timestamp",
                        "Now",
                        vec![],
                        [
                            148u8, 53u8, 50u8, 54u8, 13u8, 161u8, 57u8, 150u8, 16u8, 83u8, 144u8,
                            221u8, 59u8, 75u8, 158u8, 130u8, 39u8, 123u8, 106u8, 134u8, 202u8,
                            185u8, 83u8, 85u8, 60u8, 41u8, 120u8, 96u8, 210u8, 34u8, 2u8, 250u8,
                        ],
                    )
                }
                /// Did the timestamp get updated in this block?
                pub fn did_update(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Timestamp",
                        "DidUpdate",
                        vec![],
                        [
                            70u8, 13u8, 92u8, 186u8, 80u8, 151u8, 167u8, 90u8, 158u8, 232u8, 175u8,
                            13u8, 103u8, 135u8, 2u8, 78u8, 16u8, 6u8, 39u8, 158u8, 167u8, 85u8,
                            27u8, 47u8, 122u8, 73u8, 127u8, 26u8, 35u8, 168u8, 72u8, 204u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                /// The minimum period between blocks. Beware that this is different to the
                /// *expected* period that the block production apparatus provides.
                /// Your chosen consensus system will generally work with this to
                /// determine a sensible block time. e.g. For Aura, it will be
                /// double this period on default settings.
                pub fn minimum_period(
                    &self,
                ) -> ::subxt::constants::Address<::core::primitive::u64> {
                    ::subxt::constants::Address::new_static(
                        "Timestamp",
                        "MinimumPeriod",
                        [
                            128u8, 214u8, 205u8, 242u8, 181u8, 142u8, 124u8, 231u8, 190u8, 146u8,
                            59u8, 226u8, 157u8, 101u8, 103u8, 117u8, 249u8, 65u8, 18u8, 191u8,
                            103u8, 119u8, 53u8, 85u8, 81u8, 96u8, 220u8, 42u8, 184u8, 239u8, 42u8,
                            246u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod parachain_system {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetValidationData {
                pub data:
                    runtime_types::cumulus_primitives_parachain_inherent::ParachainInherentData,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SudoSendUpwardMessage {
                pub message: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct AuthorizeUpgrade {
                pub code_hash: ::subxt::utils::H256,
                pub check_version: ::core::primitive::bool,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct EnactAuthorizedUpgrade {
                pub code: ::std::vec::Vec<::core::primitive::u8>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Set the current validation data.
                ///
                ///This should be invoked exactly once per block. It will panic at the finalization
                ///phase if the call was not invoked.
                ///
                ///The dispatch origin for this call must be `Inherent`
                ///
                ///As a side effect, this function upgrades the current validation function
                ///if the appropriate time has come.
                pub fn set_validation_data(
                    &self,
                    data: runtime_types::cumulus_primitives_parachain_inherent::ParachainInherentData,
                ) -> ::subxt::tx::Payload<SetValidationData> {
                    ::subxt::tx::Payload::new_static(
                        "ParachainSystem",
                        "set_validation_data",
                        SetValidationData { data },
                        [
                            200u8, 80u8, 163u8, 177u8, 184u8, 117u8, 61u8, 203u8, 244u8, 214u8,
                            106u8, 151u8, 128u8, 131u8, 254u8, 120u8, 254u8, 76u8, 104u8, 39u8,
                            215u8, 227u8, 233u8, 254u8, 26u8, 62u8, 17u8, 42u8, 19u8, 127u8, 108u8,
                            242u8,
                        ],
                    )
                }
                pub fn sudo_send_upward_message(
                    &self,
                    message: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<SudoSendUpwardMessage> {
                    ::subxt::tx::Payload::new_static(
                        "ParachainSystem",
                        "sudo_send_upward_message",
                        SudoSendUpwardMessage { message },
                        [
                            127u8, 79u8, 45u8, 183u8, 190u8, 205u8, 184u8, 169u8, 255u8, 191u8,
                            86u8, 154u8, 134u8, 25u8, 249u8, 63u8, 47u8, 194u8, 108u8, 62u8, 60u8,
                            170u8, 81u8, 240u8, 113u8, 48u8, 181u8, 171u8, 95u8, 63u8, 26u8, 222u8,
                        ],
                    )
                }
                ///Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be
                /// supplied later.
                ///
                ///The `check_version` parameter sets a boolean flag for whether or not the
                /// runtime's spec version and name should be verified on upgrade.
                /// Since the authorization only has a hash, it cannot actually
                /// perform the verification.
                ///
                ///This call requires Root origin.
                pub fn authorize_upgrade(
                    &self,
                    code_hash: ::subxt::utils::H256,
                    check_version: ::core::primitive::bool,
                ) -> ::subxt::tx::Payload<AuthorizeUpgrade> {
                    ::subxt::tx::Payload::new_static(
                        "ParachainSystem",
                        "authorize_upgrade",
                        AuthorizeUpgrade { code_hash, check_version },
                        [
                            208u8, 115u8, 62u8, 35u8, 70u8, 223u8, 65u8, 57u8, 216u8, 44u8, 169u8,
                            249u8, 90u8, 112u8, 17u8, 208u8, 30u8, 131u8, 102u8, 131u8, 240u8,
                            217u8, 230u8, 214u8, 145u8, 198u8, 55u8, 13u8, 217u8, 51u8, 178u8,
                            141u8,
                        ],
                    )
                }
                ///Provide the preimage (runtime binary) `code` for an upgrade that has been
                /// authorized.
                ///
                ///If the authorization required a version check, this call will ensure the spec
                /// name remains unchanged and that the spec version has increased.
                ///
                ///Note that this function will not apply the new `code`, but only attempt to
                /// schedule the upgrade with the Relay Chain.
                ///
                ///All origins are allowed.
                pub fn enact_authorized_upgrade(
                    &self,
                    code: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<EnactAuthorizedUpgrade> {
                    ::subxt::tx::Payload::new_static(
                        "ParachainSystem",
                        "enact_authorized_upgrade",
                        EnactAuthorizedUpgrade { code },
                        [
                            43u8, 157u8, 1u8, 230u8, 134u8, 72u8, 230u8, 35u8, 159u8, 13u8, 201u8,
                            134u8, 184u8, 94u8, 167u8, 13u8, 108u8, 157u8, 145u8, 166u8, 119u8,
                            37u8, 51u8, 121u8, 252u8, 255u8, 48u8, 251u8, 126u8, 152u8, 247u8, 5u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::cumulus_pallet_parachain_system::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The validation function has been scheduled to apply.
            pub struct ValidationFunctionStored;
            impl ::subxt::events::StaticEvent for ValidationFunctionStored {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "ValidationFunctionStored";
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The validation function was applied as of the contained relay chain block number.
            pub struct ValidationFunctionApplied {
                pub relay_chain_block_num: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for ValidationFunctionApplied {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "ValidationFunctionApplied";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The relay-chain aborted the upgrade process.
            pub struct ValidationFunctionDiscarded;
            impl ::subxt::events::StaticEvent for ValidationFunctionDiscarded {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "ValidationFunctionDiscarded";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An upgrade has been authorized.
            pub struct UpgradeAuthorized {
                pub code_hash: ::subxt::utils::H256,
            }
            impl ::subxt::events::StaticEvent for UpgradeAuthorized {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "UpgradeAuthorized";
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some downward messages have been received and will be processed.
            pub struct DownwardMessagesReceived {
                pub count: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for DownwardMessagesReceived {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "DownwardMessagesReceived";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward messages were processed using the given weight.
            pub struct DownwardMessagesProcessed {
                pub weight_used: runtime_types::sp_weights::weight_v2::Weight,
                pub dmq_head: ::subxt::utils::H256,
            }
            impl ::subxt::events::StaticEvent for DownwardMessagesProcessed {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "DownwardMessagesProcessed";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An upward message was sent to the relay chain.
            pub struct UpwardMessageSent {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for UpwardMessageSent {
                const PALLET: &'static str = "ParachainSystem";
                const EVENT: &'static str = "UpwardMessageSent";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// In case of a scheduled upgrade, this storage field contains the validation code
                /// to be applied.
                ///
                /// As soon as the relay chain gives us the go-ahead signal, we will overwrite the
                /// [`:code`][well_known_keys::CODE] which will result the next
                /// block process with the new validation code. This concludes the upgrade process.
                ///
                /// [well_known_keys::CODE]: sp_core::storage::well_known_keys::CODE
                pub fn pending_validation_code(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "PendingValidationCode",
                        vec![],
                        [
                            162u8, 35u8, 108u8, 76u8, 160u8, 93u8, 215u8, 84u8, 20u8, 249u8, 57u8,
                            187u8, 88u8, 161u8, 15u8, 131u8, 213u8, 89u8, 140u8, 20u8, 227u8,
                            204u8, 79u8, 176u8, 114u8, 119u8, 8u8, 7u8, 64u8, 15u8, 90u8, 92u8,
                        ],
                    )
                }
                /// Validation code that is set by the parachain and is to be communicated to
                /// collator and consequently the relay-chain.
                ///
                /// This will be cleared in `on_initialize` of each new block if no other pallet
                /// already set the value.
                pub fn new_validation_code(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "NewValidationCode",
                        vec![],
                        [
                            224u8, 174u8, 53u8, 106u8, 240u8, 49u8, 48u8, 79u8, 219u8, 74u8, 142u8,
                            166u8, 92u8, 204u8, 244u8, 200u8, 43u8, 169u8, 177u8, 207u8, 190u8,
                            106u8, 180u8, 65u8, 245u8, 131u8, 134u8, 4u8, 53u8, 45u8, 76u8, 3u8,
                        ],
                    )
                }
                /// The [`PersistedValidationData`] set for this block.
                /// This value is expected to be set only once per block and it's never stored
                /// in the trie.
                pub fn validation_data(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::polkadot_primitives::v2::PersistedValidationData<
                        ::subxt::utils::H256,
                        ::core::primitive::u32,
                    >,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "ValidationData",
                        vec![],
                        [
                            112u8, 58u8, 240u8, 81u8, 219u8, 110u8, 244u8, 186u8, 251u8, 90u8,
                            195u8, 217u8, 229u8, 102u8, 233u8, 24u8, 109u8, 96u8, 219u8, 72u8,
                            139u8, 93u8, 58u8, 140u8, 40u8, 110u8, 167u8, 98u8, 199u8, 12u8, 138u8,
                            131u8,
                        ],
                    )
                }
                /// Were the validation data set to notify the relay chain?
                pub fn did_set_validation_code(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "DidSetValidationCode",
                        vec![],
                        [
                            89u8, 83u8, 74u8, 174u8, 234u8, 188u8, 149u8, 78u8, 140u8, 17u8, 92u8,
                            165u8, 243u8, 87u8, 59u8, 97u8, 135u8, 81u8, 192u8, 86u8, 193u8, 187u8,
                            113u8, 22u8, 108u8, 83u8, 242u8, 208u8, 174u8, 40u8, 49u8, 245u8,
                        ],
                    )
                }
                /// The relay chain block number associated with the last parachain block.
                pub fn last_relay_chain_block_number(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "LastRelayChainBlockNumber",
                        vec![],
                        [
                            68u8, 121u8, 6u8, 159u8, 181u8, 94u8, 151u8, 215u8, 225u8, 244u8, 4u8,
                            158u8, 216u8, 85u8, 55u8, 228u8, 197u8, 35u8, 200u8, 33u8, 29u8, 182u8,
                            17u8, 83u8, 59u8, 63u8, 25u8, 180u8, 132u8, 23u8, 97u8, 252u8,
                        ],
                    )
                }
                /// An option which indicates if the relay-chain restricts signalling a validation
                /// code upgrade. In other words, if this is `Some` and
                /// [`NewValidationCode`] is `Some` then the produced candidate will
                /// be invalid.
                ///
                /// This storage item is a mirror of the corresponding value for the current
                /// parachain from the relay-chain. This value is ephemeral which
                /// means it doesn't hit the storage. This value is set after the
                /// inherent.
                pub fn upgrade_restriction_signal(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::option::Option<
                        runtime_types::polkadot_primitives::v2::UpgradeRestriction,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "UpgradeRestrictionSignal",
                        vec![],
                        [
                            61u8, 3u8, 26u8, 6u8, 88u8, 114u8, 109u8, 63u8, 7u8, 115u8, 245u8,
                            198u8, 73u8, 234u8, 28u8, 228u8, 126u8, 27u8, 151u8, 18u8, 133u8, 54u8,
                            144u8, 149u8, 246u8, 43u8, 83u8, 47u8, 77u8, 238u8, 10u8, 196u8,
                        ],
                    )
                }
                /// The state proof for the last relay parent block.
                ///
                /// This field is meant to be updated each block with the validation data inherent.
                /// Therefore, before processing of the inherent, e.g. in
                /// `on_initialize` this data may be stale.
                ///
                /// This data is also absent from the genesis.
                pub fn relay_state_proof(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_trie::storage_proof::StorageProof,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "RelayStateProof",
                        vec![],
                        [
                            35u8, 124u8, 167u8, 221u8, 162u8, 145u8, 158u8, 186u8, 57u8, 154u8,
                            225u8, 6u8, 176u8, 13u8, 178u8, 195u8, 209u8, 122u8, 221u8, 26u8,
                            155u8, 126u8, 153u8, 246u8, 101u8, 221u8, 61u8, 145u8, 211u8, 236u8,
                            48u8, 130u8,
                        ],
                    )
                }
                /// The snapshot of some state related to messaging relevant to the current
                /// parachain as per the relay parent.
                ///
                /// This field is meant to be updated each block with the validation data inherent.
                /// Therefore, before processing of the inherent, e.g. in
                /// `on_initialize` this data may be stale.
                ///
                /// This data is also absent from the genesis.
                pub fn relevant_messaging_state(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                >{
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "RelevantMessagingState",
                        vec![],
                        [
                            68u8, 241u8, 114u8, 83u8, 200u8, 99u8, 8u8, 244u8, 110u8, 134u8, 106u8,
                            153u8, 17u8, 90u8, 184u8, 157u8, 100u8, 140u8, 157u8, 83u8, 25u8,
                            166u8, 173u8, 31u8, 221u8, 24u8, 236u8, 85u8, 176u8, 223u8, 237u8,
                            65u8,
                        ],
                    )
                }
                /// The parachain host configuration that was obtained from the relay parent.
                ///
                /// This field is meant to be updated each block with the validation data inherent.
                /// Therefore, before processing of the inherent, e.g. in
                /// `on_initialize` this data may be stale.
                ///
                /// This data is also absent from the genesis.
                pub fn host_configuration(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::polkadot_primitives::v2::AbridgedHostConfiguration,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "HostConfiguration",
                        vec![],
                        [
                            104u8, 200u8, 30u8, 202u8, 119u8, 204u8, 233u8, 20u8, 67u8, 199u8,
                            47u8, 166u8, 254u8, 152u8, 10u8, 187u8, 240u8, 255u8, 148u8, 201u8,
                            134u8, 41u8, 130u8, 201u8, 112u8, 65u8, 68u8, 103u8, 56u8, 123u8,
                            178u8, 113u8,
                        ],
                    )
                }
                /// The last downward message queue chain head we have observed.
                ///
                /// This value is loaded before and saved after processing inbound downward messages
                /// carried by the system inherent.
                pub fn last_dmq_mqc_head(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_primitives_parachain_inherent::MessageQueueChain,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "LastDmqMqcHead",
                        vec![],
                        [
                            176u8, 255u8, 246u8, 125u8, 36u8, 120u8, 24u8, 44u8, 26u8, 64u8, 236u8,
                            210u8, 189u8, 237u8, 50u8, 78u8, 45u8, 139u8, 58u8, 141u8, 112u8,
                            253u8, 178u8, 198u8, 87u8, 71u8, 77u8, 248u8, 21u8, 145u8, 187u8, 52u8,
                        ],
                    )
                }
                /// The message queue chain heads we have observed per each channel incoming
                /// channel.
                ///
                /// This value is loaded before and saved after processing inbound downward messages
                /// carried by the system inherent.
                pub fn last_hrmp_mqc_heads(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::KeyedVec<
                        runtime_types::polkadot_parachain::primitives::Id,
                        runtime_types::cumulus_primitives_parachain_inherent::MessageQueueChain,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "LastHrmpMqcHeads",
                        vec![],
                        [
                            55u8, 179u8, 35u8, 16u8, 173u8, 0u8, 122u8, 179u8, 236u8, 98u8, 9u8,
                            112u8, 11u8, 219u8, 241u8, 89u8, 131u8, 198u8, 64u8, 139u8, 103u8,
                            158u8, 77u8, 107u8, 83u8, 236u8, 255u8, 208u8, 47u8, 61u8, 219u8,
                            240u8,
                        ],
                    )
                }
                /// Number of downward messages processed in a block.
                ///
                /// This will be cleared in `on_initialize` of each new block.
                pub fn processed_downward_messages(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "ProcessedDownwardMessages",
                        vec![],
                        [
                            48u8, 177u8, 84u8, 228u8, 101u8, 235u8, 181u8, 27u8, 66u8, 55u8, 50u8,
                            146u8, 245u8, 223u8, 77u8, 132u8, 178u8, 80u8, 74u8, 90u8, 166u8, 81u8,
                            109u8, 25u8, 91u8, 69u8, 5u8, 69u8, 123u8, 197u8, 160u8, 146u8,
                        ],
                    )
                }
                /// HRMP watermark that was set in a block.
                ///
                /// This will be cleared in `on_initialize` of each new block.
                pub fn hrmp_watermark(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "HrmpWatermark",
                        vec![],
                        [
                            189u8, 59u8, 183u8, 195u8, 69u8, 185u8, 241u8, 226u8, 62u8, 204u8,
                            230u8, 77u8, 102u8, 75u8, 86u8, 157u8, 249u8, 140u8, 219u8, 72u8, 94u8,
                            64u8, 176u8, 72u8, 34u8, 205u8, 114u8, 103u8, 231u8, 233u8, 206u8,
                            111u8,
                        ],
                    )
                }
                /// HRMP messages that were sent in a block.
                ///
                /// This will be cleared in `on_initialize` of each new block.
                pub fn hrmp_outbound_messages(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<
                        runtime_types::polkadot_core_primitives::OutboundHrmpMessage<
                            runtime_types::polkadot_parachain::primitives::Id,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "HrmpOutboundMessages",
                        vec![],
                        [
                            74u8, 86u8, 173u8, 248u8, 90u8, 230u8, 71u8, 225u8, 127u8, 164u8,
                            221u8, 62u8, 146u8, 13u8, 73u8, 9u8, 98u8, 168u8, 6u8, 14u8, 97u8,
                            166u8, 45u8, 70u8, 62u8, 210u8, 9u8, 32u8, 83u8, 18u8, 4u8, 201u8,
                        ],
                    )
                }
                /// Upward messages that were sent in a block.
                ///
                /// This will be cleared in `on_initialize` of each new block.
                pub fn upward_messages(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "UpwardMessages",
                        vec![],
                        [
                            129u8, 208u8, 187u8, 36u8, 48u8, 108u8, 135u8, 56u8, 204u8, 60u8,
                            100u8, 158u8, 113u8, 238u8, 46u8, 92u8, 228u8, 41u8, 178u8, 177u8,
                            208u8, 195u8, 148u8, 149u8, 127u8, 21u8, 93u8, 92u8, 29u8, 115u8, 10u8,
                            248u8,
                        ],
                    )
                }
                /// Upward messages that are still pending and not yet send to the relay chain.
                pub fn pending_upward_messages(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "PendingUpwardMessages",
                        vec![],
                        [
                            223u8, 46u8, 224u8, 227u8, 222u8, 119u8, 225u8, 244u8, 59u8, 87u8,
                            127u8, 19u8, 217u8, 237u8, 103u8, 61u8, 6u8, 210u8, 107u8, 201u8,
                            117u8, 25u8, 85u8, 248u8, 36u8, 231u8, 28u8, 202u8, 41u8, 140u8, 208u8,
                            254u8,
                        ],
                    )
                }
                /// The number of HRMP messages we observed in `on_initialize` and thus used that
                /// number for announcing the weight of `on_initialize` and
                /// `on_finalize`.
                pub fn announced_hrmp_messages_per_candidate(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "AnnouncedHrmpMessagesPerCandidate",
                        vec![],
                        [
                            132u8, 61u8, 162u8, 129u8, 251u8, 243u8, 20u8, 144u8, 162u8, 73u8,
                            237u8, 51u8, 248u8, 41u8, 127u8, 171u8, 180u8, 79u8, 137u8, 23u8, 66u8,
                            134u8, 106u8, 222u8, 182u8, 154u8, 0u8, 145u8, 184u8, 156u8, 36u8,
                            97u8,
                        ],
                    )
                }
                /// The weight we reserve at the beginning of the block for processing XCMP
                /// messages. This overrides the amount set in the Config trait.
                pub fn reserved_xcmp_weight_override(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_weights::weight_v2::Weight,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "ReservedXcmpWeightOverride",
                        vec![],
                        [
                            180u8, 90u8, 34u8, 178u8, 1u8, 242u8, 211u8, 97u8, 100u8, 34u8, 39u8,
                            42u8, 142u8, 249u8, 236u8, 194u8, 244u8, 164u8, 96u8, 54u8, 98u8, 46u8,
                            92u8, 196u8, 185u8, 51u8, 231u8, 234u8, 249u8, 143u8, 244u8, 64u8,
                        ],
                    )
                }
                /// The weight we reserve at the beginning of the block for processing DMP messages.
                /// This overrides the amount set in the Config trait.
                pub fn reserved_dmp_weight_override(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_weights::weight_v2::Weight,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "ReservedDmpWeightOverride",
                        vec![],
                        [
                            90u8, 122u8, 168u8, 240u8, 95u8, 195u8, 160u8, 109u8, 175u8, 170u8,
                            227u8, 44u8, 139u8, 176u8, 32u8, 161u8, 57u8, 233u8, 56u8, 55u8, 123u8,
                            168u8, 174u8, 96u8, 159u8, 62u8, 186u8, 186u8, 17u8, 70u8, 57u8, 246u8,
                        ],
                    )
                }
                /// The next authorized upgrade, if there is one.
                pub fn authorized_upgrade(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_pallet_parachain_system::CodeUpgradeAuthorization,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "AuthorizedUpgrade",
                        vec![],
                        [
                            12u8, 212u8, 71u8, 191u8, 89u8, 101u8, 195u8, 3u8, 23u8, 180u8, 233u8,
                            52u8, 53u8, 133u8, 207u8, 94u8, 58u8, 43u8, 221u8, 236u8, 161u8, 41u8,
                            30u8, 194u8, 125u8, 2u8, 118u8, 152u8, 197u8, 49u8, 34u8, 33u8,
                        ],
                    )
                }
                /// A custom head data that should be returned as result of `validate_block`.
                ///
                /// See [`Pallet::set_custom_validation_head_data`] for more information.
                pub fn custom_validation_head_data(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainSystem",
                        "CustomValidationHeadData",
                        vec![],
                        [
                            189u8, 150u8, 234u8, 128u8, 111u8, 27u8, 173u8, 92u8, 109u8, 4u8, 98u8,
                            103u8, 158u8, 19u8, 16u8, 5u8, 107u8, 135u8, 126u8, 170u8, 62u8, 64u8,
                            149u8, 80u8, 33u8, 17u8, 83u8, 22u8, 176u8, 118u8, 26u8, 223u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod parachain_info {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn parachain_id(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::polkadot_parachain::primitives::Id,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "ParachainInfo",
                        "ParachainId",
                        vec![],
                        [
                            151u8, 191u8, 241u8, 118u8, 192u8, 47u8, 166u8, 151u8, 217u8, 240u8,
                            165u8, 232u8, 51u8, 113u8, 243u8, 1u8, 89u8, 240u8, 11u8, 1u8, 77u8,
                            104u8, 12u8, 56u8, 17u8, 135u8, 214u8, 19u8, 114u8, 135u8, 66u8, 76u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod balances {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Transfer {
                pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetBalance {
                pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                #[codec(compact)]
                pub new_free: ::core::primitive::u128,
                #[codec(compact)]
                pub new_reserved: ::core::primitive::u128,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceTransfer {
                pub source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct TransferKeepAlive {
                pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                #[codec(compact)]
                pub value: ::core::primitive::u128,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct TransferAll {
                pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                pub keep_alive: ::core::primitive::bool,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceUnreserve {
                pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                pub amount: ::core::primitive::u128,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Transfer some liquid free balance to another account.
                ///
                ///`transfer` will set the `FreeBalance` of the sender and receiver.
                ///If the sender's account is below the existential deposit as a result
                ///of the transfer, the account will be reaped.
                ///
                ///The dispatch origin for this call must be `Signed` by the transactor.
                ///
                ///## Complexity
                /// - Dependent on arguments but not critical, given proper implementations for
                ///   input config
                ///  types. See related functions below.
                /// - It contains a limited number of reads and writes internally and no complex
                ///  computation.
                ///
                ///Related functions:
                ///
                ///  - `ensure_can_withdraw` is always called internally but has a bounded
                ///    complexity.
                ///  - Transferring balances to accounts that did not exist before will cause
                ///    `T::OnNewAccount::on_new_account` to be called.
                ///  - Removing enough funds from an account will trigger
                ///    `T::DustRemoval::on_unbalanced`.
                ///  - `transfer_keep_alive` works the same way as `transfer`, but has an additional
                ///    check that the transfer will not kill the origin account.
                pub fn transfer(
                    &self,
                    dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<Transfer> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "transfer",
                        Transfer { dest, value },
                        [
                            111u8, 222u8, 32u8, 56u8, 171u8, 77u8, 252u8, 29u8, 194u8, 155u8,
                            200u8, 192u8, 198u8, 81u8, 23u8, 115u8, 236u8, 91u8, 218u8, 114u8,
                            107u8, 141u8, 138u8, 100u8, 237u8, 21u8, 58u8, 172u8, 3u8, 20u8, 216u8,
                            38u8,
                        ],
                    )
                }
                ///Set the balances of a given account.
                ///
                ///This will alter `FreeBalance` and `ReservedBalance` in storage. it will
                ///also alter the total issuance of the system (`TotalIssuance`) appropriately.
                ///If the new free or reserved balance is below the existential deposit,
                ///it will reset the account nonce (`frame_system::AccountNonce`).
                ///
                ///The dispatch origin for this call is `root`.
                pub fn set_balance(
                    &self,
                    who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    new_free: ::core::primitive::u128,
                    new_reserved: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<SetBalance> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "set_balance",
                        SetBalance { who, new_free, new_reserved },
                        [
                            234u8, 215u8, 97u8, 98u8, 243u8, 199u8, 57u8, 76u8, 59u8, 161u8, 118u8,
                            207u8, 34u8, 197u8, 198u8, 61u8, 231u8, 210u8, 169u8, 235u8, 150u8,
                            137u8, 173u8, 49u8, 28u8, 77u8, 84u8, 149u8, 143u8, 210u8, 139u8,
                            193u8,
                        ],
                    )
                }
                ///Exactly as `transfer`, except the origin must be root and the source account may
                /// be specified.
                ///## Complexity
                /// - Same as transfer, but additional read and write because the source account is
                ///   not
                ///  assumed to be in the overlay.
                pub fn force_transfer(
                    &self,
                    source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<ForceTransfer> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "force_transfer",
                        ForceTransfer { source, dest, value },
                        [
                            79u8, 174u8, 212u8, 108u8, 184u8, 33u8, 170u8, 29u8, 232u8, 254u8,
                            195u8, 218u8, 221u8, 134u8, 57u8, 99u8, 6u8, 70u8, 181u8, 227u8, 56u8,
                            239u8, 243u8, 158u8, 157u8, 245u8, 36u8, 162u8, 11u8, 237u8, 147u8,
                            15u8,
                        ],
                    )
                }
                ///Same as the [`transfer`] call, but with a check that the transfer will not kill
                /// the origin account.
                ///
                ///99% of the time you want [`transfer`] instead.
                ///
                ///[`transfer`]: struct.Pallet.html#method.transfer
                pub fn transfer_keep_alive(
                    &self,
                    dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    value: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<TransferKeepAlive> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "transfer_keep_alive",
                        TransferKeepAlive { dest, value },
                        [
                            112u8, 179u8, 75u8, 168u8, 193u8, 221u8, 9u8, 82u8, 190u8, 113u8,
                            253u8, 13u8, 130u8, 134u8, 170u8, 216u8, 136u8, 111u8, 242u8, 220u8,
                            202u8, 112u8, 47u8, 79u8, 73u8, 244u8, 226u8, 59u8, 240u8, 188u8,
                            210u8, 208u8,
                        ],
                    )
                }
                ///Transfer the entire transferable balance from the caller account.
                ///
                ///NOTE: This function only attempts to transfer _transferable_ balances. This
                /// means that any locked, reserved, or existential deposits (when
                /// `keep_alive` is `true`), will not be transferred by this
                /// function. To ensure that this function results in a killed account,
                /// you might need to prepare the account by removing any reference counters,
                /// storage deposits, etc...
                ///
                ///The dispatch origin of this call must be Signed.
                ///
                /// - `dest`: The recipient of the transfer.
                /// - `keep_alive`: A boolean to determine if the `transfer_all` operation should
                ///   send all
                ///  of the funds the account has, causing the sender account to be killed (false),
                /// or  transfer everything except at least the existential deposit,
                /// which will guarantee to  keep the sender account alive (true).
                /// ## Complexity
                /// - O(1). Just like transfer, but reading the user's transferable balance first.
                pub fn transfer_all(
                    &self,
                    dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    keep_alive: ::core::primitive::bool,
                ) -> ::subxt::tx::Payload<TransferAll> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "transfer_all",
                        TransferAll { dest, keep_alive },
                        [
                            46u8, 129u8, 29u8, 177u8, 221u8, 107u8, 245u8, 69u8, 238u8, 126u8,
                            145u8, 26u8, 219u8, 208u8, 14u8, 80u8, 149u8, 1u8, 214u8, 63u8, 67u8,
                            201u8, 144u8, 45u8, 129u8, 145u8, 174u8, 71u8, 238u8, 113u8, 208u8,
                            34u8,
                        ],
                    )
                }
                ///Unreserve some balance from a user by force.
                ///
                ///Can only be called by ROOT.
                pub fn force_unreserve(
                    &self,
                    who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    amount: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<ForceUnreserve> {
                    ::subxt::tx::Payload::new_static(
                        "Balances",
                        "force_unreserve",
                        ForceUnreserve { who, amount },
                        [
                            160u8, 146u8, 137u8, 76u8, 157u8, 187u8, 66u8, 148u8, 207u8, 76u8,
                            32u8, 254u8, 82u8, 215u8, 35u8, 161u8, 213u8, 52u8, 32u8, 98u8, 102u8,
                            106u8, 234u8, 123u8, 6u8, 175u8, 184u8, 188u8, 174u8, 106u8, 176u8,
                            78u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_balances::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An account was created with some free balance.
            pub struct Endowed {
                pub account: ::subxt::utils::AccountId32,
                pub free_balance: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Endowed {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Endowed";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An account was removed whose balance was non-zero but below ExistentialDeposit,
            ///resulting in an outright loss.
            pub struct DustLost {
                pub account: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for DustLost {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "DustLost";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Transfer succeeded.
            pub struct Transfer {
                pub from: ::subxt::utils::AccountId32,
                pub to: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Transfer {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Transfer";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A balance was set by root.
            pub struct BalanceSet {
                pub who: ::subxt::utils::AccountId32,
                pub free: ::core::primitive::u128,
                pub reserved: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for BalanceSet {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "BalanceSet";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some balance was reserved (moved from free to reserved).
            pub struct Reserved {
                pub who: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Reserved {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Reserved";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some balance was unreserved (moved from reserved to free).
            pub struct Unreserved {
                pub who: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Unreserved {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Unreserved";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some balance was moved from the reserve of the first account to the second account.
            ///Final argument indicates the destination balance type.
            pub struct ReserveRepatriated {
                pub from: ::subxt::utils::AccountId32,
                pub to: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
                pub destination_status:
                    runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
            }
            impl ::subxt::events::StaticEvent for ReserveRepatriated {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "ReserveRepatriated";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some amount was deposited (e.g. for transaction fees).
            pub struct Deposit {
                pub who: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Deposit {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Deposit";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some amount was withdrawn from the account (e.g. for transaction fees).
            pub struct Withdraw {
                pub who: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Withdraw {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Withdraw";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some amount was removed from the account (e.g. for misbehavior).
            pub struct Slashed {
                pub who: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for Slashed {
                const PALLET: &'static str = "Balances";
                const EVENT: &'static str = "Slashed";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The total units issued in the system.
                pub fn total_issuance(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u128,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "TotalIssuance",
                        vec![],
                        [
                            1u8, 206u8, 252u8, 237u8, 6u8, 30u8, 20u8, 232u8, 164u8, 115u8, 51u8,
                            156u8, 156u8, 206u8, 241u8, 187u8, 44u8, 84u8, 25u8, 164u8, 235u8,
                            20u8, 86u8, 242u8, 124u8, 23u8, 28u8, 140u8, 26u8, 73u8, 231u8, 51u8,
                        ],
                    )
                }
                /// The total units of outstanding deactivated balance in the system.
                pub fn inactive_issuance(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u128,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "InactiveIssuance",
                        vec![],
                        [
                            74u8, 203u8, 111u8, 142u8, 225u8, 104u8, 173u8, 51u8, 226u8, 12u8,
                            85u8, 135u8, 41u8, 206u8, 177u8, 238u8, 94u8, 246u8, 184u8, 250u8,
                            140u8, 213u8, 91u8, 118u8, 163u8, 111u8, 211u8, 46u8, 204u8, 160u8,
                            154u8, 21u8,
                        ],
                    )
                }
                /// The Balances pallet example of storing the balance of an account.
                ///
                /// # Example
                ///
                /// ```nocompile
                ///  impl pallet_balances::Config for Runtime {
                ///    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
                ///  }
                /// ```
                ///
                /// You can also store the balance of an account in the `System` pallet.
                ///
                /// # Example
                ///
                /// ```nocompile
                ///  impl pallet_balances::Config for Runtime {
                ///   type AccountStore = System
                ///  }
                /// ```
                ///
                /// But this comes with tradeoffs, storing account balances in the system pallet
                /// stores `frame_system` data alongside the account data contrary
                /// to storing account balances in the `Balances` pallet, which uses
                /// a `StorageMap` to store balances data only. NOTE: This is only
                /// used in the case that this pallet is used to store balances.
                pub fn account(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Account",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            246u8, 154u8, 253u8, 71u8, 192u8, 192u8, 192u8, 236u8, 128u8, 80u8,
                            40u8, 252u8, 201u8, 43u8, 3u8, 131u8, 19u8, 49u8, 141u8, 240u8, 172u8,
                            217u8, 215u8, 109u8, 87u8, 135u8, 248u8, 57u8, 98u8, 185u8, 22u8, 4u8,
                        ],
                    )
                }
                /// The Balances pallet example of storing the balance of an account.
                ///
                /// # Example
                ///
                /// ```nocompile
                ///  impl pallet_balances::Config for Runtime {
                ///    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
                ///  }
                /// ```
                ///
                /// You can also store the balance of an account in the `System` pallet.
                ///
                /// # Example
                ///
                /// ```nocompile
                ///  impl pallet_balances::Config for Runtime {
                ///   type AccountStore = System
                ///  }
                /// ```
                ///
                /// But this comes with tradeoffs, storing account balances in the system pallet
                /// stores `frame_system` data alongside the account data contrary
                /// to storing account balances in the `Balances` pallet, which uses
                /// a `StorageMap` to store balances data only. NOTE: This is only
                /// used in the case that this pallet is used to store balances.
                pub fn account_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_balances::AccountData<::core::primitive::u128>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Account",
                        Vec::new(),
                        [
                            246u8, 154u8, 253u8, 71u8, 192u8, 192u8, 192u8, 236u8, 128u8, 80u8,
                            40u8, 252u8, 201u8, 43u8, 3u8, 131u8, 19u8, 49u8, 141u8, 240u8, 172u8,
                            217u8, 215u8, 109u8, 87u8, 135u8, 248u8, 57u8, 98u8, 185u8, 22u8, 4u8,
                        ],
                    )
                }
                /// Any liquidity locks on some account balances.
                /// NOTE: Should only be accessed when setting, changing and freeing a lock.
                pub fn locks(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
                        runtime_types::pallet_balances::BalanceLock<::core::primitive::u128>,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Locks",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            216u8, 253u8, 87u8, 73u8, 24u8, 218u8, 35u8, 0u8, 244u8, 134u8, 195u8,
                            58u8, 255u8, 64u8, 153u8, 212u8, 210u8, 232u8, 4u8, 122u8, 90u8, 212u8,
                            136u8, 14u8, 127u8, 232u8, 8u8, 192u8, 40u8, 233u8, 18u8, 250u8,
                        ],
                    )
                }
                /// Any liquidity locks on some account balances.
                /// NOTE: Should only be accessed when setting, changing and freeing a lock.
                pub fn locks_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
                        runtime_types::pallet_balances::BalanceLock<::core::primitive::u128>,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Locks",
                        Vec::new(),
                        [
                            216u8, 253u8, 87u8, 73u8, 24u8, 218u8, 35u8, 0u8, 244u8, 134u8, 195u8,
                            58u8, 255u8, 64u8, 153u8, 212u8, 210u8, 232u8, 4u8, 122u8, 90u8, 212u8,
                            136u8, 14u8, 127u8, 232u8, 8u8, 192u8, 40u8, 233u8, 18u8, 250u8,
                        ],
                    )
                }
                /// Named reserves on some account balances.
                pub fn reserves(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        runtime_types::pallet_balances::ReserveData<
                            [::core::primitive::u8; 8usize],
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Reserves",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            17u8, 32u8, 191u8, 46u8, 76u8, 220u8, 101u8, 100u8, 42u8, 250u8, 128u8,
                            167u8, 117u8, 44u8, 85u8, 96u8, 105u8, 216u8, 16u8, 147u8, 74u8, 55u8,
                            183u8, 94u8, 160u8, 177u8, 26u8, 187u8, 71u8, 197u8, 187u8, 163u8,
                        ],
                    )
                }
                /// Named reserves on some account balances.
                pub fn reserves_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        runtime_types::pallet_balances::ReserveData<
                            [::core::primitive::u8; 8usize],
                            ::core::primitive::u128,
                        >,
                    >,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Balances",
                        "Reserves",
                        Vec::new(),
                        [
                            17u8, 32u8, 191u8, 46u8, 76u8, 220u8, 101u8, 100u8, 42u8, 250u8, 128u8,
                            167u8, 117u8, 44u8, 85u8, 96u8, 105u8, 216u8, 16u8, 147u8, 74u8, 55u8,
                            183u8, 94u8, 160u8, 177u8, 26u8, 187u8, 71u8, 197u8, 187u8, 163u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                /// The minimum amount required to keep an account open.
                pub fn existential_deposit(
                    &self,
                ) -> ::subxt::constants::Address<::core::primitive::u128> {
                    ::subxt::constants::Address::new_static(
                        "Balances",
                        "ExistentialDeposit",
                        [
                            84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
                            27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
                            136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
                        ],
                    )
                }
                /// The maximum number of locks that should exist on an account.
                /// Not strictly enforced, but used for weight estimation.
                pub fn max_locks(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
                    ::subxt::constants::Address::new_static(
                        "Balances",
                        "MaxLocks",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
                /// The maximum number of named reserves that can exist on an account.
                pub fn max_reserves(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
                    ::subxt::constants::Address::new_static(
                        "Balances",
                        "MaxReserves",
                        [
                            98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
                            125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
                            178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
                            145u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod transaction_payment {
        use super::{root_mod, runtime_types};
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_transaction_payment::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion
            /// fee, has been paid by `who`.
            pub struct TransactionFeePaid {
                pub who: ::subxt::utils::AccountId32,
                pub actual_fee: ::core::primitive::u128,
                pub tip: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for TransactionFeePaid {
                const PALLET: &'static str = "TransactionPayment";
                const EVENT: &'static str = "TransactionFeePaid";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                pub fn next_fee_multiplier(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_arithmetic::fixed_point::FixedU128,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "TransactionPayment",
                        "NextFeeMultiplier",
                        vec![],
                        [
                            210u8, 0u8, 206u8, 165u8, 183u8, 10u8, 206u8, 52u8, 14u8, 90u8, 218u8,
                            197u8, 189u8, 125u8, 113u8, 216u8, 52u8, 161u8, 45u8, 24u8, 245u8,
                            237u8, 121u8, 41u8, 106u8, 29u8, 45u8, 129u8, 250u8, 203u8, 206u8,
                            180u8,
                        ],
                    )
                }
                pub fn storage_version(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_transaction_payment::Releases,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "TransactionPayment",
                        "StorageVersion",
                        vec![],
                        [
                            219u8, 243u8, 82u8, 176u8, 65u8, 5u8, 132u8, 114u8, 8u8, 82u8, 176u8,
                            200u8, 97u8, 150u8, 177u8, 164u8, 166u8, 11u8, 34u8, 12u8, 12u8, 198u8,
                            58u8, 191u8, 186u8, 221u8, 221u8, 119u8, 181u8, 253u8, 154u8, 228u8,
                        ],
                    )
                }
            }
        }
        pub mod constants {
            use super::runtime_types;
            pub struct ConstantsApi;
            impl ConstantsApi {
                /// A fee mulitplier for `Operational` extrinsics to compute "virtual tip" to boost
                /// their `priority`
                ///
                /// This value is multipled by the `final_fee` to obtain a "virtual tip" that is
                /// later added to a tip component in regular `priority`
                /// calculations. It means that a `Normal` transaction can front-run
                /// a similarly-sized `Operational` extrinsic (with no tip), by
                /// including a tip value greater than the virtual tip.
                ///
                /// ```rust,ignore
                /// // For `Normal`
                /// let priority = priority_calc(tip);
                ///
                /// // For `Operational`
                /// let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;
                /// let priority = priority_calc(tip + virtual_tip);
                /// ```
                ///
                /// Note that since we use `final_fee` the multiplier applies also to the regular
                /// `tip` sent with the transaction. So, not only does the
                /// transaction get a priority bump based on the `inclusion_fee`,
                /// but we also amplify the impact of tips applied to `Operational`
                /// transactions.
                pub fn operational_fee_multiplier(
                    &self,
                ) -> ::subxt::constants::Address<::core::primitive::u8> {
                    ::subxt::constants::Address::new_static(
                        "TransactionPayment",
                        "OperationalFeeMultiplier",
                        [
                            141u8, 130u8, 11u8, 35u8, 226u8, 114u8, 92u8, 179u8, 168u8, 110u8,
                            28u8, 91u8, 221u8, 64u8, 4u8, 148u8, 201u8, 193u8, 185u8, 66u8, 226u8,
                            114u8, 97u8, 79u8, 62u8, 212u8, 202u8, 114u8, 237u8, 228u8, 183u8,
                            165u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod authorship {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Author of current block.
                pub fn author(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::AccountId32,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Authorship",
                        "Author",
                        vec![],
                        [
                            149u8, 42u8, 33u8, 147u8, 190u8, 207u8, 174u8, 227u8, 190u8, 110u8,
                            25u8, 131u8, 5u8, 167u8, 237u8, 188u8, 188u8, 33u8, 177u8, 126u8,
                            181u8, 49u8, 126u8, 118u8, 46u8, 128u8, 154u8, 95u8, 15u8, 91u8, 103u8,
                            113u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod collator_selection {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetInvulnerables {
                pub new: ::std::vec::Vec<::subxt::utils::AccountId32>,
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetDesiredCandidates {
                pub max: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetCandidacyBond {
                pub bond: ::core::primitive::u128,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct RegisterAsCandidate;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct LeaveIntent;
            pub struct TransactionApi;
            impl TransactionApi {
                ///Set the list of invulnerable (fixed) collators.
                pub fn set_invulnerables(
                    &self,
                    new: ::std::vec::Vec<::subxt::utils::AccountId32>,
                ) -> ::subxt::tx::Payload<SetInvulnerables> {
                    ::subxt::tx::Payload::new_static(
                        "CollatorSelection",
                        "set_invulnerables",
                        SetInvulnerables { new },
                        [
                            120u8, 177u8, 166u8, 239u8, 2u8, 102u8, 76u8, 143u8, 218u8, 130u8,
                            168u8, 152u8, 200u8, 107u8, 221u8, 30u8, 252u8, 18u8, 108u8, 147u8,
                            81u8, 251u8, 183u8, 185u8, 0u8, 184u8, 100u8, 251u8, 95u8, 168u8, 26u8,
                            142u8,
                        ],
                    )
                }
                ///Set the ideal number of collators (not including the invulnerables).
                ///If lowering this number, then the number of running collators could be higher
                /// than this figure. Aside from that edge case, there should be no
                /// other way to have more collators than the desired number.
                pub fn set_desired_candidates(
                    &self,
                    max: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<SetDesiredCandidates> {
                    ::subxt::tx::Payload::new_static(
                        "CollatorSelection",
                        "set_desired_candidates",
                        SetDesiredCandidates { max },
                        [
                            181u8, 32u8, 138u8, 37u8, 254u8, 213u8, 197u8, 224u8, 82u8, 26u8, 3u8,
                            113u8, 11u8, 146u8, 251u8, 35u8, 250u8, 202u8, 209u8, 2u8, 231u8,
                            176u8, 216u8, 124u8, 125u8, 43u8, 52u8, 126u8, 150u8, 140u8, 20u8,
                            113u8,
                        ],
                    )
                }
                ///Set the candidacy bond amount.
                pub fn set_candidacy_bond(
                    &self,
                    bond: ::core::primitive::u128,
                ) -> ::subxt::tx::Payload<SetCandidacyBond> {
                    ::subxt::tx::Payload::new_static(
                        "CollatorSelection",
                        "set_candidacy_bond",
                        SetCandidacyBond { bond },
                        [
                            42u8, 173u8, 79u8, 226u8, 224u8, 202u8, 70u8, 185u8, 125u8, 17u8,
                            123u8, 99u8, 107u8, 163u8, 67u8, 75u8, 110u8, 65u8, 248u8, 179u8, 39u8,
                            177u8, 135u8, 186u8, 66u8, 237u8, 30u8, 73u8, 163u8, 98u8, 81u8, 152u8,
                        ],
                    )
                }
                ///Register this account as a collator candidate. The account must (a) already have
                ///registered session keys and (b) be able to reserve the `CandidacyBond`.
                ///
                ///This call is not available to `Invulnerable` collators.
                pub fn register_as_candidate(&self) -> ::subxt::tx::Payload<RegisterAsCandidate> {
                    ::subxt::tx::Payload::new_static(
                        "CollatorSelection",
                        "register_as_candidate",
                        RegisterAsCandidate {},
                        [
                            63u8, 11u8, 114u8, 142u8, 89u8, 78u8, 120u8, 214u8, 22u8, 215u8, 125u8,
                            60u8, 203u8, 89u8, 141u8, 126u8, 124u8, 167u8, 70u8, 240u8, 85u8,
                            253u8, 34u8, 245u8, 67u8, 46u8, 240u8, 195u8, 57u8, 81u8, 138u8, 69u8,
                        ],
                    )
                }
                ///Deregister `origin` as a collator candidate. Note that the collator can only
                /// leave on session change. The `CandidacyBond` will be unreserved
                /// immediately.
                ///
                ///This call will fail if the total number of candidates would drop below
                /// `MinCandidates`.
                ///
                ///This call is not available to `Invulnerable` collators.
                pub fn leave_intent(&self) -> ::subxt::tx::Payload<LeaveIntent> {
                    ::subxt::tx::Payload::new_static(
                        "CollatorSelection",
                        "leave_intent",
                        LeaveIntent {},
                        [
                            217u8, 3u8, 35u8, 71u8, 152u8, 203u8, 203u8, 212u8, 25u8, 113u8, 158u8,
                            124u8, 161u8, 154u8, 32u8, 47u8, 116u8, 134u8, 11u8, 201u8, 154u8,
                            40u8, 138u8, 163u8, 184u8, 188u8, 33u8, 237u8, 219u8, 40u8, 63u8,
                            221u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_collator_selection::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct NewInvulnerables {
                pub invulnerables: ::std::vec::Vec<::subxt::utils::AccountId32>,
            }
            impl ::subxt::events::StaticEvent for NewInvulnerables {
                const PALLET: &'static str = "CollatorSelection";
                const EVENT: &'static str = "NewInvulnerables";
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct NewDesiredCandidates {
                pub desired_candidates: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for NewDesiredCandidates {
                const PALLET: &'static str = "CollatorSelection";
                const EVENT: &'static str = "NewDesiredCandidates";
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct NewCandidacyBond {
                pub bond_amount: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for NewCandidacyBond {
                const PALLET: &'static str = "CollatorSelection";
                const EVENT: &'static str = "NewCandidacyBond";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct CandidateAdded {
                pub account_id: ::subxt::utils::AccountId32,
                pub deposit: ::core::primitive::u128,
            }
            impl ::subxt::events::StaticEvent for CandidateAdded {
                const PALLET: &'static str = "CollatorSelection";
                const EVENT: &'static str = "CandidateAdded";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct CandidateRemoved {
                pub account_id: ::subxt::utils::AccountId32,
            }
            impl ::subxt::events::StaticEvent for CandidateRemoved {
                const PALLET: &'static str = "CollatorSelection";
                const EVENT: &'static str = "CandidateRemoved";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The invulnerable, fixed collators.
                pub fn invulnerables(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        ::subxt::utils::AccountId32,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "Invulnerables",
                        vec![],
                        [
                            215u8, 62u8, 140u8, 81u8, 0u8, 189u8, 182u8, 139u8, 32u8, 42u8, 20u8,
                            223u8, 81u8, 212u8, 100u8, 97u8, 146u8, 253u8, 75u8, 123u8, 240u8,
                            125u8, 249u8, 62u8, 226u8, 70u8, 57u8, 206u8, 16u8, 74u8, 52u8, 72u8,
                        ],
                    )
                }
                /// The (community, limited) collation candidates.
                pub fn candidates(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        runtime_types::pallet_collator_selection::pallet::CandidateInfo<
                            ::subxt::utils::AccountId32,
                            ::core::primitive::u128,
                        >,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "Candidates",
                        vec![],
                        [
                            28u8, 116u8, 232u8, 94u8, 147u8, 216u8, 214u8, 30u8, 26u8, 241u8, 68u8,
                            108u8, 165u8, 107u8, 89u8, 136u8, 111u8, 239u8, 150u8, 42u8, 210u8,
                            214u8, 192u8, 234u8, 29u8, 41u8, 157u8, 169u8, 120u8, 126u8, 192u8,
                            32u8,
                        ],
                    )
                }
                /// Last block authored by collator.
                pub fn last_authored_block(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "LastAuthoredBlock",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            53u8, 30u8, 243u8, 31u8, 228u8, 231u8, 175u8, 153u8, 204u8, 241u8,
                            76u8, 147u8, 6u8, 202u8, 255u8, 89u8, 30u8, 129u8, 85u8, 92u8, 10u8,
                            97u8, 177u8, 129u8, 88u8, 196u8, 7u8, 255u8, 74u8, 52u8, 28u8, 0u8,
                        ],
                    )
                }
                /// Last block authored by collator.
                pub fn last_authored_block_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "LastAuthoredBlock",
                        Vec::new(),
                        [
                            53u8, 30u8, 243u8, 31u8, 228u8, 231u8, 175u8, 153u8, 204u8, 241u8,
                            76u8, 147u8, 6u8, 202u8, 255u8, 89u8, 30u8, 129u8, 85u8, 92u8, 10u8,
                            97u8, 177u8, 129u8, 88u8, 196u8, 7u8, 255u8, 74u8, 52u8, 28u8, 0u8,
                        ],
                    )
                }
                /// Desired number of candidates.
                ///
                /// This should ideally always be less than [`Config::MaxCandidates`] for weights to
                /// be correct.
                pub fn desired_candidates(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "DesiredCandidates",
                        vec![],
                        [
                            161u8, 170u8, 254u8, 76u8, 112u8, 146u8, 144u8, 7u8, 177u8, 152u8,
                            146u8, 60u8, 143u8, 237u8, 1u8, 168u8, 176u8, 33u8, 103u8, 35u8, 39u8,
                            233u8, 107u8, 253u8, 47u8, 183u8, 11u8, 86u8, 230u8, 13u8, 127u8,
                            133u8,
                        ],
                    )
                }
                /// Fixed amount to deposit to become a collator.
                ///
                /// When a collator calls `leave_intent` they immediately receive the deposit back.
                pub fn candidacy_bond(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u128,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "CollatorSelection",
                        "CandidacyBond",
                        vec![],
                        [
                            1u8, 153u8, 211u8, 74u8, 138u8, 178u8, 81u8, 9u8, 205u8, 117u8, 102u8,
                            182u8, 56u8, 184u8, 56u8, 62u8, 193u8, 82u8, 224u8, 218u8, 253u8,
                            194u8, 250u8, 55u8, 220u8, 107u8, 157u8, 175u8, 62u8, 35u8, 224u8,
                            183u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod session {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetKeys {
                pub keys: runtime_types::hyperbridge_runtime::SessionKeys,
                pub proof: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct PurgeKeys;
            pub struct TransactionApi;
            impl TransactionApi {
                ///Sets the session key(s) of the function caller to `keys`.
                ///Allows an account to set its session key prior to becoming a validator.
                ///This doesn't take effect until the next session.
                ///
                ///The dispatch origin of this function must be signed.
                ///
                ///## Complexity
                /// - `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()`
                ///   which is
                ///  fixed.
                pub fn set_keys(
                    &self,
                    keys: runtime_types::hyperbridge_runtime::SessionKeys,
                    proof: ::std::vec::Vec<::core::primitive::u8>,
                ) -> ::subxt::tx::Payload<SetKeys> {
                    ::subxt::tx::Payload::new_static(
                        "Session",
                        "set_keys",
                        SetKeys { keys, proof },
                        [
                            199u8, 56u8, 39u8, 236u8, 44u8, 88u8, 207u8, 0u8, 187u8, 195u8, 218u8,
                            94u8, 126u8, 128u8, 37u8, 162u8, 216u8, 223u8, 36u8, 165u8, 18u8, 37u8,
                            16u8, 72u8, 136u8, 28u8, 134u8, 230u8, 231u8, 48u8, 230u8, 122u8,
                        ],
                    )
                }
                ///Removes any session key(s) of the function caller.
                ///
                ///This doesn't take effect until the next session.
                ///
                ///The dispatch origin of this function must be Signed and the account must be
                /// either be convertible to a validator ID using the chain's
                /// typical addressing system (this usually means being a controller
                /// account) or directly convertible into a validator ID (which
                /// usually means being a stash account).
                ///
                ///## Complexity
                /// - `O(1)` in number of key types. Actual cost depends on the number of length of
                ///  `T::Keys::key_ids()` which is fixed.
                pub fn purge_keys(&self) -> ::subxt::tx::Payload<PurgeKeys> {
                    ::subxt::tx::Payload::new_static(
                        "Session",
                        "purge_keys",
                        PurgeKeys {},
                        [
                            200u8, 255u8, 4u8, 213u8, 188u8, 92u8, 99u8, 116u8, 163u8, 152u8, 29u8,
                            35u8, 133u8, 119u8, 246u8, 44u8, 91u8, 31u8, 145u8, 23u8, 213u8, 64u8,
                            71u8, 242u8, 207u8, 239u8, 231u8, 37u8, 61u8, 63u8, 190u8, 35u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_session::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///New session has happened. Note that the argument is the session index, not the
            ///block number as the type might suggest.
            pub struct NewSession {
                pub session_index: ::core::primitive::u32,
            }
            impl ::subxt::events::StaticEvent for NewSession {
                const PALLET: &'static str = "Session";
                const EVENT: &'static str = "NewSession";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The current set of validators.
                pub fn validators(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::subxt::utils::AccountId32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "Validators",
                        vec![],
                        [
                            144u8, 235u8, 200u8, 43u8, 151u8, 57u8, 147u8, 172u8, 201u8, 202u8,
                            242u8, 96u8, 57u8, 76u8, 124u8, 77u8, 42u8, 113u8, 218u8, 220u8, 230u8,
                            32u8, 151u8, 152u8, 172u8, 106u8, 60u8, 227u8, 122u8, 118u8, 137u8,
                            68u8,
                        ],
                    )
                }
                /// Current index of the session.
                pub fn current_index(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "CurrentIndex",
                        vec![],
                        [
                            148u8, 179u8, 159u8, 15u8, 197u8, 95u8, 214u8, 30u8, 209u8, 251u8,
                            183u8, 231u8, 91u8, 25u8, 181u8, 191u8, 143u8, 252u8, 227u8, 80u8,
                            159u8, 66u8, 194u8, 67u8, 113u8, 74u8, 111u8, 91u8, 218u8, 187u8,
                            130u8, 40u8,
                        ],
                    )
                }
                /// True if the underlying economic identities or weighting behind the validators
                /// has changed in the queued validator set.
                pub fn queued_changed(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "QueuedChanged",
                        vec![],
                        [
                            105u8, 140u8, 235u8, 218u8, 96u8, 100u8, 252u8, 10u8, 58u8, 221u8,
                            244u8, 251u8, 67u8, 91u8, 80u8, 202u8, 152u8, 42u8, 50u8, 113u8, 200u8,
                            247u8, 59u8, 213u8, 77u8, 195u8, 1u8, 150u8, 220u8, 18u8, 245u8, 46u8,
                        ],
                    )
                }
                /// The queued keys for the next session. When the next session begins, these keys
                /// will be used to determine the validator's session keys.
                pub fn queued_keys(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(
                        ::subxt::utils::AccountId32,
                        runtime_types::hyperbridge_runtime::SessionKeys,
                    )>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "QueuedKeys",
                        vec![],
                        [
                            42u8, 134u8, 252u8, 233u8, 29u8, 69u8, 168u8, 107u8, 77u8, 70u8, 80u8,
                            189u8, 149u8, 227u8, 77u8, 74u8, 100u8, 175u8, 10u8, 162u8, 145u8,
                            105u8, 85u8, 196u8, 169u8, 195u8, 116u8, 255u8, 112u8, 122u8, 112u8,
                            133u8,
                        ],
                    )
                }
                /// Indices of disabled validators.
                ///
                /// The vec is always kept sorted so that we can find whether a given validator is
                /// disabled using binary search. It gets cleared when `on_session_ending` returns
                /// a new set of identities.
                pub fn disabled_validators(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u32>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "DisabledValidators",
                        vec![],
                        [
                            135u8, 22u8, 22u8, 97u8, 82u8, 217u8, 144u8, 141u8, 121u8, 240u8,
                            189u8, 16u8, 176u8, 88u8, 177u8, 31u8, 20u8, 242u8, 73u8, 104u8, 11u8,
                            110u8, 214u8, 34u8, 52u8, 217u8, 106u8, 33u8, 174u8, 174u8, 198u8,
                            84u8,
                        ],
                    )
                }
                /// The next session keys for a validator.
                pub fn next_keys(
                    &self,
                    _0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::hyperbridge_runtime::SessionKeys,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "NextKeys",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            21u8, 0u8, 237u8, 42u8, 156u8, 77u8, 229u8, 211u8, 105u8, 8u8, 231u8,
                            5u8, 246u8, 188u8, 69u8, 143u8, 202u8, 240u8, 252u8, 253u8, 106u8,
                            37u8, 51u8, 244u8, 206u8, 199u8, 249u8, 37u8, 17u8, 102u8, 20u8, 246u8,
                        ],
                    )
                }
                /// The next session keys for a validator.
                pub fn next_keys_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::hyperbridge_runtime::SessionKeys,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "NextKeys",
                        Vec::new(),
                        [
                            21u8, 0u8, 237u8, 42u8, 156u8, 77u8, 229u8, 211u8, 105u8, 8u8, 231u8,
                            5u8, 246u8, 188u8, 69u8, 143u8, 202u8, 240u8, 252u8, 253u8, 106u8,
                            37u8, 51u8, 244u8, 206u8, 199u8, 249u8, 37u8, 17u8, 102u8, 20u8, 246u8,
                        ],
                    )
                }
                /// The owner of a key. The key is the `KeyTypeId` + the encoded key.
                pub fn key_owner(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::sp_core::crypto::KeyTypeId>,
                    _1: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::AccountId32,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "KeyOwner",
                        vec![
                            ::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
                            ::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
                        ],
                        [
                            4u8, 91u8, 25u8, 84u8, 250u8, 201u8, 174u8, 129u8, 201u8, 58u8, 197u8,
                            199u8, 137u8, 240u8, 118u8, 33u8, 99u8, 2u8, 195u8, 57u8, 53u8, 172u8,
                            0u8, 148u8, 203u8, 144u8, 149u8, 64u8, 135u8, 254u8, 242u8, 215u8,
                        ],
                    )
                }
                /// The owner of a key. The key is the `KeyTypeId` + the encoded key.
                pub fn key_owner_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::AccountId32,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Session",
                        "KeyOwner",
                        Vec::new(),
                        [
                            4u8, 91u8, 25u8, 84u8, 250u8, 201u8, 174u8, 129u8, 201u8, 58u8, 197u8,
                            199u8, 137u8, 240u8, 118u8, 33u8, 99u8, 2u8, 195u8, 57u8, 53u8, 172u8,
                            0u8, 148u8, 203u8, 144u8, 149u8, 64u8, 135u8, 254u8, 242u8, 215u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod aura {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The current authority set.
                pub fn authorities(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Aura",
                        "Authorities",
                        vec![],
                        [
                            199u8, 89u8, 94u8, 48u8, 249u8, 35u8, 105u8, 90u8, 15u8, 86u8, 218u8,
                            85u8, 22u8, 236u8, 228u8, 36u8, 137u8, 64u8, 236u8, 171u8, 242u8,
                            217u8, 91u8, 240u8, 205u8, 205u8, 226u8, 16u8, 147u8, 235u8, 181u8,
                            41u8,
                        ],
                    )
                }
                /// The current slot of this block.
                ///
                /// This will be set in `on_initialize`.
                pub fn current_slot(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::sp_consensus_slots::Slot,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Aura",
                        "CurrentSlot",
                        vec![],
                        [
                            139u8, 237u8, 185u8, 137u8, 251u8, 179u8, 69u8, 167u8, 133u8, 168u8,
                            204u8, 64u8, 178u8, 123u8, 92u8, 250u8, 119u8, 190u8, 208u8, 178u8,
                            208u8, 176u8, 124u8, 187u8, 74u8, 165u8, 33u8, 78u8, 161u8, 206u8, 8u8,
                            108u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod aura_ext {
        use super::{root_mod, runtime_types};
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Serves as cache for the authorities.
                ///
                /// The authorities in AuRa are overwritten in `on_initialize` when we switch to a
                /// new session, but we require the old authorities to verify the
                /// seal when validating a PoV. This will always be updated to the
                /// latest AuRa authorities in `on_finalize`.
                pub fn authorities(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "AuraExt",
                        "Authorities",
                        vec![],
                        [
                            199u8, 89u8, 94u8, 48u8, 249u8, 35u8, 105u8, 90u8, 15u8, 86u8, 218u8,
                            85u8, 22u8, 236u8, 228u8, 36u8, 137u8, 64u8, 236u8, 171u8, 242u8,
                            217u8, 91u8, 240u8, 205u8, 205u8, 226u8, 16u8, 147u8, 235u8, 181u8,
                            41u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod sudo {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Sudo {
                pub call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SudoUncheckedWeight {
                pub call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
                pub weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SetKey {
                pub new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SudoAs {
                pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                pub call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Authenticates the sudo key and dispatches a function call with `Root` origin.
                ///
                ///The dispatch origin for this call must be _Signed_.
                ///
                ///## Complexity
                /// - O(1).
                pub fn sudo(
                    &self,
                    call: runtime_types::hyperbridge_runtime::RuntimeCall,
                ) -> ::subxt::tx::Payload<Sudo> {
                    ::subxt::tx::Payload::new_static(
                        "Sudo",
                        "sudo",
                        Sudo { call: ::std::boxed::Box::new(call) },
                        [
                            248u8, 217u8, 237u8, 188u8, 186u8, 210u8, 115u8, 5u8, 191u8, 121u8,
                            186u8, 11u8, 20u8, 176u8, 214u8, 100u8, 132u8, 228u8, 246u8, 254u8,
                            24u8, 45u8, 124u8, 207u8, 76u8, 111u8, 156u8, 24u8, 171u8, 93u8, 166u8,
                            113u8,
                        ],
                    )
                }
                ///Authenticates the sudo key and dispatches a function call with `Root` origin.
                ///This function does not check the weight of the call, and instead allows the
                ///Sudo user to specify the weight of the call.
                ///
                ///The dispatch origin for this call must be _Signed_.
                ///
                ///## Complexity
                /// - O(1).
                pub fn sudo_unchecked_weight(
                    &self,
                    call: runtime_types::hyperbridge_runtime::RuntimeCall,
                    weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<SudoUncheckedWeight> {
                    ::subxt::tx::Payload::new_static(
                        "Sudo",
                        "sudo_unchecked_weight",
                        SudoUncheckedWeight { call: ::std::boxed::Box::new(call), weight },
                        [
                            194u8, 58u8, 174u8, 80u8, 179u8, 215u8, 215u8, 217u8, 127u8, 250u8,
                            173u8, 188u8, 226u8, 60u8, 158u8, 33u8, 117u8, 177u8, 20u8, 207u8,
                            135u8, 171u8, 140u8, 120u8, 161u8, 123u8, 126u8, 15u8, 249u8, 187u8,
                            20u8, 23u8,
                        ],
                    )
                }
                ///Authenticates the current sudo key and sets the given AccountId (`new`) as the
                /// new sudo key.
                ///
                ///The dispatch origin for this call must be _Signed_.
                ///
                ///## Complexity
                /// - O(1).
                pub fn set_key(
                    &self,
                    new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                ) -> ::subxt::tx::Payload<SetKey> {
                    ::subxt::tx::Payload::new_static(
                        "Sudo",
                        "set_key",
                        SetKey { new },
                        [
                            23u8, 224u8, 218u8, 169u8, 8u8, 28u8, 111u8, 199u8, 26u8, 88u8, 225u8,
                            105u8, 17u8, 19u8, 87u8, 156u8, 97u8, 67u8, 89u8, 173u8, 70u8, 0u8,
                            5u8, 246u8, 198u8, 135u8, 182u8, 180u8, 44u8, 9u8, 212u8, 95u8,
                        ],
                    )
                }
                ///Authenticates the sudo key and dispatches a function call with `Signed` origin
                /// from a given account.
                ///
                ///The dispatch origin for this call must be _Signed_.
                ///
                ///## Complexity
                /// - O(1).
                pub fn sudo_as(
                    &self,
                    who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                    call: runtime_types::hyperbridge_runtime::RuntimeCall,
                ) -> ::subxt::tx::Payload<SudoAs> {
                    ::subxt::tx::Payload::new_static(
                        "Sudo",
                        "sudo_as",
                        SudoAs { who, call: ::std::boxed::Box::new(call) },
                        [
                            140u8, 73u8, 79u8, 189u8, 154u8, 115u8, 100u8, 122u8, 108u8, 2u8,
                            232u8, 62u8, 130u8, 84u8, 96u8, 19u8, 194u8, 209u8, 151u8, 138u8,
                            110u8, 243u8, 68u8, 82u8, 127u8, 133u8, 47u8, 92u8, 211u8, 10u8, 160u8,
                            147u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_sudo::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A sudo just took place. \[result\]
            pub struct Sudid {
                pub sudo_result:
                    ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for Sudid {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "Sudid";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The \[sudoer\] just switched identity; the old key is supplied if one existed.
            pub struct KeyChanged {
                pub old_sudoer: ::core::option::Option<::subxt::utils::AccountId32>,
            }
            impl ::subxt::events::StaticEvent for KeyChanged {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "KeyChanged";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A sudo just took place. \[result\]
            pub struct SudoAsDone {
                pub sudo_result:
                    ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
            }
            impl ::subxt::events::StaticEvent for SudoAsDone {
                const PALLET: &'static str = "Sudo";
                const EVENT: &'static str = "SudoAsDone";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The `AccountId` of the sudo key.
                pub fn key(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::AccountId32,
                    ::subxt::storage::address::Yes,
                    (),
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Sudo",
                        "Key",
                        vec![],
                        [
                            244u8, 73u8, 188u8, 136u8, 218u8, 163u8, 68u8, 179u8, 122u8, 173u8,
                            34u8, 108u8, 137u8, 28u8, 182u8, 16u8, 196u8, 92u8, 138u8, 34u8, 102u8,
                            80u8, 199u8, 88u8, 107u8, 207u8, 36u8, 22u8, 168u8, 167u8, 20u8, 142u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod xcmp_queue {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ServiceOverweight {
                pub index: ::core::primitive::u64,
                pub weight_limit: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SuspendXcmExecution;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ResumeXcmExecution;
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateSuspendThreshold {
                pub new: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateDropThreshold {
                pub new: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateResumeThreshold {
                pub new: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateThresholdWeight {
                pub new: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateWeightRestrictDecay {
                pub new: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct UpdateXcmpMaxIndividualWeight {
                pub new: runtime_types::sp_weights::weight_v2::Weight,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Services a single overweight XCM.
                ///
                /// - `origin`: Must pass `ExecuteOverweightOrigin`.
                /// - `index`: The index of the overweight XCM to service
                /// - `weight_limit`: The amount of weight that XCM execution may take.
                ///
                ///Errors:
                /// - `BadOverweightIndex`: XCM under `index` is not found in the `Overweight`
                ///   storage map.
                /// - `BadXcm`: XCM under `index` cannot be properly decoded into a valid XCM
                ///   format.
                /// - `WeightOverLimit`: XCM execution may use greater `weight_limit`.
                ///
                ///Events:
                /// - `OverweightServiced`: On success.
                pub fn service_overweight(
                    &self,
                    index: ::core::primitive::u64,
                    weight_limit: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<ServiceOverweight> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "service_overweight",
                        ServiceOverweight { index, weight_limit },
                        [
                            121u8, 236u8, 235u8, 23u8, 210u8, 238u8, 238u8, 122u8, 15u8, 86u8,
                            34u8, 119u8, 105u8, 100u8, 214u8, 236u8, 117u8, 39u8, 254u8, 235u8,
                            189u8, 15u8, 72u8, 74u8, 225u8, 134u8, 148u8, 126u8, 31u8, 203u8,
                            144u8, 106u8,
                        ],
                    )
                }
                ///Suspends all XCM executions for the XCMP queue, regardless of the sender's
                /// origin.
                ///
                /// - `origin`: Must pass `ControllerOrigin`.
                pub fn suspend_xcm_execution(&self) -> ::subxt::tx::Payload<SuspendXcmExecution> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "suspend_xcm_execution",
                        SuspendXcmExecution {},
                        [
                            139u8, 76u8, 166u8, 86u8, 106u8, 144u8, 16u8, 47u8, 105u8, 185u8, 7u8,
                            7u8, 63u8, 14u8, 250u8, 236u8, 99u8, 121u8, 101u8, 143u8, 28u8, 175u8,
                            108u8, 197u8, 226u8, 43u8, 103u8, 92u8, 186u8, 12u8, 51u8, 153u8,
                        ],
                    )
                }
                ///Resumes all XCM executions for the XCMP queue.
                ///
                ///Note that this function doesn't change the status of the in/out bound channels.
                ///
                /// - `origin`: Must pass `ControllerOrigin`.
                pub fn resume_xcm_execution(&self) -> ::subxt::tx::Payload<ResumeXcmExecution> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "resume_xcm_execution",
                        ResumeXcmExecution {},
                        [
                            67u8, 111u8, 47u8, 237u8, 79u8, 42u8, 90u8, 56u8, 245u8, 2u8, 20u8,
                            23u8, 33u8, 121u8, 135u8, 50u8, 204u8, 147u8, 195u8, 80u8, 177u8,
                            202u8, 8u8, 160u8, 164u8, 138u8, 64u8, 252u8, 178u8, 63u8, 102u8,
                            245u8,
                        ],
                    )
                }
                ///Overwrites the number of pages of messages which must be in the queue for the
                /// other side to be told to suspend their sending.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.suspend_value`
                pub fn update_suspend_threshold(
                    &self,
                    new: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<UpdateSuspendThreshold> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_suspend_threshold",
                        UpdateSuspendThreshold { new },
                        [
                            155u8, 120u8, 9u8, 228u8, 110u8, 62u8, 233u8, 36u8, 57u8, 85u8, 19u8,
                            67u8, 246u8, 88u8, 81u8, 116u8, 243u8, 236u8, 174u8, 130u8, 8u8, 246u8,
                            254u8, 97u8, 155u8, 207u8, 123u8, 60u8, 164u8, 14u8, 196u8, 97u8,
                        ],
                    )
                }
                ///Overwrites the number of pages of messages which must be in the queue after
                /// which we drop any further messages from the channel.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.drop_threshold`
                pub fn update_drop_threshold(
                    &self,
                    new: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<UpdateDropThreshold> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_drop_threshold",
                        UpdateDropThreshold { new },
                        [
                            146u8, 177u8, 164u8, 96u8, 247u8, 182u8, 229u8, 175u8, 194u8, 101u8,
                            186u8, 168u8, 94u8, 114u8, 172u8, 119u8, 35u8, 222u8, 175u8, 21u8,
                            67u8, 61u8, 216u8, 144u8, 194u8, 10u8, 181u8, 62u8, 166u8, 198u8,
                            138u8, 243u8,
                        ],
                    )
                }
                ///Overwrites the number of pages of messages which the queue must be reduced to
                /// before it signals that message sending may recommence after it
                /// has been suspended.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.resume_threshold`
                pub fn update_resume_threshold(
                    &self,
                    new: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<UpdateResumeThreshold> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_resume_threshold",
                        UpdateResumeThreshold { new },
                        [
                            231u8, 128u8, 80u8, 179u8, 61u8, 50u8, 103u8, 209u8, 103u8, 55u8,
                            101u8, 113u8, 150u8, 10u8, 202u8, 7u8, 0u8, 77u8, 58u8, 4u8, 227u8,
                            17u8, 225u8, 112u8, 121u8, 203u8, 184u8, 113u8, 231u8, 156u8, 174u8,
                            154u8,
                        ],
                    )
                }
                ///Overwrites the amount of remaining weight under which we stop processing
                /// messages.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.threshold_weight`
                pub fn update_threshold_weight(
                    &self,
                    new: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<UpdateThresholdWeight> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_threshold_weight",
                        UpdateThresholdWeight { new },
                        [
                            14u8, 144u8, 112u8, 207u8, 195u8, 208u8, 184u8, 164u8, 94u8, 41u8, 8u8,
                            58u8, 180u8, 80u8, 239u8, 39u8, 210u8, 159u8, 114u8, 169u8, 152u8,
                            176u8, 26u8, 161u8, 32u8, 43u8, 250u8, 156u8, 56u8, 21u8, 43u8, 159u8,
                        ],
                    )
                }
                ///Overwrites the speed to which the available weight approaches the maximum
                /// weight. A lower number results in a faster progression. A value
                /// of 1 makes the entire weight available initially.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.weight_restrict_decay`.
                pub fn update_weight_restrict_decay(
                    &self,
                    new: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<UpdateWeightRestrictDecay> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_weight_restrict_decay",
                        UpdateWeightRestrictDecay { new },
                        [
                            42u8, 53u8, 83u8, 191u8, 51u8, 227u8, 210u8, 193u8, 142u8, 218u8,
                            244u8, 177u8, 19u8, 87u8, 148u8, 177u8, 231u8, 197u8, 196u8, 255u8,
                            41u8, 130u8, 245u8, 139u8, 107u8, 212u8, 90u8, 161u8, 82u8, 248u8,
                            160u8, 223u8,
                        ],
                    )
                }
                ///Overwrite the maximum amount of weight any individual message may consume.
                ///Messages above this weight go into the overweight queue and may only be serviced
                /// explicitly.
                ///
                /// - `origin`: Must pass `Root`.
                /// - `new`: Desired value for `QueueConfigData.xcmp_max_individual_weight`.
                pub fn update_xcmp_max_individual_weight(
                    &self,
                    new: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<UpdateXcmpMaxIndividualWeight> {
                    ::subxt::tx::Payload::new_static(
                        "XcmpQueue",
                        "update_xcmp_max_individual_weight",
                        UpdateXcmpMaxIndividualWeight { new },
                        [
                            148u8, 185u8, 89u8, 36u8, 152u8, 220u8, 248u8, 233u8, 236u8, 82u8,
                            170u8, 111u8, 225u8, 142u8, 25u8, 211u8, 72u8, 248u8, 250u8, 14u8,
                            45u8, 72u8, 78u8, 95u8, 92u8, 196u8, 245u8, 104u8, 112u8, 128u8, 27u8,
                            109u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::cumulus_pallet_xcmp_queue::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some XCM was executed ok.
            pub struct Success {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                pub weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for Success {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "Success";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some XCM failed.
            pub struct Fail {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                pub error: runtime_types::xcm::v3::traits::Error,
                pub weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for Fail {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "Fail";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Bad XCM version used.
            pub struct BadVersion {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for BadVersion {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "BadVersion";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Bad XCM format used.
            pub struct BadFormat {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for BadFormat {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "BadFormat";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An HRMP message was sent to a sibling parachain.
            pub struct XcmpMessageSent {
                pub message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
            }
            impl ::subxt::events::StaticEvent for XcmpMessageSent {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "XcmpMessageSent";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An XCM exceeded the individual message weight budget.
            pub struct OverweightEnqueued {
                pub sender: runtime_types::polkadot_parachain::primitives::Id,
                pub sent_at: ::core::primitive::u32,
                pub index: ::core::primitive::u64,
                pub required: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for OverweightEnqueued {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "OverweightEnqueued";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An XCM from the overweight queue was executed with the given actual weight used.
            pub struct OverweightServiced {
                pub index: ::core::primitive::u64,
                pub used: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for OverweightServiced {
                const PALLET: &'static str = "XcmpQueue";
                const EVENT: &'static str = "OverweightServiced";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Status of the inbound XCMP channels.
                pub fn inbound_xcmp_status(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<
                        runtime_types::cumulus_pallet_xcmp_queue::InboundChannelDetails,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "InboundXcmpStatus",
                        vec![],
                        [
                            183u8, 198u8, 237u8, 153u8, 132u8, 201u8, 87u8, 182u8, 121u8, 164u8,
                            129u8, 241u8, 58u8, 192u8, 115u8, 152u8, 7u8, 33u8, 95u8, 51u8, 2u8,
                            176u8, 144u8, 12u8, 125u8, 83u8, 92u8, 198u8, 211u8, 101u8, 28u8, 50u8,
                        ],
                    )
                }
                /// Inbound aggregate XCMP messages. It can only be one per ParaId/block.
                pub fn inbound_xcmp_messages(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::polkadot_parachain::primitives::Id>,
                    _1: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "InboundXcmpMessages",
                        vec![
                            ::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
                            ::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
                        ],
                        [
                            157u8, 232u8, 222u8, 97u8, 218u8, 96u8, 96u8, 90u8, 216u8, 205u8, 39u8,
                            130u8, 109u8, 152u8, 127u8, 57u8, 54u8, 63u8, 104u8, 135u8, 33u8,
                            175u8, 197u8, 166u8, 238u8, 22u8, 137u8, 162u8, 226u8, 199u8, 87u8,
                            25u8,
                        ],
                    )
                }
                /// Inbound aggregate XCMP messages. It can only be one per ParaId/block.
                pub fn inbound_xcmp_messages_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "InboundXcmpMessages",
                        Vec::new(),
                        [
                            157u8, 232u8, 222u8, 97u8, 218u8, 96u8, 96u8, 90u8, 216u8, 205u8, 39u8,
                            130u8, 109u8, 152u8, 127u8, 57u8, 54u8, 63u8, 104u8, 135u8, 33u8,
                            175u8, 197u8, 166u8, 238u8, 22u8, 137u8, 162u8, 226u8, 199u8, 87u8,
                            25u8,
                        ],
                    )
                }
                /// The non-empty XCMP channels in order of becoming non-empty, and the index of the
                /// first and last outbound message. If the two indices are equal,
                /// then it indicates an empty queue and there must be a non-`Ok`
                /// `OutboundStatus`. We assume queues grow no greater than 65535
                /// items. Queue indices for normal messages begin at one; zero is reserved in
                /// case of the need to send a high-priority signal message this block.
                /// The bool is true if there is a signal message waiting to be sent.
                pub fn outbound_xcmp_status(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<
                        runtime_types::cumulus_pallet_xcmp_queue::OutboundChannelDetails,
                    >,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "OutboundXcmpStatus",
                        vec![],
                        [
                            238u8, 120u8, 185u8, 141u8, 82u8, 159u8, 41u8, 68u8, 204u8, 15u8, 46u8,
                            152u8, 144u8, 74u8, 250u8, 83u8, 71u8, 105u8, 54u8, 53u8, 226u8, 87u8,
                            14u8, 202u8, 58u8, 160u8, 54u8, 162u8, 239u8, 248u8, 227u8, 116u8,
                        ],
                    )
                }
                /// The messages outbound in a given XCMP channel.
                pub fn outbound_xcmp_messages(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::polkadot_parachain::primitives::Id>,
                    _1: impl ::std::borrow::Borrow<::core::primitive::u16>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "OutboundXcmpMessages",
                        vec![
                            ::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
                            ::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
                        ],
                        [
                            50u8, 182u8, 237u8, 191u8, 106u8, 67u8, 54u8, 1u8, 17u8, 107u8, 70u8,
                            90u8, 202u8, 8u8, 63u8, 184u8, 171u8, 111u8, 192u8, 196u8, 7u8, 31u8,
                            186u8, 68u8, 31u8, 63u8, 71u8, 61u8, 83u8, 223u8, 79u8, 200u8,
                        ],
                    )
                }
                /// The messages outbound in a given XCMP channel.
                pub fn outbound_xcmp_messages_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "OutboundXcmpMessages",
                        Vec::new(),
                        [
                            50u8, 182u8, 237u8, 191u8, 106u8, 67u8, 54u8, 1u8, 17u8, 107u8, 70u8,
                            90u8, 202u8, 8u8, 63u8, 184u8, 171u8, 111u8, 192u8, 196u8, 7u8, 31u8,
                            186u8, 68u8, 31u8, 63u8, 71u8, 61u8, 83u8, 223u8, 79u8, 200u8,
                        ],
                    )
                }
                /// Any signal messages waiting to be sent.
                pub fn signal_messages(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::polkadot_parachain::primitives::Id>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "SignalMessages",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            156u8, 242u8, 186u8, 89u8, 177u8, 195u8, 90u8, 121u8, 94u8, 106u8,
                            222u8, 78u8, 19u8, 162u8, 179u8, 96u8, 38u8, 113u8, 209u8, 148u8, 29u8,
                            110u8, 106u8, 167u8, 162u8, 96u8, 221u8, 20u8, 33u8, 179u8, 168u8,
                            142u8,
                        ],
                    )
                }
                /// Any signal messages waiting to be sent.
                pub fn signal_messages_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "SignalMessages",
                        Vec::new(),
                        [
                            156u8, 242u8, 186u8, 89u8, 177u8, 195u8, 90u8, 121u8, 94u8, 106u8,
                            222u8, 78u8, 19u8, 162u8, 179u8, 96u8, 38u8, 113u8, 209u8, 148u8, 29u8,
                            110u8, 106u8, 167u8, 162u8, 96u8, 221u8, 20u8, 33u8, 179u8, 168u8,
                            142u8,
                        ],
                    )
                }
                /// The configuration which controls the dynamics of the outbound queue.
                pub fn queue_config(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_pallet_xcmp_queue::QueueConfigData,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "QueueConfig",
                        vec![],
                        [
                            154u8, 172u8, 227u8, 208u8, 130u8, 93u8, 173u8, 129u8, 33u8, 75u8,
                            180u8, 100u8, 35u8, 154u8, 40u8, 188u8, 86u8, 53u8, 74u8, 118u8, 131u8,
                            159u8, 240u8, 159u8, 185u8, 45u8, 165u8, 6u8, 90u8, 125u8, 77u8, 253u8,
                        ],
                    )
                }
                /// The messages that exceeded max individual message weight budget.
                ///
                /// These message stay in this storage map until they are manually dispatched via
                /// `service_overweight`.
                pub fn overweight(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u64>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    (
                        runtime_types::polkadot_parachain::primitives::Id,
                        ::core::primitive::u32,
                        ::std::vec::Vec<::core::primitive::u8>,
                    ),
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "Overweight",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            222u8, 249u8, 232u8, 110u8, 117u8, 229u8, 165u8, 164u8, 219u8, 219u8,
                            149u8, 204u8, 25u8, 78u8, 204u8, 116u8, 111u8, 114u8, 120u8, 222u8,
                            56u8, 77u8, 122u8, 147u8, 108u8, 15u8, 94u8, 161u8, 212u8, 50u8, 7u8,
                            7u8,
                        ],
                    )
                }
                /// The messages that exceeded max individual message weight budget.
                ///
                /// These message stay in this storage map until they are manually dispatched via
                /// `service_overweight`.
                pub fn overweight_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    (
                        runtime_types::polkadot_parachain::primitives::Id,
                        ::core::primitive::u32,
                        ::std::vec::Vec<::core::primitive::u8>,
                    ),
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "Overweight",
                        Vec::new(),
                        [
                            222u8, 249u8, 232u8, 110u8, 117u8, 229u8, 165u8, 164u8, 219u8, 219u8,
                            149u8, 204u8, 25u8, 78u8, 204u8, 116u8, 111u8, 114u8, 120u8, 222u8,
                            56u8, 77u8, 122u8, 147u8, 108u8, 15u8, 94u8, 161u8, 212u8, 50u8, 7u8,
                            7u8,
                        ],
                    )
                }
                ///Counter for the related counted storage map
                pub fn counter_for_overweight(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "CounterForOverweight",
                        vec![],
                        [
                            148u8, 226u8, 248u8, 107u8, 165u8, 97u8, 218u8, 160u8, 127u8, 48u8,
                            185u8, 251u8, 35u8, 137u8, 119u8, 251u8, 151u8, 167u8, 189u8, 66u8,
                            80u8, 74u8, 134u8, 129u8, 222u8, 180u8, 51u8, 182u8, 50u8, 110u8, 10u8,
                            43u8,
                        ],
                    )
                }
                /// The number of overweight messages ever recorded in `Overweight`. Also doubles as
                /// the next available free overweight index.
                pub fn overweight_count(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "OverweightCount",
                        vec![],
                        [
                            102u8, 180u8, 196u8, 148u8, 115u8, 62u8, 46u8, 238u8, 97u8, 116u8,
                            117u8, 42u8, 14u8, 5u8, 72u8, 237u8, 230u8, 46u8, 150u8, 126u8, 89u8,
                            64u8, 233u8, 166u8, 180u8, 137u8, 52u8, 233u8, 252u8, 255u8, 36u8,
                            20u8,
                        ],
                    )
                }
                /// Whether or not the XCMP queue is suspended from executing incoming XCMs or not.
                pub fn queue_suspended(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::bool,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "XcmpQueue",
                        "QueueSuspended",
                        vec![],
                        [
                            23u8, 37u8, 48u8, 112u8, 222u8, 17u8, 252u8, 65u8, 160u8, 217u8, 218u8,
                            30u8, 2u8, 1u8, 204u8, 0u8, 251u8, 17u8, 138u8, 197u8, 164u8, 50u8,
                            122u8, 0u8, 31u8, 238u8, 147u8, 213u8, 30u8, 132u8, 184u8, 215u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod polkadot_xcm {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Send {
                pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct TeleportAssets {
                pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                pub fee_asset_item: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ReserveTransferAssets {
                pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                pub fee_asset_item: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Execute {
                pub message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
                pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceXcmVersion {
                pub location:
                    ::std::boxed::Box<runtime_types::xcm::v3::multilocation::MultiLocation>,
                pub xcm_version: ::core::primitive::u32,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceDefaultXcmVersion {
                pub maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceSubscribeVersionNotify {
                pub location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ForceUnsubscribeVersionNotify {
                pub location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct LimitedReserveTransferAssets {
                pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                pub fee_asset_item: ::core::primitive::u32,
                pub weight_limit: runtime_types::xcm::v3::WeightLimit,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct LimitedTeleportAssets {
                pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                pub fee_asset_item: ::core::primitive::u32,
                pub weight_limit: runtime_types::xcm::v3::WeightLimit,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                pub fn send(
                    &self,
                    dest: runtime_types::xcm::VersionedMultiLocation,
                    message: runtime_types::xcm::VersionedXcm,
                ) -> ::subxt::tx::Payload<Send> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "send",
                        Send {
                            dest: ::std::boxed::Box::new(dest),
                            message: ::std::boxed::Box::new(message),
                        },
                        [
                            246u8, 35u8, 227u8, 112u8, 223u8, 7u8, 44u8, 186u8, 60u8, 225u8, 153u8,
                            249u8, 104u8, 51u8, 123u8, 227u8, 143u8, 65u8, 232u8, 209u8, 178u8,
                            104u8, 70u8, 56u8, 230u8, 14u8, 75u8, 83u8, 250u8, 160u8, 9u8, 39u8,
                        ],
                    )
                }
                ///Teleport some assets from the local chain to some destination chain.
                ///
                ///Fee payment on the destination side is made from the asset in the `assets`
                /// vector of index `fee_asset_item`. The weight limit for fees is
                /// not provided and thus is unlimited, with all fees taken as
                /// needed from the asset.
                ///
                /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                ///   Parachain(..))` to send
                ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                /// parachain.
                /// - `beneficiary`: A beneficiary location for the assets in the context of `dest`.
                ///   Will generally be
                ///  an `AccountId32` value.
                /// - `assets`: The assets to be withdrawn. The first item should be the currency
                ///   used to to pay the fee on the
                ///  `dest` side. May not be empty.
                /// - `fee_asset_item`: The index into `assets` of the item which should be used to
                ///   pay
                ///  fees.
                pub fn teleport_assets(
                    &self,
                    dest: runtime_types::xcm::VersionedMultiLocation,
                    beneficiary: runtime_types::xcm::VersionedMultiLocation,
                    assets: runtime_types::xcm::VersionedMultiAssets,
                    fee_asset_item: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<TeleportAssets> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "teleport_assets",
                        TeleportAssets {
                            dest: ::std::boxed::Box::new(dest),
                            beneficiary: ::std::boxed::Box::new(beneficiary),
                            assets: ::std::boxed::Box::new(assets),
                            fee_asset_item,
                        },
                        [
                            187u8, 42u8, 2u8, 96u8, 105u8, 125u8, 74u8, 53u8, 2u8, 21u8, 31u8,
                            160u8, 201u8, 197u8, 157u8, 190u8, 40u8, 145u8, 5u8, 99u8, 194u8, 41u8,
                            114u8, 60u8, 165u8, 186u8, 15u8, 226u8, 85u8, 113u8, 159u8, 136u8,
                        ],
                    )
                }
                ///Transfer some assets from the local chain to the sovereign account of a
                /// destination chain and forward a notification XCM.
                ///
                ///Fee payment on the destination side is made from the asset in the `assets`
                /// vector of index `fee_asset_item`. The weight limit for fees is
                /// not provided and thus is unlimited, with all fees taken as
                /// needed from the asset.
                ///
                /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                ///   Parachain(..))` to send
                ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                /// parachain.
                /// - `beneficiary`: A beneficiary location for the assets in the context of `dest`.
                ///   Will generally be
                ///  an `AccountId32` value.
                /// - `assets`: The assets to be withdrawn. This should include the assets used to
                ///   pay the fee on the
                ///  `dest` side.
                /// - `fee_asset_item`: The index into `assets` of the item which should be used to
                ///   pay
                ///  fees.
                pub fn reserve_transfer_assets(
                    &self,
                    dest: runtime_types::xcm::VersionedMultiLocation,
                    beneficiary: runtime_types::xcm::VersionedMultiLocation,
                    assets: runtime_types::xcm::VersionedMultiAssets,
                    fee_asset_item: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<ReserveTransferAssets> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "reserve_transfer_assets",
                        ReserveTransferAssets {
                            dest: ::std::boxed::Box::new(dest),
                            beneficiary: ::std::boxed::Box::new(beneficiary),
                            assets: ::std::boxed::Box::new(assets),
                            fee_asset_item,
                        },
                        [
                            249u8, 177u8, 76u8, 204u8, 186u8, 165u8, 16u8, 186u8, 129u8, 239u8,
                            65u8, 252u8, 9u8, 132u8, 32u8, 164u8, 117u8, 177u8, 40u8, 21u8, 196u8,
                            246u8, 147u8, 2u8, 95u8, 110u8, 68u8, 162u8, 148u8, 9u8, 59u8, 170u8,
                        ],
                    )
                }
                ///Execute an XCM message from a local, signed, origin.
                ///
                ///An event is deposited indicating whether `msg` could be executed completely or
                /// only partially.
                ///
                ///No more than `max_weight` will be used in its attempted execution. If this is
                /// less than the maximum amount of weight that the message could
                /// take to be executed, then no execution attempt will be made.
                ///
                ///NOTE: A successful return to this does *not* imply that the `msg` was executed
                /// successfully to completion; only that *some* of it was executed.
                pub fn execute(
                    &self,
                    message: runtime_types::xcm::VersionedXcm,
                    max_weight: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<Execute> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "execute",
                        Execute { message: ::std::boxed::Box::new(message), max_weight },
                        [
                            102u8, 41u8, 146u8, 29u8, 241u8, 205u8, 95u8, 153u8, 228u8, 141u8,
                            11u8, 228u8, 13u8, 44u8, 75u8, 204u8, 174u8, 35u8, 155u8, 104u8, 204u8,
                            82u8, 239u8, 98u8, 249u8, 187u8, 193u8, 1u8, 122u8, 88u8, 162u8, 200u8,
                        ],
                    )
                }
                ///Extoll that a particular destination can be communicated with through a
                /// particular version of XCM.
                ///
                /// - `origin`: Must be Root.
                /// - `location`: The destination that is being described.
                /// - `xcm_version`: The latest version of XCM that `location` supports.
                pub fn force_xcm_version(
                    &self,
                    location: runtime_types::xcm::v3::multilocation::MultiLocation,
                    xcm_version: ::core::primitive::u32,
                ) -> ::subxt::tx::Payload<ForceXcmVersion> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "force_xcm_version",
                        ForceXcmVersion { location: ::std::boxed::Box::new(location), xcm_version },
                        [
                            68u8, 48u8, 95u8, 61u8, 152u8, 95u8, 213u8, 126u8, 209u8, 176u8, 230u8,
                            160u8, 164u8, 42u8, 128u8, 62u8, 175u8, 3u8, 161u8, 170u8, 20u8, 31u8,
                            216u8, 122u8, 31u8, 77u8, 64u8, 182u8, 121u8, 41u8, 23u8, 80u8,
                        ],
                    )
                }
                ///Set a safe XCM version (the version that XCM should be encoded with if the most
                /// recent version a destination can accept is unknown).
                ///
                /// - `origin`: Must be Root.
                /// - `maybe_xcm_version`: The default XCM encoding version, or `None` to disable.
                pub fn force_default_xcm_version(
                    &self,
                    maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
                ) -> ::subxt::tx::Payload<ForceDefaultXcmVersion> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "force_default_xcm_version",
                        ForceDefaultXcmVersion { maybe_xcm_version },
                        [
                            38u8, 36u8, 59u8, 231u8, 18u8, 79u8, 76u8, 9u8, 200u8, 125u8, 214u8,
                            166u8, 37u8, 99u8, 111u8, 161u8, 135u8, 2u8, 133u8, 157u8, 165u8, 18u8,
                            152u8, 81u8, 209u8, 255u8, 137u8, 237u8, 28u8, 126u8, 224u8, 141u8,
                        ],
                    )
                }
                ///Ask a location to notify us regarding their XCM version and any changes to it.
                ///
                /// - `origin`: Must be Root.
                /// - `location`: The location to which we should subscribe for XCM version
                ///   notifications.
                pub fn force_subscribe_version_notify(
                    &self,
                    location: runtime_types::xcm::VersionedMultiLocation,
                ) -> ::subxt::tx::Payload<ForceSubscribeVersionNotify> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "force_subscribe_version_notify",
                        ForceSubscribeVersionNotify { location: ::std::boxed::Box::new(location) },
                        [
                            236u8, 37u8, 153u8, 26u8, 174u8, 187u8, 154u8, 38u8, 179u8, 223u8,
                            130u8, 32u8, 128u8, 30u8, 148u8, 229u8, 7u8, 185u8, 174u8, 9u8, 96u8,
                            215u8, 189u8, 178u8, 148u8, 141u8, 249u8, 118u8, 7u8, 238u8, 1u8, 49u8,
                        ],
                    )
                }
                ///Require that a particular destination should no longer notify us regarding any
                /// XCM version changes.
                ///
                /// - `origin`: Must be Root.
                /// - `location`: The location to which we are currently subscribed for XCM version
                ///  notifications which we no longer desire.
                pub fn force_unsubscribe_version_notify(
                    &self,
                    location: runtime_types::xcm::VersionedMultiLocation,
                ) -> ::subxt::tx::Payload<ForceUnsubscribeVersionNotify> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "force_unsubscribe_version_notify",
                        ForceUnsubscribeVersionNotify {
                            location: ::std::boxed::Box::new(location),
                        },
                        [
                            154u8, 169u8, 145u8, 211u8, 185u8, 71u8, 9u8, 63u8, 3u8, 158u8, 187u8,
                            173u8, 115u8, 166u8, 100u8, 66u8, 12u8, 40u8, 198u8, 40u8, 213u8,
                            104u8, 95u8, 183u8, 215u8, 53u8, 94u8, 158u8, 106u8, 56u8, 149u8, 52u8,
                        ],
                    )
                }
                ///Transfer some assets from the local chain to the sovereign account of a
                /// destination chain and forward a notification XCM.
                ///
                ///Fee payment on the destination side is made from the asset in the `assets`
                /// vector of index `fee_asset_item`, up to enough to pay for
                /// `weight_limit` of weight. If more weight is needed than
                /// `weight_limit`, then the operation will fail and the assets send may be
                /// at risk.
                ///
                /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                ///   Parachain(..))` to send
                ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                /// parachain.
                /// - `beneficiary`: A beneficiary location for the assets in the context of `dest`.
                ///   Will generally be
                ///  an `AccountId32` value.
                /// - `assets`: The assets to be withdrawn. This should include the assets used to
                ///   pay the fee on the
                ///  `dest` side.
                /// - `fee_asset_item`: The index into `assets` of the item which should be used to
                ///   pay
                ///  fees.
                /// - `weight_limit`: The remote-side weight limit, if any, for the XCM fee
                ///   purchase.
                pub fn limited_reserve_transfer_assets(
                    &self,
                    dest: runtime_types::xcm::VersionedMultiLocation,
                    beneficiary: runtime_types::xcm::VersionedMultiLocation,
                    assets: runtime_types::xcm::VersionedMultiAssets,
                    fee_asset_item: ::core::primitive::u32,
                    weight_limit: runtime_types::xcm::v3::WeightLimit,
                ) -> ::subxt::tx::Payload<LimitedReserveTransferAssets> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "limited_reserve_transfer_assets",
                        LimitedReserveTransferAssets {
                            dest: ::std::boxed::Box::new(dest),
                            beneficiary: ::std::boxed::Box::new(beneficiary),
                            assets: ::std::boxed::Box::new(assets),
                            fee_asset_item,
                            weight_limit,
                        },
                        [
                            131u8, 191u8, 89u8, 27u8, 236u8, 142u8, 130u8, 129u8, 245u8, 95u8,
                            159u8, 96u8, 252u8, 80u8, 28u8, 40u8, 128u8, 55u8, 41u8, 123u8, 22u8,
                            18u8, 0u8, 236u8, 77u8, 68u8, 135u8, 181u8, 40u8, 47u8, 92u8, 240u8,
                        ],
                    )
                }
                ///Teleport some assets from the local chain to some destination chain.
                ///
                ///Fee payment on the destination side is made from the asset in the `assets`
                /// vector of index `fee_asset_item`, up to enough to pay for
                /// `weight_limit` of weight. If more weight is needed than
                /// `weight_limit`, then the operation will fail and the assets send may be
                /// at risk.
                ///
                /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                ///   Parachain(..))` to send
                ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                /// parachain.
                /// - `beneficiary`: A beneficiary location for the assets in the context of `dest`.
                ///   Will generally be
                ///  an `AccountId32` value.
                /// - `assets`: The assets to be withdrawn. The first item should be the currency
                ///   used to to pay the fee on the
                ///  `dest` side. May not be empty.
                /// - `fee_asset_item`: The index into `assets` of the item which should be used to
                ///   pay
                ///  fees.
                /// - `weight_limit`: The remote-side weight limit, if any, for the XCM fee
                ///   purchase.
                pub fn limited_teleport_assets(
                    &self,
                    dest: runtime_types::xcm::VersionedMultiLocation,
                    beneficiary: runtime_types::xcm::VersionedMultiLocation,
                    assets: runtime_types::xcm::VersionedMultiAssets,
                    fee_asset_item: ::core::primitive::u32,
                    weight_limit: runtime_types::xcm::v3::WeightLimit,
                ) -> ::subxt::tx::Payload<LimitedTeleportAssets> {
                    ::subxt::tx::Payload::new_static(
                        "PolkadotXcm",
                        "limited_teleport_assets",
                        LimitedTeleportAssets {
                            dest: ::std::boxed::Box::new(dest),
                            beneficiary: ::std::boxed::Box::new(beneficiary),
                            assets: ::std::boxed::Box::new(assets),
                            fee_asset_item,
                            weight_limit,
                        },
                        [
                            234u8, 19u8, 104u8, 174u8, 98u8, 159u8, 205u8, 110u8, 240u8, 78u8,
                            186u8, 138u8, 236u8, 116u8, 104u8, 215u8, 57u8, 178u8, 166u8, 208u8,
                            197u8, 113u8, 101u8, 56u8, 23u8, 56u8, 84u8, 14u8, 173u8, 70u8, 211u8,
                            201u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::pallet_xcm::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Execution of an XCM message was attempted.
            ///
            ///\[ outcome \]
            pub struct Attempted(pub runtime_types::xcm::v3::traits::Outcome);
            impl ::subxt::events::StaticEvent for Attempted {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "Attempted";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A XCM message was sent.
            ///
            ///\[ origin, destination, message \]
            pub struct Sent(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::Xcm,
            );
            impl ::subxt::events::StaticEvent for Sent {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "Sent";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response received which does not match a registered query. This may be because
            /// a matching query was never registered, it may be because it is a
            /// duplicate response, or because the query timed out.
            ///
            ///\[ origin location, id \]
            pub struct UnexpectedResponse(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
            );
            impl ::subxt::events::StaticEvent for UnexpectedResponse {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "UnexpectedResponse";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response has been received and is ready for taking with `take_response`. There
            /// is no registered notification call.
            ///
            ///\[ id, response \]
            pub struct ResponseReady(
                pub ::core::primitive::u64,
                pub runtime_types::xcm::v3::Response,
            );
            impl ::subxt::events::StaticEvent for ResponseReady {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "ResponseReady";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response has been received and query is removed. The registered notification
            /// has been dispatched and executed successfully.
            ///
            ///\[ id, pallet index, call index \]
            pub struct Notified(
                pub ::core::primitive::u64,
                pub ::core::primitive::u8,
                pub ::core::primitive::u8,
            );
            impl ::subxt::events::StaticEvent for Notified {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "Notified";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response has been received and query is removed. The registered notification
            /// could not be dispatched because the dispatch weight is greater than the
            /// maximum weight originally budgeted by this runtime for the query result.
            ///
            ///\[ id, pallet index, call index, actual weight, max budgeted weight \]
            pub struct NotifyOverweight(
                pub ::core::primitive::u64,
                pub ::core::primitive::u8,
                pub ::core::primitive::u8,
                pub runtime_types::sp_weights::weight_v2::Weight,
                pub runtime_types::sp_weights::weight_v2::Weight,
            );
            impl ::subxt::events::StaticEvent for NotifyOverweight {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "NotifyOverweight";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response has been received and query is removed. There was a general error
            /// with dispatching the notification call.
            ///
            ///\[ id, pallet index, call index \]
            pub struct NotifyDispatchError(
                pub ::core::primitive::u64,
                pub ::core::primitive::u8,
                pub ::core::primitive::u8,
            );
            impl ::subxt::events::StaticEvent for NotifyDispatchError {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "NotifyDispatchError";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Query response has been received and query is removed. The dispatch was unable to be
            ///decoded into a `Call`; this might be due to dispatch function having a signature
            /// which is not `(origin, QueryId, Response)`.
            ///
            ///\[ id, pallet index, call index \]
            pub struct NotifyDecodeFailed(
                pub ::core::primitive::u64,
                pub ::core::primitive::u8,
                pub ::core::primitive::u8,
            );
            impl ::subxt::events::StaticEvent for NotifyDecodeFailed {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "NotifyDecodeFailed";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Expected query response has been received but the origin location of the response
            /// does not match that expected. The query remains registered for a later,
            /// valid, response to be received and acted upon.
            ///
            ///\[ origin location, id, expected location \]
            pub struct InvalidResponder(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
                pub ::core::option::Option<runtime_types::xcm::v3::multilocation::MultiLocation>,
            );
            impl ::subxt::events::StaticEvent for InvalidResponder {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "InvalidResponder";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Expected query response has been received but the expected origin location placed in
            ///storage by this runtime previously cannot be decoded. The query remains registered.
            ///
            ///This is unexpected (since a location placed in storage in a previously executing
            ///runtime should be readable prior to query timeout) and dangerous since the possibly
            ///valid response will be dropped. Manual governance intervention is probably going to
            /// be needed.
            ///
            ///\[ origin location, id \]
            pub struct InvalidResponderVersion(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
            );
            impl ::subxt::events::StaticEvent for InvalidResponderVersion {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "InvalidResponderVersion";
            }
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Received query response has been read and removed.
            ///
            ///\[ id \]
            pub struct ResponseTaken(pub ::core::primitive::u64);
            impl ::subxt::events::StaticEvent for ResponseTaken {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "ResponseTaken";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some assets have been placed in an asset trap.
            ///
            ///\[ hash, origin, assets \]
            pub struct AssetsTrapped(
                pub ::subxt::utils::H256,
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::VersionedMultiAssets,
            );
            impl ::subxt::events::StaticEvent for AssetsTrapped {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "AssetsTrapped";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///An XCM version change notification message has been attempted to be sent.
            ///
            ///The cost of sending it (borne by the chain) is included.
            ///
            ///\[ destination, result, cost \]
            pub struct VersionChangeNotified(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u32,
                pub runtime_types::xcm::v3::multiasset::MultiAssets,
            );
            impl ::subxt::events::StaticEvent for VersionChangeNotified {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "VersionChangeNotified";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The supported version of a location has been changed. This might be through an
            ///automatic notification or a manual intervention.
            ///
            ///\[ location, XCM version \]
            pub struct SupportedVersionChanged(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u32,
            );
            impl ::subxt::events::StaticEvent for SupportedVersionChanged {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "SupportedVersionChanged";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A given location which had a version change subscription was dropped owing to an
            /// error sending the notification to it.
            ///
            ///\[ location, query ID, error \]
            pub struct NotifyTargetSendFail(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
                pub runtime_types::xcm::v3::traits::Error,
            );
            impl ::subxt::events::StaticEvent for NotifyTargetSendFail {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "NotifyTargetSendFail";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A given location which had a version change subscription was dropped owing to an
            /// error migrating the location to our new XCM format.
            ///
            ///\[ location, query ID \]
            pub struct NotifyTargetMigrationFail(
                pub runtime_types::xcm::VersionedMultiLocation,
                pub ::core::primitive::u64,
            );
            impl ::subxt::events::StaticEvent for NotifyTargetMigrationFail {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "NotifyTargetMigrationFail";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Expected query response has been received but the expected querier location placed
            /// in storage by this runtime previously cannot be decoded. The query
            /// remains registered.
            ///
            ///This is unexpected (since a location placed in storage in a previously executing
            ///runtime should be readable prior to query timeout) and dangerous since the possibly
            ///valid response will be dropped. Manual governance intervention is probably going to
            /// be needed.
            ///
            ///\[ origin location, id \]
            pub struct InvalidQuerierVersion(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
            );
            impl ::subxt::events::StaticEvent for InvalidQuerierVersion {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "InvalidQuerierVersion";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Expected query response has been received but the querier location of the response
            /// does not match the expected. The query remains registered for a later,
            /// valid, response to be received and acted upon.
            ///
            ///\[ origin location, id, expected querier, maybe actual querier \]
            pub struct InvalidQuerier(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::primitive::u64,
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub ::core::option::Option<runtime_types::xcm::v3::multilocation::MultiLocation>,
            );
            impl ::subxt::events::StaticEvent for InvalidQuerier {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "InvalidQuerier";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///A remote has requested XCM version change notification from us and we have honored
            /// it. A version information message is sent to them and its cost is
            /// included.
            ///
            ///\[ destination location, cost \]
            pub struct VersionNotifyStarted(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::multiasset::MultiAssets,
            );
            impl ::subxt::events::StaticEvent for VersionNotifyStarted {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "VersionNotifyStarted";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///We have requested that a remote chain sends us XCM version change notifications.
            ///
            ///\[ destination location, cost \]
            pub struct VersionNotifyRequested(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::multiasset::MultiAssets,
            );
            impl ::subxt::events::StaticEvent for VersionNotifyRequested {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "VersionNotifyRequested";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///We have requested that a remote chain stops sending us XCM version change
            /// notifications.
            ///
            ///\[ destination location, cost \]
            pub struct VersionNotifyUnrequested(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::multiasset::MultiAssets,
            );
            impl ::subxt::events::StaticEvent for VersionNotifyUnrequested {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "VersionNotifyUnrequested";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Fees were paid from a location for an operation (often for using `SendXcm`).
            ///
            ///\[ paying location, fees \]
            pub struct FeesPaid(
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::v3::multiasset::MultiAssets,
            );
            impl ::subxt::events::StaticEvent for FeesPaid {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "FeesPaid";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some assets have been claimed from an asset trap
            ///
            ///\[ hash, origin, assets \]
            pub struct AssetsClaimed(
                pub ::subxt::utils::H256,
                pub runtime_types::xcm::v3::multilocation::MultiLocation,
                pub runtime_types::xcm::VersionedMultiAssets,
            );
            impl ::subxt::events::StaticEvent for AssetsClaimed {
                const PALLET: &'static str = "PolkadotXcm";
                const EVENT: &'static str = "AssetsClaimed";
            }
        }
    }
    pub mod cumulus_xcm {
        use super::{root_mod, runtime_types};
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::cumulus_pallet_xcm::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message is invalid XCM.
            ///\[ id \]
            pub struct InvalidFormat(pub [::core::primitive::u8; 32usize]);
            impl ::subxt::events::StaticEvent for InvalidFormat {
                const PALLET: &'static str = "CumulusXcm";
                const EVENT: &'static str = "InvalidFormat";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message is unsupported version of XCM.
            ///\[ id \]
            pub struct UnsupportedVersion(pub [::core::primitive::u8; 32usize]);
            impl ::subxt::events::StaticEvent for UnsupportedVersion {
                const PALLET: &'static str = "CumulusXcm";
                const EVENT: &'static str = "UnsupportedVersion";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message executed with the given outcome.
            ///\[ id, outcome \]
            pub struct ExecutedDownward(
                pub [::core::primitive::u8; 32usize],
                pub runtime_types::xcm::v3::traits::Outcome,
            );
            impl ::subxt::events::StaticEvent for ExecutedDownward {
                const PALLET: &'static str = "CumulusXcm";
                const EVENT: &'static str = "ExecutedDownward";
            }
        }
    }
    pub mod dmp_queue {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ServiceOverweight {
                pub index: ::core::primitive::u64,
                pub weight_limit: runtime_types::sp_weights::weight_v2::Weight,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Service a single overweight message.
                pub fn service_overweight(
                    &self,
                    index: ::core::primitive::u64,
                    weight_limit: runtime_types::sp_weights::weight_v2::Weight,
                ) -> ::subxt::tx::Payload<ServiceOverweight> {
                    ::subxt::tx::Payload::new_static(
                        "DmpQueue",
                        "service_overweight",
                        ServiceOverweight { index, weight_limit },
                        [
                            121u8, 236u8, 235u8, 23u8, 210u8, 238u8, 238u8, 122u8, 15u8, 86u8,
                            34u8, 119u8, 105u8, 100u8, 214u8, 236u8, 117u8, 39u8, 254u8, 235u8,
                            189u8, 15u8, 72u8, 74u8, 225u8, 134u8, 148u8, 126u8, 31u8, 203u8,
                            144u8, 106u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::cumulus_pallet_dmp_queue::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message is invalid XCM.
            pub struct InvalidFormat {
                pub message_id: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for InvalidFormat {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "InvalidFormat";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message is unsupported version of XCM.
            pub struct UnsupportedVersion {
                pub message_id: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for UnsupportedVersion {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "UnsupportedVersion";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message executed with the given outcome.
            pub struct ExecutedDownward {
                pub message_id: [::core::primitive::u8; 32usize],
                pub outcome: runtime_types::xcm::v3::traits::Outcome,
            }
            impl ::subxt::events::StaticEvent for ExecutedDownward {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "ExecutedDownward";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The weight limit for handling downward messages was reached.
            pub struct WeightExhausted {
                pub message_id: [::core::primitive::u8; 32usize],
                pub remaining_weight: runtime_types::sp_weights::weight_v2::Weight,
                pub required_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for WeightExhausted {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "WeightExhausted";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message is overweight and was placed in the overweight queue.
            pub struct OverweightEnqueued {
                pub message_id: [::core::primitive::u8; 32usize],
                pub overweight_index: ::core::primitive::u64,
                pub required_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for OverweightEnqueued {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "OverweightEnqueued";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Downward message from the overweight queue was executed.
            pub struct OverweightServiced {
                pub overweight_index: ::core::primitive::u64,
                pub weight_used: runtime_types::sp_weights::weight_v2::Weight,
            }
            impl ::subxt::events::StaticEvent for OverweightServiced {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "OverweightServiced";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///The maximum number of downward messages was.
            pub struct MaxMessagesExhausted {
                pub message_id: [::core::primitive::u8; 32usize],
            }
            impl ::subxt::events::StaticEvent for MaxMessagesExhausted {
                const PALLET: &'static str = "DmpQueue";
                const EVENT: &'static str = "MaxMessagesExhausted";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// The configuration.
                pub fn configuration(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_pallet_dmp_queue::ConfigData,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "Configuration",
                        vec![],
                        [
                            133u8, 113u8, 115u8, 164u8, 128u8, 145u8, 234u8, 106u8, 150u8, 54u8,
                            247u8, 135u8, 181u8, 197u8, 178u8, 30u8, 204u8, 46u8, 6u8, 137u8, 82u8,
                            1u8, 75u8, 171u8, 7u8, 157u8, 3u8, 19u8, 92u8, 10u8, 234u8, 66u8,
                        ],
                    )
                }
                /// The page index.
                pub fn page_index(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::cumulus_pallet_dmp_queue::PageIndexData,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "PageIndex",
                        vec![],
                        [
                            94u8, 132u8, 34u8, 67u8, 10u8, 22u8, 235u8, 96u8, 168u8, 26u8, 57u8,
                            200u8, 130u8, 218u8, 37u8, 71u8, 28u8, 119u8, 78u8, 107u8, 209u8,
                            120u8, 190u8, 2u8, 101u8, 215u8, 122u8, 187u8, 94u8, 38u8, 255u8,
                            234u8,
                        ],
                    )
                }
                /// The queue pages.
                pub fn pages(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(
                        ::core::primitive::u32,
                        ::std::vec::Vec<::core::primitive::u8>,
                    )>,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "Pages",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            228u8, 86u8, 33u8, 107u8, 248u8, 4u8, 223u8, 175u8, 222u8, 25u8, 204u8,
                            42u8, 235u8, 21u8, 215u8, 91u8, 167u8, 14u8, 133u8, 151u8, 190u8, 57u8,
                            138u8, 208u8, 79u8, 244u8, 132u8, 14u8, 48u8, 247u8, 171u8, 108u8,
                        ],
                    )
                }
                /// The queue pages.
                pub fn pages_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(
                        ::core::primitive::u32,
                        ::std::vec::Vec<::core::primitive::u8>,
                    )>,
                    (),
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "Pages",
                        Vec::new(),
                        [
                            228u8, 86u8, 33u8, 107u8, 248u8, 4u8, 223u8, 175u8, 222u8, 25u8, 204u8,
                            42u8, 235u8, 21u8, 215u8, 91u8, 167u8, 14u8, 133u8, 151u8, 190u8, 57u8,
                            138u8, 208u8, 79u8, 244u8, 132u8, 14u8, 48u8, 247u8, 171u8, 108u8,
                        ],
                    )
                }
                /// The overweight messages.
                pub fn overweight(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u64>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    (::core::primitive::u32, ::std::vec::Vec<::core::primitive::u8>),
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "Overweight",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            222u8, 85u8, 143u8, 49u8, 42u8, 248u8, 138u8, 163u8, 46u8, 199u8,
                            188u8, 61u8, 137u8, 135u8, 127u8, 146u8, 210u8, 254u8, 121u8, 42u8,
                            112u8, 114u8, 22u8, 228u8, 207u8, 207u8, 245u8, 175u8, 152u8, 140u8,
                            225u8, 237u8,
                        ],
                    )
                }
                /// The overweight messages.
                pub fn overweight_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    (::core::primitive::u32, ::std::vec::Vec<::core::primitive::u8>),
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "Overweight",
                        Vec::new(),
                        [
                            222u8, 85u8, 143u8, 49u8, 42u8, 248u8, 138u8, 163u8, 46u8, 199u8,
                            188u8, 61u8, 137u8, 135u8, 127u8, 146u8, 210u8, 254u8, 121u8, 42u8,
                            112u8, 114u8, 22u8, 228u8, 207u8, 207u8, 245u8, 175u8, 152u8, 140u8,
                            225u8, 237u8,
                        ],
                    )
                }
                ///Counter for the related counted storage map
                pub fn counter_for_overweight(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u32,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "DmpQueue",
                        "CounterForOverweight",
                        vec![],
                        [
                            148u8, 226u8, 248u8, 107u8, 165u8, 97u8, 218u8, 160u8, 127u8, 48u8,
                            185u8, 251u8, 35u8, 137u8, 119u8, 251u8, 151u8, 167u8, 189u8, 66u8,
                            80u8, 74u8, 134u8, 129u8, 222u8, 180u8, 51u8, 182u8, 50u8, 110u8, 10u8,
                            43u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod ismp {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Handle {
                pub messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct CreateConsensusClient {
                pub message: runtime_types::ismp::messaging::CreateConsensusClient,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                ///Handles ismp messages
                pub fn handle(
                    &self,
                    messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message>,
                ) -> ::subxt::tx::Payload<Handle> {
                    ::subxt::tx::Payload::new_static(
                        "Ismp",
                        "handle",
                        Handle { messages },
                        [
                            139u8, 102u8, 187u8, 247u8, 187u8, 226u8, 112u8, 184u8, 56u8, 19u8,
                            108u8, 110u8, 57u8, 108u8, 55u8, 163u8, 79u8, 4u8, 95u8, 252u8, 64u8,
                            29u8, 82u8, 240u8, 28u8, 217u8, 99u8, 73u8, 107u8, 237u8, 234u8, 129u8,
                        ],
                    )
                }
                ///Create consensus clients
                pub fn create_consensus_client(
                    &self,
                    message: runtime_types::ismp::messaging::CreateConsensusClient,
                ) -> ::subxt::tx::Payload<CreateConsensusClient> {
                    ::subxt::tx::Payload::new_static(
                        "Ismp",
                        "create_consensus_client",
                        CreateConsensusClient { message },
                        [
                            33u8, 216u8, 80u8, 63u8, 220u8, 183u8, 218u8, 172u8, 134u8, 129u8,
                            154u8, 164u8, 188u8, 170u8, 173u8, 249u8, 228u8, 12u8, 217u8, 83u8,
                            15u8, 95u8, 85u8, 71u8, 214u8, 101u8, 137u8, 211u8, 139u8, 94u8, 74u8,
                            235u8,
                        ],
                    )
                }
            }
        }
        ///Events are a simple means of reporting specific conditions and
        ///circumstances that have happened that users, Dapps and/or chain explorers would find
        ///interesting and otherwise difficult to detect.
        ///This attribute generate the function `deposit_event` to deposit one of this pallet
        /// event, it is optional, it is also possible to provide a custom implementation.
        pub type Event = runtime_types::pallet_ismp::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Emitted when a state machine is successfully updated to a new height
            pub struct StateMachineUpdated {
                pub state_machine_id: runtime_types::ismp::consensus::StateMachineId,
                pub latest_height: ::core::primitive::u64,
            }
            impl ::subxt::events::StaticEvent for StateMachineUpdated {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "StateMachineUpdated";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Signifies that a client has begun it's challenge period
            pub struct ChallengePeriodStarted {
                pub consensus_client_id: [::core::primitive::u8; 4usize],
                pub state_machines: ::std::vec::Vec<(
                    runtime_types::ismp::consensus::StateMachineHeight,
                    runtime_types::ismp::consensus::StateMachineHeight,
                )>,
            }
            impl ::subxt::events::StaticEvent for ChallengePeriodStarted {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "ChallengePeriodStarted";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Indicates that a consensus client has been created
            pub struct ConsensusClientCreated {
                pub consensus_client_id: [::core::primitive::u8; 4usize],
            }
            impl ::subxt::events::StaticEvent for ConsensusClientCreated {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "ConsensusClientCreated";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Response was process successfully
            pub struct Response {
                pub dest_chain: runtime_types::ismp::host::StateMachine,
                pub source_chain: runtime_types::ismp::host::StateMachine,
                pub request_nonce: ::core::primitive::u64,
            }
            impl ::subxt::events::StaticEvent for Response {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "Response";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Request processed successfully
            pub struct Request {
                pub dest_chain: runtime_types::ismp::host::StateMachine,
                pub source_chain: runtime_types::ismp::host::StateMachine,
                pub request_nonce: ::core::primitive::u64,
            }
            impl ::subxt::events::StaticEvent for Request {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "Request";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            ///Some errors handling some ismp messages
            pub struct HandlingErrors {
                pub errors: ::std::vec::Vec<runtime_types::pallet_ismp::errors::HandlingError>,
            }
            impl ::subxt::events::StaticEvent for HandlingErrors {
                const PALLET: &'static str = "Ismp";
                const EVENT: &'static str = "HandlingErrors";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Latest MMR Root hash
                pub fn root_hash(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "RootHash",
                        vec![],
                        [
                            182u8, 163u8, 37u8, 44u8, 2u8, 163u8, 57u8, 184u8, 97u8, 55u8, 1u8,
                            116u8, 55u8, 169u8, 23u8, 221u8, 182u8, 5u8, 174u8, 217u8, 111u8, 55u8,
                            180u8, 161u8, 69u8, 120u8, 212u8, 73u8, 2u8, 1u8, 39u8, 224u8,
                        ],
                    )
                }
                /// Current size of the MMR (number of leaves) for requests.
                pub fn number_of_leaves(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "NumberOfLeaves",
                        vec![],
                        [
                            138u8, 124u8, 23u8, 186u8, 255u8, 231u8, 187u8, 122u8, 213u8, 160u8,
                            29u8, 24u8, 88u8, 98u8, 171u8, 36u8, 195u8, 216u8, 27u8, 190u8, 192u8,
                            152u8, 8u8, 13u8, 210u8, 232u8, 45u8, 184u8, 240u8, 255u8, 156u8,
                            204u8,
                        ],
                    )
                }
                /// Hashes of the nodes in the MMR for requests.
                ///
                /// Note this collection only contains MMR peaks, the inner nodes (and leaves)
                /// are pruned and only stored in the Offchain DB.
                pub fn nodes(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u64>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "Nodes",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            188u8, 148u8, 126u8, 226u8, 142u8, 91u8, 61u8, 52u8, 213u8, 36u8,
                            120u8, 232u8, 20u8, 11u8, 61u8, 1u8, 130u8, 155u8, 81u8, 34u8, 153u8,
                            149u8, 210u8, 232u8, 113u8, 242u8, 249u8, 8u8, 61u8, 51u8, 148u8, 98u8,
                        ],
                    )
                }
                /// Hashes of the nodes in the MMR for requests.
                ///
                /// Note this collection only contains MMR peaks, the inner nodes (and leaves)
                /// are pruned and only stored in the Offchain DB.
                pub fn nodes_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "Nodes",
                        Vec::new(),
                        [
                            188u8, 148u8, 126u8, 226u8, 142u8, 91u8, 61u8, 52u8, 213u8, 36u8,
                            120u8, 232u8, 20u8, 11u8, 61u8, 1u8, 130u8, 155u8, 81u8, 34u8, 153u8,
                            149u8, 210u8, 232u8, 113u8, 242u8, 249u8, 8u8, 61u8, 51u8, 148u8, 98u8,
                        ],
                    )
                }
                pub fn state_commitments(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::ismp::consensus::StateMachineHeight>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::ismp::consensus::StateCommitment,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "StateCommitments",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            129u8, 199u8, 182u8, 168u8, 212u8, 13u8, 187u8, 119u8, 240u8, 106u8,
                            129u8, 199u8, 189u8, 213u8, 144u8, 246u8, 5u8, 65u8, 65u8, 241u8,
                            102u8, 160u8, 186u8, 250u8, 217u8, 77u8, 244u8, 52u8, 251u8, 96u8,
                            142u8, 61u8,
                        ],
                    )
                }
                pub fn state_commitments_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::ismp::consensus::StateCommitment,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "StateCommitments",
                        Vec::new(),
                        [
                            129u8, 199u8, 182u8, 168u8, 212u8, 13u8, 187u8, 119u8, 240u8, 106u8,
                            129u8, 199u8, 189u8, 213u8, 144u8, 246u8, 5u8, 65u8, 65u8, 241u8,
                            102u8, 160u8, 186u8, 250u8, 217u8, 77u8, 244u8, 52u8, 251u8, 96u8,
                            142u8, 61u8,
                        ],
                    )
                }
                pub fn consensus_states(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusStates",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            193u8, 56u8, 141u8, 69u8, 230u8, 137u8, 142u8, 24u8, 65u8, 121u8,
                            155u8, 69u8, 184u8, 153u8, 173u8, 252u8, 141u8, 189u8, 170u8, 236u8,
                            167u8, 238u8, 146u8, 121u8, 226u8, 97u8, 206u8, 211u8, 28u8, 61u8,
                            180u8, 4u8,
                        ],
                    )
                }
                pub fn consensus_states_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<::core::primitive::u8>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusStates",
                        Vec::new(),
                        [
                            193u8, 56u8, 141u8, 69u8, 230u8, 137u8, 142u8, 24u8, 65u8, 121u8,
                            155u8, 69u8, 184u8, 153u8, 173u8, 252u8, 141u8, 189u8, 170u8, 236u8,
                            167u8, 238u8, 146u8, 121u8, 226u8, 97u8, 206u8, 211u8, 28u8, 61u8,
                            180u8, 4u8,
                        ],
                    )
                }
                pub fn frozen_heights(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::ismp::consensus::StateMachineId>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "FrozenHeights",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            143u8, 251u8, 150u8, 72u8, 187u8, 250u8, 65u8, 166u8, 20u8, 52u8, 74u8,
                            101u8, 24u8, 158u8, 68u8, 7u8, 187u8, 10u8, 76u8, 189u8, 15u8, 206u8,
                            55u8, 208u8, 54u8, 108u8, 32u8, 163u8, 149u8, 95u8, 21u8, 114u8,
                        ],
                    )
                }
                pub fn frozen_heights_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "FrozenHeights",
                        Vec::new(),
                        [
                            143u8, 251u8, 150u8, 72u8, 187u8, 250u8, 65u8, 166u8, 20u8, 52u8, 74u8,
                            101u8, 24u8, 158u8, 68u8, 7u8, 187u8, 10u8, 76u8, 189u8, 15u8, 206u8,
                            55u8, 208u8, 54u8, 108u8, 32u8, 163u8, 149u8, 95u8, 21u8, 114u8,
                        ],
                    )
                }
                /// The latest accepted state machine height
                pub fn latest_state_machine_height(
                    &self,
                    _0: impl ::std::borrow::Borrow<runtime_types::ismp::consensus::StateMachineId>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "LatestStateMachineHeight",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            208u8, 210u8, 180u8, 204u8, 20u8, 192u8, 127u8, 60u8, 9u8, 158u8, 38u8,
                            126u8, 110u8, 199u8, 82u8, 211u8, 182u8, 146u8, 108u8, 233u8, 223u8,
                            175u8, 177u8, 29u8, 101u8, 32u8, 220u8, 85u8, 238u8, 187u8, 49u8,
                            135u8,
                        ],
                    )
                }
                /// The latest accepted state machine height
                pub fn latest_state_machine_height_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "LatestStateMachineHeight",
                        Vec::new(),
                        [
                            208u8, 210u8, 180u8, 204u8, 20u8, 192u8, 127u8, 60u8, 9u8, 158u8, 38u8,
                            126u8, 110u8, 199u8, 82u8, 211u8, 182u8, 146u8, 108u8, 233u8, 223u8,
                            175u8, 177u8, 29u8, 101u8, 32u8, 220u8, 85u8, 238u8, 187u8, 49u8,
                            135u8,
                        ],
                    )
                }
                pub fn consensus_client_update_time(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusClientUpdateTime",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            10u8, 78u8, 75u8, 111u8, 244u8, 215u8, 53u8, 111u8, 71u8, 202u8, 171u8,
                            90u8, 251u8, 125u8, 73u8, 78u8, 1u8, 103u8, 33u8, 34u8, 233u8, 101u8,
                            186u8, 154u8, 26u8, 85u8, 196u8, 46u8, 17u8, 236u8, 241u8, 62u8,
                        ],
                    )
                }
                pub fn consensus_client_update_time_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusClientUpdateTime",
                        Vec::new(),
                        [
                            10u8, 78u8, 75u8, 111u8, 244u8, 215u8, 53u8, 111u8, 71u8, 202u8, 171u8,
                            90u8, 251u8, 125u8, 73u8, 78u8, 1u8, 103u8, 33u8, 34u8, 233u8, 101u8,
                            186u8, 154u8, 26u8, 85u8, 196u8, 46u8, 17u8, 236u8, 241u8, 62u8,
                        ],
                    )
                }
                /// Acknowledgements for receipt of requests
                /// The key is the request commitment
                pub fn request_acks(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_ismp::router::Receipt,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "RequestAcks",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            127u8, 78u8, 209u8, 0u8, 190u8, 222u8, 160u8, 195u8, 186u8, 73u8, 96u8,
                            40u8, 39u8, 110u8, 13u8, 229u8, 22u8, 67u8, 62u8, 70u8, 223u8, 24u8,
                            43u8, 210u8, 104u8, 234u8, 201u8, 125u8, 75u8, 241u8, 208u8, 19u8,
                        ],
                    )
                }
                /// Acknowledgements for receipt of requests
                /// The key is the request commitment
                pub fn request_acks_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_ismp::router::Receipt,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "RequestAcks",
                        Vec::new(),
                        [
                            127u8, 78u8, 209u8, 0u8, 190u8, 222u8, 160u8, 195u8, 186u8, 73u8, 96u8,
                            40u8, 39u8, 110u8, 13u8, 229u8, 22u8, 67u8, 62u8, 70u8, 223u8, 24u8,
                            43u8, 210u8, 104u8, 234u8, 201u8, 125u8, 75u8, 241u8, 208u8, 19u8,
                        ],
                    )
                }
                /// Acknowledgements for receipt of responses
                /// The key is the response commitment
                pub fn response_acks(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_ismp::router::Receipt,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ResponseAcks",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            101u8, 220u8, 40u8, 242u8, 9u8, 180u8, 121u8, 173u8, 186u8, 15u8, 52u8,
                            203u8, 17u8, 10u8, 39u8, 7u8, 174u8, 160u8, 170u8, 0u8, 59u8, 133u8,
                            193u8, 6u8, 82u8, 150u8, 253u8, 84u8, 124u8, 237u8, 57u8, 213u8,
                        ],
                    )
                }
                /// Acknowledgements for receipt of responses
                /// The key is the response commitment
                pub fn response_acks_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    runtime_types::pallet_ismp::router::Receipt,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ResponseAcks",
                        Vec::new(),
                        [
                            101u8, 220u8, 40u8, 242u8, 9u8, 180u8, 121u8, 173u8, 186u8, 15u8, 52u8,
                            203u8, 17u8, 10u8, 39u8, 7u8, 174u8, 160u8, 170u8, 0u8, 59u8, 133u8,
                            193u8, 6u8, 82u8, 150u8, 253u8, 84u8, 124u8, 237u8, 57u8, 213u8,
                        ],
                    )
                }
                /// Consensus update results still in challenge period
                /// Set contains a tuple of previous height and latest height
                pub fn consensus_update_results(
                    &self,
                    _0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(
                        runtime_types::ismp::consensus::StateMachineHeight,
                        runtime_types::ismp::consensus::StateMachineHeight,
                    )>,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusUpdateResults",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            109u8, 234u8, 64u8, 145u8, 235u8, 68u8, 179u8, 204u8, 47u8, 157u8,
                            78u8, 120u8, 55u8, 26u8, 164u8, 195u8, 115u8, 129u8, 144u8, 171u8,
                            216u8, 47u8, 253u8, 226u8, 133u8, 83u8, 136u8, 100u8, 69u8, 100u8,
                            83u8, 249u8,
                        ],
                    )
                }
                /// Consensus update results still in challenge period
                /// Set contains a tuple of previous height and latest height
                pub fn consensus_update_results_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::std::vec::Vec<(
                        runtime_types::ismp::consensus::StateMachineHeight,
                        runtime_types::ismp::consensus::StateMachineHeight,
                    )>,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "ConsensusUpdateResults",
                        Vec::new(),
                        [
                            109u8, 234u8, 64u8, 145u8, 235u8, 68u8, 179u8, 204u8, 47u8, 157u8,
                            78u8, 120u8, 55u8, 26u8, 164u8, 195u8, 115u8, 129u8, 144u8, 171u8,
                            216u8, 47u8, 253u8, 226u8, 133u8, 83u8, 136u8, 100u8, 69u8, 100u8,
                            83u8, 249u8,
                        ],
                    )
                }
                /// Latest Nonce value for messages sent from this chain
                pub fn nonce(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::core::primitive::u64,
                    ::subxt::storage::address::Yes,
                    ::subxt::storage::address::Yes,
                    (),
                > {
                    ::subxt::storage::address::Address::new_static(
                        "Ismp",
                        "Nonce",
                        vec![],
                        [
                            122u8, 169u8, 95u8, 131u8, 85u8, 32u8, 154u8, 114u8, 143u8, 56u8, 12u8,
                            182u8, 64u8, 150u8, 241u8, 249u8, 254u8, 251u8, 160u8, 235u8, 192u8,
                            41u8, 101u8, 232u8, 186u8, 108u8, 187u8, 149u8, 210u8, 91u8, 179u8,
                            98u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod ismp_parachain {
        use super::{root_mod, runtime_types};
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::ismp_parachain::pallet::Event;
        pub mod events {
            use super::runtime_types;
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {
                /// Mapping of relay chain heights to it's state root. Gotten from parachain-system.
                pub fn relay_chain_state(
                    &self,
                    _0: impl ::std::borrow::Borrow<::core::primitive::u32>,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    ::subxt::storage::address::Yes,
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "IsmpParachain",
                        "RelayChainState",
                        vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
                        [
                            151u8, 241u8, 203u8, 75u8, 225u8, 194u8, 185u8, 153u8, 216u8, 62u8,
                            239u8, 206u8, 174u8, 148u8, 248u8, 252u8, 243u8, 169u8, 157u8, 136u8,
                            43u8, 166u8, 182u8, 118u8, 209u8, 181u8, 136u8, 246u8, 157u8, 233u8,
                            176u8, 18u8,
                        ],
                    )
                }
                /// Mapping of relay chain heights to it's state root. Gotten from parachain-system.
                pub fn relay_chain_state_root(
                    &self,
                ) -> ::subxt::storage::address::Address<
                    ::subxt::storage::address::StaticStorageMapKey,
                    ::subxt::utils::H256,
                    (),
                    (),
                    ::subxt::storage::address::Yes,
                > {
                    ::subxt::storage::address::Address::new_static(
                        "IsmpParachain",
                        "RelayChainState",
                        Vec::new(),
                        [
                            151u8, 241u8, 203u8, 75u8, 225u8, 194u8, 185u8, 153u8, 216u8, 62u8,
                            239u8, 206u8, 174u8, 148u8, 248u8, 252u8, 243u8, 169u8, 157u8, 136u8,
                            43u8, 166u8, 182u8, 118u8, 209u8, 181u8, 136u8, 246u8, 157u8, 233u8,
                            176u8, 18u8,
                        ],
                    )
                }
            }
        }
    }
    pub mod ismp_assets {
        use super::{root_mod, runtime_types};
        ///Contains one variant per dispatchable that can be called by an extrinsic.
        pub mod calls {
            use super::{root_mod, runtime_types};
            type DispatchError = runtime_types::sp_runtime::DispatchError;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Transfer {
                pub params: runtime_types::ismp_assets::pallet::TransferParams<
                    ::subxt::utils::AccountId32,
                    ::core::primitive::u128,
                >,
            }
            pub struct TransactionApi;
            impl TransactionApi {
                pub fn transfer(
                    &self,
                    params: runtime_types::ismp_assets::pallet::TransferParams<
                        ::subxt::utils::AccountId32,
                        ::core::primitive::u128,
                    >,
                ) -> ::subxt::tx::Payload<Transfer> {
                    ::subxt::tx::Payload::new_static(
                        "IsmpAssets",
                        "transfer",
                        Transfer { params },
                        [
                            34u8, 85u8, 45u8, 105u8, 45u8, 207u8, 24u8, 120u8, 8u8, 138u8, 29u8,
                            158u8, 216u8, 254u8, 146u8, 75u8, 118u8, 140u8, 122u8, 69u8, 70u8, 5u8,
                            98u8, 74u8, 242u8, 85u8, 181u8, 84u8, 226u8, 7u8, 228u8, 71u8,
                        ],
                    )
                }
            }
        }
        /**
        The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
        by this pallet.
        */
        pub type Event = runtime_types::ismp_assets::pallet::Event;
        pub mod events {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct BalanceTransferred {
                pub from: ::subxt::utils::AccountId32,
                pub to: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
                pub dest_chain: runtime_types::ismp::host::StateMachine,
            }
            impl ::subxt::events::StaticEvent for BalanceTransferred {
                const PALLET: &'static str = "IsmpAssets";
                const EVENT: &'static str = "BalanceTransferred";
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct BalanceReceived {
                pub from: ::subxt::utils::AccountId32,
                pub to: ::subxt::utils::AccountId32,
                pub amount: ::core::primitive::u128,
                pub source_chain: runtime_types::ismp::host::StateMachine,
            }
            impl ::subxt::events::StaticEvent for BalanceReceived {
                const PALLET: &'static str = "IsmpAssets";
                const EVENT: &'static str = "BalanceReceived";
            }
        }
        pub mod storage {
            use super::runtime_types;
            pub struct StorageApi;
            impl StorageApi {}
        }
    }
    pub mod runtime_types {
        use super::runtime_types;
        pub mod bounded_collections {
            use super::runtime_types;
            pub mod bounded_vec {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct BoundedVec<_0>(pub ::std::vec::Vec<_0>);
            }
            pub mod weak_bounded_vec {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct WeakBoundedVec<_0>(pub ::std::vec::Vec<_0>);
            }
        }
        pub mod cumulus_pallet_dmp_queue {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Service a single overweight message.
                    service_overweight {
                        index: ::core::primitive::u64,
                        weight_limit: runtime_types::sp_weights::weight_v2::Weight,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///The message index given is unknown.
                    Unknown,
                    #[codec(index = 1)]
                    ///The amount of weight given is possibly not enough for executing the
                    /// message.
                    OverLimit,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///Downward message is invalid XCM.
                    InvalidFormat { message_id: [::core::primitive::u8; 32usize] },
                    #[codec(index = 1)]
                    ///Downward message is unsupported version of XCM.
                    UnsupportedVersion { message_id: [::core::primitive::u8; 32usize] },
                    #[codec(index = 2)]
                    ///Downward message executed with the given outcome.
                    ExecutedDownward {
                        message_id: [::core::primitive::u8; 32usize],
                        outcome: runtime_types::xcm::v3::traits::Outcome,
                    },
                    #[codec(index = 3)]
                    ///The weight limit for handling downward messages was reached.
                    WeightExhausted {
                        message_id: [::core::primitive::u8; 32usize],
                        remaining_weight: runtime_types::sp_weights::weight_v2::Weight,
                        required_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 4)]
                    ///Downward message is overweight and was placed in the overweight queue.
                    OverweightEnqueued {
                        message_id: [::core::primitive::u8; 32usize],
                        overweight_index: ::core::primitive::u64,
                        required_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 5)]
                    ///Downward message from the overweight queue was executed.
                    OverweightServiced {
                        overweight_index: ::core::primitive::u64,
                        weight_used: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 6)]
                    ///The maximum number of downward messages was.
                    MaxMessagesExhausted { message_id: [::core::primitive::u8; 32usize] },
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ConfigData {
                pub max_individual: runtime_types::sp_weights::weight_v2::Weight,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct PageIndexData {
                pub begin_used: ::core::primitive::u32,
                pub end_used: ::core::primitive::u32,
                pub overweight_count: ::core::primitive::u64,
            }
        }
        pub mod cumulus_pallet_parachain_system {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Set the current validation data.
                    ///
                    ///This should be invoked exactly once per block. It will panic at the finalization
                    ///phase if the call was not invoked.
                    ///
                    ///The dispatch origin for this call must be `Inherent`
                    ///
                    ///As a side effect, this function upgrades the current validation function
                    ///if the appropriate time has come.
                    set_validation_data {
                        data: runtime_types::cumulus_primitives_parachain_inherent::ParachainInherentData,
                    },
                    #[codec(index = 1)]
                    sudo_send_upward_message {
                        message: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 2)]
                    ///Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied
                    ///later.
                    ///
                    ///The `check_version` parameter sets a boolean flag for whether or not the runtime's spec
                    ///version and name should be verified on upgrade. Since the authorization only has a hash,
                    ///it cannot actually perform the verification.
                    ///
                    ///This call requires Root origin.
                    authorize_upgrade {
                        code_hash: ::subxt::utils::H256,
                        check_version: ::core::primitive::bool,
                    },
                    #[codec(index = 3)]
                    ///Provide the preimage (runtime binary) `code` for an upgrade that has been authorized.
                    ///
                    ///If the authorization required a version check, this call will ensure the spec name
                    ///remains unchanged and that the spec version has increased.
                    ///
                    ///Note that this function will not apply the new `code`, but only attempt to schedule the
                    ///upgrade with the Relay Chain.
                    ///
                    ///All origins are allowed.
                    enact_authorized_upgrade {
                        code: ::std::vec::Vec<::core::primitive::u8>,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///Attempt to upgrade validation function while existing upgrade pending.
                    OverlappingUpgrades,
                    #[codec(index = 1)]
                    ///Polkadot currently prohibits this parachain from upgrading its validation
                    /// function.
                    ProhibitedByPolkadot,
                    #[codec(index = 2)]
                    ///The supplied validation function has compiled into a blob larger than
                    /// Polkadot is willing to run.
                    TooBig,
                    #[codec(index = 3)]
                    ///The inherent which supplies the validation data did not run this block.
                    ValidationDataNotAvailable,
                    #[codec(index = 4)]
                    ///The inherent which supplies the host configuration did not run this block.
                    HostConfigurationNotAvailable,
                    #[codec(index = 5)]
                    ///No validation function upgrade is currently scheduled.
                    NotScheduled,
                    #[codec(index = 6)]
                    ///No code upgrade has been authorized.
                    NothingAuthorized,
                    #[codec(index = 7)]
                    ///The given code upgrade has not been authorized.
                    Unauthorized,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///The validation function has been scheduled to apply.
                    ValidationFunctionStored,
                    #[codec(index = 1)]
                    ///The validation function was applied as of the contained relay chain block
                    /// number.
                    ValidationFunctionApplied { relay_chain_block_num: ::core::primitive::u32 },
                    #[codec(index = 2)]
                    ///The relay-chain aborted the upgrade process.
                    ValidationFunctionDiscarded,
                    #[codec(index = 3)]
                    ///An upgrade has been authorized.
                    UpgradeAuthorized { code_hash: ::subxt::utils::H256 },
                    #[codec(index = 4)]
                    ///Some downward messages have been received and will be processed.
                    DownwardMessagesReceived { count: ::core::primitive::u32 },
                    #[codec(index = 5)]
                    ///Downward messages were processed using the given weight.
                    DownwardMessagesProcessed {
                        weight_used: runtime_types::sp_weights::weight_v2::Weight,
                        dmq_head: ::subxt::utils::H256,
                    },
                    #[codec(index = 6)]
                    ///An upward message was sent to the relay chain.
                    UpwardMessageSent {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                }
            }
            pub mod relay_state_snapshot {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct MessagingStateSnapshot {
                    pub dmq_mqc_head: ::subxt::utils::H256,
                    pub relay_dispatch_queue_size: (::core::primitive::u32, ::core::primitive::u32),
                    pub ingress_channels: ::std::vec::Vec<(
                        runtime_types::polkadot_parachain::primitives::Id,
                        runtime_types::polkadot_primitives::v2::AbridgedHrmpChannel,
                    )>,
                    pub egress_channels: ::std::vec::Vec<(
                        runtime_types::polkadot_parachain::primitives::Id,
                        runtime_types::polkadot_primitives::v2::AbridgedHrmpChannel,
                    )>,
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct CodeUpgradeAuthorization {
                pub code_hash: ::subxt::utils::H256,
                pub check_version: ::core::primitive::bool,
            }
        }
        pub mod cumulus_pallet_xcm {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {}
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///Downward message is invalid XCM.
                    ///\[ id \]
                    InvalidFormat([::core::primitive::u8; 32usize]),
                    #[codec(index = 1)]
                    ///Downward message is unsupported version of XCM.
                    ///\[ id \]
                    UnsupportedVersion([::core::primitive::u8; 32usize]),
                    #[codec(index = 2)]
                    ///Downward message executed with the given outcome.
                    ///\[ id, outcome \]
                    ExecutedDownward(
                        [::core::primitive::u8; 32usize],
                        runtime_types::xcm::v3::traits::Outcome,
                    ),
                }
            }
        }
        pub mod cumulus_pallet_xcmp_queue {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Services a single overweight XCM.
                    ///
                    /// - `origin`: Must pass `ExecuteOverweightOrigin`.
                    /// - `index`: The index of the overweight XCM to service
                    /// - `weight_limit`: The amount of weight that XCM execution may take.
                    ///
                    ///Errors:
                    /// - `BadOverweightIndex`: XCM under `index` is not found in the `Overweight`
                    ///   storage map.
                    /// - `BadXcm`: XCM under `index` cannot be properly decoded into a valid XCM
                    ///   format.
                    /// - `WeightOverLimit`: XCM execution may use greater `weight_limit`.
                    ///
                    ///Events:
                    /// - `OverweightServiced`: On success.
                    service_overweight {
                        index: ::core::primitive::u64,
                        weight_limit: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 1)]
                    ///Suspends all XCM executions for the XCMP queue, regardless of the sender's
                    /// origin.
                    ///
                    /// - `origin`: Must pass `ControllerOrigin`.
                    suspend_xcm_execution,
                    #[codec(index = 2)]
                    ///Resumes all XCM executions for the XCMP queue.
                    ///
                    ///Note that this function doesn't change the status of the in/out bound
                    /// channels.
                    ///
                    /// - `origin`: Must pass `ControllerOrigin`.
                    resume_xcm_execution,
                    #[codec(index = 3)]
                    ///Overwrites the number of pages of messages which must be in the queue for
                    /// the other side to be told to suspend their sending.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.suspend_value`
                    update_suspend_threshold { new: ::core::primitive::u32 },
                    #[codec(index = 4)]
                    ///Overwrites the number of pages of messages which must be in the queue after
                    /// which we drop any further messages from the channel.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.drop_threshold`
                    update_drop_threshold { new: ::core::primitive::u32 },
                    #[codec(index = 5)]
                    ///Overwrites the number of pages of messages which the queue must be reduced
                    /// to before it signals that message sending may
                    /// recommence after it has been suspended.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.resume_threshold`
                    update_resume_threshold { new: ::core::primitive::u32 },
                    #[codec(index = 6)]
                    ///Overwrites the amount of remaining weight under which we stop processing
                    /// messages.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.threshold_weight`
                    update_threshold_weight { new: runtime_types::sp_weights::weight_v2::Weight },
                    #[codec(index = 7)]
                    ///Overwrites the speed to which the available weight approaches the maximum
                    /// weight. A lower number results in a faster progression.
                    /// A value of 1 makes the entire weight available initially.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.weight_restrict_decay`.
                    update_weight_restrict_decay {
                        new: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 8)]
                    ///Overwrite the maximum amount of weight any individual message may consume.
                    ///Messages above this weight go into the overweight queue and may only be
                    /// serviced explicitly.
                    ///
                    /// - `origin`: Must pass `Root`.
                    /// - `new`: Desired value for `QueueConfigData.xcmp_max_individual_weight`.
                    update_xcmp_max_individual_weight {
                        new: runtime_types::sp_weights::weight_v2::Weight,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///Failed to send XCM message.
                    FailedToSend,
                    #[codec(index = 1)]
                    ///Bad XCM origin.
                    BadXcmOrigin,
                    #[codec(index = 2)]
                    ///Bad XCM data.
                    BadXcm,
                    #[codec(index = 3)]
                    ///Bad overweight index.
                    BadOverweightIndex,
                    #[codec(index = 4)]
                    ///Provided weight is possibly not enough to execute the message.
                    WeightOverLimit,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///Some XCM was executed ok.
                    Success {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                        weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 1)]
                    ///Some XCM failed.
                    Fail {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                        error: runtime_types::xcm::v3::traits::Error,
                        weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 2)]
                    ///Bad XCM version used.
                    BadVersion {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                    #[codec(index = 3)]
                    ///Bad XCM format used.
                    BadFormat {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                    #[codec(index = 4)]
                    ///An HRMP message was sent to a sibling parachain.
                    XcmpMessageSent {
                        message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
                    },
                    #[codec(index = 5)]
                    ///An XCM exceeded the individual message weight budget.
                    OverweightEnqueued {
                        sender: runtime_types::polkadot_parachain::primitives::Id,
                        sent_at: ::core::primitive::u32,
                        index: ::core::primitive::u64,
                        required: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 6)]
                    ///An XCM from the overweight queue was executed with the given actual weight
                    /// used.
                    OverweightServiced {
                        index: ::core::primitive::u64,
                        used: runtime_types::sp_weights::weight_v2::Weight,
                    },
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct InboundChannelDetails {
                pub sender: runtime_types::polkadot_parachain::primitives::Id,
                pub state: runtime_types::cumulus_pallet_xcmp_queue::InboundState,
                pub message_metadata: ::std::vec::Vec<(
                    ::core::primitive::u32,
                    runtime_types::polkadot_parachain::primitives::XcmpMessageFormat,
                )>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum InboundState {
                #[codec(index = 0)]
                Ok,
                #[codec(index = 1)]
                Suspended,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct OutboundChannelDetails {
                pub recipient: runtime_types::polkadot_parachain::primitives::Id,
                pub state: runtime_types::cumulus_pallet_xcmp_queue::OutboundState,
                pub signals_exist: ::core::primitive::bool,
                pub first_index: ::core::primitive::u16,
                pub last_index: ::core::primitive::u16,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum OutboundState {
                #[codec(index = 0)]
                Ok,
                #[codec(index = 1)]
                Suspended,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct QueueConfigData {
                pub suspend_threshold: ::core::primitive::u32,
                pub drop_threshold: ::core::primitive::u32,
                pub resume_threshold: ::core::primitive::u32,
                pub threshold_weight: runtime_types::sp_weights::weight_v2::Weight,
                pub weight_restrict_decay: runtime_types::sp_weights::weight_v2::Weight,
                pub xcmp_max_individual_weight: runtime_types::sp_weights::weight_v2::Weight,
            }
        }
        pub mod cumulus_primitives_parachain_inherent {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct MessageQueueChain(pub ::subxt::utils::H256);
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ParachainInherentData {
                pub validation_data:
                    runtime_types::polkadot_primitives::v2::PersistedValidationData<
                        ::subxt::utils::H256,
                        ::core::primitive::u32,
                    >,
                pub relay_chain_state: runtime_types::sp_trie::storage_proof::StorageProof,
                pub downward_messages: ::std::vec::Vec<
                    runtime_types::polkadot_core_primitives::InboundDownwardMessage<
                        ::core::primitive::u32,
                    >,
                >,
                pub horizontal_messages: ::subxt::utils::KeyedVec<
                    runtime_types::polkadot_parachain::primitives::Id,
                    ::std::vec::Vec<
                        runtime_types::polkadot_core_primitives::InboundHrmpMessage<
                            ::core::primitive::u32,
                        >,
                    >,
                >,
            }
        }
        pub mod frame_support {
            use super::runtime_types;
            pub mod dispatch {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum DispatchClass {
                    #[codec(index = 0)]
                    Normal,
                    #[codec(index = 1)]
                    Operational,
                    #[codec(index = 2)]
                    Mandatory,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct DispatchInfo {
                    pub weight: runtime_types::sp_weights::weight_v2::Weight,
                    pub class: runtime_types::frame_support::dispatch::DispatchClass,
                    pub pays_fee: runtime_types::frame_support::dispatch::Pays,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Pays {
                    #[codec(index = 0)]
                    Yes,
                    #[codec(index = 1)]
                    No,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct PerDispatchClass<_0> {
                    pub normal: _0,
                    pub operational: _0,
                    pub mandatory: _0,
                }
            }
            pub mod traits {
                use super::runtime_types;
                pub mod tokens {
                    use super::runtime_types;
                    pub mod misc {
                        use super::runtime_types;
                        #[derive(
                            ::subxt::ext::codec::Decode,
                            ::subxt::ext::codec::Encode,
                            ::subxt::ext::scale_decode::DecodeAsType,
                            ::subxt::ext::scale_encode::EncodeAsType,
                            Debug,
                        )]
                        #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                        #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                        pub enum BalanceStatus {
                            #[codec(index = 0)]
                            Free,
                            #[codec(index = 1)]
                            Reserved,
                        }
                    }
                }
            }
        }
        pub mod frame_system {
            use super::runtime_types;
            pub mod extensions {
                use super::runtime_types;
                pub mod check_genesis {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckGenesis;
                }
                pub mod check_mortality {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckMortality(pub runtime_types::sp_runtime::generic::era::Era);
                }
                pub mod check_non_zero_sender {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckNonZeroSender;
                }
                pub mod check_nonce {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckNonce(#[codec(compact)] pub ::core::primitive::u32);
                }
                pub mod check_spec_version {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckSpecVersion;
                }
                pub mod check_tx_version {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckTxVersion;
                }
                pub mod check_weight {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct CheckWeight;
                }
            }
            pub mod limits {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct BlockLength {
                    pub max: runtime_types::frame_support::dispatch::PerDispatchClass<
                        ::core::primitive::u32,
                    >,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct BlockWeights {
                    pub base_block: runtime_types::sp_weights::weight_v2::Weight,
                    pub max_block: runtime_types::sp_weights::weight_v2::Weight,
                    pub per_class: runtime_types::frame_support::dispatch::PerDispatchClass<
                        runtime_types::frame_system::limits::WeightsPerClass,
                    >,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct WeightsPerClass {
                    pub base_extrinsic: runtime_types::sp_weights::weight_v2::Weight,
                    pub max_extrinsic:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                    pub max_total:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                    pub reserved:
                        ::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
                }
            }
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Make some on-chain remark.
                    ///
                    ///## Complexity
                    /// - `O(1)`
                    remark { remark: ::std::vec::Vec<::core::primitive::u8> },
                    #[codec(index = 1)]
                    ///Set the number of pages in the WebAssembly environment's heap.
                    set_heap_pages { pages: ::core::primitive::u64 },
                    #[codec(index = 2)]
                    ///Set the new runtime code.
                    ///
                    ///## Complexity
                    /// - `O(C + S)` where `C` length of `code` and `S` complexity of
                    ///   `can_set_code`
                    set_code { code: ::std::vec::Vec<::core::primitive::u8> },
                    #[codec(index = 3)]
                    ///Set the new runtime code without doing any checks of the given `code`.
                    ///
                    ///## Complexity
                    /// - `O(C)` where `C` length of `code`
                    set_code_without_checks { code: ::std::vec::Vec<::core::primitive::u8> },
                    #[codec(index = 4)]
                    ///Set some items of storage.
                    set_storage {
                        items: ::std::vec::Vec<(
                            ::std::vec::Vec<::core::primitive::u8>,
                            ::std::vec::Vec<::core::primitive::u8>,
                        )>,
                    },
                    #[codec(index = 5)]
                    ///Kill some items from storage.
                    kill_storage { keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>> },
                    #[codec(index = 6)]
                    ///Kill all storage items with a key that starts with the given prefix.
                    ///
                    ///**NOTE:** We rely on the Root origin to provide us the number of subkeys
                    /// under the prefix we are removing to accurately
                    /// calculate the weight of this function.
                    kill_prefix {
                        prefix: ::std::vec::Vec<::core::primitive::u8>,
                        subkeys: ::core::primitive::u32,
                    },
                    #[codec(index = 7)]
                    ///Make some on-chain remark and emit event.
                    remark_with_event { remark: ::std::vec::Vec<::core::primitive::u8> },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Error for the System pallet
                pub enum Error {
                    #[codec(index = 0)]
                    ///The name of specification does not match between the current runtime
                    ///and the new runtime.
                    InvalidSpecName,
                    #[codec(index = 1)]
                    ///The specification version is not allowed to decrease between the current
                    /// runtime and the new runtime.
                    SpecVersionNeedsToIncrease,
                    #[codec(index = 2)]
                    ///Failed to extract the runtime version from the new runtime.
                    ///
                    ///Either calling `Core_version` or decoding `RuntimeVersion` failed.
                    FailedToExtractRuntimeVersion,
                    #[codec(index = 3)]
                    ///Suicide called when the account has non-default composite data.
                    NonDefaultComposite,
                    #[codec(index = 4)]
                    ///There is a non-zero reference count preventing the account from being
                    /// purged.
                    NonZeroRefCount,
                    #[codec(index = 5)]
                    ///The origin filter prevent the call to be dispatched.
                    CallFiltered,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Event for the System pallet.
                pub enum Event {
                    #[codec(index = 0)]
                    ///An extrinsic completed successfully.
                    ExtrinsicSuccess {
                        dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
                    },
                    #[codec(index = 1)]
                    ///An extrinsic failed.
                    ExtrinsicFailed {
                        dispatch_error: runtime_types::sp_runtime::DispatchError,
                        dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
                    },
                    #[codec(index = 2)]
                    ///`:code` was updated.
                    CodeUpdated,
                    #[codec(index = 3)]
                    ///A new account was created.
                    NewAccount { account: ::subxt::utils::AccountId32 },
                    #[codec(index = 4)]
                    ///An account was reaped.
                    KilledAccount { account: ::subxt::utils::AccountId32 },
                    #[codec(index = 5)]
                    ///On on-chain remark happened.
                    Remarked { sender: ::subxt::utils::AccountId32, hash: ::subxt::utils::H256 },
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct AccountInfo<_0, _1> {
                pub nonce: _0,
                pub consumers: _0,
                pub providers: _0,
                pub sufficients: _0,
                pub data: _1,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct EventRecord<_0, _1> {
                pub phase: runtime_types::frame_system::Phase,
                pub event: _0,
                pub topics: ::std::vec::Vec<_1>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct LastRuntimeUpgradeInfo {
                #[codec(compact)]
                pub spec_version: ::core::primitive::u32,
                pub spec_name: ::std::string::String,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum Phase {
                #[codec(index = 0)]
                ApplyExtrinsic(::core::primitive::u32),
                #[codec(index = 1)]
                Finalization,
                #[codec(index = 2)]
                Initialization,
            }
        }
        pub mod hyperbridge_runtime {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Runtime;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum RuntimeCall {
                #[codec(index = 0)]
                System(runtime_types::frame_system::pallet::Call),
                #[codec(index = 1)]
                Timestamp(runtime_types::pallet_timestamp::pallet::Call),
                #[codec(index = 2)]
                ParachainSystem(runtime_types::cumulus_pallet_parachain_system::pallet::Call),
                #[codec(index = 10)]
                Balances(runtime_types::pallet_balances::pallet::Call),
                #[codec(index = 21)]
                CollatorSelection(runtime_types::pallet_collator_selection::pallet::Call),
                #[codec(index = 22)]
                Session(runtime_types::pallet_session::pallet::Call),
                #[codec(index = 25)]
                Sudo(runtime_types::pallet_sudo::pallet::Call),
                #[codec(index = 30)]
                XcmpQueue(runtime_types::cumulus_pallet_xcmp_queue::pallet::Call),
                #[codec(index = 31)]
                PolkadotXcm(runtime_types::pallet_xcm::pallet::Call),
                #[codec(index = 33)]
                DmpQueue(runtime_types::cumulus_pallet_dmp_queue::pallet::Call),
                #[codec(index = 40)]
                Ismp(runtime_types::pallet_ismp::pallet::Call),
                #[codec(index = 42)]
                IsmpAssets(runtime_types::ismp_assets::pallet::Call),
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum RuntimeEvent {
                #[codec(index = 0)]
                System(runtime_types::frame_system::pallet::Event),
                #[codec(index = 2)]
                ParachainSystem(runtime_types::cumulus_pallet_parachain_system::pallet::Event),
                #[codec(index = 10)]
                Balances(runtime_types::pallet_balances::pallet::Event),
                #[codec(index = 11)]
                TransactionPayment(runtime_types::pallet_transaction_payment::pallet::Event),
                #[codec(index = 21)]
                CollatorSelection(runtime_types::pallet_collator_selection::pallet::Event),
                #[codec(index = 22)]
                Session(runtime_types::pallet_session::pallet::Event),
                #[codec(index = 25)]
                Sudo(runtime_types::pallet_sudo::pallet::Event),
                #[codec(index = 30)]
                XcmpQueue(runtime_types::cumulus_pallet_xcmp_queue::pallet::Event),
                #[codec(index = 31)]
                PolkadotXcm(runtime_types::pallet_xcm::pallet::Event),
                #[codec(index = 32)]
                CumulusXcm(runtime_types::cumulus_pallet_xcm::pallet::Event),
                #[codec(index = 33)]
                DmpQueue(runtime_types::cumulus_pallet_dmp_queue::pallet::Event),
                #[codec(index = 40)]
                Ismp(runtime_types::pallet_ismp::pallet::Event),
                #[codec(index = 41)]
                IsmpParachain(runtime_types::ismp_parachain::pallet::Event),
                #[codec(index = 42)]
                IsmpAssets(runtime_types::ismp_assets::pallet::Event),
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct SessionKeys {
                pub aura: runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public,
            }
        }
        pub mod ismp {
            use super::runtime_types;
            pub mod consensus {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct IntermediateState {
                    pub height: runtime_types::ismp::consensus::StateMachineHeight,
                    pub commitment: runtime_types::ismp::consensus::StateCommitment,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct StateCommitment {
                    pub timestamp: ::core::primitive::u64,
                    pub ismp_root: ::core::option::Option<::subxt::utils::H256>,
                    pub state_root: ::subxt::utils::H256,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct StateMachineHeight {
                    pub id: runtime_types::ismp::consensus::StateMachineId,
                    pub height: ::core::primitive::u64,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct StateMachineId {
                    pub state_id: runtime_types::ismp::host::StateMachine,
                    pub consensus_client: [::core::primitive::u8; 4usize],
                }
            }
            pub mod host {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum StateMachine {
                    #[codec(index = 0)]
                    Ethereum,
                    #[codec(index = 1)]
                    Arbitrum,
                    #[codec(index = 2)]
                    Optimism,
                    #[codec(index = 3)]
                    Base,
                    #[codec(index = 4)]
                    Polkadot(::core::primitive::u32),
                    #[codec(index = 5)]
                    Kusama(::core::primitive::u32),
                }
            }
            pub mod messaging {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct ConsensusMessage {
                    pub consensus_proof: ::std::vec::Vec<::core::primitive::u8>,
                    pub consensus_client_id: [::core::primitive::u8; 4usize],
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct CreateConsensusClient {
                    pub consensus_state: ::std::vec::Vec<::core::primitive::u8>,
                    pub consensus_client_id: [::core::primitive::u8; 4usize],
                    pub state_machine_commitments:
                        ::std::vec::Vec<runtime_types::ismp::consensus::IntermediateState>,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Message {
                    #[codec(index = 0)]
                    Consensus(runtime_types::ismp::messaging::ConsensusMessage),
                    #[codec(index = 1)]
                    Request(runtime_types::ismp::messaging::RequestMessage),
                    #[codec(index = 2)]
                    Response(runtime_types::ismp::messaging::ResponseMessage),
                    #[codec(index = 3)]
                    Timeout(runtime_types::ismp::messaging::TimeoutMessage),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Proof {
                    pub height: runtime_types::ismp::consensus::StateMachineHeight,
                    pub proof: ::std::vec::Vec<::core::primitive::u8>,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct RequestMessage {
                    pub requests: ::std::vec::Vec<runtime_types::ismp::router::Request>,
                    pub proof: runtime_types::ismp::messaging::Proof,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct ResponseMessage {
                    pub responses: ::std::vec::Vec<runtime_types::ismp::router::Response>,
                    pub proof: runtime_types::ismp::messaging::Proof,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct TimeoutMessage {
                    pub requests: ::std::vec::Vec<runtime_types::ismp::router::Request>,
                    pub timeout_proof: runtime_types::ismp::messaging::Proof,
                }
            }
            pub mod router {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Get {
                    pub source_chain: runtime_types::ismp::host::StateMachine,
                    pub dest_chain: runtime_types::ismp::host::StateMachine,
                    pub nonce: ::core::primitive::u64,
                    pub from: ::std::vec::Vec<::core::primitive::u8>,
                    pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                    pub height: runtime_types::ismp::consensus::StateMachineHeight,
                    pub timeout_timestamp: ::core::primitive::u64,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Post {
                    pub source_chain: runtime_types::ismp::host::StateMachine,
                    pub dest_chain: runtime_types::ismp::host::StateMachine,
                    pub nonce: ::core::primitive::u64,
                    pub from: ::std::vec::Vec<::core::primitive::u8>,
                    pub to: ::std::vec::Vec<::core::primitive::u8>,
                    pub timeout_timestamp: ::core::primitive::u64,
                    pub data: ::std::vec::Vec<::core::primitive::u8>,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Request {
                    #[codec(index = 0)]
                    Post(runtime_types::ismp::router::Post),
                    #[codec(index = 1)]
                    Get(runtime_types::ismp::router::Get),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Response {
                    pub request: runtime_types::ismp::router::Request,
                    pub response: ::std::vec::Vec<::core::primitive::u8>,
                }
            }
        }
        pub mod ismp_assets {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    transfer {
                        params: runtime_types::ismp_assets::pallet::TransferParams<
                            ::subxt::utils::AccountId32,
                            ::core::primitive::u128,
                        >,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    TransferFailed,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    BalanceTransferred {
                        from: ::subxt::utils::AccountId32,
                        to: ::subxt::utils::AccountId32,
                        amount: ::core::primitive::u128,
                        dest_chain: runtime_types::ismp::host::StateMachine,
                    },
                    #[codec(index = 1)]
                    BalanceReceived {
                        from: ::subxt::utils::AccountId32,
                        to: ::subxt::utils::AccountId32,
                        amount: ::core::primitive::u128,
                        source_chain: runtime_types::ismp::host::StateMachine,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct TransferParams<_0, _1> {
                    pub to: _0,
                    pub amount: _1,
                    pub dest_chain: runtime_types::ismp::host::StateMachine,
                    pub timeout: ::core::primitive::u64,
                }
            }
        }
        pub mod ismp_parachain {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {}
            }
        }
        pub mod pallet_balances {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Transfer some liquid free balance to another account.
                    ///
                    ///`transfer` will set the `FreeBalance` of the sender and receiver.
                    ///If the sender's account is below the existential deposit as a result
                    ///of the transfer, the account will be reaped.
                    ///
                    ///The dispatch origin for this call must be `Signed` by the transactor.
                    ///
                    ///## Complexity
                    /// - Dependent on arguments but not critical, given proper implementations for
                    ///   input config
                    ///  types. See related functions below.
                    /// - It contains a limited number of reads and writes internally and no
                    ///   complex
                    ///  computation.
                    ///
                    ///Related functions:
                    ///
                    ///  - `ensure_can_withdraw` is always called internally but has a bounded
                    ///    complexity.
                    ///  - Transferring balances to accounts that did not exist before will cause
                    ///    `T::OnNewAccount::on_new_account` to be called.
                    ///  - Removing enough funds from an account will trigger
                    ///    `T::DustRemoval::on_unbalanced`.
                    ///  - `transfer_keep_alive` works the same way as `transfer`, but has an
                    ///    additional check that the transfer will not kill the origin account.
                    transfer {
                        dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 1)]
                    ///Set the balances of a given account.
                    ///
                    ///This will alter `FreeBalance` and `ReservedBalance` in storage. it will
                    ///also alter the total issuance of the system (`TotalIssuance`)
                    /// appropriately. If the new free or reserved balance is
                    /// below the existential deposit, it will reset the
                    /// account nonce (`frame_system::AccountNonce`).
                    ///
                    ///The dispatch origin for this call is `root`.
                    set_balance {
                        who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        #[codec(compact)]
                        new_free: ::core::primitive::u128,
                        #[codec(compact)]
                        new_reserved: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    ///Exactly as `transfer`, except the origin must be root and the source
                    /// account may be specified.
                    ///## Complexity
                    /// - Same as transfer, but additional read and write because the source
                    ///   account is not
                    ///  assumed to be in the overlay.
                    force_transfer {
                        source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    ///Same as the [`transfer`] call, but with a check that the transfer will not
                    /// kill the origin account.
                    ///
                    ///99% of the time you want [`transfer`] instead.
                    ///
                    ///[`transfer`]: struct.Pallet.html#method.transfer
                    transfer_keep_alive {
                        dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        #[codec(compact)]
                        value: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    ///Transfer the entire transferable balance from the caller account.
                    ///
                    ///NOTE: This function only attempts to transfer _transferable_ balances. This
                    /// means that any locked, reserved, or existential
                    /// deposits (when `keep_alive` is `true`), will not be
                    /// transferred by this function. To ensure that this function results in a
                    /// killed account, you might need to prepare the account
                    /// by removing any reference counters, storage
                    /// deposits, etc...
                    ///
                    ///The dispatch origin of this call must be Signed.
                    ///
                    /// - `dest`: The recipient of the transfer.
                    /// - `keep_alive`: A boolean to determine if the `transfer_all` operation
                    ///   should send all
                    ///  of the funds the account has, causing the sender account to be killed
                    /// (false), or  transfer everything except at least the
                    /// existential deposit, which will guarantee to
                    ///  keep the sender account alive (true). ## Complexity
                    /// - O(1). Just like transfer, but reading the user's transferable balance
                    ///   first.
                    transfer_all {
                        dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        keep_alive: ::core::primitive::bool,
                    },
                    #[codec(index = 5)]
                    ///Unreserve some balance from a user by force.
                    ///
                    ///Can only be called by ROOT.
                    force_unreserve {
                        who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        amount: ::core::primitive::u128,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///Vesting balance too high to send value
                    VestingBalance,
                    #[codec(index = 1)]
                    ///Account liquidity restrictions prevent withdrawal
                    LiquidityRestrictions,
                    #[codec(index = 2)]
                    ///Balance too low to send value.
                    InsufficientBalance,
                    #[codec(index = 3)]
                    ///Value too low to create account due to existential deposit
                    ExistentialDeposit,
                    #[codec(index = 4)]
                    ///Transfer/payment would kill account
                    KeepAlive,
                    #[codec(index = 5)]
                    ///A vesting schedule already exists for this account
                    ExistingVestingSchedule,
                    #[codec(index = 6)]
                    ///Beneficiary account must pre-exist
                    DeadAccount,
                    #[codec(index = 7)]
                    ///Number of named reserves exceed MaxReserves
                    TooManyReserves,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///An account was created with some free balance.
                    Endowed {
                        account: ::subxt::utils::AccountId32,
                        free_balance: ::core::primitive::u128,
                    },
                    #[codec(index = 1)]
                    ///An account was removed whose balance was non-zero but below
                    /// ExistentialDeposit, resulting in an outright loss.
                    DustLost {
                        account: ::subxt::utils::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 2)]
                    ///Transfer succeeded.
                    Transfer {
                        from: ::subxt::utils::AccountId32,
                        to: ::subxt::utils::AccountId32,
                        amount: ::core::primitive::u128,
                    },
                    #[codec(index = 3)]
                    ///A balance was set by root.
                    BalanceSet {
                        who: ::subxt::utils::AccountId32,
                        free: ::core::primitive::u128,
                        reserved: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    ///Some balance was reserved (moved from free to reserved).
                    Reserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
                    #[codec(index = 5)]
                    ///Some balance was unreserved (moved from reserved to free).
                    Unreserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
                    #[codec(index = 6)]
                    ///Some balance was moved from the reserve of the first account to the second
                    /// account. Final argument indicates the destination
                    /// balance type.
                    ReserveRepatriated {
                        from: ::subxt::utils::AccountId32,
                        to: ::subxt::utils::AccountId32,
                        amount: ::core::primitive::u128,
                        destination_status:
                            runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
                    },
                    #[codec(index = 7)]
                    ///Some amount was deposited (e.g. for transaction fees).
                    Deposit { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
                    #[codec(index = 8)]
                    ///Some amount was withdrawn from the account (e.g. for transaction fees).
                    Withdraw { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
                    #[codec(index = 9)]
                    ///Some amount was removed from the account (e.g. for misbehavior).
                    Slashed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct AccountData<_0> {
                pub free: _0,
                pub reserved: _0,
                pub misc_frozen: _0,
                pub fee_frozen: _0,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct BalanceLock<_0> {
                pub id: [::core::primitive::u8; 8usize],
                pub amount: _0,
                pub reasons: runtime_types::pallet_balances::Reasons,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum Reasons {
                #[codec(index = 0)]
                Fee,
                #[codec(index = 1)]
                Misc,
                #[codec(index = 2)]
                All,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ReserveData<_0, _1> {
                pub id: _0,
                pub amount: _1,
            }
        }
        pub mod pallet_collator_selection {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Set the list of invulnerable (fixed) collators.
                    set_invulnerables { new: ::std::vec::Vec<::subxt::utils::AccountId32> },
                    #[codec(index = 1)]
                    ///Set the ideal number of collators (not including the invulnerables).
                    ///If lowering this number, then the number of running collators could be
                    /// higher than this figure. Aside from that edge case,
                    /// there should be no other way to have more collators than the desired
                    /// number.
                    set_desired_candidates { max: ::core::primitive::u32 },
                    #[codec(index = 2)]
                    ///Set the candidacy bond amount.
                    set_candidacy_bond { bond: ::core::primitive::u128 },
                    #[codec(index = 3)]
                    ///Register this account as a collator candidate. The account must (a) already
                    /// have registered session keys and (b) be able to reserve
                    /// the `CandidacyBond`.
                    ///
                    ///This call is not available to `Invulnerable` collators.
                    register_as_candidate,
                    #[codec(index = 4)]
                    ///Deregister `origin` as a collator candidate. Note that the collator can
                    /// only leave on session change. The `CandidacyBond` will
                    /// be unreserved immediately.
                    ///
                    ///This call will fail if the total number of candidates would drop below
                    /// `MinCandidates`.
                    ///
                    ///This call is not available to `Invulnerable` collators.
                    leave_intent,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct CandidateInfo<_0, _1> {
                    pub who: _0,
                    pub deposit: _1,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///Too many candidates
                    TooManyCandidates,
                    #[codec(index = 1)]
                    ///Too few candidates
                    TooFewCandidates,
                    #[codec(index = 2)]
                    ///Unknown error
                    Unknown,
                    #[codec(index = 3)]
                    ///Permission issue
                    Permission,
                    #[codec(index = 4)]
                    ///User is already a candidate
                    AlreadyCandidate,
                    #[codec(index = 5)]
                    ///User is not a candidate
                    NotCandidate,
                    #[codec(index = 6)]
                    ///Too many invulnerables
                    TooManyInvulnerables,
                    #[codec(index = 7)]
                    ///User is already an Invulnerable
                    AlreadyInvulnerable,
                    #[codec(index = 8)]
                    ///Account has no associated validator ID
                    NoAssociatedValidatorId,
                    #[codec(index = 9)]
                    ///Validator ID is not yet registered
                    ValidatorNotRegistered,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    NewInvulnerables { invulnerables: ::std::vec::Vec<::subxt::utils::AccountId32> },
                    #[codec(index = 1)]
                    NewDesiredCandidates { desired_candidates: ::core::primitive::u32 },
                    #[codec(index = 2)]
                    NewCandidacyBond { bond_amount: ::core::primitive::u128 },
                    #[codec(index = 3)]
                    CandidateAdded {
                        account_id: ::subxt::utils::AccountId32,
                        deposit: ::core::primitive::u128,
                    },
                    #[codec(index = 4)]
                    CandidateRemoved { account_id: ::subxt::utils::AccountId32 },
                }
            }
        }
        pub mod pallet_ismp {
            use super::runtime_types;
            pub mod errors {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum HandlingError {
                    #[codec(index = 0)]
                    ChallengePeriodNotElapsed {
                        update_time: ::core::primitive::u64,
                        current_time: ::core::primitive::u64,
                        delay_period: ::core::option::Option<::core::primitive::u64>,
                        consensus_client_id:
                            ::core::option::Option<[::core::primitive::u8; 4usize]>,
                    },
                    #[codec(index = 1)]
                    ConsensusStateNotFound { id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 2)]
                    StateCommitmentNotFound {
                        height: runtime_types::ismp::consensus::StateMachineHeight,
                    },
                    #[codec(index = 3)]
                    FrozenConsensusClient { id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 4)]
                    FrozenStateMachine {
                        height: runtime_types::ismp::consensus::StateMachineHeight,
                    },
                    #[codec(index = 5)]
                    RequestCommitmentNotFound {
                        nonce: ::core::primitive::u64,
                        source: runtime_types::ismp::host::StateMachine,
                        dest: runtime_types::ismp::host::StateMachine,
                    },
                    #[codec(index = 6)]
                    RequestVerificationFailed {
                        nonce: ::core::primitive::u64,
                        source: runtime_types::ismp::host::StateMachine,
                        dest: runtime_types::ismp::host::StateMachine,
                    },
                    #[codec(index = 7)]
                    ResponseVerificationFailed {
                        nonce: ::core::primitive::u64,
                        source: runtime_types::ismp::host::StateMachine,
                        dest: runtime_types::ismp::host::StateMachine,
                    },
                    #[codec(index = 8)]
                    ConsensusProofVerificationFailed { id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 9)]
                    ExpiredConsensusClient { id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 10)]
                    CannotHandleMessage,
                    #[codec(index = 11)]
                    ImplementationSpecific { msg: ::std::vec::Vec<::core::primitive::u8> },
                    #[codec(index = 12)]
                    UnbondingPeriodElapsed { consensus_id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 13)]
                    MembershipProofVerificationFailed {
                        msg: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 14)]
                    NonMembershipProofVerificationFailed {
                        msg: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 15)]
                    CannotCreateAlreadyExistingConsensusClient {
                        id: [::core::primitive::u8; 4usize],
                    },
                    #[codec(index = 16)]
                    RequestTimeoutNotElapsed {
                        nonce: ::core::primitive::u64,
                        source: runtime_types::ismp::host::StateMachine,
                        dest: runtime_types::ismp::host::StateMachine,
                        timeout_timestamp: ::core::primitive::u64,
                        state_machine_time: ::core::primitive::u64,
                    },
                    #[codec(index = 17)]
                    RequestTimeoutVerificationFailed {
                        nonce: ::core::primitive::u64,
                        source: runtime_types::ismp::host::StateMachine,
                        dest: runtime_types::ismp::host::StateMachine,
                    },
                }
            }
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Handles ismp messages
                    handle { messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message> },
                    #[codec(index = 1)]
                    ///Create consensus clients
                    create_consensus_client {
                        message: runtime_types::ismp::messaging::CreateConsensusClient,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    InvalidMessage,
                    #[codec(index = 1)]
                    ConsensusClientCreationFailed,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Events are a simple means of reporting specific conditions and
                ///circumstances that have happened that users, Dapps and/or chain explorers would
                /// find interesting and otherwise difficult to detect.
                ///This attribute generate the function `deposit_event` to deposit one of this
                /// pallet event, it is optional, it is also possible to provide a
                /// custom implementation.
                pub enum Event {
                    #[codec(index = 0)]
                    ///Emitted when a state machine is successfully updated to a new height
                    StateMachineUpdated {
                        state_machine_id: runtime_types::ismp::consensus::StateMachineId,
                        latest_height: ::core::primitive::u64,
                    },
                    #[codec(index = 1)]
                    ///Signifies that a client has begun it's challenge period
                    ChallengePeriodStarted {
                        consensus_client_id: [::core::primitive::u8; 4usize],
                        state_machines: ::std::vec::Vec<(
                            runtime_types::ismp::consensus::StateMachineHeight,
                            runtime_types::ismp::consensus::StateMachineHeight,
                        )>,
                    },
                    #[codec(index = 2)]
                    ///Indicates that a consensus client has been created
                    ConsensusClientCreated { consensus_client_id: [::core::primitive::u8; 4usize] },
                    #[codec(index = 3)]
                    ///Response was process successfully
                    Response {
                        dest_chain: runtime_types::ismp::host::StateMachine,
                        source_chain: runtime_types::ismp::host::StateMachine,
                        request_nonce: ::core::primitive::u64,
                    },
                    #[codec(index = 4)]
                    ///Request processed successfully
                    Request {
                        dest_chain: runtime_types::ismp::host::StateMachine,
                        source_chain: runtime_types::ismp::host::StateMachine,
                        request_nonce: ::core::primitive::u64,
                    },
                    #[codec(index = 5)]
                    ///Some errors handling some ismp messages
                    HandlingErrors {
                        errors: ::std::vec::Vec<runtime_types::pallet_ismp::errors::HandlingError>,
                    },
                }
            }
            pub mod router {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Receipt {
                    #[codec(index = 0)]
                    Ok,
                }
            }
        }
        pub mod pallet_session {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Sets the session key(s) of the function caller to `keys`.
                    ///Allows an account to set its session key prior to becoming a validator.
                    ///This doesn't take effect until the next session.
                    ///
                    ///The dispatch origin of this function must be signed.
                    ///
                    ///## Complexity
                    /// - `O(1)`. Actual cost depends on the number of length of
                    ///   `T::Keys::key_ids()` which is
                    ///  fixed.
                    set_keys {
                        keys: runtime_types::hyperbridge_runtime::SessionKeys,
                        proof: ::std::vec::Vec<::core::primitive::u8>,
                    },
                    #[codec(index = 1)]
                    ///Removes any session key(s) of the function caller.
                    ///
                    ///This doesn't take effect until the next session.
                    ///
                    ///The dispatch origin of this function must be Signed and the account must be
                    /// either be convertible to a validator ID using the
                    /// chain's typical addressing system (this usually
                    /// means being a controller account) or directly convertible into a validator
                    /// ID (which usually means being a stash account).
                    ///
                    ///## Complexity
                    /// - `O(1)` in number of key types. Actual cost depends on the number of
                    ///   length of
                    ///  `T::Keys::key_ids()` which is fixed.
                    purge_keys,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Error for the session pallet.
                pub enum Error {
                    #[codec(index = 0)]
                    ///Invalid ownership proof.
                    InvalidProof,
                    #[codec(index = 1)]
                    ///No associated validator ID for account.
                    NoAssociatedValidatorId,
                    #[codec(index = 2)]
                    ///Registered duplicate key.
                    DuplicatedKey,
                    #[codec(index = 3)]
                    ///No keys are associated with this account.
                    NoKeys,
                    #[codec(index = 4)]
                    ///Key setting account is not live, so it's impossible to associate keys.
                    NoAccount,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///New session has happened. Note that the argument is the session index, not
                    /// the block number as the type might suggest.
                    NewSession { session_index: ::core::primitive::u32 },
                }
            }
        }
        pub mod pallet_sudo {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Authenticates the sudo key and dispatches a function call with `Root`
                    /// origin.
                    ///
                    ///The dispatch origin for this call must be _Signed_.
                    ///
                    ///## Complexity
                    /// - O(1).
                    sudo {
                        call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
                    },
                    #[codec(index = 1)]
                    ///Authenticates the sudo key and dispatches a function call with `Root`
                    /// origin. This function does not check the weight of the
                    /// call, and instead allows the Sudo user to specify the
                    /// weight of the call.
                    ///
                    ///The dispatch origin for this call must be _Signed_.
                    ///
                    ///## Complexity
                    /// - O(1).
                    sudo_unchecked_weight {
                        call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
                        weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 2)]
                    ///Authenticates the current sudo key and sets the given AccountId (`new`) as
                    /// the new sudo key.
                    ///
                    ///The dispatch origin for this call must be _Signed_.
                    ///
                    ///## Complexity
                    /// - O(1).
                    set_key { new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()> },
                    #[codec(index = 3)]
                    ///Authenticates the sudo key and dispatches a function call with `Signed`
                    /// origin from a given account.
                    ///
                    ///The dispatch origin for this call must be _Signed_.
                    ///
                    ///## Complexity
                    /// - O(1).
                    sudo_as {
                        who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
                        call: ::std::boxed::Box<runtime_types::hyperbridge_runtime::RuntimeCall>,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Error for the Sudo pallet
                pub enum Error {
                    #[codec(index = 0)]
                    ///Sender must be the Sudo account
                    RequireSudo,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///A sudo just took place. \[result\]
                    Sudid {
                        sudo_result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                    #[codec(index = 1)]
                    ///The \[sudoer\] just switched identity; the old key is supplied if one
                    /// existed.
                    KeyChanged { old_sudoer: ::core::option::Option<::subxt::utils::AccountId32> },
                    #[codec(index = 2)]
                    ///A sudo just took place. \[result\]
                    SudoAsDone {
                        sudo_result:
                            ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
                    },
                }
            }
        }
        pub mod pallet_timestamp {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    ///Set the current time.
                    ///
                    ///This call should be invoked exactly once per block. It will panic at the
                    /// finalization phase, if this call hasn't been invoked by
                    /// that time.
                    ///
                    ///The timestamp should be greater than the previous one by the amount
                    /// specified by `MinimumPeriod`.
                    ///
                    ///The dispatch origin for this call must be `Inherent`.
                    ///
                    ///## Complexity
                    /// - `O(1)` (Note that implementations of `OnTimestampSet` must also be
                    ///   `O(1)`)
                    /// - 1 storage read and 1 storage mutation (codec `O(1)`). (because of
                    ///   `DidUpdate::take` in
                    ///  `on_finalize`)
                    /// - 1 event handler `on_timestamp_set`. Must be `O(1)`.
                    set {
                        #[codec(compact)]
                        now: ::core::primitive::u64,
                    },
                }
            }
        }
        pub mod pallet_transaction_payment {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///A transaction fee `actual_fee`, of which `tip` was added to the minimum
                    /// inclusion fee, has been paid by `who`.
                    TransactionFeePaid {
                        who: ::subxt::utils::AccountId32,
                        actual_fee: ::core::primitive::u128,
                        tip: ::core::primitive::u128,
                    },
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ChargeTransactionPayment(#[codec(compact)] pub ::core::primitive::u128);
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum Releases {
                #[codec(index = 0)]
                V1Ancient,
                #[codec(index = 1)]
                V2,
            }
        }
        pub mod pallet_xcm {
            use super::runtime_types;
            pub mod pallet {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                ///Contains one variant per dispatchable that can be called by an extrinsic.
                pub enum Call {
                    #[codec(index = 0)]
                    send {
                        dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
                    },
                    #[codec(index = 1)]
                    ///Teleport some assets from the local chain to some destination chain.
                    ///
                    ///Fee payment on the destination side is made from the asset in the `assets`
                    /// vector of index `fee_asset_item`. The weight limit for
                    /// fees is not provided and thus is unlimited,
                    /// with all fees taken as needed from the asset.
                    ///
                    /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                    /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                    ///   Parachain(..))` to send
                    ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                    /// parachain.
                    /// - `beneficiary`: A beneficiary location for the assets in the context of
                    ///   `dest`. Will generally be
                    ///  an `AccountId32` value.
                    /// - `assets`: The assets to be withdrawn. The first item should be the
                    ///   currency used to to pay the fee on the
                    ///  `dest` side. May not be empty.
                    /// - `fee_asset_item`: The index into `assets` of the item which should be
                    ///   used to pay
                    ///  fees.
                    teleport_assets {
                        dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                        fee_asset_item: ::core::primitive::u32,
                    },
                    #[codec(index = 2)]
                    ///Transfer some assets from the local chain to the sovereign account of a
                    /// destination chain and forward a notification XCM.
                    ///
                    ///Fee payment on the destination side is made from the asset in the `assets`
                    /// vector of index `fee_asset_item`. The weight limit for
                    /// fees is not provided and thus is unlimited,
                    /// with all fees taken as needed from the asset.
                    ///
                    /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                    /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                    ///   Parachain(..))` to send
                    ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                    /// parachain.
                    /// - `beneficiary`: A beneficiary location for the assets in the context of
                    ///   `dest`. Will generally be
                    ///  an `AccountId32` value.
                    /// - `assets`: The assets to be withdrawn. This should include the assets used
                    ///   to pay the fee on the
                    ///  `dest` side.
                    /// - `fee_asset_item`: The index into `assets` of the item which should be
                    ///   used to pay
                    ///  fees.
                    reserve_transfer_assets {
                        dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                        fee_asset_item: ::core::primitive::u32,
                    },
                    #[codec(index = 3)]
                    ///Execute an XCM message from a local, signed, origin.
                    ///
                    ///An event is deposited indicating whether `msg` could be executed completely
                    /// or only partially.
                    ///
                    ///No more than `max_weight` will be used in its attempted execution. If this
                    /// is less than the maximum amount of weight that the
                    /// message could take to be executed, then no execution
                    /// attempt will be made.
                    ///
                    ///NOTE: A successful return to this does *not* imply that the `msg` was
                    /// executed successfully to completion; only that *some*
                    /// of it was executed.
                    execute {
                        message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
                        max_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 4)]
                    ///Extoll that a particular destination can be communicated with through a
                    /// particular version of XCM.
                    ///
                    /// - `origin`: Must be Root.
                    /// - `location`: The destination that is being described.
                    /// - `xcm_version`: The latest version of XCM that `location` supports.
                    force_xcm_version {
                        location:
                            ::std::boxed::Box<runtime_types::xcm::v3::multilocation::MultiLocation>,
                        xcm_version: ::core::primitive::u32,
                    },
                    #[codec(index = 5)]
                    ///Set a safe XCM version (the version that XCM should be encoded with if the
                    /// most recent version a destination can accept is
                    /// unknown).
                    ///
                    /// - `origin`: Must be Root.
                    /// - `maybe_xcm_version`: The default XCM encoding version, or `None` to
                    ///   disable.
                    force_default_xcm_version {
                        maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
                    },
                    #[codec(index = 6)]
                    ///Ask a location to notify us regarding their XCM version and any changes to
                    /// it.
                    ///
                    /// - `origin`: Must be Root.
                    /// - `location`: The location to which we should subscribe for XCM version
                    ///   notifications.
                    force_subscribe_version_notify {
                        location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                    },
                    #[codec(index = 7)]
                    ///Require that a particular destination should no longer notify us regarding
                    /// any XCM version changes.
                    ///
                    /// - `origin`: Must be Root.
                    /// - `location`: The location to which we are currently subscribed for XCM
                    ///   version
                    ///  notifications which we no longer desire.
                    force_unsubscribe_version_notify {
                        location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                    },
                    #[codec(index = 8)]
                    ///Transfer some assets from the local chain to the sovereign account of a
                    /// destination chain and forward a notification XCM.
                    ///
                    ///Fee payment on the destination side is made from the asset in the `assets`
                    /// vector of index `fee_asset_item`, up to enough to pay
                    /// for `weight_limit` of weight. If more weight
                    /// is needed than `weight_limit`, then the operation will fail and the assets
                    /// send may be at risk.
                    ///
                    /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                    /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                    ///   Parachain(..))` to send
                    ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                    /// parachain.
                    /// - `beneficiary`: A beneficiary location for the assets in the context of
                    ///   `dest`. Will generally be
                    ///  an `AccountId32` value.
                    /// - `assets`: The assets to be withdrawn. This should include the assets used
                    ///   to pay the fee on the
                    ///  `dest` side.
                    /// - `fee_asset_item`: The index into `assets` of the item which should be
                    ///   used to pay
                    ///  fees.
                    /// - `weight_limit`: The remote-side weight limit, if any, for the XCM fee
                    ///   purchase.
                    limited_reserve_transfer_assets {
                        dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                        fee_asset_item: ::core::primitive::u32,
                        weight_limit: runtime_types::xcm::v3::WeightLimit,
                    },
                    #[codec(index = 9)]
                    ///Teleport some assets from the local chain to some destination chain.
                    ///
                    ///Fee payment on the destination side is made from the asset in the `assets`
                    /// vector of index `fee_asset_item`, up to enough to pay
                    /// for `weight_limit` of weight. If more weight
                    /// is needed than `weight_limit`, then the operation will fail and the assets
                    /// send may be at risk.
                    ///
                    /// - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
                    /// - `dest`: Destination context for the assets. Will typically be `X2(Parent,
                    ///   Parachain(..))` to send
                    ///  from parachain to parachain, or `X1(Parachain(..))` to send from relay to
                    /// parachain.
                    /// - `beneficiary`: A beneficiary location for the assets in the context of
                    ///   `dest`. Will generally be
                    ///  an `AccountId32` value.
                    /// - `assets`: The assets to be withdrawn. The first item should be the
                    ///   currency used to to pay the fee on the
                    ///  `dest` side. May not be empty.
                    /// - `fee_asset_item`: The index into `assets` of the item which should be
                    ///   used to pay
                    ///  fees.
                    /// - `weight_limit`: The remote-side weight limit, if any, for the XCM fee
                    ///   purchase.
                    limited_teleport_assets {
                        dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
                        assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
                        fee_asset_item: ::core::primitive::u32,
                        weight_limit: runtime_types::xcm::v3::WeightLimit,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                Custom [dispatch errors](https://docs.substrate.io/main-docs/build/events-errors/)
                of this pallet.
                */
                pub enum Error {
                    #[codec(index = 0)]
                    ///The desired destination was unreachable, generally because there is a no
                    /// way of routing to it.
                    Unreachable,
                    #[codec(index = 1)]
                    ///There was some other issue (i.e. not to do with routing) in sending the
                    /// message. Perhaps a lack of space for buffering the
                    /// message.
                    SendFailure,
                    #[codec(index = 2)]
                    ///The message execution fails the filter.
                    Filtered,
                    #[codec(index = 3)]
                    ///The message's weight could not be determined.
                    UnweighableMessage,
                    #[codec(index = 4)]
                    ///The destination `MultiLocation` provided cannot be inverted.
                    DestinationNotInvertible,
                    #[codec(index = 5)]
                    ///The assets to be sent are empty.
                    Empty,
                    #[codec(index = 6)]
                    ///Could not re-anchor the assets to declare the fees for the destination
                    /// chain.
                    CannotReanchor,
                    #[codec(index = 7)]
                    ///Too many assets have been attempted for transfer.
                    TooManyAssets,
                    #[codec(index = 8)]
                    ///Origin is invalid for sending.
                    InvalidOrigin,
                    #[codec(index = 9)]
                    ///The version of the `Versioned` value used is not able to be interpreted.
                    BadVersion,
                    #[codec(index = 10)]
                    ///The given location could not be used (e.g. because it cannot be expressed
                    /// in the desired version of XCM).
                    BadLocation,
                    #[codec(index = 11)]
                    ///The referenced subscription could not be found.
                    NoSubscription,
                    #[codec(index = 12)]
                    ///The location is invalid since it already has a subscription from us.
                    AlreadySubscribed,
                    #[codec(index = 13)]
                    ///Invalid asset for the operation.
                    InvalidAsset,
                    #[codec(index = 14)]
                    ///The owner does not own (all) of the asset that they wish to do the
                    /// operation on.
                    LowBalance,
                    #[codec(index = 15)]
                    ///The asset owner has too many locks on the asset.
                    TooManyLocks,
                    #[codec(index = 16)]
                    ///The given account is not an identifiable sovereign account for any
                    /// location.
                    AccountNotSovereign,
                    #[codec(index = 17)]
                    ///The operation required fees to be paid which the initiator could not meet.
                    FeesNotMet,
                    #[codec(index = 18)]
                    ///A remote lock with the corresponding data could not be found.
                    LockNotFound,
                    #[codec(index = 19)]
                    ///The unlock operation cannot succeed because there are still users of the
                    /// lock.
                    InUse,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                /**
                The [event](https://docs.substrate.io/main-docs/build/events-errors/) emitted
                by this pallet.
                */
                pub enum Event {
                    #[codec(index = 0)]
                    ///Execution of an XCM message was attempted.
                    ///
                    ///\[ outcome \]
                    Attempted(runtime_types::xcm::v3::traits::Outcome),
                    #[codec(index = 1)]
                    ///A XCM message was sent.
                    ///
                    ///\[ origin, destination, message \]
                    Sent(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::Xcm,
                    ),
                    #[codec(index = 2)]
                    ///Query response received which does not match a registered query. This may
                    /// be because a matching query was never registered, it
                    /// may be because it is a duplicate response, or
                    /// because the query timed out.
                    ///
                    ///\[ origin location, id \]
                    UnexpectedResponse(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                    ),
                    #[codec(index = 3)]
                    ///Query response has been received and is ready for taking with
                    /// `take_response`. There is no registered notification
                    /// call.
                    ///
                    ///\[ id, response \]
                    ResponseReady(::core::primitive::u64, runtime_types::xcm::v3::Response),
                    #[codec(index = 4)]
                    ///Query response has been received and query is removed. The registered
                    /// notification has been dispatched and executed
                    /// successfully.
                    ///
                    ///\[ id, pallet index, call index \]
                    Notified(::core::primitive::u64, ::core::primitive::u8, ::core::primitive::u8),
                    #[codec(index = 5)]
                    ///Query response has been received and query is removed. The registered
                    /// notification could not be dispatched because the
                    /// dispatch weight is greater than the maximum weight
                    /// originally budgeted by this runtime for the query result.
                    ///
                    ///\[ id, pallet index, call index, actual weight, max budgeted weight \]
                    NotifyOverweight(
                        ::core::primitive::u64,
                        ::core::primitive::u8,
                        ::core::primitive::u8,
                        runtime_types::sp_weights::weight_v2::Weight,
                        runtime_types::sp_weights::weight_v2::Weight,
                    ),
                    #[codec(index = 6)]
                    ///Query response has been received and query is removed. There was a general
                    /// error with dispatching the notification call.
                    ///
                    ///\[ id, pallet index, call index \]
                    NotifyDispatchError(
                        ::core::primitive::u64,
                        ::core::primitive::u8,
                        ::core::primitive::u8,
                    ),
                    #[codec(index = 7)]
                    ///Query response has been received and query is removed. The dispatch was
                    /// unable to be decoded into a `Call`; this might be due
                    /// to dispatch function having a signature which
                    /// is not `(origin, QueryId, Response)`.
                    ///
                    ///\[ id, pallet index, call index \]
                    NotifyDecodeFailed(
                        ::core::primitive::u64,
                        ::core::primitive::u8,
                        ::core::primitive::u8,
                    ),
                    #[codec(index = 8)]
                    ///Expected query response has been received but the origin location of the
                    /// response does not match that expected. The query
                    /// remains registered for a later, valid, response to
                    /// be received and acted upon.
                    ///
                    ///\[ origin location, id, expected location \]
                    InvalidResponder(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                        ::core::option::Option<
                            runtime_types::xcm::v3::multilocation::MultiLocation,
                        >,
                    ),
                    #[codec(index = 9)]
                    ///Expected query response has been received but the expected origin location
                    /// placed in storage by this runtime previously cannot be
                    /// decoded. The query remains registered.
                    ///
                    ///This is unexpected (since a location placed in storage in a previously
                    /// executing runtime should be readable prior to query
                    /// timeout) and dangerous since the possibly
                    /// valid response will be dropped. Manual governance intervention is probably
                    /// going to be needed.
                    ///
                    ///\[ origin location, id \]
                    InvalidResponderVersion(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                    ),
                    #[codec(index = 10)]
                    ///Received query response has been read and removed.
                    ///
                    ///\[ id \]
                    ResponseTaken(::core::primitive::u64),
                    #[codec(index = 11)]
                    ///Some assets have been placed in an asset trap.
                    ///
                    ///\[ hash, origin, assets \]
                    AssetsTrapped(
                        ::subxt::utils::H256,
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::VersionedMultiAssets,
                    ),
                    #[codec(index = 12)]
                    ///An XCM version change notification message has been attempted to be sent.
                    ///
                    ///The cost of sending it (borne by the chain) is included.
                    ///
                    ///\[ destination, result, cost \]
                    VersionChangeNotified(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u32,
                        runtime_types::xcm::v3::multiasset::MultiAssets,
                    ),
                    #[codec(index = 13)]
                    ///The supported version of a location has been changed. This might be through
                    /// an automatic notification or a manual intervention.
                    ///
                    ///\[ location, XCM version \]
                    SupportedVersionChanged(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u32,
                    ),
                    #[codec(index = 14)]
                    ///A given location which had a version change subscription was dropped owing
                    /// to an error sending the notification to it.
                    ///
                    ///\[ location, query ID, error \]
                    NotifyTargetSendFail(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                        runtime_types::xcm::v3::traits::Error,
                    ),
                    #[codec(index = 15)]
                    ///A given location which had a version change subscription was dropped owing
                    /// to an error migrating the location to our new XCM
                    /// format.
                    ///
                    ///\[ location, query ID \]
                    NotifyTargetMigrationFail(
                        runtime_types::xcm::VersionedMultiLocation,
                        ::core::primitive::u64,
                    ),
                    #[codec(index = 16)]
                    ///Expected query response has been received but the expected querier location
                    /// placed in storage by this runtime previously cannot be
                    /// decoded. The query remains registered.
                    ///
                    ///This is unexpected (since a location placed in storage in a previously
                    /// executing runtime should be readable prior to query
                    /// timeout) and dangerous since the possibly
                    /// valid response will be dropped. Manual governance intervention is probably
                    /// going to be needed.
                    ///
                    ///\[ origin location, id \]
                    InvalidQuerierVersion(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                    ),
                    #[codec(index = 17)]
                    ///Expected query response has been received but the querier location of the
                    /// response does not match the expected. The query remains
                    /// registered for a later, valid, response to be received
                    /// and acted upon.
                    ///
                    ///\[ origin location, id, expected querier, maybe actual querier \]
                    InvalidQuerier(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::primitive::u64,
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        ::core::option::Option<
                            runtime_types::xcm::v3::multilocation::MultiLocation,
                        >,
                    ),
                    #[codec(index = 18)]
                    ///A remote has requested XCM version change notification from us and we have
                    /// honored it. A version information message is sent to
                    /// them and its cost is included.
                    ///
                    ///\[ destination location, cost \]
                    VersionNotifyStarted(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::multiasset::MultiAssets,
                    ),
                    #[codec(index = 19)]
                    ///We have requested that a remote chain sends us XCM version change
                    /// notifications.
                    ///
                    ///\[ destination location, cost \]
                    VersionNotifyRequested(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::multiasset::MultiAssets,
                    ),
                    #[codec(index = 20)]
                    ///We have requested that a remote chain stops sending us XCM version change
                    /// notifications.
                    ///
                    ///\[ destination location, cost \]
                    VersionNotifyUnrequested(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::multiasset::MultiAssets,
                    ),
                    #[codec(index = 21)]
                    ///Fees were paid from a location for an operation (often for using
                    /// `SendXcm`).
                    ///
                    ///\[ paying location, fees \]
                    FeesPaid(
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::v3::multiasset::MultiAssets,
                    ),
                    #[codec(index = 22)]
                    ///Some assets have been claimed from an asset trap
                    ///
                    ///\[ hash, origin, assets \]
                    AssetsClaimed(
                        ::subxt::utils::H256,
                        runtime_types::xcm::v3::multilocation::MultiLocation,
                        runtime_types::xcm::VersionedMultiAssets,
                    ),
                }
            }
        }
        pub mod polkadot_core_primitives {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct InboundDownwardMessage<_0> {
                pub sent_at: _0,
                pub msg: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct InboundHrmpMessage<_0> {
                pub sent_at: _0,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct OutboundHrmpMessage<_0> {
                pub recipient: _0,
                pub data: ::std::vec::Vec<::core::primitive::u8>,
            }
        }
        pub mod polkadot_parachain {
            use super::runtime_types;
            pub mod primitives {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct HeadData(pub ::std::vec::Vec<::core::primitive::u8>);
                #[derive(
                    ::subxt::ext::codec::CompactAs,
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Id(pub ::core::primitive::u32);
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum XcmpMessageFormat {
                    #[codec(index = 0)]
                    ConcatenatedVersionedXcm,
                    #[codec(index = 1)]
                    ConcatenatedEncodedBlob,
                    #[codec(index = 2)]
                    Signals,
                }
            }
        }
        pub mod polkadot_primitives {
            use super::runtime_types;
            pub mod v2 {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct AbridgedHostConfiguration {
                    pub max_code_size: ::core::primitive::u32,
                    pub max_head_data_size: ::core::primitive::u32,
                    pub max_upward_queue_count: ::core::primitive::u32,
                    pub max_upward_queue_size: ::core::primitive::u32,
                    pub max_upward_message_size: ::core::primitive::u32,
                    pub max_upward_message_num_per_candidate: ::core::primitive::u32,
                    pub hrmp_max_message_num_per_candidate: ::core::primitive::u32,
                    pub validation_upgrade_cooldown: ::core::primitive::u32,
                    pub validation_upgrade_delay: ::core::primitive::u32,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct AbridgedHrmpChannel {
                    pub max_capacity: ::core::primitive::u32,
                    pub max_total_size: ::core::primitive::u32,
                    pub max_message_size: ::core::primitive::u32,
                    pub msg_count: ::core::primitive::u32,
                    pub total_size: ::core::primitive::u32,
                    pub mqc_head: ::core::option::Option<::subxt::utils::H256>,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct PersistedValidationData<_0, _1> {
                    pub parent_head: runtime_types::polkadot_parachain::primitives::HeadData,
                    pub relay_parent_number: _1,
                    pub relay_parent_storage_root: _0,
                    pub max_pov_size: _1,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum UpgradeRestriction {
                    #[codec(index = 0)]
                    Present,
                }
            }
        }
        pub mod sp_arithmetic {
            use super::runtime_types;
            pub mod fixed_point {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::CompactAs,
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct FixedU128(pub ::core::primitive::u128);
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum ArithmeticError {
                #[codec(index = 0)]
                Underflow,
                #[codec(index = 1)]
                Overflow,
                #[codec(index = 2)]
                DivisionByZero,
            }
        }
        pub mod sp_consensus_aura {
            use super::runtime_types;
            pub mod sr25519 {
                use super::runtime_types;
                pub mod app_sr25519 {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct Public(pub runtime_types::sp_core::sr25519::Public);
                }
            }
        }
        pub mod sp_consensus_slots {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::CompactAs,
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct Slot(pub ::core::primitive::u64);
        }
        pub mod sp_core {
            use super::runtime_types;
            pub mod crypto {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct KeyTypeId(pub [::core::primitive::u8; 4usize]);
            }
            pub mod ecdsa {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Signature(pub [::core::primitive::u8; 65usize]);
            }
            pub mod ed25519 {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Signature(pub [::core::primitive::u8; 64usize]);
            }
            pub mod sr25519 {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Public(pub [::core::primitive::u8; 32usize]);
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Signature(pub [::core::primitive::u8; 64usize]);
            }
        }
        pub mod sp_runtime {
            use super::runtime_types;
            pub mod generic {
                use super::runtime_types;
                pub mod digest {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct Digest {
                        pub logs:
                            ::std::vec::Vec<runtime_types::sp_runtime::generic::digest::DigestItem>,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum DigestItem {
                        #[codec(index = 6)]
                        PreRuntime(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 4)]
                        Consensus(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 5)]
                        Seal(
                            [::core::primitive::u8; 4usize],
                            ::std::vec::Vec<::core::primitive::u8>,
                        ),
                        #[codec(index = 0)]
                        Other(::std::vec::Vec<::core::primitive::u8>),
                        #[codec(index = 8)]
                        RuntimeEnvironmentUpdated,
                    }
                }
                pub mod era {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Era {
                        #[codec(index = 0)]
                        Immortal,
                        #[codec(index = 1)]
                        Mortal1(::core::primitive::u8),
                        #[codec(index = 2)]
                        Mortal2(::core::primitive::u8),
                        #[codec(index = 3)]
                        Mortal3(::core::primitive::u8),
                        #[codec(index = 4)]
                        Mortal4(::core::primitive::u8),
                        #[codec(index = 5)]
                        Mortal5(::core::primitive::u8),
                        #[codec(index = 6)]
                        Mortal6(::core::primitive::u8),
                        #[codec(index = 7)]
                        Mortal7(::core::primitive::u8),
                        #[codec(index = 8)]
                        Mortal8(::core::primitive::u8),
                        #[codec(index = 9)]
                        Mortal9(::core::primitive::u8),
                        #[codec(index = 10)]
                        Mortal10(::core::primitive::u8),
                        #[codec(index = 11)]
                        Mortal11(::core::primitive::u8),
                        #[codec(index = 12)]
                        Mortal12(::core::primitive::u8),
                        #[codec(index = 13)]
                        Mortal13(::core::primitive::u8),
                        #[codec(index = 14)]
                        Mortal14(::core::primitive::u8),
                        #[codec(index = 15)]
                        Mortal15(::core::primitive::u8),
                        #[codec(index = 16)]
                        Mortal16(::core::primitive::u8),
                        #[codec(index = 17)]
                        Mortal17(::core::primitive::u8),
                        #[codec(index = 18)]
                        Mortal18(::core::primitive::u8),
                        #[codec(index = 19)]
                        Mortal19(::core::primitive::u8),
                        #[codec(index = 20)]
                        Mortal20(::core::primitive::u8),
                        #[codec(index = 21)]
                        Mortal21(::core::primitive::u8),
                        #[codec(index = 22)]
                        Mortal22(::core::primitive::u8),
                        #[codec(index = 23)]
                        Mortal23(::core::primitive::u8),
                        #[codec(index = 24)]
                        Mortal24(::core::primitive::u8),
                        #[codec(index = 25)]
                        Mortal25(::core::primitive::u8),
                        #[codec(index = 26)]
                        Mortal26(::core::primitive::u8),
                        #[codec(index = 27)]
                        Mortal27(::core::primitive::u8),
                        #[codec(index = 28)]
                        Mortal28(::core::primitive::u8),
                        #[codec(index = 29)]
                        Mortal29(::core::primitive::u8),
                        #[codec(index = 30)]
                        Mortal30(::core::primitive::u8),
                        #[codec(index = 31)]
                        Mortal31(::core::primitive::u8),
                        #[codec(index = 32)]
                        Mortal32(::core::primitive::u8),
                        #[codec(index = 33)]
                        Mortal33(::core::primitive::u8),
                        #[codec(index = 34)]
                        Mortal34(::core::primitive::u8),
                        #[codec(index = 35)]
                        Mortal35(::core::primitive::u8),
                        #[codec(index = 36)]
                        Mortal36(::core::primitive::u8),
                        #[codec(index = 37)]
                        Mortal37(::core::primitive::u8),
                        #[codec(index = 38)]
                        Mortal38(::core::primitive::u8),
                        #[codec(index = 39)]
                        Mortal39(::core::primitive::u8),
                        #[codec(index = 40)]
                        Mortal40(::core::primitive::u8),
                        #[codec(index = 41)]
                        Mortal41(::core::primitive::u8),
                        #[codec(index = 42)]
                        Mortal42(::core::primitive::u8),
                        #[codec(index = 43)]
                        Mortal43(::core::primitive::u8),
                        #[codec(index = 44)]
                        Mortal44(::core::primitive::u8),
                        #[codec(index = 45)]
                        Mortal45(::core::primitive::u8),
                        #[codec(index = 46)]
                        Mortal46(::core::primitive::u8),
                        #[codec(index = 47)]
                        Mortal47(::core::primitive::u8),
                        #[codec(index = 48)]
                        Mortal48(::core::primitive::u8),
                        #[codec(index = 49)]
                        Mortal49(::core::primitive::u8),
                        #[codec(index = 50)]
                        Mortal50(::core::primitive::u8),
                        #[codec(index = 51)]
                        Mortal51(::core::primitive::u8),
                        #[codec(index = 52)]
                        Mortal52(::core::primitive::u8),
                        #[codec(index = 53)]
                        Mortal53(::core::primitive::u8),
                        #[codec(index = 54)]
                        Mortal54(::core::primitive::u8),
                        #[codec(index = 55)]
                        Mortal55(::core::primitive::u8),
                        #[codec(index = 56)]
                        Mortal56(::core::primitive::u8),
                        #[codec(index = 57)]
                        Mortal57(::core::primitive::u8),
                        #[codec(index = 58)]
                        Mortal58(::core::primitive::u8),
                        #[codec(index = 59)]
                        Mortal59(::core::primitive::u8),
                        #[codec(index = 60)]
                        Mortal60(::core::primitive::u8),
                        #[codec(index = 61)]
                        Mortal61(::core::primitive::u8),
                        #[codec(index = 62)]
                        Mortal62(::core::primitive::u8),
                        #[codec(index = 63)]
                        Mortal63(::core::primitive::u8),
                        #[codec(index = 64)]
                        Mortal64(::core::primitive::u8),
                        #[codec(index = 65)]
                        Mortal65(::core::primitive::u8),
                        #[codec(index = 66)]
                        Mortal66(::core::primitive::u8),
                        #[codec(index = 67)]
                        Mortal67(::core::primitive::u8),
                        #[codec(index = 68)]
                        Mortal68(::core::primitive::u8),
                        #[codec(index = 69)]
                        Mortal69(::core::primitive::u8),
                        #[codec(index = 70)]
                        Mortal70(::core::primitive::u8),
                        #[codec(index = 71)]
                        Mortal71(::core::primitive::u8),
                        #[codec(index = 72)]
                        Mortal72(::core::primitive::u8),
                        #[codec(index = 73)]
                        Mortal73(::core::primitive::u8),
                        #[codec(index = 74)]
                        Mortal74(::core::primitive::u8),
                        #[codec(index = 75)]
                        Mortal75(::core::primitive::u8),
                        #[codec(index = 76)]
                        Mortal76(::core::primitive::u8),
                        #[codec(index = 77)]
                        Mortal77(::core::primitive::u8),
                        #[codec(index = 78)]
                        Mortal78(::core::primitive::u8),
                        #[codec(index = 79)]
                        Mortal79(::core::primitive::u8),
                        #[codec(index = 80)]
                        Mortal80(::core::primitive::u8),
                        #[codec(index = 81)]
                        Mortal81(::core::primitive::u8),
                        #[codec(index = 82)]
                        Mortal82(::core::primitive::u8),
                        #[codec(index = 83)]
                        Mortal83(::core::primitive::u8),
                        #[codec(index = 84)]
                        Mortal84(::core::primitive::u8),
                        #[codec(index = 85)]
                        Mortal85(::core::primitive::u8),
                        #[codec(index = 86)]
                        Mortal86(::core::primitive::u8),
                        #[codec(index = 87)]
                        Mortal87(::core::primitive::u8),
                        #[codec(index = 88)]
                        Mortal88(::core::primitive::u8),
                        #[codec(index = 89)]
                        Mortal89(::core::primitive::u8),
                        #[codec(index = 90)]
                        Mortal90(::core::primitive::u8),
                        #[codec(index = 91)]
                        Mortal91(::core::primitive::u8),
                        #[codec(index = 92)]
                        Mortal92(::core::primitive::u8),
                        #[codec(index = 93)]
                        Mortal93(::core::primitive::u8),
                        #[codec(index = 94)]
                        Mortal94(::core::primitive::u8),
                        #[codec(index = 95)]
                        Mortal95(::core::primitive::u8),
                        #[codec(index = 96)]
                        Mortal96(::core::primitive::u8),
                        #[codec(index = 97)]
                        Mortal97(::core::primitive::u8),
                        #[codec(index = 98)]
                        Mortal98(::core::primitive::u8),
                        #[codec(index = 99)]
                        Mortal99(::core::primitive::u8),
                        #[codec(index = 100)]
                        Mortal100(::core::primitive::u8),
                        #[codec(index = 101)]
                        Mortal101(::core::primitive::u8),
                        #[codec(index = 102)]
                        Mortal102(::core::primitive::u8),
                        #[codec(index = 103)]
                        Mortal103(::core::primitive::u8),
                        #[codec(index = 104)]
                        Mortal104(::core::primitive::u8),
                        #[codec(index = 105)]
                        Mortal105(::core::primitive::u8),
                        #[codec(index = 106)]
                        Mortal106(::core::primitive::u8),
                        #[codec(index = 107)]
                        Mortal107(::core::primitive::u8),
                        #[codec(index = 108)]
                        Mortal108(::core::primitive::u8),
                        #[codec(index = 109)]
                        Mortal109(::core::primitive::u8),
                        #[codec(index = 110)]
                        Mortal110(::core::primitive::u8),
                        #[codec(index = 111)]
                        Mortal111(::core::primitive::u8),
                        #[codec(index = 112)]
                        Mortal112(::core::primitive::u8),
                        #[codec(index = 113)]
                        Mortal113(::core::primitive::u8),
                        #[codec(index = 114)]
                        Mortal114(::core::primitive::u8),
                        #[codec(index = 115)]
                        Mortal115(::core::primitive::u8),
                        #[codec(index = 116)]
                        Mortal116(::core::primitive::u8),
                        #[codec(index = 117)]
                        Mortal117(::core::primitive::u8),
                        #[codec(index = 118)]
                        Mortal118(::core::primitive::u8),
                        #[codec(index = 119)]
                        Mortal119(::core::primitive::u8),
                        #[codec(index = 120)]
                        Mortal120(::core::primitive::u8),
                        #[codec(index = 121)]
                        Mortal121(::core::primitive::u8),
                        #[codec(index = 122)]
                        Mortal122(::core::primitive::u8),
                        #[codec(index = 123)]
                        Mortal123(::core::primitive::u8),
                        #[codec(index = 124)]
                        Mortal124(::core::primitive::u8),
                        #[codec(index = 125)]
                        Mortal125(::core::primitive::u8),
                        #[codec(index = 126)]
                        Mortal126(::core::primitive::u8),
                        #[codec(index = 127)]
                        Mortal127(::core::primitive::u8),
                        #[codec(index = 128)]
                        Mortal128(::core::primitive::u8),
                        #[codec(index = 129)]
                        Mortal129(::core::primitive::u8),
                        #[codec(index = 130)]
                        Mortal130(::core::primitive::u8),
                        #[codec(index = 131)]
                        Mortal131(::core::primitive::u8),
                        #[codec(index = 132)]
                        Mortal132(::core::primitive::u8),
                        #[codec(index = 133)]
                        Mortal133(::core::primitive::u8),
                        #[codec(index = 134)]
                        Mortal134(::core::primitive::u8),
                        #[codec(index = 135)]
                        Mortal135(::core::primitive::u8),
                        #[codec(index = 136)]
                        Mortal136(::core::primitive::u8),
                        #[codec(index = 137)]
                        Mortal137(::core::primitive::u8),
                        #[codec(index = 138)]
                        Mortal138(::core::primitive::u8),
                        #[codec(index = 139)]
                        Mortal139(::core::primitive::u8),
                        #[codec(index = 140)]
                        Mortal140(::core::primitive::u8),
                        #[codec(index = 141)]
                        Mortal141(::core::primitive::u8),
                        #[codec(index = 142)]
                        Mortal142(::core::primitive::u8),
                        #[codec(index = 143)]
                        Mortal143(::core::primitive::u8),
                        #[codec(index = 144)]
                        Mortal144(::core::primitive::u8),
                        #[codec(index = 145)]
                        Mortal145(::core::primitive::u8),
                        #[codec(index = 146)]
                        Mortal146(::core::primitive::u8),
                        #[codec(index = 147)]
                        Mortal147(::core::primitive::u8),
                        #[codec(index = 148)]
                        Mortal148(::core::primitive::u8),
                        #[codec(index = 149)]
                        Mortal149(::core::primitive::u8),
                        #[codec(index = 150)]
                        Mortal150(::core::primitive::u8),
                        #[codec(index = 151)]
                        Mortal151(::core::primitive::u8),
                        #[codec(index = 152)]
                        Mortal152(::core::primitive::u8),
                        #[codec(index = 153)]
                        Mortal153(::core::primitive::u8),
                        #[codec(index = 154)]
                        Mortal154(::core::primitive::u8),
                        #[codec(index = 155)]
                        Mortal155(::core::primitive::u8),
                        #[codec(index = 156)]
                        Mortal156(::core::primitive::u8),
                        #[codec(index = 157)]
                        Mortal157(::core::primitive::u8),
                        #[codec(index = 158)]
                        Mortal158(::core::primitive::u8),
                        #[codec(index = 159)]
                        Mortal159(::core::primitive::u8),
                        #[codec(index = 160)]
                        Mortal160(::core::primitive::u8),
                        #[codec(index = 161)]
                        Mortal161(::core::primitive::u8),
                        #[codec(index = 162)]
                        Mortal162(::core::primitive::u8),
                        #[codec(index = 163)]
                        Mortal163(::core::primitive::u8),
                        #[codec(index = 164)]
                        Mortal164(::core::primitive::u8),
                        #[codec(index = 165)]
                        Mortal165(::core::primitive::u8),
                        #[codec(index = 166)]
                        Mortal166(::core::primitive::u8),
                        #[codec(index = 167)]
                        Mortal167(::core::primitive::u8),
                        #[codec(index = 168)]
                        Mortal168(::core::primitive::u8),
                        #[codec(index = 169)]
                        Mortal169(::core::primitive::u8),
                        #[codec(index = 170)]
                        Mortal170(::core::primitive::u8),
                        #[codec(index = 171)]
                        Mortal171(::core::primitive::u8),
                        #[codec(index = 172)]
                        Mortal172(::core::primitive::u8),
                        #[codec(index = 173)]
                        Mortal173(::core::primitive::u8),
                        #[codec(index = 174)]
                        Mortal174(::core::primitive::u8),
                        #[codec(index = 175)]
                        Mortal175(::core::primitive::u8),
                        #[codec(index = 176)]
                        Mortal176(::core::primitive::u8),
                        #[codec(index = 177)]
                        Mortal177(::core::primitive::u8),
                        #[codec(index = 178)]
                        Mortal178(::core::primitive::u8),
                        #[codec(index = 179)]
                        Mortal179(::core::primitive::u8),
                        #[codec(index = 180)]
                        Mortal180(::core::primitive::u8),
                        #[codec(index = 181)]
                        Mortal181(::core::primitive::u8),
                        #[codec(index = 182)]
                        Mortal182(::core::primitive::u8),
                        #[codec(index = 183)]
                        Mortal183(::core::primitive::u8),
                        #[codec(index = 184)]
                        Mortal184(::core::primitive::u8),
                        #[codec(index = 185)]
                        Mortal185(::core::primitive::u8),
                        #[codec(index = 186)]
                        Mortal186(::core::primitive::u8),
                        #[codec(index = 187)]
                        Mortal187(::core::primitive::u8),
                        #[codec(index = 188)]
                        Mortal188(::core::primitive::u8),
                        #[codec(index = 189)]
                        Mortal189(::core::primitive::u8),
                        #[codec(index = 190)]
                        Mortal190(::core::primitive::u8),
                        #[codec(index = 191)]
                        Mortal191(::core::primitive::u8),
                        #[codec(index = 192)]
                        Mortal192(::core::primitive::u8),
                        #[codec(index = 193)]
                        Mortal193(::core::primitive::u8),
                        #[codec(index = 194)]
                        Mortal194(::core::primitive::u8),
                        #[codec(index = 195)]
                        Mortal195(::core::primitive::u8),
                        #[codec(index = 196)]
                        Mortal196(::core::primitive::u8),
                        #[codec(index = 197)]
                        Mortal197(::core::primitive::u8),
                        #[codec(index = 198)]
                        Mortal198(::core::primitive::u8),
                        #[codec(index = 199)]
                        Mortal199(::core::primitive::u8),
                        #[codec(index = 200)]
                        Mortal200(::core::primitive::u8),
                        #[codec(index = 201)]
                        Mortal201(::core::primitive::u8),
                        #[codec(index = 202)]
                        Mortal202(::core::primitive::u8),
                        #[codec(index = 203)]
                        Mortal203(::core::primitive::u8),
                        #[codec(index = 204)]
                        Mortal204(::core::primitive::u8),
                        #[codec(index = 205)]
                        Mortal205(::core::primitive::u8),
                        #[codec(index = 206)]
                        Mortal206(::core::primitive::u8),
                        #[codec(index = 207)]
                        Mortal207(::core::primitive::u8),
                        #[codec(index = 208)]
                        Mortal208(::core::primitive::u8),
                        #[codec(index = 209)]
                        Mortal209(::core::primitive::u8),
                        #[codec(index = 210)]
                        Mortal210(::core::primitive::u8),
                        #[codec(index = 211)]
                        Mortal211(::core::primitive::u8),
                        #[codec(index = 212)]
                        Mortal212(::core::primitive::u8),
                        #[codec(index = 213)]
                        Mortal213(::core::primitive::u8),
                        #[codec(index = 214)]
                        Mortal214(::core::primitive::u8),
                        #[codec(index = 215)]
                        Mortal215(::core::primitive::u8),
                        #[codec(index = 216)]
                        Mortal216(::core::primitive::u8),
                        #[codec(index = 217)]
                        Mortal217(::core::primitive::u8),
                        #[codec(index = 218)]
                        Mortal218(::core::primitive::u8),
                        #[codec(index = 219)]
                        Mortal219(::core::primitive::u8),
                        #[codec(index = 220)]
                        Mortal220(::core::primitive::u8),
                        #[codec(index = 221)]
                        Mortal221(::core::primitive::u8),
                        #[codec(index = 222)]
                        Mortal222(::core::primitive::u8),
                        #[codec(index = 223)]
                        Mortal223(::core::primitive::u8),
                        #[codec(index = 224)]
                        Mortal224(::core::primitive::u8),
                        #[codec(index = 225)]
                        Mortal225(::core::primitive::u8),
                        #[codec(index = 226)]
                        Mortal226(::core::primitive::u8),
                        #[codec(index = 227)]
                        Mortal227(::core::primitive::u8),
                        #[codec(index = 228)]
                        Mortal228(::core::primitive::u8),
                        #[codec(index = 229)]
                        Mortal229(::core::primitive::u8),
                        #[codec(index = 230)]
                        Mortal230(::core::primitive::u8),
                        #[codec(index = 231)]
                        Mortal231(::core::primitive::u8),
                        #[codec(index = 232)]
                        Mortal232(::core::primitive::u8),
                        #[codec(index = 233)]
                        Mortal233(::core::primitive::u8),
                        #[codec(index = 234)]
                        Mortal234(::core::primitive::u8),
                        #[codec(index = 235)]
                        Mortal235(::core::primitive::u8),
                        #[codec(index = 236)]
                        Mortal236(::core::primitive::u8),
                        #[codec(index = 237)]
                        Mortal237(::core::primitive::u8),
                        #[codec(index = 238)]
                        Mortal238(::core::primitive::u8),
                        #[codec(index = 239)]
                        Mortal239(::core::primitive::u8),
                        #[codec(index = 240)]
                        Mortal240(::core::primitive::u8),
                        #[codec(index = 241)]
                        Mortal241(::core::primitive::u8),
                        #[codec(index = 242)]
                        Mortal242(::core::primitive::u8),
                        #[codec(index = 243)]
                        Mortal243(::core::primitive::u8),
                        #[codec(index = 244)]
                        Mortal244(::core::primitive::u8),
                        #[codec(index = 245)]
                        Mortal245(::core::primitive::u8),
                        #[codec(index = 246)]
                        Mortal246(::core::primitive::u8),
                        #[codec(index = 247)]
                        Mortal247(::core::primitive::u8),
                        #[codec(index = 248)]
                        Mortal248(::core::primitive::u8),
                        #[codec(index = 249)]
                        Mortal249(::core::primitive::u8),
                        #[codec(index = 250)]
                        Mortal250(::core::primitive::u8),
                        #[codec(index = 251)]
                        Mortal251(::core::primitive::u8),
                        #[codec(index = 252)]
                        Mortal252(::core::primitive::u8),
                        #[codec(index = 253)]
                        Mortal253(::core::primitive::u8),
                        #[codec(index = 254)]
                        Mortal254(::core::primitive::u8),
                        #[codec(index = 255)]
                        Mortal255(::core::primitive::u8),
                    }
                }
                pub mod unchecked_extrinsic {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct UncheckedExtrinsic<_0, _1, _2, _3>(
                        pub ::std::vec::Vec<::core::primitive::u8>,
                        #[codec(skip)] pub ::core::marker::PhantomData<(_0, _1, _2, _3)>,
                    );
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum DispatchError {
                #[codec(index = 0)]
                Other,
                #[codec(index = 1)]
                CannotLookup,
                #[codec(index = 2)]
                BadOrigin,
                #[codec(index = 3)]
                Module(runtime_types::sp_runtime::ModuleError),
                #[codec(index = 4)]
                ConsumerRemaining,
                #[codec(index = 5)]
                NoProviders,
                #[codec(index = 6)]
                TooManyConsumers,
                #[codec(index = 7)]
                Token(runtime_types::sp_runtime::TokenError),
                #[codec(index = 8)]
                Arithmetic(runtime_types::sp_arithmetic::ArithmeticError),
                #[codec(index = 9)]
                Transactional(runtime_types::sp_runtime::TransactionalError),
                #[codec(index = 10)]
                Exhausted,
                #[codec(index = 11)]
                Corruption,
                #[codec(index = 12)]
                Unavailable,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct ModuleError {
                pub index: ::core::primitive::u8,
                pub error: [::core::primitive::u8; 4usize],
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum MultiSignature {
                #[codec(index = 0)]
                Ed25519(runtime_types::sp_core::ed25519::Signature),
                #[codec(index = 1)]
                Sr25519(runtime_types::sp_core::sr25519::Signature),
                #[codec(index = 2)]
                Ecdsa(runtime_types::sp_core::ecdsa::Signature),
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum TokenError {
                #[codec(index = 0)]
                NoFunds,
                #[codec(index = 1)]
                WouldDie,
                #[codec(index = 2)]
                BelowMinimum,
                #[codec(index = 3)]
                CannotCreate,
                #[codec(index = 4)]
                UnknownAsset,
                #[codec(index = 5)]
                Frozen,
                #[codec(index = 6)]
                Unsupported,
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum TransactionalError {
                #[codec(index = 0)]
                LimitReached,
                #[codec(index = 1)]
                NoLayer,
            }
        }
        pub mod sp_trie {
            use super::runtime_types;
            pub mod storage_proof {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct StorageProof {
                    pub trie_nodes: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
                }
            }
        }
        pub mod sp_version {
            use super::runtime_types;
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct RuntimeVersion {
                pub spec_name: ::std::string::String,
                pub impl_name: ::std::string::String,
                pub authoring_version: ::core::primitive::u32,
                pub spec_version: ::core::primitive::u32,
                pub impl_version: ::core::primitive::u32,
                pub apis:
                    ::std::vec::Vec<([::core::primitive::u8; 8usize], ::core::primitive::u32)>,
                pub transaction_version: ::core::primitive::u32,
                pub state_version: ::core::primitive::u8,
            }
        }
        pub mod sp_weights {
            use super::runtime_types;
            pub mod weight_v2 {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Weight {
                    #[codec(compact)]
                    pub ref_time: ::core::primitive::u64,
                    #[codec(compact)]
                    pub proof_size: ::core::primitive::u64,
                }
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub struct RuntimeDbWeight {
                pub read: ::core::primitive::u64,
                pub write: ::core::primitive::u64,
            }
        }
        pub mod xcm {
            use super::runtime_types;
            pub mod double_encoded {
                use super::runtime_types;
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct DoubleEncoded {
                    pub encoded: ::std::vec::Vec<::core::primitive::u8>,
                }
            }
            pub mod v2 {
                use super::runtime_types;
                pub mod junction {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Junction {
                        #[codec(index = 0)]
                        Parachain(#[codec(compact)] ::core::primitive::u32),
                        #[codec(index = 1)]
                        AccountId32 {
                            network: runtime_types::xcm::v2::NetworkId,
                            id: [::core::primitive::u8; 32usize],
                        },
                        #[codec(index = 2)]
                        AccountIndex64 {
                            network: runtime_types::xcm::v2::NetworkId,
                            #[codec(compact)]
                            index: ::core::primitive::u64,
                        },
                        #[codec(index = 3)]
                        AccountKey20 {
                            network: runtime_types::xcm::v2::NetworkId,
                            key: [::core::primitive::u8; 20usize],
                        },
                        #[codec(index = 4)]
                        PalletInstance(::core::primitive::u8),
                        #[codec(index = 5)]
                        GeneralIndex(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 6)]
                        GeneralKey(
                            runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
                                ::core::primitive::u8,
                            >,
                        ),
                        #[codec(index = 7)]
                        OnlyChild,
                        #[codec(index = 8)]
                        Plurality {
                            id: runtime_types::xcm::v2::BodyId,
                            part: runtime_types::xcm::v2::BodyPart,
                        },
                    }
                }
                pub mod multiasset {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum AssetId {
                        #[codec(index = 0)]
                        Concrete(runtime_types::xcm::v2::multilocation::MultiLocation),
                        #[codec(index = 1)]
                        Abstract(::std::vec::Vec<::core::primitive::u8>),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum AssetInstance {
                        #[codec(index = 0)]
                        Undefined,
                        #[codec(index = 1)]
                        Index(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 2)]
                        Array4([::core::primitive::u8; 4usize]),
                        #[codec(index = 3)]
                        Array8([::core::primitive::u8; 8usize]),
                        #[codec(index = 4)]
                        Array16([::core::primitive::u8; 16usize]),
                        #[codec(index = 5)]
                        Array32([::core::primitive::u8; 32usize]),
                        #[codec(index = 6)]
                        Blob(::std::vec::Vec<::core::primitive::u8>),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Fungibility {
                        #[codec(index = 0)]
                        Fungible(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 1)]
                        NonFungible(runtime_types::xcm::v2::multiasset::AssetInstance),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiAsset {
                        pub id: runtime_types::xcm::v2::multiasset::AssetId,
                        pub fun: runtime_types::xcm::v2::multiasset::Fungibility,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum MultiAssetFilter {
                        #[codec(index = 0)]
                        Definite(runtime_types::xcm::v2::multiasset::MultiAssets),
                        #[codec(index = 1)]
                        Wild(runtime_types::xcm::v2::multiasset::WildMultiAsset),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiAssets(
                        pub ::std::vec::Vec<runtime_types::xcm::v2::multiasset::MultiAsset>,
                    );
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum WildFungibility {
                        #[codec(index = 0)]
                        Fungible,
                        #[codec(index = 1)]
                        NonFungible,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum WildMultiAsset {
                        #[codec(index = 0)]
                        All,
                        #[codec(index = 1)]
                        AllOf {
                            id: runtime_types::xcm::v2::multiasset::AssetId,
                            fun: runtime_types::xcm::v2::multiasset::WildFungibility,
                        },
                    }
                }
                pub mod multilocation {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Junctions {
                        #[codec(index = 0)]
                        Here,
                        #[codec(index = 1)]
                        X1(runtime_types::xcm::v2::junction::Junction),
                        #[codec(index = 2)]
                        X2(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 3)]
                        X3(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 4)]
                        X4(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 5)]
                        X5(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 6)]
                        X6(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 7)]
                        X7(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                        #[codec(index = 8)]
                        X8(
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                            runtime_types::xcm::v2::junction::Junction,
                        ),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiLocation {
                        pub parents: ::core::primitive::u8,
                        pub interior: runtime_types::xcm::v2::multilocation::Junctions,
                    }
                }
                pub mod traits {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Error {
                        #[codec(index = 0)]
                        Overflow,
                        #[codec(index = 1)]
                        Unimplemented,
                        #[codec(index = 2)]
                        UntrustedReserveLocation,
                        #[codec(index = 3)]
                        UntrustedTeleportLocation,
                        #[codec(index = 4)]
                        MultiLocationFull,
                        #[codec(index = 5)]
                        MultiLocationNotInvertible,
                        #[codec(index = 6)]
                        BadOrigin,
                        #[codec(index = 7)]
                        InvalidLocation,
                        #[codec(index = 8)]
                        AssetNotFound,
                        #[codec(index = 9)]
                        FailedToTransactAsset,
                        #[codec(index = 10)]
                        NotWithdrawable,
                        #[codec(index = 11)]
                        LocationCannotHold,
                        #[codec(index = 12)]
                        ExceedsMaxMessageSize,
                        #[codec(index = 13)]
                        DestinationUnsupported,
                        #[codec(index = 14)]
                        Transport,
                        #[codec(index = 15)]
                        Unroutable,
                        #[codec(index = 16)]
                        UnknownClaim,
                        #[codec(index = 17)]
                        FailedToDecode,
                        #[codec(index = 18)]
                        MaxWeightInvalid,
                        #[codec(index = 19)]
                        NotHoldingFees,
                        #[codec(index = 20)]
                        TooExpensive,
                        #[codec(index = 21)]
                        Trap(::core::primitive::u64),
                        #[codec(index = 22)]
                        UnhandledXcmVersion,
                        #[codec(index = 23)]
                        WeightLimitReached(::core::primitive::u64),
                        #[codec(index = 24)]
                        Barrier,
                        #[codec(index = 25)]
                        WeightNotComputable,
                    }
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum BodyId {
                    #[codec(index = 0)]
                    Unit,
                    #[codec(index = 1)]
                    Named(
                        runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
                            ::core::primitive::u8,
                        >,
                    ),
                    #[codec(index = 2)]
                    Index(#[codec(compact)] ::core::primitive::u32),
                    #[codec(index = 3)]
                    Executive,
                    #[codec(index = 4)]
                    Technical,
                    #[codec(index = 5)]
                    Legislative,
                    #[codec(index = 6)]
                    Judicial,
                    #[codec(index = 7)]
                    Defense,
                    #[codec(index = 8)]
                    Administration,
                    #[codec(index = 9)]
                    Treasury,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum BodyPart {
                    #[codec(index = 0)]
                    Voice,
                    #[codec(index = 1)]
                    Members {
                        #[codec(compact)]
                        count: ::core::primitive::u32,
                    },
                    #[codec(index = 2)]
                    Fraction {
                        #[codec(compact)]
                        nom: ::core::primitive::u32,
                        #[codec(compact)]
                        denom: ::core::primitive::u32,
                    },
                    #[codec(index = 3)]
                    AtLeastProportion {
                        #[codec(compact)]
                        nom: ::core::primitive::u32,
                        #[codec(compact)]
                        denom: ::core::primitive::u32,
                    },
                    #[codec(index = 4)]
                    MoreThanProportion {
                        #[codec(compact)]
                        nom: ::core::primitive::u32,
                        #[codec(compact)]
                        denom: ::core::primitive::u32,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Instruction {
                    #[codec(index = 0)]
                    WithdrawAsset(runtime_types::xcm::v2::multiasset::MultiAssets),
                    #[codec(index = 1)]
                    ReserveAssetDeposited(runtime_types::xcm::v2::multiasset::MultiAssets),
                    #[codec(index = 2)]
                    ReceiveTeleportedAsset(runtime_types::xcm::v2::multiasset::MultiAssets),
                    #[codec(index = 3)]
                    QueryResponse {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        response: runtime_types::xcm::v2::Response,
                        #[codec(compact)]
                        max_weight: ::core::primitive::u64,
                    },
                    #[codec(index = 4)]
                    TransferAsset {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssets,
                        beneficiary: runtime_types::xcm::v2::multilocation::MultiLocation,
                    },
                    #[codec(index = 5)]
                    TransferReserveAsset {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssets,
                        dest: runtime_types::xcm::v2::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v2::Xcm,
                    },
                    #[codec(index = 6)]
                    Transact {
                        origin_type: runtime_types::xcm::v2::OriginKind,
                        #[codec(compact)]
                        require_weight_at_most: ::core::primitive::u64,
                        call: runtime_types::xcm::double_encoded::DoubleEncoded,
                    },
                    #[codec(index = 7)]
                    HrmpNewChannelOpenRequest {
                        #[codec(compact)]
                        sender: ::core::primitive::u32,
                        #[codec(compact)]
                        max_message_size: ::core::primitive::u32,
                        #[codec(compact)]
                        max_capacity: ::core::primitive::u32,
                    },
                    #[codec(index = 8)]
                    HrmpChannelAccepted {
                        #[codec(compact)]
                        recipient: ::core::primitive::u32,
                    },
                    #[codec(index = 9)]
                    HrmpChannelClosing {
                        #[codec(compact)]
                        initiator: ::core::primitive::u32,
                        #[codec(compact)]
                        sender: ::core::primitive::u32,
                        #[codec(compact)]
                        recipient: ::core::primitive::u32,
                    },
                    #[codec(index = 10)]
                    ClearOrigin,
                    #[codec(index = 11)]
                    DescendOrigin(runtime_types::xcm::v2::multilocation::Junctions),
                    #[codec(index = 12)]
                    ReportError {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        dest: runtime_types::xcm::v2::multilocation::MultiLocation,
                        #[codec(compact)]
                        max_response_weight: ::core::primitive::u64,
                    },
                    #[codec(index = 13)]
                    DepositAsset {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        #[codec(compact)]
                        max_assets: ::core::primitive::u32,
                        beneficiary: runtime_types::xcm::v2::multilocation::MultiLocation,
                    },
                    #[codec(index = 14)]
                    DepositReserveAsset {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        #[codec(compact)]
                        max_assets: ::core::primitive::u32,
                        dest: runtime_types::xcm::v2::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v2::Xcm,
                    },
                    #[codec(index = 15)]
                    ExchangeAsset {
                        give: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        receive: runtime_types::xcm::v2::multiasset::MultiAssets,
                    },
                    #[codec(index = 16)]
                    InitiateReserveWithdraw {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        reserve: runtime_types::xcm::v2::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v2::Xcm,
                    },
                    #[codec(index = 17)]
                    InitiateTeleport {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        dest: runtime_types::xcm::v2::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v2::Xcm,
                    },
                    #[codec(index = 18)]
                    QueryHolding {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        dest: runtime_types::xcm::v2::multilocation::MultiLocation,
                        assets: runtime_types::xcm::v2::multiasset::MultiAssetFilter,
                        #[codec(compact)]
                        max_response_weight: ::core::primitive::u64,
                    },
                    #[codec(index = 19)]
                    BuyExecution {
                        fees: runtime_types::xcm::v2::multiasset::MultiAsset,
                        weight_limit: runtime_types::xcm::v2::WeightLimit,
                    },
                    #[codec(index = 20)]
                    RefundSurplus,
                    #[codec(index = 21)]
                    SetErrorHandler(runtime_types::xcm::v2::Xcm),
                    #[codec(index = 22)]
                    SetAppendix(runtime_types::xcm::v2::Xcm),
                    #[codec(index = 23)]
                    ClearError,
                    #[codec(index = 24)]
                    ClaimAsset {
                        assets: runtime_types::xcm::v2::multiasset::MultiAssets,
                        ticket: runtime_types::xcm::v2::multilocation::MultiLocation,
                    },
                    #[codec(index = 25)]
                    Trap(#[codec(compact)] ::core::primitive::u64),
                    #[codec(index = 26)]
                    SubscribeVersion {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        #[codec(compact)]
                        max_response_weight: ::core::primitive::u64,
                    },
                    #[codec(index = 27)]
                    UnsubscribeVersion,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum NetworkId {
                    #[codec(index = 0)]
                    Any,
                    #[codec(index = 1)]
                    Named(
                        runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
                            ::core::primitive::u8,
                        >,
                    ),
                    #[codec(index = 2)]
                    Polkadot,
                    #[codec(index = 3)]
                    Kusama,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum OriginKind {
                    #[codec(index = 0)]
                    Native,
                    #[codec(index = 1)]
                    SovereignAccount,
                    #[codec(index = 2)]
                    Superuser,
                    #[codec(index = 3)]
                    Xcm,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Response {
                    #[codec(index = 0)]
                    Null,
                    #[codec(index = 1)]
                    Assets(runtime_types::xcm::v2::multiasset::MultiAssets),
                    #[codec(index = 2)]
                    ExecutionResult(
                        ::core::option::Option<(
                            ::core::primitive::u32,
                            runtime_types::xcm::v2::traits::Error,
                        )>,
                    ),
                    #[codec(index = 3)]
                    Version(::core::primitive::u32),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum WeightLimit {
                    #[codec(index = 0)]
                    Unlimited,
                    #[codec(index = 1)]
                    Limited(#[codec(compact)] ::core::primitive::u64),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Xcm(pub ::std::vec::Vec<runtime_types::xcm::v2::Instruction>);
            }
            pub mod v3 {
                use super::runtime_types;
                pub mod junction {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum BodyId {
                        #[codec(index = 0)]
                        Unit,
                        #[codec(index = 1)]
                        Moniker([::core::primitive::u8; 4usize]),
                        #[codec(index = 2)]
                        Index(#[codec(compact)] ::core::primitive::u32),
                        #[codec(index = 3)]
                        Executive,
                        #[codec(index = 4)]
                        Technical,
                        #[codec(index = 5)]
                        Legislative,
                        #[codec(index = 6)]
                        Judicial,
                        #[codec(index = 7)]
                        Defense,
                        #[codec(index = 8)]
                        Administration,
                        #[codec(index = 9)]
                        Treasury,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum BodyPart {
                        #[codec(index = 0)]
                        Voice,
                        #[codec(index = 1)]
                        Members {
                            #[codec(compact)]
                            count: ::core::primitive::u32,
                        },
                        #[codec(index = 2)]
                        Fraction {
                            #[codec(compact)]
                            nom: ::core::primitive::u32,
                            #[codec(compact)]
                            denom: ::core::primitive::u32,
                        },
                        #[codec(index = 3)]
                        AtLeastProportion {
                            #[codec(compact)]
                            nom: ::core::primitive::u32,
                            #[codec(compact)]
                            denom: ::core::primitive::u32,
                        },
                        #[codec(index = 4)]
                        MoreThanProportion {
                            #[codec(compact)]
                            nom: ::core::primitive::u32,
                            #[codec(compact)]
                            denom: ::core::primitive::u32,
                        },
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Junction {
                        #[codec(index = 0)]
                        Parachain(#[codec(compact)] ::core::primitive::u32),
                        #[codec(index = 1)]
                        AccountId32 {
                            network:
                                ::core::option::Option<runtime_types::xcm::v3::junction::NetworkId>,
                            id: [::core::primitive::u8; 32usize],
                        },
                        #[codec(index = 2)]
                        AccountIndex64 {
                            network:
                                ::core::option::Option<runtime_types::xcm::v3::junction::NetworkId>,
                            #[codec(compact)]
                            index: ::core::primitive::u64,
                        },
                        #[codec(index = 3)]
                        AccountKey20 {
                            network:
                                ::core::option::Option<runtime_types::xcm::v3::junction::NetworkId>,
                            key: [::core::primitive::u8; 20usize],
                        },
                        #[codec(index = 4)]
                        PalletInstance(::core::primitive::u8),
                        #[codec(index = 5)]
                        GeneralIndex(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 6)]
                        GeneralKey {
                            length: ::core::primitive::u8,
                            data: [::core::primitive::u8; 32usize],
                        },
                        #[codec(index = 7)]
                        OnlyChild,
                        #[codec(index = 8)]
                        Plurality {
                            id: runtime_types::xcm::v3::junction::BodyId,
                            part: runtime_types::xcm::v3::junction::BodyPart,
                        },
                        #[codec(index = 9)]
                        GlobalConsensus(runtime_types::xcm::v3::junction::NetworkId),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum NetworkId {
                        #[codec(index = 0)]
                        ByGenesis([::core::primitive::u8; 32usize]),
                        #[codec(index = 1)]
                        ByFork {
                            block_number: ::core::primitive::u64,
                            block_hash: [::core::primitive::u8; 32usize],
                        },
                        #[codec(index = 2)]
                        Polkadot,
                        #[codec(index = 3)]
                        Kusama,
                        #[codec(index = 4)]
                        Westend,
                        #[codec(index = 5)]
                        Rococo,
                        #[codec(index = 6)]
                        Wococo,
                        #[codec(index = 7)]
                        Ethereum {
                            #[codec(compact)]
                            chain_id: ::core::primitive::u64,
                        },
                        #[codec(index = 8)]
                        BitcoinCore,
                        #[codec(index = 9)]
                        BitcoinCash,
                    }
                }
                pub mod junctions {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Junctions {
                        #[codec(index = 0)]
                        Here,
                        #[codec(index = 1)]
                        X1(runtime_types::xcm::v3::junction::Junction),
                        #[codec(index = 2)]
                        X2(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 3)]
                        X3(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 4)]
                        X4(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 5)]
                        X5(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 6)]
                        X6(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 7)]
                        X7(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                        #[codec(index = 8)]
                        X8(
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                            runtime_types::xcm::v3::junction::Junction,
                        ),
                    }
                }
                pub mod multiasset {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum AssetId {
                        #[codec(index = 0)]
                        Concrete(runtime_types::xcm::v3::multilocation::MultiLocation),
                        #[codec(index = 1)]
                        Abstract([::core::primitive::u8; 32usize]),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum AssetInstance {
                        #[codec(index = 0)]
                        Undefined,
                        #[codec(index = 1)]
                        Index(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 2)]
                        Array4([::core::primitive::u8; 4usize]),
                        #[codec(index = 3)]
                        Array8([::core::primitive::u8; 8usize]),
                        #[codec(index = 4)]
                        Array16([::core::primitive::u8; 16usize]),
                        #[codec(index = 5)]
                        Array32([::core::primitive::u8; 32usize]),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Fungibility {
                        #[codec(index = 0)]
                        Fungible(#[codec(compact)] ::core::primitive::u128),
                        #[codec(index = 1)]
                        NonFungible(runtime_types::xcm::v3::multiasset::AssetInstance),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiAsset {
                        pub id: runtime_types::xcm::v3::multiasset::AssetId,
                        pub fun: runtime_types::xcm::v3::multiasset::Fungibility,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum MultiAssetFilter {
                        #[codec(index = 0)]
                        Definite(runtime_types::xcm::v3::multiasset::MultiAssets),
                        #[codec(index = 1)]
                        Wild(runtime_types::xcm::v3::multiasset::WildMultiAsset),
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiAssets(
                        pub ::std::vec::Vec<runtime_types::xcm::v3::multiasset::MultiAsset>,
                    );
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum WildFungibility {
                        #[codec(index = 0)]
                        Fungible,
                        #[codec(index = 1)]
                        NonFungible,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum WildMultiAsset {
                        #[codec(index = 0)]
                        All,
                        #[codec(index = 1)]
                        AllOf {
                            id: runtime_types::xcm::v3::multiasset::AssetId,
                            fun: runtime_types::xcm::v3::multiasset::WildFungibility,
                        },
                        #[codec(index = 2)]
                        AllCounted(#[codec(compact)] ::core::primitive::u32),
                        #[codec(index = 3)]
                        AllOfCounted {
                            id: runtime_types::xcm::v3::multiasset::AssetId,
                            fun: runtime_types::xcm::v3::multiasset::WildFungibility,
                            #[codec(compact)]
                            count: ::core::primitive::u32,
                        },
                    }
                }
                pub mod multilocation {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub struct MultiLocation {
                        pub parents: ::core::primitive::u8,
                        pub interior: runtime_types::xcm::v3::junctions::Junctions,
                    }
                }
                pub mod traits {
                    use super::runtime_types;
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Error {
                        #[codec(index = 0)]
                        Overflow,
                        #[codec(index = 1)]
                        Unimplemented,
                        #[codec(index = 2)]
                        UntrustedReserveLocation,
                        #[codec(index = 3)]
                        UntrustedTeleportLocation,
                        #[codec(index = 4)]
                        LocationFull,
                        #[codec(index = 5)]
                        LocationNotInvertible,
                        #[codec(index = 6)]
                        BadOrigin,
                        #[codec(index = 7)]
                        InvalidLocation,
                        #[codec(index = 8)]
                        AssetNotFound,
                        #[codec(index = 9)]
                        FailedToTransactAsset,
                        #[codec(index = 10)]
                        NotWithdrawable,
                        #[codec(index = 11)]
                        LocationCannotHold,
                        #[codec(index = 12)]
                        ExceedsMaxMessageSize,
                        #[codec(index = 13)]
                        DestinationUnsupported,
                        #[codec(index = 14)]
                        Transport,
                        #[codec(index = 15)]
                        Unroutable,
                        #[codec(index = 16)]
                        UnknownClaim,
                        #[codec(index = 17)]
                        FailedToDecode,
                        #[codec(index = 18)]
                        MaxWeightInvalid,
                        #[codec(index = 19)]
                        NotHoldingFees,
                        #[codec(index = 20)]
                        TooExpensive,
                        #[codec(index = 21)]
                        Trap(::core::primitive::u64),
                        #[codec(index = 22)]
                        ExpectationFalse,
                        #[codec(index = 23)]
                        PalletNotFound,
                        #[codec(index = 24)]
                        NameMismatch,
                        #[codec(index = 25)]
                        VersionIncompatible,
                        #[codec(index = 26)]
                        HoldingWouldOverflow,
                        #[codec(index = 27)]
                        ExportError,
                        #[codec(index = 28)]
                        ReanchorFailed,
                        #[codec(index = 29)]
                        NoDeal,
                        #[codec(index = 30)]
                        FeesNotMet,
                        #[codec(index = 31)]
                        LockError,
                        #[codec(index = 32)]
                        NoPermission,
                        #[codec(index = 33)]
                        Unanchored,
                        #[codec(index = 34)]
                        NotDepositable,
                        #[codec(index = 35)]
                        UnhandledXcmVersion,
                        #[codec(index = 36)]
                        WeightLimitReached(runtime_types::sp_weights::weight_v2::Weight),
                        #[codec(index = 37)]
                        Barrier,
                        #[codec(index = 38)]
                        WeightNotComputable,
                        #[codec(index = 39)]
                        ExceedsStackLimit,
                    }
                    #[derive(
                        ::subxt::ext::codec::Decode,
                        ::subxt::ext::codec::Encode,
                        ::subxt::ext::scale_decode::DecodeAsType,
                        ::subxt::ext::scale_encode::EncodeAsType,
                        Debug,
                    )]
                    #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                    #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                    pub enum Outcome {
                        #[codec(index = 0)]
                        Complete(runtime_types::sp_weights::weight_v2::Weight),
                        #[codec(index = 1)]
                        Incomplete(
                            runtime_types::sp_weights::weight_v2::Weight,
                            runtime_types::xcm::v3::traits::Error,
                        ),
                        #[codec(index = 2)]
                        Error(runtime_types::xcm::v3::traits::Error),
                    }
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Instruction {
                    #[codec(index = 0)]
                    WithdrawAsset(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 1)]
                    ReserveAssetDeposited(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 2)]
                    ReceiveTeleportedAsset(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 3)]
                    QueryResponse {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        response: runtime_types::xcm::v3::Response,
                        max_weight: runtime_types::sp_weights::weight_v2::Weight,
                        querier: ::core::option::Option<
                            runtime_types::xcm::v3::multilocation::MultiLocation,
                        >,
                    },
                    #[codec(index = 4)]
                    TransferAsset {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssets,
                        beneficiary: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 5)]
                    TransferReserveAsset {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssets,
                        dest: runtime_types::xcm::v3::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v3::Xcm,
                    },
                    #[codec(index = 6)]
                    Transact {
                        origin_kind: runtime_types::xcm::v2::OriginKind,
                        require_weight_at_most: runtime_types::sp_weights::weight_v2::Weight,
                        call: runtime_types::xcm::double_encoded::DoubleEncoded,
                    },
                    #[codec(index = 7)]
                    HrmpNewChannelOpenRequest {
                        #[codec(compact)]
                        sender: ::core::primitive::u32,
                        #[codec(compact)]
                        max_message_size: ::core::primitive::u32,
                        #[codec(compact)]
                        max_capacity: ::core::primitive::u32,
                    },
                    #[codec(index = 8)]
                    HrmpChannelAccepted {
                        #[codec(compact)]
                        recipient: ::core::primitive::u32,
                    },
                    #[codec(index = 9)]
                    HrmpChannelClosing {
                        #[codec(compact)]
                        initiator: ::core::primitive::u32,
                        #[codec(compact)]
                        sender: ::core::primitive::u32,
                        #[codec(compact)]
                        recipient: ::core::primitive::u32,
                    },
                    #[codec(index = 10)]
                    ClearOrigin,
                    #[codec(index = 11)]
                    DescendOrigin(runtime_types::xcm::v3::junctions::Junctions),
                    #[codec(index = 12)]
                    ReportError(runtime_types::xcm::v3::QueryResponseInfo),
                    #[codec(index = 13)]
                    DepositAsset {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                        beneficiary: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 14)]
                    DepositReserveAsset {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                        dest: runtime_types::xcm::v3::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v3::Xcm,
                    },
                    #[codec(index = 15)]
                    ExchangeAsset {
                        give: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                        want: runtime_types::xcm::v3::multiasset::MultiAssets,
                        maximal: ::core::primitive::bool,
                    },
                    #[codec(index = 16)]
                    InitiateReserveWithdraw {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                        reserve: runtime_types::xcm::v3::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v3::Xcm,
                    },
                    #[codec(index = 17)]
                    InitiateTeleport {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                        dest: runtime_types::xcm::v3::multilocation::MultiLocation,
                        xcm: runtime_types::xcm::v3::Xcm,
                    },
                    #[codec(index = 18)]
                    ReportHolding {
                        response_info: runtime_types::xcm::v3::QueryResponseInfo,
                        assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
                    },
                    #[codec(index = 19)]
                    BuyExecution {
                        fees: runtime_types::xcm::v3::multiasset::MultiAsset,
                        weight_limit: runtime_types::xcm::v3::WeightLimit,
                    },
                    #[codec(index = 20)]
                    RefundSurplus,
                    #[codec(index = 21)]
                    SetErrorHandler(runtime_types::xcm::v3::Xcm),
                    #[codec(index = 22)]
                    SetAppendix(runtime_types::xcm::v3::Xcm),
                    #[codec(index = 23)]
                    ClearError,
                    #[codec(index = 24)]
                    ClaimAsset {
                        assets: runtime_types::xcm::v3::multiasset::MultiAssets,
                        ticket: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 25)]
                    Trap(#[codec(compact)] ::core::primitive::u64),
                    #[codec(index = 26)]
                    SubscribeVersion {
                        #[codec(compact)]
                        query_id: ::core::primitive::u64,
                        max_response_weight: runtime_types::sp_weights::weight_v2::Weight,
                    },
                    #[codec(index = 27)]
                    UnsubscribeVersion,
                    #[codec(index = 28)]
                    BurnAsset(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 29)]
                    ExpectAsset(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 30)]
                    ExpectOrigin(
                        ::core::option::Option<
                            runtime_types::xcm::v3::multilocation::MultiLocation,
                        >,
                    ),
                    #[codec(index = 31)]
                    ExpectError(
                        ::core::option::Option<(
                            ::core::primitive::u32,
                            runtime_types::xcm::v3::traits::Error,
                        )>,
                    ),
                    #[codec(index = 32)]
                    ExpectTransactStatus(runtime_types::xcm::v3::MaybeErrorCode),
                    #[codec(index = 33)]
                    QueryPallet {
                        module_name: ::std::vec::Vec<::core::primitive::u8>,
                        response_info: runtime_types::xcm::v3::QueryResponseInfo,
                    },
                    #[codec(index = 34)]
                    ExpectPallet {
                        #[codec(compact)]
                        index: ::core::primitive::u32,
                        name: ::std::vec::Vec<::core::primitive::u8>,
                        module_name: ::std::vec::Vec<::core::primitive::u8>,
                        #[codec(compact)]
                        crate_major: ::core::primitive::u32,
                        #[codec(compact)]
                        min_crate_minor: ::core::primitive::u32,
                    },
                    #[codec(index = 35)]
                    ReportTransactStatus(runtime_types::xcm::v3::QueryResponseInfo),
                    #[codec(index = 36)]
                    ClearTransactStatus,
                    #[codec(index = 37)]
                    UniversalOrigin(runtime_types::xcm::v3::junction::Junction),
                    #[codec(index = 38)]
                    ExportMessage {
                        network: runtime_types::xcm::v3::junction::NetworkId,
                        destination: runtime_types::xcm::v3::junctions::Junctions,
                        xcm: runtime_types::xcm::v3::Xcm,
                    },
                    #[codec(index = 39)]
                    LockAsset {
                        asset: runtime_types::xcm::v3::multiasset::MultiAsset,
                        unlocker: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 40)]
                    UnlockAsset {
                        asset: runtime_types::xcm::v3::multiasset::MultiAsset,
                        target: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 41)]
                    NoteUnlockable {
                        asset: runtime_types::xcm::v3::multiasset::MultiAsset,
                        owner: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 42)]
                    RequestUnlock {
                        asset: runtime_types::xcm::v3::multiasset::MultiAsset,
                        locker: runtime_types::xcm::v3::multilocation::MultiLocation,
                    },
                    #[codec(index = 43)]
                    SetFeesMode { jit_withdraw: ::core::primitive::bool },
                    #[codec(index = 44)]
                    SetTopic([::core::primitive::u8; 32usize]),
                    #[codec(index = 45)]
                    ClearTopic,
                    #[codec(index = 46)]
                    AliasOrigin(runtime_types::xcm::v3::multilocation::MultiLocation),
                    #[codec(index = 47)]
                    UnpaidExecution {
                        weight_limit: runtime_types::xcm::v3::WeightLimit,
                        check_origin: ::core::option::Option<
                            runtime_types::xcm::v3::multilocation::MultiLocation,
                        >,
                    },
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum MaybeErrorCode {
                    #[codec(index = 0)]
                    Success,
                    #[codec(index = 1)]
                    Error(
                        runtime_types::bounded_collections::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    ),
                    #[codec(index = 2)]
                    TruncatedError(
                        runtime_types::bounded_collections::bounded_vec::BoundedVec<
                            ::core::primitive::u8,
                        >,
                    ),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct PalletInfo {
                    #[codec(compact)]
                    pub index: ::core::primitive::u32,
                    pub name: runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        ::core::primitive::u8,
                    >,
                    pub module_name: runtime_types::bounded_collections::bounded_vec::BoundedVec<
                        ::core::primitive::u8,
                    >,
                    #[codec(compact)]
                    pub major: ::core::primitive::u32,
                    #[codec(compact)]
                    pub minor: ::core::primitive::u32,
                    #[codec(compact)]
                    pub patch: ::core::primitive::u32,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct QueryResponseInfo {
                    pub destination: runtime_types::xcm::v3::multilocation::MultiLocation,
                    #[codec(compact)]
                    pub query_id: ::core::primitive::u64,
                    pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum Response {
                    #[codec(index = 0)]
                    Null,
                    #[codec(index = 1)]
                    Assets(runtime_types::xcm::v3::multiasset::MultiAssets),
                    #[codec(index = 2)]
                    ExecutionResult(
                        ::core::option::Option<(
                            ::core::primitive::u32,
                            runtime_types::xcm::v3::traits::Error,
                        )>,
                    ),
                    #[codec(index = 3)]
                    Version(::core::primitive::u32),
                    #[codec(index = 4)]
                    PalletsInfo(
                        runtime_types::bounded_collections::bounded_vec::BoundedVec<
                            runtime_types::xcm::v3::PalletInfo,
                        >,
                    ),
                    #[codec(index = 5)]
                    DispatchResult(runtime_types::xcm::v3::MaybeErrorCode),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub enum WeightLimit {
                    #[codec(index = 0)]
                    Unlimited,
                    #[codec(index = 1)]
                    Limited(runtime_types::sp_weights::weight_v2::Weight),
                }
                #[derive(
                    ::subxt::ext::codec::Decode,
                    ::subxt::ext::codec::Encode,
                    ::subxt::ext::scale_decode::DecodeAsType,
                    ::subxt::ext::scale_encode::EncodeAsType,
                    Debug,
                )]
                #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
                #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
                pub struct Xcm(pub ::std::vec::Vec<runtime_types::xcm::v3::Instruction>);
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum VersionedMultiAssets {
                #[codec(index = 1)]
                V2(runtime_types::xcm::v2::multiasset::MultiAssets),
                #[codec(index = 3)]
                V3(runtime_types::xcm::v3::multiasset::MultiAssets),
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum VersionedMultiLocation {
                #[codec(index = 1)]
                V2(runtime_types::xcm::v2::multilocation::MultiLocation),
                #[codec(index = 3)]
                V3(runtime_types::xcm::v3::multilocation::MultiLocation),
            }
            #[derive(
                ::subxt::ext::codec::Decode,
                ::subxt::ext::codec::Encode,
                ::subxt::ext::scale_decode::DecodeAsType,
                ::subxt::ext::scale_encode::EncodeAsType,
                Debug,
            )]
            #[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
            #[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
            pub enum VersionedXcm {
                #[codec(index = 2)]
                V2(runtime_types::xcm::v2::Xcm),
                #[codec(index = 3)]
                V3(runtime_types::xcm::v3::Xcm),
            }
        }
    }
}
