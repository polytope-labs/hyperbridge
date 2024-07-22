#[allow(dead_code, unused_imports, non_camel_case_types)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
pub mod api {
	#[allow(unused_imports)]
	mod root_mod {
		pub use super::*;
	}
	pub static PALLETS: [&str; 28usize] = [
		"System",
		"Timestamp",
		"ParachainSystem",
		"ParachainInfo",
		"Utility",
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
		"Mmr",
		"Ismp",
		"MessageQueue",
		"IsmpParachain",
		"IsmpSyncCommittee",
		"IsmpDemo",
		"Relayer",
		"HostExecutive",
		"CallDecompressor",
		"Gateway",
		"Assets",
		"TokenGovernor",
	];
	pub static RUNTIME_APIS: [&str; 17usize] = [
		"AuraApi",
		"AuraUnincludedSegmentApi",
		"Core",
		"Metadata",
		"BlockBuilder",
		"TaggedTransactionQueue",
		"OffchainWorkerApi",
		"SessionKeys",
		"AccountNonceApi",
		"TransactionPaymentApi",
		"TransactionPaymentCallApi",
		"MmrRuntimeApi",
		"IsmpRuntimeApi",
		"IsmpParachainApi",
		"CollectCollationInfo",
		"GenesisBuilder",
		"CreateTransactionApi",
	];
	#[doc = r" The error type returned when there is a runtime issue."]
	pub type DispatchError = runtime_types::sp_runtime::DispatchError;
	#[doc = r" The outer event enum."]
	pub type Event = runtime_types::gargantua_runtime::RuntimeEvent;
	#[doc = r" The outer extrinsic enum."]
	pub type Call = runtime_types::gargantua_runtime::RuntimeCall;
	#[doc = r" The outer error enum representing the DispatchError's Module variant."]
	pub type Error = runtime_types::gargantua_runtime::RuntimeError;
	pub fn constants() -> ConstantsApi {
		ConstantsApi
	}
	pub fn storage() -> StorageApi {
		StorageApi
	}
	pub fn tx() -> TransactionApi {
		TransactionApi
	}
	pub fn apis() -> runtime_apis::RuntimeApi {
		runtime_apis::RuntimeApi
	}
	pub mod runtime_apis {
		use super::{root_mod, runtime_types};
		use ::subxt::ext::codec::Encode;
		pub struct RuntimeApi;
		impl RuntimeApi {
			pub fn aura_api(&self) -> aura_api::AuraApi {
				aura_api::AuraApi
			}
			pub fn aura_unincluded_segment_api(
				&self,
			) -> aura_unincluded_segment_api::AuraUnincludedSegmentApi {
				aura_unincluded_segment_api::AuraUnincludedSegmentApi
			}
			pub fn core(&self) -> core::Core {
				core::Core
			}
			pub fn metadata(&self) -> metadata::Metadata {
				metadata::Metadata
			}
			pub fn block_builder(&self) -> block_builder::BlockBuilder {
				block_builder::BlockBuilder
			}
			pub fn tagged_transaction_queue(
				&self,
			) -> tagged_transaction_queue::TaggedTransactionQueue {
				tagged_transaction_queue::TaggedTransactionQueue
			}
			pub fn offchain_worker_api(&self) -> offchain_worker_api::OffchainWorkerApi {
				offchain_worker_api::OffchainWorkerApi
			}
			pub fn session_keys(&self) -> session_keys::SessionKeys {
				session_keys::SessionKeys
			}
			pub fn account_nonce_api(&self) -> account_nonce_api::AccountNonceApi {
				account_nonce_api::AccountNonceApi
			}
			pub fn transaction_payment_api(
				&self,
			) -> transaction_payment_api::TransactionPaymentApi {
				transaction_payment_api::TransactionPaymentApi
			}
			pub fn transaction_payment_call_api(
				&self,
			) -> transaction_payment_call_api::TransactionPaymentCallApi {
				transaction_payment_call_api::TransactionPaymentCallApi
			}
			pub fn mmr_runtime_api(&self) -> mmr_runtime_api::MmrRuntimeApi {
				mmr_runtime_api::MmrRuntimeApi
			}
			pub fn ismp_runtime_api(&self) -> ismp_runtime_api::IsmpRuntimeApi {
				ismp_runtime_api::IsmpRuntimeApi
			}
			pub fn ismp_parachain_api(&self) -> ismp_parachain_api::IsmpParachainApi {
				ismp_parachain_api::IsmpParachainApi
			}
			pub fn collect_collation_info(&self) -> collect_collation_info::CollectCollationInfo {
				collect_collation_info::CollectCollationInfo
			}
			pub fn genesis_builder(&self) -> genesis_builder::GenesisBuilder {
				genesis_builder::GenesisBuilder
			}
			pub fn create_transaction_api(&self) -> create_transaction_api::CreateTransactionApi {
				create_transaction_api::CreateTransactionApi
			}
		}
		pub mod aura_api {
			use super::{root_mod, runtime_types};
			#[doc = " API necessary for block authorship with aura."]
			pub struct AuraApi;
			impl AuraApi {
				#[doc = " Returns the slot duration for Aura."]
				#[doc = ""]
				#[doc = " Currently, only the value provided by this type at genesis will be used."]
				pub fn slot_duration(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::SlotDuration,
					runtime_types::sp_consensus_slots::SlotDuration,
				> {
					::subxt::runtime_api::Payload::new_static(
						"AuraApi",
						"slot_duration",
						types::SlotDuration {},
						[
							233u8, 210u8, 132u8, 172u8, 100u8, 125u8, 239u8, 92u8, 114u8, 82u8,
							7u8, 110u8, 179u8, 196u8, 10u8, 19u8, 211u8, 15u8, 174u8, 2u8, 91u8,
							73u8, 133u8, 100u8, 205u8, 201u8, 191u8, 60u8, 163u8, 122u8, 215u8,
							10u8,
						],
					)
				}
				#[doc = " Return the current set of authorities."]
				pub fn authorities(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Authorities,
					::std::vec::Vec<runtime_types::sp_consensus_aura::sr25519::app_sr25519::Public>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"AuraApi",
						"authorities",
						types::Authorities {},
						[
							96u8, 136u8, 226u8, 244u8, 105u8, 189u8, 8u8, 250u8, 71u8, 230u8, 37u8,
							123u8, 218u8, 47u8, 179u8, 16u8, 170u8, 181u8, 165u8, 77u8, 102u8,
							51u8, 43u8, 51u8, 186u8, 84u8, 49u8, 15u8, 208u8, 226u8, 129u8, 230u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SlotDuration {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Authorities {}
			}
		}
		pub mod aura_unincluded_segment_api {
			use super::{root_mod, runtime_types};
			#[doc = " This runtime API is used to inform potential block authors whether they will"]
			#[doc = " have the right to author at a slot, assuming they have claimed the slot."]
			#[doc = ""]
			#[doc = " In particular, this API allows Aura-based parachains to regulate their \"unincluded segment\","]
			#[doc = " which is the section of the head of the chain which has not yet been made available in the"]
			#[doc = " relay chain."]
			#[doc = ""]
			#[doc = " When the unincluded segment is short, Aura chains will allow authors to create multiple"]
			#[doc = " blocks per slot in order to build a backlog. When it is saturated, this API will limit"]
			#[doc = " the amount of blocks that can be created."]
			pub struct AuraUnincludedSegmentApi;
			impl AuraUnincludedSegmentApi {
				#[doc = " Whether it is legal to extend the chain, assuming the given block is the most"]
				#[doc = " recently included one as-of the relay parent that will be built against, and"]
				#[doc = " the given slot."]
				#[doc = ""]
				#[doc = " This should be consistent with the logic the runtime uses when validating blocks to"]
				#[doc = " avoid issues."]
				#[doc = ""]
				#[doc = " When the unincluded segment is empty, i.e. `included_hash == at`, where at is the block"]
				#[doc = " whose state we are querying against, this must always return `true` as long as the slot"]
				#[doc = " is more recent than the included block itself."]
				pub fn can_build_upon(
					&self,
					included_hash: ::subxt::utils::H256,
					slot: runtime_types::sp_consensus_slots::Slot,
				) -> ::subxt::runtime_api::Payload<types::CanBuildUpon, ::core::primitive::bool>
				{
					::subxt::runtime_api::Payload::new_static(
						"AuraUnincludedSegmentApi",
						"can_build_upon",
						types::CanBuildUpon { included_hash, slot },
						[
							255u8, 59u8, 225u8, 229u8, 189u8, 250u8, 48u8, 150u8, 92u8, 226u8,
							221u8, 202u8, 143u8, 145u8, 107u8, 112u8, 151u8, 146u8, 136u8, 155u8,
							118u8, 174u8, 52u8, 178u8, 14u8, 89u8, 194u8, 157u8, 110u8, 103u8,
							92u8, 72u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CanBuildUpon {
					pub included_hash: ::subxt::utils::H256,
					pub slot: runtime_types::sp_consensus_slots::Slot,
				}
			}
		}
		pub mod core {
			use super::{root_mod, runtime_types};
			#[doc = " The `Core` runtime api that every Substrate runtime needs to implement."]
			pub struct Core;
			impl Core {
				#[doc = " Returns the version of the runtime."]
				pub fn version(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Version,
					runtime_types::sp_version::RuntimeVersion,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"version",
						types::Version {},
						[
							76u8, 202u8, 17u8, 117u8, 189u8, 237u8, 239u8, 237u8, 151u8, 17u8,
							125u8, 159u8, 218u8, 92u8, 57u8, 238u8, 64u8, 147u8, 40u8, 72u8, 157u8,
							116u8, 37u8, 195u8, 156u8, 27u8, 123u8, 173u8, 178u8, 102u8, 136u8,
							6u8,
						],
					)
				}
				#[doc = " Execute the given block."]
				pub fn execute_block(
					&self,
					block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > >,
				) -> ::subxt::runtime_api::Payload<types::ExecuteBlock, ()> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"execute_block",
						types::ExecuteBlock { block },
						[
							133u8, 135u8, 228u8, 65u8, 106u8, 27u8, 85u8, 158u8, 112u8, 254u8,
							93u8, 26u8, 102u8, 201u8, 118u8, 216u8, 249u8, 247u8, 91u8, 74u8, 56u8,
							208u8, 231u8, 115u8, 131u8, 29u8, 209u8, 6u8, 65u8, 57u8, 214u8, 125u8,
						],
					)
				}
				#[doc = " Initialize a block with the given header."]
				pub fn initialize_block(
					&self,
					header: runtime_types::sp_runtime::generic::header::Header<
						::core::primitive::u32,
					>,
				) -> ::subxt::runtime_api::Payload<types::InitializeBlock, ()> {
					::subxt::runtime_api::Payload::new_static(
						"Core",
						"initialize_block",
						types::InitializeBlock { header },
						[
							146u8, 138u8, 72u8, 240u8, 63u8, 96u8, 110u8, 189u8, 77u8, 92u8, 96u8,
							232u8, 41u8, 217u8, 105u8, 148u8, 83u8, 190u8, 152u8, 219u8, 19u8,
							87u8, 163u8, 1u8, 232u8, 25u8, 221u8, 74u8, 224u8, 67u8, 223u8, 34u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Version {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExecuteBlock { pub block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InitializeBlock {
					pub header:
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				}
			}
		}
		pub mod metadata {
			use super::{root_mod, runtime_types};
			#[doc = " The `Metadata` api trait that returns metadata for the runtime."]
			pub struct Metadata;
			impl Metadata {
				#[doc = " Returns the metadata of a runtime."]
				pub fn metadata(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::Metadata,
					runtime_types::sp_core::OpaqueMetadata,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata",
						types::Metadata {},
						[
							231u8, 24u8, 67u8, 152u8, 23u8, 26u8, 188u8, 82u8, 229u8, 6u8, 185u8,
							27u8, 175u8, 68u8, 83u8, 122u8, 69u8, 89u8, 185u8, 74u8, 248u8, 87u8,
							217u8, 124u8, 193u8, 252u8, 199u8, 186u8, 196u8, 179u8, 179u8, 96u8,
						],
					)
				}
				#[doc = " Returns the metadata at a given version."]
				#[doc = ""]
				#[doc = " If the given `version` isn't supported, this will return `None`."]
				#[doc = " Use [`Self::metadata_versions`] to find out about supported metadata version of the runtime."]
				pub fn metadata_at_version(
					&self,
					version: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::MetadataAtVersion,
					::core::option::Option<runtime_types::sp_core::OpaqueMetadata>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata_at_version",
						types::MetadataAtVersion { version },
						[
							131u8, 53u8, 212u8, 234u8, 16u8, 25u8, 120u8, 252u8, 153u8, 153u8,
							216u8, 28u8, 54u8, 113u8, 52u8, 236u8, 146u8, 68u8, 142u8, 8u8, 10u8,
							169u8, 131u8, 142u8, 204u8, 38u8, 48u8, 108u8, 134u8, 86u8, 226u8,
							61u8,
						],
					)
				}
				#[doc = " Returns the supported metadata versions."]
				#[doc = ""]
				#[doc = " This can be used to call `metadata_at_version`."]
				pub fn metadata_versions(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::MetadataVersions,
					::std::vec::Vec<::core::primitive::u32>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"Metadata",
						"metadata_versions",
						types::MetadataVersions {},
						[
							23u8, 144u8, 137u8, 91u8, 188u8, 39u8, 231u8, 208u8, 252u8, 218u8,
							224u8, 176u8, 77u8, 32u8, 130u8, 212u8, 223u8, 76u8, 100u8, 190u8,
							82u8, 94u8, 190u8, 8u8, 82u8, 244u8, 225u8, 179u8, 85u8, 176u8, 56u8,
							16u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Metadata {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MetadataAtVersion {
					pub version: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MetadataVersions {}
			}
		}
		pub mod block_builder {
			use super::{root_mod, runtime_types};
			#[doc = " The `BlockBuilder` api trait that provides the required functionality for building a block."]
			pub struct BlockBuilder;
			impl BlockBuilder {
				#[doc = " Apply the given extrinsic."]
				#[doc = ""]
				#[doc = " Returns an inclusion outcome which specifies if this extrinsic is included in"]
				#[doc = " this block or not."]
				pub fn apply_extrinsic(
					&self,
					extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
				) -> ::subxt::runtime_api::Payload<
					types::ApplyExtrinsic,
					::core::result::Result<
						::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
						runtime_types::sp_runtime::transaction_validity::TransactionValidityError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"apply_extrinsic",
						types::ApplyExtrinsic { extrinsic },
						[
							72u8, 54u8, 139u8, 3u8, 118u8, 136u8, 65u8, 47u8, 6u8, 105u8, 125u8,
							223u8, 160u8, 29u8, 103u8, 74u8, 79u8, 149u8, 48u8, 90u8, 237u8, 2u8,
							97u8, 201u8, 123u8, 34u8, 167u8, 37u8, 187u8, 35u8, 176u8, 97u8,
						],
					)
				}
				#[doc = " Finish the current block."]
				pub fn finalize_block(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::FinalizeBlock,
					runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"finalize_block",
						types::FinalizeBlock {},
						[
							244u8, 207u8, 24u8, 33u8, 13u8, 69u8, 9u8, 249u8, 145u8, 143u8, 122u8,
							96u8, 197u8, 55u8, 64u8, 111u8, 238u8, 224u8, 34u8, 201u8, 27u8, 146u8,
							232u8, 99u8, 191u8, 30u8, 114u8, 16u8, 32u8, 220u8, 58u8, 62u8,
						],
					)
				}
				#[doc = " Generate inherent extrinsics. The inherent data will vary from chain to chain."]				pub fn inherent_extrinsics (& self , inherent : runtime_types :: sp_inherents :: InherentData ,) -> :: subxt :: runtime_api :: Payload < types :: InherentExtrinsics , :: std :: vec :: Vec < :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > >{
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"inherent_extrinsics",
						types::InherentExtrinsics { inherent },
						[
							254u8, 110u8, 245u8, 201u8, 250u8, 192u8, 27u8, 228u8, 151u8, 213u8,
							166u8, 89u8, 94u8, 81u8, 189u8, 234u8, 64u8, 18u8, 245u8, 80u8, 29u8,
							18u8, 140u8, 129u8, 113u8, 236u8, 135u8, 55u8, 79u8, 159u8, 175u8,
							183u8,
						],
					)
				}
				#[doc = " Check that the inherents are valid. The inherent data will vary from chain to chain."]
				pub fn check_inherents(
					&self,
					block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > >,
					data: runtime_types::sp_inherents::InherentData,
				) -> ::subxt::runtime_api::Payload<
					types::CheckInherents,
					runtime_types::sp_inherents::CheckInherentsResult,
				> {
					::subxt::runtime_api::Payload::new_static(
						"BlockBuilder",
						"check_inherents",
						types::CheckInherents { block, data },
						[
							153u8, 134u8, 1u8, 215u8, 139u8, 11u8, 53u8, 51u8, 210u8, 175u8, 197u8,
							28u8, 38u8, 209u8, 175u8, 247u8, 142u8, 157u8, 50u8, 151u8, 164u8,
							191u8, 181u8, 118u8, 80u8, 97u8, 160u8, 248u8, 110u8, 217u8, 181u8,
							234u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ApplyExtrinsic { pub extrinsic : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FinalizeBlock {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InherentExtrinsics {
					pub inherent: runtime_types::sp_inherents::InherentData,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CheckInherents { pub block : runtime_types :: sp_runtime :: generic :: block :: Block < runtime_types :: sp_runtime :: generic :: header :: Header < :: core :: primitive :: u32 > , :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > > , pub data : runtime_types :: sp_inherents :: InherentData , }
			}
		}
		pub mod tagged_transaction_queue {
			use super::{root_mod, runtime_types};
			#[doc = " The `TaggedTransactionQueue` api trait for interfering with the transaction queue."]
			pub struct TaggedTransactionQueue;
			impl TaggedTransactionQueue {
				#[doc = " Validate the transaction."]
				#[doc = ""]
				#[doc = " This method is invoked by the transaction pool to learn details about given transaction."]
				#[doc = " The implementation should make sure to verify the correctness of the transaction"]
				#[doc = " against current state. The given `block_hash` corresponds to the hash of the block"]
				#[doc = " that is used as current state."]
				#[doc = ""]
				#[doc = " Note that this call may be performed by the pool multiple times and transactions"]
				#[doc = " might be verified in any possible order."]
				pub fn validate_transaction(
					&self,
					source: runtime_types::sp_runtime::transaction_validity::TransactionSource,
					tx : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					block_hash: ::subxt::utils::H256,
				) -> ::subxt::runtime_api::Payload<
					types::ValidateTransaction,
					::core::result::Result<
						runtime_types::sp_runtime::transaction_validity::ValidTransaction,
						runtime_types::sp_runtime::transaction_validity::TransactionValidityError,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TaggedTransactionQueue",
						"validate_transaction",
						types::ValidateTransaction { source, tx, block_hash },
						[
							196u8, 50u8, 90u8, 49u8, 109u8, 251u8, 200u8, 35u8, 23u8, 150u8, 140u8,
							143u8, 232u8, 164u8, 133u8, 89u8, 32u8, 240u8, 115u8, 39u8, 95u8, 70u8,
							162u8, 76u8, 122u8, 73u8, 151u8, 144u8, 234u8, 120u8, 100u8, 29u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ValidateTransaction { pub source : runtime_types :: sp_runtime :: transaction_validity :: TransactionSource , pub tx : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub block_hash : :: subxt :: utils :: H256 , }
			}
		}
		pub mod offchain_worker_api {
			use super::{root_mod, runtime_types};
			#[doc = " The offchain worker api."]
			pub struct OffchainWorkerApi;
			impl OffchainWorkerApi {
				#[doc = " Starts the off-chain task for given block header."]
				pub fn offchain_worker(
					&self,
					header: runtime_types::sp_runtime::generic::header::Header<
						::core::primitive::u32,
					>,
				) -> ::subxt::runtime_api::Payload<types::OffchainWorker, ()> {
					::subxt::runtime_api::Payload::new_static(
						"OffchainWorkerApi",
						"offchain_worker",
						types::OffchainWorker { header },
						[
							10u8, 135u8, 19u8, 153u8, 33u8, 216u8, 18u8, 242u8, 33u8, 140u8, 4u8,
							223u8, 200u8, 130u8, 103u8, 118u8, 137u8, 24u8, 19u8, 127u8, 161u8,
							29u8, 184u8, 111u8, 222u8, 111u8, 253u8, 73u8, 45u8, 31u8, 79u8, 60u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct OffchainWorker {
					pub header:
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				}
			}
		}
		pub mod session_keys {
			use super::{root_mod, runtime_types};
			#[doc = " Session keys runtime api."]
			pub struct SessionKeys;
			impl SessionKeys {
				#[doc = " Generate a set of session keys with optionally using the given seed."]
				#[doc = " The keys should be stored within the keystore exposed via runtime"]
				#[doc = " externalities."]
				#[doc = ""]
				#[doc = " The seed needs to be a valid `utf8` string."]
				#[doc = ""]
				#[doc = " Returns the concatenated SCALE encoded public keys."]
				pub fn generate_session_keys(
					&self,
					seed: ::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateSessionKeys,
					::std::vec::Vec<::core::primitive::u8>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"SessionKeys",
						"generate_session_keys",
						types::GenerateSessionKeys { seed },
						[
							96u8, 171u8, 164u8, 166u8, 175u8, 102u8, 101u8, 47u8, 133u8, 95u8,
							102u8, 202u8, 83u8, 26u8, 238u8, 47u8, 126u8, 132u8, 22u8, 11u8, 33u8,
							190u8, 175u8, 94u8, 58u8, 245u8, 46u8, 80u8, 195u8, 184u8, 107u8, 65u8,
						],
					)
				}
				#[doc = " Decode the given public session keys."]
				#[doc = ""]
				#[doc = " Returns the list of public raw public keys + key type."]
				pub fn decode_session_keys(
					&self,
					encoded: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::runtime_api::Payload<
					types::DecodeSessionKeys,
					::core::option::Option<
						::std::vec::Vec<(
							::std::vec::Vec<::core::primitive::u8>,
							runtime_types::sp_core::crypto::KeyTypeId,
						)>,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"SessionKeys",
						"decode_session_keys",
						types::DecodeSessionKeys { encoded },
						[
							57u8, 242u8, 18u8, 51u8, 132u8, 110u8, 238u8, 255u8, 39u8, 194u8, 8u8,
							54u8, 198u8, 178u8, 75u8, 151u8, 148u8, 176u8, 144u8, 197u8, 87u8,
							29u8, 179u8, 235u8, 176u8, 78u8, 252u8, 103u8, 72u8, 203u8, 151u8,
							248u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GenerateSessionKeys {
					pub seed: ::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DecodeSessionKeys {
					pub encoded: ::std::vec::Vec<::core::primitive::u8>,
				}
			}
		}
		pub mod account_nonce_api {
			use super::{root_mod, runtime_types};
			#[doc = " The API to query account nonce."]
			pub struct AccountNonceApi;
			impl AccountNonceApi {
				#[doc = " Get current account nonce of given `AccountId`."]
				pub fn account_nonce(
					&self,
					account: ::subxt::utils::AccountId32,
				) -> ::subxt::runtime_api::Payload<types::AccountNonce, ::core::primitive::u32>
				{
					::subxt::runtime_api::Payload::new_static(
						"AccountNonceApi",
						"account_nonce",
						types::AccountNonce { account },
						[
							231u8, 82u8, 7u8, 227u8, 131u8, 2u8, 215u8, 252u8, 173u8, 82u8, 11u8,
							103u8, 200u8, 25u8, 114u8, 116u8, 79u8, 229u8, 152u8, 150u8, 236u8,
							37u8, 101u8, 26u8, 220u8, 146u8, 182u8, 101u8, 73u8, 55u8, 191u8,
							171u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccountNonce {
					pub account: ::subxt::utils::AccountId32,
				}
			}
		}
		pub mod transaction_payment_api {
			use super::{root_mod, runtime_types};
			pub struct TransactionPaymentApi;
			impl TransactionPaymentApi {
				pub fn query_info(
					&self,
					uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryInfo,
					runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
						::core::primitive::u128,
						runtime_types::sp_weights::weight_v2::Weight,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_info",
						types::QueryInfo { uxt, len },
						[
							56u8, 30u8, 174u8, 34u8, 202u8, 24u8, 177u8, 189u8, 145u8, 36u8, 1u8,
							156u8, 98u8, 209u8, 178u8, 49u8, 198u8, 23u8, 150u8, 173u8, 35u8,
							205u8, 147u8, 129u8, 42u8, 22u8, 69u8, 3u8, 129u8, 8u8, 196u8, 139u8,
						],
					)
				}
				pub fn query_fee_details(
					&self,
					uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) >,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryFeeDetails,
					runtime_types::pallet_transaction_payment::types::FeeDetails<
						::core::primitive::u128,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_fee_details",
						types::QueryFeeDetails { uxt, len },
						[
							117u8, 60u8, 137u8, 159u8, 237u8, 252u8, 216u8, 238u8, 232u8, 1u8,
							100u8, 152u8, 26u8, 185u8, 145u8, 125u8, 68u8, 189u8, 4u8, 30u8, 125u8,
							7u8, 196u8, 153u8, 235u8, 51u8, 219u8, 108u8, 185u8, 254u8, 100u8,
							201u8,
						],
					)
				}
				pub fn query_weight_to_fee(
					&self,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::runtime_api::Payload<types::QueryWeightToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_weight_to_fee",
						types::QueryWeightToFee { weight },
						[
							206u8, 243u8, 189u8, 83u8, 231u8, 244u8, 247u8, 52u8, 126u8, 208u8,
							224u8, 5u8, 163u8, 108u8, 254u8, 114u8, 214u8, 156u8, 227u8, 217u8,
							211u8, 198u8, 121u8, 164u8, 110u8, 54u8, 181u8, 146u8, 50u8, 146u8,
							146u8, 23u8,
						],
					)
				}
				pub fn query_length_to_fee(
					&self,
					length: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<types::QueryLengthToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentApi",
						"query_length_to_fee",
						types::QueryLengthToFee { length },
						[
							92u8, 132u8, 29u8, 119u8, 66u8, 11u8, 196u8, 224u8, 129u8, 23u8, 249u8,
							12u8, 32u8, 28u8, 92u8, 50u8, 188u8, 101u8, 203u8, 229u8, 248u8, 216u8,
							130u8, 150u8, 212u8, 161u8, 81u8, 254u8, 116u8, 89u8, 162u8, 48u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryInfo { pub uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub len : :: core :: primitive :: u32 , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryFeeDetails { pub uxt : :: subxt :: utils :: UncheckedExtrinsic < :: subxt :: utils :: MultiAddress < :: subxt :: utils :: AccountId32 , () > , runtime_types :: gargantua_runtime :: RuntimeCall , runtime_types :: sp_runtime :: MultiSignature , (runtime_types :: frame_system :: extensions :: check_non_zero_sender :: CheckNonZeroSender , runtime_types :: frame_system :: extensions :: check_spec_version :: CheckSpecVersion , runtime_types :: frame_system :: extensions :: check_tx_version :: CheckTxVersion , runtime_types :: frame_system :: extensions :: check_genesis :: CheckGenesis , runtime_types :: frame_system :: extensions :: check_mortality :: CheckMortality , runtime_types :: frame_system :: extensions :: check_nonce :: CheckNonce , runtime_types :: frame_system :: extensions :: check_weight :: CheckWeight , runtime_types :: pallet_transaction_payment :: ChargeTransactionPayment ,) > , pub len : :: core :: primitive :: u32 , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryWeightToFee {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryLengthToFee {
					pub length: ::core::primitive::u32,
				}
			}
		}
		pub mod transaction_payment_call_api {
			use super::{root_mod, runtime_types};
			pub struct TransactionPaymentCallApi;
			impl TransactionPaymentCallApi {
				#[doc = " Query information of a dispatch class, weight, and fee of a given encoded `Call`."]
				pub fn query_call_info(
					&self,
					call: runtime_types::gargantua_runtime::RuntimeCall,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallInfo,
					runtime_types::pallet_transaction_payment::types::RuntimeDispatchInfo<
						::core::primitive::u128,
						runtime_types::sp_weights::weight_v2::Weight,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_info",
						types::QueryCallInfo { call, len },
						[
							220u8, 59u8, 17u8, 207u8, 34u8, 8u8, 175u8, 39u8, 240u8, 111u8, 55u8,
							14u8, 137u8, 127u8, 214u8, 179u8, 154u8, 120u8, 107u8, 126u8, 94u8,
							243u8, 34u8, 154u8, 112u8, 100u8, 206u8, 89u8, 48u8, 250u8, 159u8,
							136u8,
						],
					)
				}
				#[doc = " Query fee details of a given encoded `Call`."]
				pub fn query_call_fee_details(
					&self,
					call: runtime_types::gargantua_runtime::RuntimeCall,
					len: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<
					types::QueryCallFeeDetails,
					runtime_types::pallet_transaction_payment::types::FeeDetails<
						::core::primitive::u128,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_call_fee_details",
						types::QueryCallFeeDetails { call, len },
						[
							0u8, 152u8, 207u8, 19u8, 91u8, 183u8, 116u8, 131u8, 0u8, 14u8, 86u8,
							49u8, 178u8, 70u8, 7u8, 124u8, 240u8, 132u8, 79u8, 143u8, 213u8, 155u8,
							197u8, 133u8, 81u8, 17u8, 5u8, 83u8, 168u8, 146u8, 176u8, 177u8,
						],
					)
				}
				#[doc = " Query the output of the current `WeightToFee` given some input."]
				pub fn query_weight_to_fee(
					&self,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::runtime_api::Payload<types::QueryWeightToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_weight_to_fee",
						types::QueryWeightToFee { weight },
						[
							117u8, 91u8, 94u8, 22u8, 248u8, 212u8, 15u8, 23u8, 97u8, 116u8, 64u8,
							228u8, 83u8, 123u8, 87u8, 77u8, 97u8, 7u8, 98u8, 181u8, 6u8, 165u8,
							114u8, 141u8, 164u8, 113u8, 126u8, 88u8, 174u8, 171u8, 224u8, 35u8,
						],
					)
				}
				#[doc = " Query the output of the current `LengthToFee` given some input."]
				pub fn query_length_to_fee(
					&self,
					length: ::core::primitive::u32,
				) -> ::subxt::runtime_api::Payload<types::QueryLengthToFee, ::core::primitive::u128>
				{
					::subxt::runtime_api::Payload::new_static(
						"TransactionPaymentCallApi",
						"query_length_to_fee",
						types::QueryLengthToFee { length },
						[
							246u8, 40u8, 4u8, 160u8, 152u8, 94u8, 170u8, 53u8, 205u8, 122u8, 5u8,
							69u8, 70u8, 25u8, 128u8, 156u8, 119u8, 134u8, 116u8, 147u8, 14u8,
							164u8, 65u8, 140u8, 86u8, 13u8, 250u8, 218u8, 89u8, 95u8, 234u8, 228u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryCallInfo {
					pub call: runtime_types::gargantua_runtime::RuntimeCall,
					pub len: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryCallFeeDetails {
					pub call: runtime_types::gargantua_runtime::RuntimeCall,
					pub len: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryWeightToFee {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryLengthToFee {
					pub length: ::core::primitive::u32,
				}
			}
		}
		pub mod mmr_runtime_api {
			use super::{root_mod, runtime_types};
			#[doc = " MmrRuntimeApi"]
			pub struct MmrRuntimeApi;
			impl MmrRuntimeApi {
				#[doc = " Return Block number where pallet-mmr was added to the runtime"]
				pub fn pallet_genesis(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::PalletGenesis,
					::core::result::Result<
						::core::option::Option<::core::primitive::u32>,
						runtime_types::sp_mmr_primitives::Error,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MmrRuntimeApi",
						"pallet_genesis",
						types::PalletGenesis {},
						[
							221u8, 97u8, 190u8, 29u8, 109u8, 247u8, 150u8, 7u8, 215u8, 120u8,
							147u8, 9u8, 156u8, 229u8, 221u8, 233u8, 232u8, 182u8, 22u8, 245u8,
							84u8, 220u8, 83u8, 183u8, 166u8, 161u8, 170u8, 129u8, 232u8, 96u8,
							250u8, 109u8,
						],
					)
				}
				#[doc = " Return the number of MMR leaves."]
				pub fn mmr_leaf_count(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::MmrLeafCount,
					::core::result::Result<
						::core::primitive::u64,
						runtime_types::sp_mmr_primitives::Error,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MmrRuntimeApi",
						"mmr_leaf_count",
						types::MmrLeafCount {},
						[
							37u8, 82u8, 195u8, 123u8, 240u8, 88u8, 235u8, 246u8, 28u8, 199u8, 48u8,
							42u8, 8u8, 36u8, 187u8, 77u8, 115u8, 0u8, 254u8, 139u8, 171u8, 90u8,
							64u8, 196u8, 131u8, 252u8, 116u8, 176u8, 25u8, 179u8, 44u8, 152u8,
						],
					)
				}
				#[doc = " Return the on-chain MMR root hash."]
				pub fn mmr_root(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::MmrRoot,
					::core::result::Result<
						::subxt::utils::H256,
						runtime_types::sp_mmr_primitives::Error,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MmrRuntimeApi",
						"mmr_root",
						types::MmrRoot {},
						[
							50u8, 174u8, 21u8, 228u8, 17u8, 52u8, 134u8, 55u8, 249u8, 78u8, 197u8,
							117u8, 185u8, 16u8, 209u8, 206u8, 121u8, 39u8, 149u8, 216u8, 53u8,
							212u8, 120u8, 156u8, 27u8, 77u8, 193u8, 46u8, 228u8, 21u8, 42u8, 240u8,
						],
					)
				}
				#[doc = " Return the unique hash used as the offchain prefix at a particular block"]
				pub fn fork_identifier(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::ForkIdentifier,
					::core::result::Result<
						::subxt::utils::H256,
						runtime_types::sp_mmr_primitives::Error,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"MmrRuntimeApi",
						"fork_identifier",
						types::ForkIdentifier {},
						[
							99u8, 151u8, 55u8, 71u8, 9u8, 53u8, 56u8, 23u8, 78u8, 62u8, 37u8,
							139u8, 196u8, 64u8, 138u8, 225u8, 237u8, 49u8, 154u8, 231u8, 107u8,
							78u8, 243u8, 211u8, 174u8, 155u8, 184u8, 217u8, 114u8, 183u8, 13u8,
							20u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PalletGenesis {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MmrLeafCount {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MmrRoot {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForkIdentifier {}
			}
		}
		pub mod ismp_runtime_api {
			use super::{root_mod, runtime_types};
			#[doc = " Required runtime APIs needed for client subsystems like the RPC"]
			pub struct IsmpRuntimeApi;
			impl IsmpRuntimeApi {
				#[doc = " Should return the host's state machine identifier"]
				pub fn host_state_machine(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::HostStateMachine,
					runtime_types::ismp::host::StateMachine,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"host_state_machine",
						types::HostStateMachine {},
						[
							102u8, 67u8, 86u8, 123u8, 218u8, 20u8, 206u8, 36u8, 51u8, 149u8, 84u8,
							106u8, 20u8, 209u8, 236u8, 134u8, 143u8, 22u8, 182u8, 1u8, 93u8, 114u8,
							58u8, 37u8, 86u8, 57u8, 67u8, 200u8, 3u8, 155u8, 98u8, 188u8,
						],
					)
				}
				#[doc = " Generate a proof for the provided leaf indices"]
				pub fn generate_proof(
					&self,
					commitments: runtime_types::pallet_ismp::mmr::ProofKeys,
				) -> ::subxt::runtime_api::Payload<
					types::GenerateProof,
					::core::result::Result<
						(
							::std::vec::Vec<runtime_types::pallet_ismp::mmr::Leaf>,
							runtime_types::pallet_ismp::mmr::Proof<::subxt::utils::H256>,
						),
						runtime_types::sp_mmr_primitives::Error,
					>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"generate_proof",
						types::GenerateProof { commitments },
						[
							195u8, 59u8, 24u8, 130u8, 146u8, 41u8, 244u8, 115u8, 22u8, 204u8, 12u8,
							238u8, 78u8, 228u8, 250u8, 182u8, 97u8, 66u8, 131u8, 113u8, 119u8,
							224u8, 39u8, 101u8, 92u8, 162u8, 249u8, 36u8, 173u8, 169u8, 20u8,
							204u8,
						],
					)
				}
				#[doc = " Fetch all ISMP events"]
				pub fn block_events(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::BlockEvents,
					::std::vec::Vec<runtime_types::ismp::events::Event>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"block_events",
						types::BlockEvents {},
						[
							117u8, 117u8, 10u8, 114u8, 126u8, 138u8, 41u8, 13u8, 44u8, 180u8,
							220u8, 185u8, 50u8, 144u8, 58u8, 232u8, 120u8, 130u8, 129u8, 69u8,
							145u8, 6u8, 189u8, 255u8, 184u8, 204u8, 97u8, 41u8, 83u8, 127u8, 191u8,
							247u8,
						],
					)
				}
				#[doc = " Fetch all ISMP events and their extrinsic metadata"]
				pub fn block_events_with_metadata(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::BlockEventsWithMetadata,
					::std::vec::Vec<(
						runtime_types::ismp::events::Event,
						::core::option::Option<::core::primitive::u32>,
					)>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"block_events_with_metadata",
						types::BlockEventsWithMetadata {},
						[
							22u8, 59u8, 32u8, 3u8, 104u8, 73u8, 250u8, 52u8, 57u8, 109u8, 84u8,
							218u8, 81u8, 125u8, 85u8, 158u8, 110u8, 208u8, 129u8, 32u8, 97u8, 60u8,
							61u8, 214u8, 51u8, 37u8, 218u8, 161u8, 25u8, 41u8, 58u8, 197u8,
						],
					)
				}
				#[doc = " Return the scale encoded consensus state"]
				pub fn consensus_state(
					&self,
					id: [::core::primitive::u8; 4usize],
				) -> ::subxt::runtime_api::Payload<
					types::ConsensusState,
					::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"consensus_state",
						types::ConsensusState { id },
						[
							210u8, 111u8, 190u8, 64u8, 68u8, 221u8, 113u8, 196u8, 214u8, 50u8,
							128u8, 16u8, 86u8, 45u8, 203u8, 253u8, 204u8, 94u8, 61u8, 116u8, 8u8,
							145u8, 2u8, 105u8, 133u8, 82u8, 187u8, 37u8, 254u8, 43u8, 26u8, 210u8,
						],
					)
				}
				#[doc = " Return the timestamp this client was last updated in seconds"]
				pub fn consensus_update_time(
					&self,
					id: [::core::primitive::u8; 4usize],
				) -> ::subxt::runtime_api::Payload<
					types::ConsensusUpdateTime,
					::core::option::Option<::core::primitive::u64>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"consensus_update_time",
						types::ConsensusUpdateTime { id },
						[
							56u8, 254u8, 10u8, 204u8, 210u8, 108u8, 29u8, 214u8, 14u8, 87u8, 147u8,
							140u8, 251u8, 18u8, 142u8, 173u8, 161u8, 255u8, 102u8, 177u8, 173u8,
							173u8, 25u8, 43u8, 208u8, 169u8, 244u8, 23u8, 109u8, 97u8, 70u8, 216u8,
						],
					)
				}
				#[doc = " Return the challenge period timestamp"]
				pub fn challenge_period(
					&self,
					id: [::core::primitive::u8; 4usize],
				) -> ::subxt::runtime_api::Payload<
					types::ChallengePeriod,
					::core::option::Option<::core::primitive::u64>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"challenge_period",
						types::ChallengePeriod { id },
						[
							27u8, 37u8, 95u8, 172u8, 219u8, 48u8, 148u8, 49u8, 235u8, 202u8, 36u8,
							4u8, 63u8, 13u8, 241u8, 33u8, 67u8, 249u8, 130u8, 158u8, 128u8, 221u8,
							221u8, 112u8, 79u8, 140u8, 221u8, 76u8, 149u8, 75u8, 206u8, 114u8,
						],
					)
				}
				#[doc = " Return the latest height of the state machine"]
				pub fn latest_state_machine_height(
					&self,
					id: runtime_types::ismp::consensus::StateMachineId,
				) -> ::subxt::runtime_api::Payload<
					types::LatestStateMachineHeight,
					::core::option::Option<::core::primitive::u64>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"latest_state_machine_height",
						types::LatestStateMachineHeight { id },
						[
							202u8, 52u8, 204u8, 47u8, 112u8, 86u8, 211u8, 225u8, 142u8, 185u8,
							173u8, 37u8, 167u8, 199u8, 48u8, 204u8, 32u8, 24u8, 57u8, 2u8, 200u8,
							151u8, 193u8, 39u8, 124u8, 99u8, 8u8, 223u8, 127u8, 45u8, 173u8, 193u8,
						],
					)
				}
				#[doc = " Fetch the requests for the given commitments."]
				pub fn requests(
					&self,
					request_commitments: ::std::vec::Vec<::subxt::utils::H256>,
				) -> ::subxt::runtime_api::Payload<
					types::Requests,
					::std::vec::Vec<runtime_types::ismp::router::Request>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"requests",
						types::Requests { request_commitments },
						[
							241u8, 12u8, 67u8, 122u8, 104u8, 91u8, 182u8, 225u8, 151u8, 222u8,
							59u8, 119u8, 211u8, 216u8, 32u8, 180u8, 171u8, 37u8, 4u8, 103u8, 209u8,
							16u8, 0u8, 234u8, 115u8, 251u8, 113u8, 105u8, 81u8, 205u8, 243u8, 30u8,
						],
					)
				}
				#[doc = " Fetch the responses for the given commitments."]
				pub fn responses(
					&self,
					response_commitments: ::std::vec::Vec<::subxt::utils::H256>,
				) -> ::subxt::runtime_api::Payload<
					types::Responses,
					::std::vec::Vec<runtime_types::ismp::router::Response>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpRuntimeApi",
						"responses",
						types::Responses { response_commitments },
						[
							15u8, 124u8, 147u8, 173u8, 119u8, 27u8, 148u8, 229u8, 110u8, 88u8,
							236u8, 126u8, 29u8, 187u8, 220u8, 183u8, 134u8, 118u8, 117u8, 111u8,
							54u8, 30u8, 222u8, 96u8, 113u8, 70u8, 207u8, 121u8, 147u8, 252u8, 28u8,
							239u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct HostStateMachine {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GenerateProof {
					pub commitments: runtime_types::pallet_ismp::mmr::ProofKeys,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockEvents {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockEventsWithMetadata {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ConsensusState {
					pub id: [::core::primitive::u8; 4usize],
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ConsensusUpdateTime {
					pub id: [::core::primitive::u8; 4usize],
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ChallengePeriod {
					pub id: [::core::primitive::u8; 4usize],
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct LatestStateMachineHeight {
					pub id: runtime_types::ismp::consensus::StateMachineId,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Requests {
					pub request_commitments: ::std::vec::Vec<::subxt::utils::H256>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Responses {
					pub response_commitments: ::std::vec::Vec<::subxt::utils::H256>,
				}
			}
		}
		pub mod ismp_parachain_api {
			use super::{root_mod, runtime_types};
			#[doc = " Ismp Parachain consensus client runtime APIs"]
			pub struct IsmpParachainApi;
			impl IsmpParachainApi {
				#[doc = " Return all the para_ids this runtime is interested in. Used by the inherent provider"]
				pub fn para_ids(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::ParaIds,
					::std::vec::Vec<::core::primitive::u32>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpParachainApi",
						"para_ids",
						types::ParaIds {},
						[
							227u8, 236u8, 2u8, 42u8, 16u8, 132u8, 78u8, 252u8, 213u8, 139u8, 173u8,
							152u8, 215u8, 94u8, 246u8, 41u8, 72u8, 191u8, 86u8, 118u8, 155u8,
							205u8, 217u8, 98u8, 187u8, 142u8, 230u8, 38u8, 18u8, 3u8, 193u8, 181u8,
						],
					)
				}
				#[doc = " Return the current relay chain state."]
				pub fn current_relay_chain_state(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::CurrentRelayChainState,
					runtime_types::cumulus_pallet_parachain_system::RelayChainState,
				> {
					::subxt::runtime_api::Payload::new_static(
						"IsmpParachainApi",
						"current_relay_chain_state",
						types::CurrentRelayChainState {},
						[
							182u8, 121u8, 104u8, 187u8, 36u8, 61u8, 229u8, 11u8, 100u8, 188u8,
							126u8, 30u8, 190u8, 87u8, 113u8, 6u8, 158u8, 245u8, 127u8, 178u8, 70u8,
							132u8, 189u8, 136u8, 24u8, 155u8, 123u8, 102u8, 128u8, 57u8, 67u8,
							143u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ParaIds {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CurrentRelayChainState {}
			}
		}
		pub mod collect_collation_info {
			use super::{root_mod, runtime_types};
			#[doc = " Runtime api to collect information about a collation."]
			pub struct CollectCollationInfo;
			impl CollectCollationInfo {
				#[doc = " Collect information about a collation."]
				#[doc = ""]
				#[doc = " The given `header` is the header of the built block for that"]
				#[doc = " we are collecting the collation info for."]
				pub fn collect_collation_info(
					&self,
					header: runtime_types::sp_runtime::generic::header::Header<
						::core::primitive::u32,
					>,
				) -> ::subxt::runtime_api::Payload<
					types::CollectCollationInfo,
					runtime_types::cumulus_primitives_core::CollationInfo,
				> {
					::subxt::runtime_api::Payload::new_static(
						"CollectCollationInfo",
						"collect_collation_info",
						types::CollectCollationInfo { header },
						[
							56u8, 138u8, 105u8, 91u8, 216u8, 40u8, 255u8, 98u8, 86u8, 138u8, 185u8,
							155u8, 80u8, 141u8, 85u8, 48u8, 252u8, 235u8, 178u8, 231u8, 111u8,
							216u8, 71u8, 20u8, 33u8, 202u8, 24u8, 215u8, 214u8, 132u8, 51u8, 166u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CollectCollationInfo {
					pub header:
						runtime_types::sp_runtime::generic::header::Header<::core::primitive::u32>,
				}
			}
		}
		pub mod genesis_builder {
			use super::{root_mod, runtime_types};
			#[doc = " API to interact with GenesisConfig for the runtime"]
			pub struct GenesisBuilder;
			impl GenesisBuilder {
				#[doc = " Creates the default `GenesisConfig` and returns it as a JSON blob."]
				#[doc = ""]
				#[doc = " This function instantiates the default `GenesisConfig` struct for the runtime and serializes it into a JSON"]
				#[doc = " blob. It returns a `Vec<u8>` containing the JSON representation of the default `GenesisConfig`."]
				pub fn create_default_config(
					&self,
				) -> ::subxt::runtime_api::Payload<
					types::CreateDefaultConfig,
					::std::vec::Vec<::core::primitive::u8>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"create_default_config",
						types::CreateDefaultConfig {},
						[
							238u8, 5u8, 139u8, 81u8, 184u8, 155u8, 221u8, 118u8, 190u8, 76u8,
							229u8, 67u8, 132u8, 89u8, 83u8, 80u8, 56u8, 171u8, 169u8, 64u8, 123u8,
							20u8, 129u8, 159u8, 28u8, 135u8, 84u8, 52u8, 192u8, 98u8, 104u8, 214u8,
						],
					)
				}
				#[doc = " Build `GenesisConfig` from a JSON blob not using any defaults and store it in the storage."]
				#[doc = ""]
				#[doc = " This function deserializes the full `GenesisConfig` from the given JSON blob and puts it into the storage."]
				#[doc = " If the provided JSON blob is incorrect or incomplete or the deserialization fails, an error is returned."]
				#[doc = " It is recommended to log any errors encountered during the process."]
				#[doc = ""]
				#[doc = " Please note that provided json blob must contain all `GenesisConfig` fields, no defaults will be used."]
				pub fn build_config(
					&self,
					json: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::runtime_api::Payload<
					types::BuildConfig,
					::core::result::Result<(), ::std::string::String>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"GenesisBuilder",
						"build_config",
						types::BuildConfig { json },
						[
							6u8, 98u8, 68u8, 125u8, 157u8, 26u8, 107u8, 86u8, 213u8, 227u8, 26u8,
							229u8, 122u8, 161u8, 229u8, 114u8, 123u8, 192u8, 66u8, 231u8, 148u8,
							175u8, 5u8, 185u8, 248u8, 88u8, 40u8, 122u8, 230u8, 209u8, 170u8,
							254u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateDefaultConfig {}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BuildConfig {
					pub json: ::std::vec::Vec<::core::primitive::u8>,
				}
			}
		}
		pub mod create_transaction_api {
			use super::{root_mod, runtime_types};
			#[doc = " Create transaction."]
			#[doc = " This trait is meant to be implemented by the runtime and is responsible for constructing"]
			#[doc = " a transaction to be included in the block."]
			pub struct CreateTransactionApi;
			impl CreateTransactionApi {
				#[doc = " Attempt to create signed transaction"]
				#[doc = " Runtime implementation is free to construct the payload to sign in any way it wants."]
				#[doc = " Returns a scale encoded extrinsic Should panic if signed transaction cannot be created."]
				pub fn create_transaction(
					&self,
					account: ::subxt::utils::AccountId32,
					call: runtime_types::gargantua_runtime::RuntimeCall,
				) -> ::subxt::runtime_api::Payload<
					types::CreateTransaction,
					::std::vec::Vec<::core::primitive::u8>,
				> {
					::subxt::runtime_api::Payload::new_static(
						"CreateTransactionApi",
						"create_transaction",
						types::CreateTransaction { account, call },
						[
							138u8, 178u8, 65u8, 241u8, 195u8, 251u8, 188u8, 77u8, 145u8, 214u8,
							108u8, 10u8, 250u8, 124u8, 181u8, 9u8, 44u8, 226u8, 113u8, 131u8,
							109u8, 147u8, 144u8, 39u8, 178u8, 159u8, 189u8, 17u8, 79u8, 97u8, 14u8,
							203u8,
						],
					)
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateTransaction {
					pub account: ::subxt::utils::AccountId32,
					pub call: runtime_types::gargantua_runtime::RuntimeCall,
				}
			}
		}
	}
	pub struct ConstantsApi;
	impl ConstantsApi {
		pub fn system(&self) -> system::constants::ConstantsApi {
			system::constants::ConstantsApi
		}
		pub fn timestamp(&self) -> timestamp::constants::ConstantsApi {
			timestamp::constants::ConstantsApi
		}
		pub fn utility(&self) -> utility::constants::ConstantsApi {
			utility::constants::ConstantsApi
		}
		pub fn balances(&self) -> balances::constants::ConstantsApi {
			balances::constants::ConstantsApi
		}
		pub fn transaction_payment(&self) -> transaction_payment::constants::ConstantsApi {
			transaction_payment::constants::ConstantsApi
		}
		pub fn xcmp_queue(&self) -> xcmp_queue::constants::ConstantsApi {
			xcmp_queue::constants::ConstantsApi
		}
		pub fn message_queue(&self) -> message_queue::constants::ConstantsApi {
			message_queue::constants::ConstantsApi
		}
		pub fn gateway(&self) -> gateway::constants::ConstantsApi {
			gateway::constants::ConstantsApi
		}
		pub fn assets(&self) -> assets::constants::ConstantsApi {
			assets::constants::ConstantsApi
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
		pub fn polkadot_xcm(&self) -> polkadot_xcm::storage::StorageApi {
			polkadot_xcm::storage::StorageApi
		}
		pub fn mmr(&self) -> mmr::storage::StorageApi {
			mmr::storage::StorageApi
		}
		pub fn ismp(&self) -> ismp::storage::StorageApi {
			ismp::storage::StorageApi
		}
		pub fn message_queue(&self) -> message_queue::storage::StorageApi {
			message_queue::storage::StorageApi
		}
		pub fn ismp_parachain(&self) -> ismp_parachain::storage::StorageApi {
			ismp_parachain::storage::StorageApi
		}
		pub fn relayer(&self) -> relayer::storage::StorageApi {
			relayer::storage::StorageApi
		}
		pub fn host_executive(&self) -> host_executive::storage::StorageApi {
			host_executive::storage::StorageApi
		}
		pub fn gateway(&self) -> gateway::storage::StorageApi {
			gateway::storage::StorageApi
		}
		pub fn assets(&self) -> assets::storage::StorageApi {
			assets::storage::StorageApi
		}
		pub fn token_governor(&self) -> token_governor::storage::StorageApi {
			token_governor::storage::StorageApi
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
		pub fn parachain_info(&self) -> parachain_info::calls::TransactionApi {
			parachain_info::calls::TransactionApi
		}
		pub fn utility(&self) -> utility::calls::TransactionApi {
			utility::calls::TransactionApi
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
		pub fn ismp(&self) -> ismp::calls::TransactionApi {
			ismp::calls::TransactionApi
		}
		pub fn message_queue(&self) -> message_queue::calls::TransactionApi {
			message_queue::calls::TransactionApi
		}
		pub fn ismp_parachain(&self) -> ismp_parachain::calls::TransactionApi {
			ismp_parachain::calls::TransactionApi
		}
		pub fn ismp_sync_committee(&self) -> ismp_sync_committee::calls::TransactionApi {
			ismp_sync_committee::calls::TransactionApi
		}
		pub fn ismp_demo(&self) -> ismp_demo::calls::TransactionApi {
			ismp_demo::calls::TransactionApi
		}
		pub fn relayer(&self) -> relayer::calls::TransactionApi {
			relayer::calls::TransactionApi
		}
		pub fn host_executive(&self) -> host_executive::calls::TransactionApi {
			host_executive::calls::TransactionApi
		}
		pub fn call_decompressor(&self) -> call_decompressor::calls::TransactionApi {
			call_decompressor::calls::TransactionApi
		}
		pub fn gateway(&self) -> gateway::calls::TransactionApi {
			gateway::calls::TransactionApi
		}
		pub fn assets(&self) -> assets::calls::TransactionApi {
			assets::calls::TransactionApi
		}
		pub fn token_governor(&self) -> token_governor::calls::TransactionApi {
			token_governor::calls::TransactionApi
		}
	}
	#[doc = r" check whether the metadata provided is aligned with this statically generated code."]
	pub fn is_codegen_valid_for(metadata: &::subxt::Metadata) -> bool {
		let runtime_metadata_hash = metadata
			.hasher()
			.only_these_pallets(&PALLETS)
			.only_these_runtime_apis(&RUNTIME_APIS)
			.hash();
		runtime_metadata_hash ==
			[
				192u8, 136u8, 61u8, 78u8, 100u8, 173u8, 111u8, 148u8, 186u8, 99u8, 70u8, 209u8,
				207u8, 189u8, 81u8, 16u8, 14u8, 222u8, 119u8, 206u8, 144u8, 140u8, 99u8, 122u8,
				146u8, 19u8, 190u8, 59u8, 162u8, 177u8, 149u8, 128u8,
			]
	}
	pub mod system {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the System pallet"]
		pub type Error = runtime_types::frame_system::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::frame_system::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Remark {
					pub remark: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Remark {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetHeapPages {
					pub pages: ::core::primitive::u64,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetHeapPages {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_heap_pages";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetCode {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetCode {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_code";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetCodeWithoutChecks {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetCodeWithoutChecks {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_code_without_checks";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetStorage {
					pub items: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::std::vec::Vec<::core::primitive::u8>,
					)>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetStorage {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "set_storage";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KillStorage {
					pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				}
				impl ::subxt::blocks::StaticExtrinsic for KillStorage {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "kill_storage";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KillPrefix {
					pub prefix: ::std::vec::Vec<::core::primitive::u8>,
					pub subkeys: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for KillPrefix {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "kill_prefix";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemarkWithEvent {
					pub remark: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for RemarkWithEvent {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "remark_with_event";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AuthorizeUpgrade {
					pub code_hash: ::subxt::utils::H256,
				}
				impl ::subxt::blocks::StaticExtrinsic for AuthorizeUpgrade {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "authorize_upgrade";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AuthorizeUpgradeWithoutChecks {
					pub code_hash: ::subxt::utils::H256,
				}
				impl ::subxt::blocks::StaticExtrinsic for AuthorizeUpgradeWithoutChecks {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "authorize_upgrade_without_checks";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ApplyAuthorizedUpgrade {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ApplyAuthorizedUpgrade {
					const PALLET: &'static str = "System";
					const CALL: &'static str = "apply_authorized_upgrade";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::remark`]."]
				pub fn remark(
					&self,
					remark: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::Remark> {
					::subxt::tx::Payload::new_static(
						"System",
						"remark",
						types::Remark { remark },
						[
							43u8, 126u8, 180u8, 174u8, 141u8, 48u8, 52u8, 125u8, 166u8, 212u8,
							216u8, 98u8, 100u8, 24u8, 132u8, 71u8, 101u8, 64u8, 246u8, 169u8, 33u8,
							250u8, 147u8, 208u8, 2u8, 40u8, 129u8, 209u8, 232u8, 207u8, 207u8,
							13u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_heap_pages`]."]
				pub fn set_heap_pages(
					&self,
					pages: ::core::primitive::u64,
				) -> ::subxt::tx::Payload<types::SetHeapPages> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_heap_pages",
						types::SetHeapPages { pages },
						[
							188u8, 191u8, 99u8, 216u8, 219u8, 109u8, 141u8, 50u8, 78u8, 235u8,
							215u8, 242u8, 195u8, 24u8, 111u8, 76u8, 229u8, 64u8, 99u8, 225u8,
							134u8, 121u8, 81u8, 209u8, 127u8, 223u8, 98u8, 215u8, 150u8, 70u8,
							57u8, 147u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_code`]."]
				pub fn set_code(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetCode> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_code",
						types::SetCode { code },
						[
							233u8, 248u8, 88u8, 245u8, 28u8, 65u8, 25u8, 169u8, 35u8, 237u8, 19u8,
							203u8, 136u8, 160u8, 18u8, 3u8, 20u8, 197u8, 81u8, 169u8, 244u8, 188u8,
							27u8, 147u8, 147u8, 236u8, 65u8, 25u8, 3u8, 143u8, 182u8, 22u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_code_without_checks`]."]
				pub fn set_code_without_checks(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetCodeWithoutChecks> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_code_without_checks",
						types::SetCodeWithoutChecks { code },
						[
							82u8, 212u8, 157u8, 44u8, 70u8, 0u8, 143u8, 15u8, 109u8, 109u8, 107u8,
							157u8, 141u8, 42u8, 169u8, 11u8, 15u8, 186u8, 252u8, 138u8, 10u8,
							147u8, 15u8, 178u8, 247u8, 229u8, 213u8, 98u8, 207u8, 231u8, 119u8,
							115u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_storage`]."]
				pub fn set_storage(
					&self,
					items: ::std::vec::Vec<(
						::std::vec::Vec<::core::primitive::u8>,
						::std::vec::Vec<::core::primitive::u8>,
					)>,
				) -> ::subxt::tx::Payload<types::SetStorage> {
					::subxt::tx::Payload::new_static(
						"System",
						"set_storage",
						types::SetStorage { items },
						[
							141u8, 216u8, 52u8, 222u8, 223u8, 136u8, 123u8, 181u8, 19u8, 75u8,
							163u8, 102u8, 229u8, 189u8, 158u8, 142u8, 95u8, 235u8, 240u8, 49u8,
							150u8, 76u8, 78u8, 137u8, 126u8, 88u8, 183u8, 88u8, 231u8, 146u8,
							234u8, 43u8,
						],
					)
				}
				#[doc = "See [`Pallet::kill_storage`]."]
				pub fn kill_storage(
					&self,
					keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				) -> ::subxt::tx::Payload<types::KillStorage> {
					::subxt::tx::Payload::new_static(
						"System",
						"kill_storage",
						types::KillStorage { keys },
						[
							73u8, 63u8, 196u8, 36u8, 144u8, 114u8, 34u8, 213u8, 108u8, 93u8, 209u8,
							234u8, 153u8, 185u8, 33u8, 91u8, 187u8, 195u8, 223u8, 130u8, 58u8,
							156u8, 63u8, 47u8, 228u8, 249u8, 216u8, 139u8, 143u8, 177u8, 41u8,
							35u8,
						],
					)
				}
				#[doc = "See [`Pallet::kill_prefix`]."]
				pub fn kill_prefix(
					&self,
					prefix: ::std::vec::Vec<::core::primitive::u8>,
					subkeys: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::KillPrefix> {
					::subxt::tx::Payload::new_static(
						"System",
						"kill_prefix",
						types::KillPrefix { prefix, subkeys },
						[
							184u8, 57u8, 139u8, 24u8, 208u8, 87u8, 108u8, 215u8, 198u8, 189u8,
							175u8, 242u8, 167u8, 215u8, 97u8, 63u8, 110u8, 166u8, 238u8, 98u8,
							67u8, 236u8, 111u8, 110u8, 234u8, 81u8, 102u8, 5u8, 182u8, 5u8, 214u8,
							85u8,
						],
					)
				}
				#[doc = "See [`Pallet::remark_with_event`]."]
				pub fn remark_with_event(
					&self,
					remark: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::RemarkWithEvent> {
					::subxt::tx::Payload::new_static(
						"System",
						"remark_with_event",
						types::RemarkWithEvent { remark },
						[
							120u8, 120u8, 153u8, 92u8, 184u8, 85u8, 34u8, 2u8, 174u8, 206u8, 105u8,
							228u8, 233u8, 130u8, 80u8, 246u8, 228u8, 59u8, 234u8, 240u8, 4u8, 49u8,
							147u8, 170u8, 115u8, 91u8, 149u8, 200u8, 228u8, 181u8, 8u8, 154u8,
						],
					)
				}
				#[doc = "See [`Pallet::authorize_upgrade`]."]
				pub fn authorize_upgrade(
					&self,
					code_hash: ::subxt::utils::H256,
				) -> ::subxt::tx::Payload<types::AuthorizeUpgrade> {
					::subxt::tx::Payload::new_static(
						"System",
						"authorize_upgrade",
						types::AuthorizeUpgrade { code_hash },
						[
							4u8, 14u8, 76u8, 107u8, 209u8, 129u8, 9u8, 39u8, 193u8, 17u8, 84u8,
							254u8, 170u8, 214u8, 24u8, 155u8, 29u8, 184u8, 249u8, 241u8, 109u8,
							58u8, 145u8, 131u8, 109u8, 63u8, 38u8, 165u8, 107u8, 215u8, 217u8,
							172u8,
						],
					)
				}
				#[doc = "See [`Pallet::authorize_upgrade_without_checks`]."]
				pub fn authorize_upgrade_without_checks(
					&self,
					code_hash: ::subxt::utils::H256,
				) -> ::subxt::tx::Payload<types::AuthorizeUpgradeWithoutChecks> {
					::subxt::tx::Payload::new_static(
						"System",
						"authorize_upgrade_without_checks",
						types::AuthorizeUpgradeWithoutChecks { code_hash },
						[
							126u8, 126u8, 55u8, 26u8, 47u8, 55u8, 66u8, 8u8, 167u8, 18u8, 29u8,
							136u8, 146u8, 14u8, 189u8, 117u8, 16u8, 227u8, 162u8, 61u8, 149u8,
							197u8, 104u8, 184u8, 185u8, 161u8, 99u8, 154u8, 80u8, 125u8, 181u8,
							233u8,
						],
					)
				}
				#[doc = "See [`Pallet::apply_authorized_upgrade`]."]
				pub fn apply_authorized_upgrade(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::ApplyAuthorizedUpgrade> {
					::subxt::tx::Payload::new_static(
						"System",
						"apply_authorized_upgrade",
						types::ApplyAuthorizedUpgrade { code },
						[
							232u8, 107u8, 127u8, 38u8, 230u8, 29u8, 97u8, 4u8, 160u8, 191u8, 222u8,
							156u8, 245u8, 102u8, 196u8, 141u8, 44u8, 163u8, 98u8, 68u8, 125u8,
							32u8, 124u8, 101u8, 108u8, 93u8, 211u8, 52u8, 0u8, 231u8, 33u8, 227u8,
						],
					)
				}
			}
		}
		#[doc = "Event for the System pallet."]
		pub type Event = runtime_types::frame_system::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An extrinsic completed successfully."]
			pub struct ExtrinsicSuccess {
				pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
			}
			impl ::subxt::events::StaticEvent for ExtrinsicSuccess {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "ExtrinsicSuccess";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An extrinsic failed."]
			pub struct ExtrinsicFailed {
				pub dispatch_error: runtime_types::sp_runtime::DispatchError,
				pub dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
			}
			impl ::subxt::events::StaticEvent for ExtrinsicFailed {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "ExtrinsicFailed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "`:code` was updated."]
			pub struct CodeUpdated;
			impl ::subxt::events::StaticEvent for CodeUpdated {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "CodeUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new account was created."]
			pub struct NewAccount {
				pub account: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for NewAccount {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "NewAccount";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was reaped."]
			pub struct KilledAccount {
				pub account: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for KilledAccount {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "KilledAccount";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "On on-chain remark happened."]
			pub struct Remarked {
				pub sender: ::subxt::utils::AccountId32,
				pub hash: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for Remarked {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "Remarked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An upgrade was authorized."]
			pub struct UpgradeAuthorized {
				pub code_hash: ::subxt::utils::H256,
				pub check_version: ::core::primitive::bool,
			}
			impl ::subxt::events::StaticEvent for UpgradeAuthorized {
				const PALLET: &'static str = "System";
				const EVENT: &'static str = "UpgradeAuthorized";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The full account information for a particular account ID."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::AccountInfo<
						::core::primitive::u32,
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
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
							14u8, 233u8, 115u8, 214u8, 0u8, 109u8, 222u8, 121u8, 162u8, 65u8, 60u8,
							175u8, 209u8, 79u8, 222u8, 124u8, 22u8, 235u8, 138u8, 176u8, 133u8,
							124u8, 90u8, 158u8, 85u8, 45u8, 37u8, 174u8, 47u8, 79u8, 47u8, 166u8,
						],
					)
				}
				#[doc = " The full account information for a particular account ID."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::AccountInfo<
						::core::primitive::u32,
						runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
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
							14u8, 233u8, 115u8, 214u8, 0u8, 109u8, 222u8, 121u8, 162u8, 65u8, 60u8,
							175u8, 209u8, 79u8, 222u8, 124u8, 22u8, 235u8, 138u8, 176u8, 133u8,
							124u8, 90u8, 158u8, 85u8, 45u8, 37u8, 174u8, 47u8, 79u8, 47u8, 166u8,
						],
					)
				}
				#[doc = " Total extrinsics count for the current block."]
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
							102u8, 76u8, 236u8, 42u8, 40u8, 231u8, 33u8, 222u8, 123u8, 147u8,
							153u8, 148u8, 234u8, 203u8, 181u8, 119u8, 6u8, 187u8, 177u8, 199u8,
							120u8, 47u8, 137u8, 254u8, 96u8, 100u8, 165u8, 182u8, 249u8, 230u8,
							159u8, 79u8,
						],
					)
				}
				#[doc = " The current weight for the block."]
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
							158u8, 46u8, 228u8, 89u8, 210u8, 214u8, 84u8, 154u8, 50u8, 68u8, 63u8,
							62u8, 43u8, 42u8, 99u8, 27u8, 54u8, 42u8, 146u8, 44u8, 241u8, 216u8,
							229u8, 30u8, 216u8, 255u8, 165u8, 238u8, 181u8, 130u8, 36u8, 102u8,
						],
					)
				}
				#[doc = " Total length (in bytes) for all extrinsics put together, for the current block."]
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
							117u8, 86u8, 61u8, 243u8, 41u8, 51u8, 102u8, 214u8, 137u8, 100u8,
							243u8, 185u8, 122u8, 174u8, 187u8, 117u8, 86u8, 189u8, 63u8, 135u8,
							101u8, 218u8, 203u8, 201u8, 237u8, 254u8, 128u8, 183u8, 169u8, 221u8,
							242u8, 65u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
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
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Map of block numbers to block hashes."]
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
							217u8, 32u8, 215u8, 253u8, 24u8, 182u8, 207u8, 178u8, 157u8, 24u8,
							103u8, 100u8, 195u8, 165u8, 69u8, 152u8, 112u8, 181u8, 56u8, 192u8,
							164u8, 16u8, 20u8, 222u8, 28u8, 214u8, 144u8, 142u8, 146u8, 69u8,
							202u8, 118u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
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
							160u8, 180u8, 122u8, 18u8, 196u8, 26u8, 2u8, 37u8, 115u8, 232u8, 133u8,
							220u8, 106u8, 245u8, 4u8, 129u8, 42u8, 84u8, 241u8, 45u8, 199u8, 179u8,
							128u8, 61u8, 170u8, 137u8, 231u8, 156u8, 247u8, 57u8, 47u8, 38u8,
						],
					)
				}
				#[doc = " Extrinsics data for the current block (maps an extrinsic's index to its data)."]
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
							160u8, 180u8, 122u8, 18u8, 196u8, 26u8, 2u8, 37u8, 115u8, 232u8, 133u8,
							220u8, 106u8, 245u8, 4u8, 129u8, 42u8, 84u8, 241u8, 45u8, 199u8, 179u8,
							128u8, 61u8, 170u8, 137u8, 231u8, 156u8, 247u8, 57u8, 47u8, 38u8,
						],
					)
				}
				#[doc = " The current block number being processed. Set by `execute_block`."]
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
							30u8, 194u8, 177u8, 90u8, 194u8, 232u8, 46u8, 180u8, 85u8, 129u8, 14u8,
							9u8, 8u8, 8u8, 23u8, 95u8, 230u8, 5u8, 13u8, 105u8, 125u8, 2u8, 22u8,
							200u8, 78u8, 93u8, 115u8, 28u8, 150u8, 113u8, 48u8, 53u8,
						],
					)
				}
				#[doc = " Hash of the previous block."]
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
							26u8, 130u8, 11u8, 216u8, 155u8, 71u8, 128u8, 170u8, 30u8, 153u8, 21u8,
							192u8, 62u8, 93u8, 137u8, 80u8, 120u8, 81u8, 202u8, 94u8, 248u8, 125u8,
							71u8, 82u8, 141u8, 229u8, 32u8, 56u8, 73u8, 50u8, 101u8, 78u8,
						],
					)
				}
				#[doc = " Digest of the current block, also part of the block header."]
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
							61u8, 64u8, 237u8, 91u8, 145u8, 232u8, 17u8, 254u8, 181u8, 16u8, 234u8,
							91u8, 51u8, 140u8, 254u8, 131u8, 98u8, 135u8, 21u8, 37u8, 251u8, 20u8,
							58u8, 92u8, 123u8, 141u8, 14u8, 227u8, 146u8, 46u8, 222u8, 117u8,
						],
					)
				}
				#[doc = " Events deposited for the current block."]
				#[doc = ""]
				#[doc = " NOTE: The item is unbound and should therefore never be read on chain."]
				#[doc = " It could otherwise inflate the PoV size of a block."]
				#[doc = ""]
				#[doc = " Events have a large in-memory size. Box the events to not go out-of-memory"]
				#[doc = " just in case someone still reads them from within the runtime."]
				pub fn events(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<
						runtime_types::frame_system::EventRecord<
							runtime_types::gargantua_runtime::RuntimeEvent,
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
							171u8, 90u8, 196u8, 103u8, 250u8, 229u8, 76u8, 85u8, 41u8, 111u8,
							229u8, 196u8, 7u8, 128u8, 176u8, 143u8, 182u8, 15u8, 217u8, 170u8,
							128u8, 221u8, 100u8, 50u8, 227u8, 239u8, 118u8, 84u8, 112u8, 186u8,
							56u8, 26u8,
						],
					)
				}
				#[doc = " The number of events in the `Events<T>` list."]
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
							175u8, 24u8, 252u8, 184u8, 210u8, 167u8, 146u8, 143u8, 164u8, 80u8,
							151u8, 205u8, 189u8, 189u8, 55u8, 220u8, 47u8, 101u8, 181u8, 33u8,
							254u8, 131u8, 13u8, 143u8, 3u8, 244u8, 245u8, 45u8, 2u8, 210u8, 79u8,
							133u8,
						],
					)
				}
				#[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
				#[doc = " of events in the `<Events<T>>` list."]
				#[doc = ""]
				#[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
				#[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
				#[doc = " in case of changes fetch the list of events of interest."]
				#[doc = ""]
				#[doc = " The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just"]
				#[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
				#[doc = " no notification will be triggered thus the event might be lost."]
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
							40u8, 225u8, 14u8, 75u8, 44u8, 176u8, 76u8, 34u8, 143u8, 107u8, 69u8,
							133u8, 114u8, 13u8, 172u8, 250u8, 141u8, 73u8, 12u8, 65u8, 217u8, 63u8,
							120u8, 241u8, 48u8, 106u8, 143u8, 161u8, 128u8, 100u8, 166u8, 59u8,
						],
					)
				}
				#[doc = " Mapping between a topic (represented by T::Hash) and a vector of indexes"]
				#[doc = " of events in the `<Events<T>>` list."]
				#[doc = ""]
				#[doc = " All topic vectors have deterministic storage locations depending on the topic. This"]
				#[doc = " allows light-clients to leverage the changes trie storage tracking mechanism and"]
				#[doc = " in case of changes fetch the list of events of interest."]
				#[doc = ""]
				#[doc = " The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just"]
				#[doc = " the `EventIndex` then in case if the topic has the same contents on the next block"]
				#[doc = " no notification will be triggered thus the event might be lost."]
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
							40u8, 225u8, 14u8, 75u8, 44u8, 176u8, 76u8, 34u8, 143u8, 107u8, 69u8,
							133u8, 114u8, 13u8, 172u8, 250u8, 141u8, 73u8, 12u8, 65u8, 217u8, 63u8,
							120u8, 241u8, 48u8, 106u8, 143u8, 161u8, 128u8, 100u8, 166u8, 59u8,
						],
					)
				}
				#[doc = " Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened."]
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
							137u8, 29u8, 175u8, 75u8, 197u8, 208u8, 91u8, 207u8, 156u8, 87u8,
							148u8, 68u8, 91u8, 140u8, 22u8, 233u8, 1u8, 229u8, 56u8, 34u8, 40u8,
							194u8, 253u8, 30u8, 163u8, 39u8, 54u8, 209u8, 13u8, 27u8, 139u8, 184u8,
						],
					)
				}
				#[doc = " True if we have upgraded so that `type RefCount` is `u32`. False (default) if not."]
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
							229u8, 73u8, 9u8, 132u8, 186u8, 116u8, 151u8, 171u8, 145u8, 29u8, 34u8,
							130u8, 52u8, 146u8, 124u8, 175u8, 79u8, 189u8, 147u8, 230u8, 234u8,
							107u8, 124u8, 31u8, 2u8, 22u8, 86u8, 190u8, 4u8, 147u8, 50u8, 245u8,
						],
					)
				}
				#[doc = " True if we have upgraded so that AccountInfo contains three types of `RefCount`. False"]
				#[doc = " (default) if not."]
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
							97u8, 66u8, 124u8, 243u8, 27u8, 167u8, 147u8, 81u8, 254u8, 201u8,
							101u8, 24u8, 40u8, 231u8, 14u8, 179u8, 154u8, 163u8, 71u8, 81u8, 185u8,
							167u8, 82u8, 254u8, 189u8, 3u8, 101u8, 207u8, 206u8, 194u8, 155u8,
							151u8,
						],
					)
				}
				#[doc = " The execution phase of the block."]
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
							191u8, 129u8, 100u8, 134u8, 126u8, 116u8, 154u8, 203u8, 220u8, 200u8,
							0u8, 26u8, 161u8, 250u8, 133u8, 205u8, 146u8, 24u8, 5u8, 156u8, 158u8,
							35u8, 36u8, 253u8, 52u8, 235u8, 86u8, 167u8, 35u8, 100u8, 119u8, 27u8,
						],
					)
				}
				#[doc = " `Some` if a code upgrade has been authorized."]
				pub fn authorized_upgrade(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::frame_system::CodeUpgradeAuthorization,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"System",
						"AuthorizedUpgrade",
						vec![],
						[
							165u8, 97u8, 27u8, 138u8, 2u8, 28u8, 55u8, 92u8, 96u8, 96u8, 168u8,
							169u8, 55u8, 178u8, 44u8, 127u8, 58u8, 140u8, 206u8, 178u8, 1u8, 37u8,
							214u8, 213u8, 251u8, 123u8, 5u8, 111u8, 90u8, 148u8, 217u8, 135u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Block & extrinsics weights: base values and limits."]
				pub fn block_weights(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockWeights>
				{
					::subxt::constants::Address::new_static(
						"System",
						"BlockWeights",
						[
							176u8, 124u8, 225u8, 136u8, 25u8, 73u8, 247u8, 33u8, 82u8, 206u8, 85u8,
							190u8, 127u8, 102u8, 71u8, 11u8, 185u8, 8u8, 58u8, 0u8, 94u8, 55u8,
							163u8, 177u8, 104u8, 59u8, 60u8, 136u8, 246u8, 116u8, 0u8, 239u8,
						],
					)
				}
				#[doc = " The maximum length of a block (in bytes)."]
				pub fn block_length(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_system::limits::BlockLength>
				{
					::subxt::constants::Address::new_static(
						"System",
						"BlockLength",
						[
							23u8, 242u8, 225u8, 39u8, 225u8, 67u8, 152u8, 41u8, 155u8, 104u8, 68u8,
							229u8, 185u8, 133u8, 10u8, 143u8, 184u8, 152u8, 234u8, 44u8, 140u8,
							96u8, 166u8, 235u8, 162u8, 160u8, 72u8, 7u8, 35u8, 194u8, 3u8, 37u8,
						],
					)
				}
				#[doc = " Maximum number of block number to block hash mappings to keep (oldest pruned first)."]
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
				#[doc = " The weight of runtime database operations the runtime can invoke."]
				pub fn db_weight(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_weights::RuntimeDbWeight> {
					::subxt::constants::Address::new_static(
						"System",
						"DbWeight",
						[
							42u8, 43u8, 178u8, 142u8, 243u8, 203u8, 60u8, 173u8, 118u8, 111u8,
							200u8, 170u8, 102u8, 70u8, 237u8, 187u8, 198u8, 120u8, 153u8, 232u8,
							183u8, 76u8, 74u8, 10u8, 70u8, 243u8, 14u8, 218u8, 213u8, 126u8, 29u8,
							177u8,
						],
					)
				}
				#[doc = " Get the chain's current version."]
				pub fn version(
					&self,
				) -> ::subxt::constants::Address<runtime_types::sp_version::RuntimeVersion> {
					::subxt::constants::Address::new_static(
						"System",
						"Version",
						[
							219u8, 45u8, 162u8, 245u8, 177u8, 246u8, 48u8, 126u8, 191u8, 157u8,
							228u8, 83u8, 111u8, 133u8, 183u8, 13u8, 148u8, 108u8, 92u8, 102u8,
							72u8, 205u8, 74u8, 242u8, 233u8, 79u8, 20u8, 170u8, 72u8, 202u8, 158u8,
							165u8,
						],
					)
				}
				#[doc = " The designated SS58 prefix of this chain."]
				#[doc = ""]
				#[doc = " This replaces the \"ss58Format\" property declared in the chain spec. Reason is"]
				#[doc = " that the runtime should know about the prefix in order to make use of it as"]
				#[doc = " an identifier of the chain."]
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
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_timestamp::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Set {
					#[codec(compact)]
					pub now: ::core::primitive::u64,
				}
				impl ::subxt::blocks::StaticExtrinsic for Set {
					const PALLET: &'static str = "Timestamp";
					const CALL: &'static str = "set";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set`]."]
				pub fn set(&self, now: ::core::primitive::u64) -> ::subxt::tx::Payload<types::Set> {
					::subxt::tx::Payload::new_static(
						"Timestamp",
						"set",
						types::Set { now },
						[
							37u8, 95u8, 49u8, 218u8, 24u8, 22u8, 0u8, 95u8, 72u8, 35u8, 155u8,
							199u8, 213u8, 54u8, 207u8, 22u8, 185u8, 193u8, 221u8, 70u8, 18u8,
							200u8, 4u8, 231u8, 195u8, 173u8, 6u8, 122u8, 11u8, 203u8, 231u8, 227u8,
						],
					)
				}
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The current time for the current block."]
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
							44u8, 50u8, 80u8, 30u8, 195u8, 146u8, 123u8, 238u8, 8u8, 163u8, 187u8,
							92u8, 61u8, 39u8, 51u8, 29u8, 173u8, 169u8, 217u8, 158u8, 85u8, 187u8,
							141u8, 26u8, 12u8, 115u8, 51u8, 11u8, 200u8, 244u8, 138u8, 152u8,
						],
					)
				}
				#[doc = " Whether the timestamp has been updated in this block."]
				#[doc = ""]
				#[doc = " This value is updated to `true` upon successful submission of a timestamp by a node."]
				#[doc = " It is then checked at the end of each block execution in the `on_finalize` hook."]
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
							229u8, 175u8, 246u8, 102u8, 237u8, 158u8, 212u8, 229u8, 238u8, 214u8,
							205u8, 160u8, 164u8, 252u8, 195u8, 75u8, 139u8, 110u8, 22u8, 34u8,
							248u8, 204u8, 107u8, 46u8, 20u8, 200u8, 238u8, 167u8, 71u8, 41u8,
							214u8, 140u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The minimum period between blocks."]
				#[doc = ""]
				#[doc = " Be aware that this is different to the *expected* period that the block production"]
				#[doc = " apparatus provides. Your chosen consensus system will generally work with this to"]
				#[doc = " determine a sensible block time. For example, in the Aura pallet it will be double this"]
				#[doc = " period on default settings."]
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
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::cumulus_pallet_parachain_system::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::cumulus_pallet_parachain_system::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetValidationData {
					pub data:
						runtime_types::cumulus_primitives_parachain_inherent::ParachainInherentData,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetValidationData {
					const PALLET: &'static str = "ParachainSystem";
					const CALL: &'static str = "set_validation_data";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SudoSendUpwardMessage {
					pub message: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SudoSendUpwardMessage {
					const PALLET: &'static str = "ParachainSystem";
					const CALL: &'static str = "sudo_send_upward_message";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AuthorizeUpgrade {
					pub code_hash: ::subxt::utils::H256,
					pub check_version: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for AuthorizeUpgrade {
					const PALLET: &'static str = "ParachainSystem";
					const CALL: &'static str = "authorize_upgrade";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct EnactAuthorizedUpgrade {
					pub code: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for EnactAuthorizedUpgrade {
					const PALLET: &'static str = "ParachainSystem";
					const CALL: &'static str = "enact_authorized_upgrade";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_validation_data`]."]
				pub fn set_validation_data(
					&self,
					data : runtime_types :: cumulus_primitives_parachain_inherent :: ParachainInherentData,
				) -> ::subxt::tx::Payload<types::SetValidationData> {
					::subxt::tx::Payload::new_static(
						"ParachainSystem",
						"set_validation_data",
						types::SetValidationData { data },
						[
							167u8, 126u8, 75u8, 137u8, 220u8, 60u8, 106u8, 214u8, 92u8, 170u8,
							136u8, 176u8, 98u8, 0u8, 234u8, 217u8, 146u8, 113u8, 149u8, 88u8,
							114u8, 141u8, 228u8, 105u8, 136u8, 71u8, 233u8, 18u8, 70u8, 36u8, 24u8,
							249u8,
						],
					)
				}
				#[doc = "See [`Pallet::sudo_send_upward_message`]."]
				pub fn sudo_send_upward_message(
					&self,
					message: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SudoSendUpwardMessage> {
					::subxt::tx::Payload::new_static(
						"ParachainSystem",
						"sudo_send_upward_message",
						types::SudoSendUpwardMessage { message },
						[
							1u8, 231u8, 11u8, 78u8, 127u8, 117u8, 248u8, 67u8, 230u8, 199u8, 126u8,
							47u8, 20u8, 62u8, 252u8, 138u8, 199u8, 48u8, 41u8, 21u8, 28u8, 157u8,
							218u8, 143u8, 4u8, 253u8, 62u8, 192u8, 94u8, 252u8, 92u8, 180u8,
						],
					)
				}
				#[doc = "See [`Pallet::authorize_upgrade`]."]
				pub fn authorize_upgrade(
					&self,
					code_hash: ::subxt::utils::H256,
					check_version: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::AuthorizeUpgrade> {
					::subxt::tx::Payload::new_static(
						"ParachainSystem",
						"authorize_upgrade",
						types::AuthorizeUpgrade { code_hash, check_version },
						[
							213u8, 114u8, 107u8, 169u8, 223u8, 147u8, 205u8, 204u8, 3u8, 81u8,
							228u8, 0u8, 82u8, 57u8, 43u8, 95u8, 12u8, 59u8, 241u8, 176u8, 143u8,
							131u8, 253u8, 166u8, 98u8, 187u8, 94u8, 235u8, 177u8, 110u8, 162u8,
							218u8,
						],
					)
				}
				#[doc = "See [`Pallet::enact_authorized_upgrade`]."]
				pub fn enact_authorized_upgrade(
					&self,
					code: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::EnactAuthorizedUpgrade> {
					::subxt::tx::Payload::new_static(
						"ParachainSystem",
						"enact_authorized_upgrade",
						types::EnactAuthorizedUpgrade { code },
						[
							232u8, 135u8, 114u8, 87u8, 196u8, 146u8, 244u8, 19u8, 106u8, 73u8,
							88u8, 193u8, 48u8, 14u8, 72u8, 133u8, 247u8, 147u8, 50u8, 95u8, 252u8,
							213u8, 192u8, 47u8, 244u8, 102u8, 195u8, 120u8, 179u8, 87u8, 94u8, 8u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::cumulus_pallet_parachain_system::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The validation function has been scheduled to apply."]
			pub struct ValidationFunctionStored;
			impl ::subxt::events::StaticEvent for ValidationFunctionStored {
				const PALLET: &'static str = "ParachainSystem";
				const EVENT: &'static str = "ValidationFunctionStored";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The validation function was applied as of the contained relay chain block number."]
			pub struct ValidationFunctionApplied {
				pub relay_chain_block_num: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for ValidationFunctionApplied {
				const PALLET: &'static str = "ParachainSystem";
				const EVENT: &'static str = "ValidationFunctionApplied";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The relay-chain aborted the upgrade process."]
			pub struct ValidationFunctionDiscarded;
			impl ::subxt::events::StaticEvent for ValidationFunctionDiscarded {
				const PALLET: &'static str = "ParachainSystem";
				const EVENT: &'static str = "ValidationFunctionDiscarded";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some downward messages have been received and will be processed."]
			pub struct DownwardMessagesReceived {
				pub count: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for DownwardMessagesReceived {
				const PALLET: &'static str = "ParachainSystem";
				const EVENT: &'static str = "DownwardMessagesReceived";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Downward messages were processed using the given weight."]
			pub struct DownwardMessagesProcessed {
				pub weight_used: runtime_types::sp_weights::weight_v2::Weight,
				pub dmq_head: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for DownwardMessagesProcessed {
				const PALLET: &'static str = "ParachainSystem";
				const EVENT: &'static str = "DownwardMessagesProcessed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An upward message was sent to the relay chain."]
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
				#[doc = " Latest included block descendants the runtime accepted. In other words, these are"]
				#[doc = " ancestors of the currently executing block which have not been included in the observed"]
				#[doc = " relay-chain state."]
				#[doc = ""]
				#[doc = " The segment length is limited by the capacity returned from the [`ConsensusHook`] configured"]
				#[doc = " in the pallet."]				pub fn unincluded_segment (& self ,) -> :: subxt :: storage :: address :: Address :: < :: subxt :: storage :: address :: StaticStorageMapKey , :: std :: vec :: Vec < runtime_types :: cumulus_pallet_parachain_system :: unincluded_segment :: Ancestor < :: subxt :: utils :: H256 > > , :: subxt :: storage :: address :: Yes , :: subxt :: storage :: address :: Yes , () >{
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"UnincludedSegment",
						vec![],
						[
							73u8, 83u8, 226u8, 16u8, 203u8, 233u8, 221u8, 109u8, 23u8, 114u8, 56u8,
							154u8, 100u8, 116u8, 253u8, 10u8, 164u8, 22u8, 110u8, 73u8, 245u8,
							226u8, 54u8, 146u8, 67u8, 109u8, 149u8, 142u8, 154u8, 218u8, 55u8,
							178u8,
						],
					)
				}
				#[doc = " Storage field that keeps track of bandwidth used by the unincluded segment along with the"]
				#[doc = " latest HRMP watermark. Used for limiting the acceptance of new blocks with"]
				#[doc = " respect to relay chain constraints."]				pub fn aggregated_unincluded_segment (& self ,) -> :: subxt :: storage :: address :: Address :: < :: subxt :: storage :: address :: StaticStorageMapKey , runtime_types :: cumulus_pallet_parachain_system :: unincluded_segment :: SegmentTracker < :: subxt :: utils :: H256 > , :: subxt :: storage :: address :: Yes , () , () >{
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"AggregatedUnincludedSegment",
						vec![],
						[
							165u8, 51u8, 182u8, 156u8, 65u8, 114u8, 167u8, 133u8, 245u8, 52u8,
							32u8, 119u8, 159u8, 65u8, 201u8, 108u8, 99u8, 43u8, 84u8, 63u8, 95u8,
							182u8, 134u8, 163u8, 51u8, 202u8, 243u8, 82u8, 225u8, 192u8, 186u8,
							2u8,
						],
					)
				}
				#[doc = " In case of a scheduled upgrade, this storage field contains the validation code to be"]
				#[doc = " applied."]
				#[doc = ""]
				#[doc = " As soon as the relay chain gives us the go-ahead signal, we will overwrite the"]
				#[doc = " [`:code`][sp_core::storage::well_known_keys::CODE] which will result the next block process"]
				#[doc = " with the new validation code. This concludes the upgrade process."]
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
							78u8, 159u8, 219u8, 211u8, 177u8, 80u8, 102u8, 93u8, 83u8, 146u8, 90u8,
							233u8, 232u8, 11u8, 104u8, 172u8, 93u8, 68u8, 44u8, 228u8, 99u8, 197u8,
							254u8, 28u8, 181u8, 215u8, 247u8, 238u8, 49u8, 49u8, 195u8, 249u8,
						],
					)
				}
				#[doc = " Validation code that is set by the parachain and is to be communicated to collator and"]
				#[doc = " consequently the relay-chain."]
				#[doc = ""]
				#[doc = " This will be cleared in `on_initialize` of each new block if no other pallet already set"]
				#[doc = " the value."]
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
							185u8, 123u8, 152u8, 122u8, 230u8, 136u8, 79u8, 73u8, 206u8, 19u8,
							59u8, 57u8, 75u8, 250u8, 83u8, 185u8, 29u8, 76u8, 89u8, 137u8, 77u8,
							163u8, 25u8, 125u8, 182u8, 67u8, 2u8, 180u8, 48u8, 237u8, 49u8, 171u8,
						],
					)
				}
				#[doc = " The [`PersistedValidationData`] set for this block."]
				#[doc = " This value is expected to be set only once per block and it's never stored"]
				#[doc = " in the trie."]
				pub fn validation_data(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::polkadot_primitives::v6::PersistedValidationData<
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
							193u8, 240u8, 25u8, 56u8, 103u8, 173u8, 56u8, 56u8, 229u8, 243u8, 91u8,
							25u8, 249u8, 95u8, 122u8, 93u8, 37u8, 181u8, 54u8, 244u8, 217u8, 200u8,
							62u8, 136u8, 80u8, 148u8, 16u8, 177u8, 124u8, 211u8, 95u8, 24u8,
						],
					)
				}
				#[doc = " Were the validation data set to notify the relay chain?"]
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
							233u8, 228u8, 48u8, 111u8, 200u8, 35u8, 30u8, 139u8, 251u8, 77u8,
							196u8, 252u8, 35u8, 222u8, 129u8, 235u8, 7u8, 19u8, 156u8, 82u8, 126u8,
							173u8, 29u8, 62u8, 20u8, 67u8, 166u8, 116u8, 108u8, 182u8, 57u8, 246u8,
						],
					)
				}
				#[doc = " The relay chain block number associated with the last parachain block."]
				#[doc = ""]
				#[doc = " This is updated in `on_finalize`."]
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
							17u8, 65u8, 131u8, 169u8, 195u8, 243u8, 195u8, 93u8, 220u8, 174u8,
							75u8, 216u8, 214u8, 227u8, 96u8, 40u8, 8u8, 153u8, 116u8, 160u8, 79u8,
							255u8, 35u8, 232u8, 242u8, 42u8, 100u8, 150u8, 208u8, 210u8, 142u8,
							186u8,
						],
					)
				}
				#[doc = " An option which indicates if the relay-chain restricts signalling a validation code upgrade."]
				#[doc = " In other words, if this is `Some` and [`NewValidationCode`] is `Some` then the produced"]
				#[doc = " candidate will be invalid."]
				#[doc = ""]
				#[doc = " This storage item is a mirror of the corresponding value for the current parachain from the"]
				#[doc = " relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is"]
				#[doc = " set after the inherent."]
				pub fn upgrade_restriction_signal(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::option::Option<
						runtime_types::polkadot_primitives::v6::UpgradeRestriction,
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
							235u8, 240u8, 37u8, 44u8, 181u8, 52u8, 7u8, 216u8, 20u8, 139u8, 69u8,
							124u8, 21u8, 173u8, 237u8, 64u8, 105u8, 88u8, 49u8, 69u8, 123u8, 55u8,
							181u8, 167u8, 112u8, 183u8, 190u8, 231u8, 231u8, 127u8, 77u8, 148u8,
						],
					)
				}
				#[doc = " Optional upgrade go-ahead signal from the relay-chain."]
				#[doc = ""]
				#[doc = " This storage item is a mirror of the corresponding value for the current parachain from the"]
				#[doc = " relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is"]
				#[doc = " set after the inherent."]
				pub fn upgrade_go_ahead(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::option::Option<runtime_types::polkadot_primitives::v6::UpgradeGoAhead>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"UpgradeGoAhead",
						vec![],
						[
							149u8, 144u8, 186u8, 88u8, 180u8, 34u8, 82u8, 226u8, 100u8, 148u8,
							246u8, 55u8, 233u8, 97u8, 43u8, 0u8, 48u8, 31u8, 69u8, 154u8, 29u8,
							147u8, 241u8, 91u8, 81u8, 126u8, 206u8, 117u8, 14u8, 149u8, 87u8, 88u8,
						],
					)
				}
				#[doc = " The state proof for the last relay parent block."]
				#[doc = ""]
				#[doc = " This field is meant to be updated each block with the validation data inherent. Therefore,"]
				#[doc = " before processing of the inherent, e.g. in `on_initialize` this data may be stale."]
				#[doc = ""]
				#[doc = " This data is also absent from the genesis."]
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
							46u8, 115u8, 163u8, 190u8, 246u8, 47u8, 200u8, 159u8, 206u8, 204u8,
							94u8, 250u8, 127u8, 112u8, 109u8, 111u8, 210u8, 195u8, 244u8, 41u8,
							36u8, 187u8, 71u8, 150u8, 149u8, 253u8, 143u8, 33u8, 83u8, 189u8,
							182u8, 238u8,
						],
					)
				}
				#[doc = " The snapshot of some state related to messaging relevant to the current parachain as per"]
				#[doc = " the relay parent."]
				#[doc = ""]
				#[doc = " This field is meant to be updated each block with the validation data inherent. Therefore,"]
				#[doc = " before processing of the inherent, e.g. in `on_initialize` this data may be stale."]
				#[doc = ""]
				#[doc = " This data is also absent from the genesis."]				pub fn relevant_messaging_state (& self ,) -> :: subxt :: storage :: address :: Address :: < :: subxt :: storage :: address :: StaticStorageMapKey , runtime_types :: cumulus_pallet_parachain_system :: relay_state_snapshot :: MessagingStateSnapshot , :: subxt :: storage :: address :: Yes , () , () >{
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"RelevantMessagingState",
						vec![],
						[
							117u8, 166u8, 186u8, 126u8, 21u8, 174u8, 86u8, 253u8, 163u8, 90u8,
							54u8, 226u8, 186u8, 253u8, 126u8, 168u8, 145u8, 45u8, 155u8, 32u8,
							97u8, 110u8, 208u8, 125u8, 47u8, 113u8, 165u8, 199u8, 210u8, 118u8,
							217u8, 73u8,
						],
					)
				}
				#[doc = " The parachain host configuration that was obtained from the relay parent."]
				#[doc = ""]
				#[doc = " This field is meant to be updated each block with the validation data inherent. Therefore,"]
				#[doc = " before processing of the inherent, e.g. in `on_initialize` this data may be stale."]
				#[doc = ""]
				#[doc = " This data is also absent from the genesis."]
				pub fn host_configuration(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::polkadot_primitives::v6::AbridgedHostConfiguration,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"HostConfiguration",
						vec![],
						[
							252u8, 23u8, 111u8, 189u8, 120u8, 204u8, 129u8, 223u8, 248u8, 179u8,
							239u8, 173u8, 133u8, 61u8, 140u8, 2u8, 75u8, 32u8, 204u8, 178u8, 69u8,
							21u8, 44u8, 227u8, 178u8, 179u8, 33u8, 26u8, 131u8, 156u8, 78u8, 85u8,
						],
					)
				}
				#[doc = " The last downward message queue chain head we have observed."]
				#[doc = ""]
				#[doc = " This value is loaded before and saved after processing inbound downward messages carried"]
				#[doc = " by the system inherent."]
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
							1u8, 70u8, 140u8, 40u8, 51u8, 127u8, 75u8, 80u8, 5u8, 49u8, 196u8,
							31u8, 30u8, 61u8, 54u8, 252u8, 0u8, 0u8, 100u8, 115u8, 177u8, 250u8,
							138u8, 48u8, 107u8, 41u8, 93u8, 87u8, 195u8, 107u8, 206u8, 227u8,
						],
					)
				}
				#[doc = " The message queue chain heads we have observed per each channel incoming channel."]
				#[doc = ""]
				#[doc = " This value is loaded before and saved after processing inbound downward messages carried"]
				#[doc = " by the system inherent."]
				pub fn last_hrmp_mqc_heads(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::KeyedVec<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
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
							131u8, 170u8, 142u8, 30u8, 101u8, 113u8, 131u8, 81u8, 38u8, 168u8,
							98u8, 3u8, 9u8, 109u8, 96u8, 179u8, 115u8, 177u8, 128u8, 11u8, 238u8,
							54u8, 81u8, 60u8, 97u8, 112u8, 224u8, 175u8, 86u8, 133u8, 182u8, 76u8,
						],
					)
				}
				#[doc = " Number of downward messages processed in a block."]
				#[doc = ""]
				#[doc = " This will be cleared in `on_initialize` of each new block."]
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
							151u8, 234u8, 196u8, 87u8, 130u8, 79u8, 4u8, 102u8, 47u8, 10u8, 33u8,
							132u8, 149u8, 118u8, 61u8, 141u8, 5u8, 1u8, 30u8, 120u8, 220u8, 156u8,
							16u8, 11u8, 14u8, 52u8, 126u8, 151u8, 244u8, 149u8, 197u8, 51u8,
						],
					)
				}
				#[doc = " HRMP watermark that was set in a block."]
				#[doc = ""]
				#[doc = " This will be cleared in `on_initialize` of each new block."]
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
							77u8, 62u8, 59u8, 220u8, 7u8, 125u8, 98u8, 249u8, 108u8, 212u8, 223u8,
							99u8, 152u8, 13u8, 29u8, 80u8, 166u8, 65u8, 232u8, 113u8, 145u8, 128u8,
							123u8, 35u8, 238u8, 31u8, 113u8, 156u8, 220u8, 104u8, 217u8, 165u8,
						],
					)
				}
				#[doc = " HRMP messages that were sent in a block."]
				#[doc = ""]
				#[doc = " This will be cleared in `on_initialize` of each new block."]
				pub fn hrmp_outbound_messages(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<
						runtime_types::polkadot_core_primitives::OutboundHrmpMessage<
							runtime_types::polkadot_parachain_primitives::primitives::Id,
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
							42u8, 9u8, 96u8, 217u8, 25u8, 101u8, 129u8, 147u8, 150u8, 20u8, 164u8,
							186u8, 217u8, 178u8, 15u8, 201u8, 233u8, 104u8, 92u8, 120u8, 29u8,
							245u8, 196u8, 13u8, 141u8, 210u8, 102u8, 62u8, 216u8, 80u8, 246u8,
							145u8,
						],
					)
				}
				#[doc = " Upward messages that were sent in a block."]
				#[doc = ""]
				#[doc = " This will be cleared in `on_initialize` of each new block."]
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
							179u8, 127u8, 8u8, 94u8, 194u8, 246u8, 53u8, 79u8, 80u8, 22u8, 18u8,
							75u8, 116u8, 163u8, 90u8, 161u8, 30u8, 140u8, 57u8, 126u8, 60u8, 91u8,
							23u8, 30u8, 120u8, 245u8, 125u8, 96u8, 152u8, 25u8, 248u8, 85u8,
						],
					)
				}
				#[doc = " Upward messages that are still pending and not yet send to the relay chain."]
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
							239u8, 45u8, 18u8, 173u8, 148u8, 150u8, 55u8, 176u8, 173u8, 156u8,
							246u8, 226u8, 198u8, 214u8, 104u8, 187u8, 186u8, 13u8, 83u8, 194u8,
							153u8, 29u8, 228u8, 109u8, 26u8, 18u8, 212u8, 151u8, 246u8, 24u8,
							133u8, 216u8,
						],
					)
				}
				#[doc = " The factor to multiply the base delivery fee by for UMP."]
				pub fn upward_delivery_fee_factor(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_arithmetic::fixed_point::FixedU128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ParachainSystem",
						"UpwardDeliveryFeeFactor",
						vec![],
						[
							40u8, 217u8, 164u8, 111u8, 151u8, 132u8, 69u8, 226u8, 163u8, 175u8,
							43u8, 239u8, 179u8, 217u8, 136u8, 161u8, 13u8, 251u8, 163u8, 102u8,
							24u8, 27u8, 168u8, 89u8, 221u8, 83u8, 93u8, 64u8, 96u8, 117u8, 146u8,
							71u8,
						],
					)
				}
				#[doc = " The number of HRMP messages we observed in `on_initialize` and thus used that number for"]
				#[doc = " announcing the weight of `on_initialize` and `on_finalize`."]
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
							93u8, 11u8, 229u8, 172u8, 73u8, 87u8, 13u8, 149u8, 15u8, 94u8, 163u8,
							107u8, 156u8, 22u8, 131u8, 177u8, 96u8, 247u8, 213u8, 224u8, 41u8,
							126u8, 157u8, 33u8, 154u8, 194u8, 95u8, 234u8, 65u8, 19u8, 58u8, 161u8,
						],
					)
				}
				#[doc = " The weight we reserve at the beginning of the block for processing XCMP messages. This"]
				#[doc = " overrides the amount set in the Config trait."]
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
							176u8, 93u8, 203u8, 74u8, 18u8, 170u8, 246u8, 203u8, 109u8, 89u8, 86u8,
							77u8, 96u8, 66u8, 189u8, 79u8, 184u8, 253u8, 11u8, 230u8, 87u8, 120u8,
							1u8, 254u8, 215u8, 41u8, 210u8, 86u8, 239u8, 206u8, 60u8, 2u8,
						],
					)
				}
				#[doc = " The weight we reserve at the beginning of the block for processing DMP messages. This"]
				#[doc = " overrides the amount set in the Config trait."]
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
							205u8, 124u8, 9u8, 156u8, 255u8, 207u8, 208u8, 23u8, 179u8, 132u8,
							254u8, 157u8, 237u8, 240u8, 167u8, 203u8, 253u8, 111u8, 136u8, 32u8,
							100u8, 152u8, 16u8, 19u8, 175u8, 14u8, 108u8, 61u8, 59u8, 231u8, 70u8,
							112u8,
						],
					)
				}
				#[doc = " A custom head data that should be returned as result of `validate_block`."]
				#[doc = ""]
				#[doc = " See `Pallet::set_custom_validation_head_data` for more information."]
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
							52u8, 186u8, 187u8, 57u8, 245u8, 171u8, 202u8, 23u8, 92u8, 80u8, 118u8,
							66u8, 251u8, 156u8, 175u8, 254u8, 141u8, 185u8, 115u8, 209u8, 170u8,
							165u8, 1u8, 242u8, 120u8, 234u8, 162u8, 24u8, 135u8, 105u8, 8u8, 177u8,
						],
					)
				}
			}
		}
	}
	pub mod parachain_info {
		use super::{root_mod, runtime_types};
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::staging_parachain_info::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
			}
			pub struct TransactionApi;
			impl TransactionApi {}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn parachain_id(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::polkadot_parachain_primitives::primitives::Id,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"ParachainInfo",
						"ParachainId",
						vec![],
						[
							160u8, 130u8, 74u8, 181u8, 231u8, 180u8, 246u8, 152u8, 204u8, 44u8,
							245u8, 91u8, 113u8, 246u8, 218u8, 50u8, 254u8, 248u8, 35u8, 219u8,
							83u8, 144u8, 228u8, 245u8, 122u8, 53u8, 194u8, 172u8, 222u8, 118u8,
							202u8, 91u8,
						],
					)
				}
			}
		}
	}
	pub mod utility {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_utility::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_utility::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Batch {
					pub calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Batch {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "batch";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AsDerivative {
					pub index: ::core::primitive::u16,
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for AsDerivative {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "as_derivative";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BatchAll {
					pub calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for BatchAll {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "batch_all";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DispatchAs {
					pub as_origin:
						::std::boxed::Box<runtime_types::gargantua_runtime::OriginCaller>,
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for DispatchAs {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "dispatch_as";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceBatch {
					pub calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceBatch {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "force_batch";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WithWeight {
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				impl ::subxt::blocks::StaticExtrinsic for WithWeight {
					const PALLET: &'static str = "Utility";
					const CALL: &'static str = "with_weight";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::batch`]."]
				pub fn batch(
					&self,
					calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				) -> ::subxt::tx::Payload<types::Batch> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"batch",
						types::Batch { calls },
						[
							93u8, 175u8, 209u8, 184u8, 132u8, 238u8, 19u8, 64u8, 250u8, 72u8,
							185u8, 186u8, 205u8, 185u8, 184u8, 237u8, 177u8, 90u8, 132u8, 210u8,
							0u8, 37u8, 214u8, 210u8, 39u8, 232u8, 185u8, 239u8, 254u8, 80u8, 162u8,
							15u8,
						],
					)
				}
				#[doc = "See [`Pallet::as_derivative`]."]
				pub fn as_derivative(
					&self,
					index: ::core::primitive::u16,
					call: runtime_types::gargantua_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::AsDerivative> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"as_derivative",
						types::AsDerivative { index, call: ::std::boxed::Box::new(call) },
						[
							252u8, 127u8, 210u8, 53u8, 89u8, 50u8, 233u8, 67u8, 227u8, 250u8, 30u8,
							218u8, 205u8, 212u8, 80u8, 163u8, 222u8, 194u8, 117u8, 29u8, 253u8,
							83u8, 95u8, 136u8, 82u8, 53u8, 215u8, 134u8, 28u8, 130u8, 138u8, 155u8,
						],
					)
				}
				#[doc = "See [`Pallet::batch_all`]."]
				pub fn batch_all(
					&self,
					calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				) -> ::subxt::tx::Payload<types::BatchAll> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"batch_all",
						types::BatchAll { calls },
						[
							48u8, 18u8, 232u8, 253u8, 37u8, 145u8, 38u8, 25u8, 237u8, 243u8, 61u8,
							81u8, 27u8, 233u8, 89u8, 229u8, 96u8, 85u8, 205u8, 244u8, 105u8, 118u8,
							112u8, 159u8, 111u8, 210u8, 44u8, 26u8, 103u8, 13u8, 28u8, 181u8,
						],
					)
				}
				#[doc = "See [`Pallet::dispatch_as`]."]
				pub fn dispatch_as(
					&self,
					as_origin: runtime_types::gargantua_runtime::OriginCaller,
					call: runtime_types::gargantua_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::DispatchAs> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"dispatch_as",
						types::DispatchAs {
							as_origin: ::std::boxed::Box::new(as_origin),
							call: ::std::boxed::Box::new(call),
						},
						[
							46u8, 81u8, 79u8, 182u8, 125u8, 70u8, 130u8, 55u8, 169u8, 54u8, 29u8,
							15u8, 197u8, 64u8, 77u8, 66u8, 246u8, 8u8, 247u8, 174u8, 125u8, 198u8,
							78u8, 52u8, 7u8, 134u8, 54u8, 220u8, 10u8, 24u8, 9u8, 85u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_batch`]."]
				pub fn force_batch(
					&self,
					calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
				) -> ::subxt::tx::Payload<types::ForceBatch> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"force_batch",
						types::ForceBatch { calls },
						[
							250u8, 153u8, 177u8, 178u8, 117u8, 253u8, 38u8, 145u8, 67u8, 247u8,
							15u8, 128u8, 51u8, 73u8, 217u8, 135u8, 141u8, 83u8, 182u8, 178u8,
							248u8, 156u8, 39u8, 72u8, 9u8, 253u8, 31u8, 159u8, 33u8, 123u8, 208u8,
							251u8,
						],
					)
				}
				#[doc = "See [`Pallet::with_weight`]."]
				pub fn with_weight(
					&self,
					call: runtime_types::gargantua_runtime::RuntimeCall,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::tx::Payload<types::WithWeight> {
					::subxt::tx::Payload::new_static(
						"Utility",
						"with_weight",
						types::WithWeight { call: ::std::boxed::Box::new(call), weight },
						[
							61u8, 170u8, 214u8, 0u8, 228u8, 190u8, 95u8, 96u8, 232u8, 30u8, 1u8,
							211u8, 82u8, 111u8, 177u8, 210u8, 6u8, 142u8, 92u8, 114u8, 42u8, 127u8,
							237u8, 153u8, 87u8, 22u8, 156u8, 210u8, 88u8, 91u8, 251u8, 68u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_utility::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Batch of dispatches did not complete fully. Index of first failing dispatch given, as"]
			#[doc = "well as the error."]
			pub struct BatchInterrupted {
				pub index: ::core::primitive::u32,
				pub error: runtime_types::sp_runtime::DispatchError,
			}
			impl ::subxt::events::StaticEvent for BatchInterrupted {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "BatchInterrupted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Batch of dispatches completed fully with no error."]
			pub struct BatchCompleted;
			impl ::subxt::events::StaticEvent for BatchCompleted {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "BatchCompleted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Batch of dispatches completed but has errors."]
			pub struct BatchCompletedWithErrors;
			impl ::subxt::events::StaticEvent for BatchCompletedWithErrors {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "BatchCompletedWithErrors";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A single item within a Batch of dispatches has completed with no error."]
			pub struct ItemCompleted;
			impl ::subxt::events::StaticEvent for ItemCompleted {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "ItemCompleted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A single item within a Batch of dispatches has completed with error."]
			pub struct ItemFailed {
				pub error: runtime_types::sp_runtime::DispatchError,
			}
			impl ::subxt::events::StaticEvent for ItemFailed {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "ItemFailed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A call was dispatched."]
			pub struct DispatchedAs {
				pub result: ::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
			}
			impl ::subxt::events::StaticEvent for DispatchedAs {
				const PALLET: &'static str = "Utility";
				const EVENT: &'static str = "DispatchedAs";
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The limit on the number of batched calls."]
				pub fn batched_calls_limit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Utility",
						"batched_calls_limit",
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
	pub mod balances {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_balances::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_balances::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAllowDeath {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAllowDeath {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "transfer_allow_death";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceTransfer {
					pub source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceTransfer {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "force_transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferKeepAlive {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub value: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferKeepAlive {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "transfer_keep_alive";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAll {
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub keep_alive: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAll {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "transfer_all";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceUnreserve {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceUnreserve {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "force_unreserve";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpgradeAccounts {
					pub who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpgradeAccounts {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "upgrade_accounts";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSetBalance {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub new_free: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetBalance {
					const PALLET: &'static str = "Balances";
					const CALL: &'static str = "force_set_balance";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::transfer_allow_death`]."]
				pub fn transfer_allow_death(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferAllowDeath> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"transfer_allow_death",
						types::TransferAllowDeath { dest, value },
						[
							51u8, 166u8, 195u8, 10u8, 139u8, 218u8, 55u8, 130u8, 6u8, 194u8, 35u8,
							140u8, 27u8, 205u8, 214u8, 222u8, 102u8, 43u8, 143u8, 145u8, 86u8,
							219u8, 210u8, 147u8, 13u8, 39u8, 51u8, 21u8, 237u8, 179u8, 132u8,
							130u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_transfer`]."]
				pub fn force_transfer(
					&self,
					source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceTransfer> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"force_transfer",
						types::ForceTransfer { source, dest, value },
						[
							154u8, 93u8, 222u8, 27u8, 12u8, 248u8, 63u8, 213u8, 224u8, 86u8, 250u8,
							153u8, 249u8, 102u8, 83u8, 160u8, 79u8, 125u8, 105u8, 222u8, 77u8,
							180u8, 90u8, 105u8, 81u8, 217u8, 60u8, 25u8, 213u8, 51u8, 185u8, 96u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_keep_alive`]."]
				pub fn transfer_keep_alive(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					value: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferKeepAlive> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"transfer_keep_alive",
						types::TransferKeepAlive { dest, value },
						[
							245u8, 14u8, 190u8, 193u8, 32u8, 210u8, 74u8, 92u8, 25u8, 182u8, 76u8,
							55u8, 247u8, 83u8, 114u8, 75u8, 143u8, 236u8, 117u8, 25u8, 54u8, 157u8,
							208u8, 207u8, 233u8, 89u8, 70u8, 161u8, 235u8, 242u8, 222u8, 59u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_all`]."]
				pub fn transfer_all(
					&self,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					keep_alive: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::TransferAll> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"transfer_all",
						types::TransferAll { dest, keep_alive },
						[
							105u8, 132u8, 49u8, 144u8, 195u8, 250u8, 34u8, 46u8, 213u8, 248u8,
							112u8, 188u8, 81u8, 228u8, 136u8, 18u8, 67u8, 172u8, 37u8, 38u8, 238u8,
							9u8, 34u8, 15u8, 67u8, 34u8, 148u8, 195u8, 223u8, 29u8, 154u8, 6u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_unreserve`]."]
				pub fn force_unreserve(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceUnreserve> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"force_unreserve",
						types::ForceUnreserve { who, amount },
						[
							142u8, 151u8, 64u8, 205u8, 46u8, 64u8, 62u8, 122u8, 108u8, 49u8, 223u8,
							140u8, 120u8, 153u8, 35u8, 165u8, 187u8, 38u8, 157u8, 200u8, 123u8,
							199u8, 198u8, 168u8, 208u8, 159u8, 39u8, 134u8, 92u8, 103u8, 84u8,
							171u8,
						],
					)
				}
				#[doc = "See [`Pallet::upgrade_accounts`]."]
				pub fn upgrade_accounts(
					&self,
					who: ::std::vec::Vec<::subxt::utils::AccountId32>,
				) -> ::subxt::tx::Payload<types::UpgradeAccounts> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"upgrade_accounts",
						types::UpgradeAccounts { who },
						[
							66u8, 200u8, 179u8, 104u8, 65u8, 2u8, 101u8, 56u8, 130u8, 161u8, 224u8,
							233u8, 255u8, 124u8, 70u8, 122u8, 8u8, 49u8, 103u8, 178u8, 68u8, 47u8,
							214u8, 166u8, 217u8, 116u8, 178u8, 50u8, 212u8, 164u8, 98u8, 226u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_set_balance`]."]
				pub fn force_set_balance(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					new_free: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceSetBalance> {
					::subxt::tx::Payload::new_static(
						"Balances",
						"force_set_balance",
						types::ForceSetBalance { who, new_free },
						[
							114u8, 229u8, 59u8, 204u8, 180u8, 83u8, 17u8, 4u8, 59u8, 4u8, 55u8,
							39u8, 151u8, 196u8, 124u8, 60u8, 209u8, 65u8, 193u8, 11u8, 44u8, 164u8,
							116u8, 93u8, 169u8, 30u8, 199u8, 165u8, 55u8, 231u8, 223u8, 43u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_balances::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was created with some free balance."]
			pub struct Endowed {
				pub account: ::subxt::utils::AccountId32,
				pub free_balance: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Endowed {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Endowed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
			#[doc = "resulting in an outright loss."]
			pub struct DustLost {
				pub account: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for DustLost {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "DustLost";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Transfer succeeded."]
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
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A balance was set by root."]
			pub struct BalanceSet {
				pub who: ::subxt::utils::AccountId32,
				pub free: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for BalanceSet {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "BalanceSet";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was reserved (moved from free to reserved)."]
			pub struct Reserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Reserved {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Reserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unreserved (moved from reserved to free)."]
			pub struct Unreserved {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unreserved {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Unreserved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was moved from the reserve of the first account to the second account."]
			#[doc = "Final argument indicates the destination balance type."]
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
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was deposited (e.g. for transaction fees)."]
			pub struct Deposit {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Deposit {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Deposit";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
			pub struct Withdraw {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Withdraw {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Withdraw";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
			pub struct Slashed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Slashed {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Slashed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was minted into an account."]
			pub struct Minted {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Minted {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Minted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was burned from an account."]
			pub struct Burned {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Burned {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Burned";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was suspended from an account (it can be restored later)."]
			pub struct Suspended {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Suspended {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Suspended";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some amount was restored into an account."]
			pub struct Restored {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Restored {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Restored";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was upgraded."]
			pub struct Upgraded {
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Upgraded {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Upgraded";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
			pub struct Issued {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Issued";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
			pub struct Rescinded {
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Rescinded {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Rescinded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was locked."]
			pub struct Locked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Locked {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Locked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was unlocked."]
			pub struct Unlocked {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Unlocked {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Unlocked";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was frozen."]
			pub struct Frozen {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Frozen {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Frozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance was thawed."]
			pub struct Thawed {
				pub who: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "Balances";
				const EVENT: &'static str = "Thawed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The total units issued in the system."]
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
							116u8, 70u8, 119u8, 194u8, 69u8, 37u8, 116u8, 206u8, 171u8, 70u8,
							171u8, 210u8, 226u8, 111u8, 184u8, 204u8, 206u8, 11u8, 68u8, 72u8,
							255u8, 19u8, 194u8, 11u8, 27u8, 194u8, 81u8, 204u8, 59u8, 224u8, 202u8,
							185u8,
						],
					)
				}
				#[doc = " The total units of outstanding deactivated balance in the system."]
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
							212u8, 185u8, 19u8, 50u8, 250u8, 72u8, 173u8, 50u8, 4u8, 104u8, 161u8,
							249u8, 77u8, 247u8, 204u8, 248u8, 11u8, 18u8, 57u8, 4u8, 82u8, 110u8,
							30u8, 216u8, 16u8, 37u8, 87u8, 67u8, 189u8, 235u8, 214u8, 155u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Account",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " The Balances pallet example of storing the balance of an account."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " You can also store the balance of an account in the `System` pallet."]
				#[doc = ""]
				#[doc = " # Example"]
				#[doc = ""]
				#[doc = " ```nocompile"]
				#[doc = "  impl pallet_balances::Config for Runtime {"]
				#[doc = "   type AccountStore = System"]
				#[doc = "  }"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " But this comes with tradeoffs, storing account balances in the system pallet stores"]
				#[doc = " `frame_system` data alongside the account data contrary to storing account balances in the"]
				#[doc = " `Balances` pallet, which uses a `StorageMap` to store balances data only."]
				#[doc = " NOTE: This is only used in the case that this pallet is used to store balances."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_balances::types::AccountData<::core::primitive::u128>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Account",
						Vec::new(),
						[
							213u8, 38u8, 200u8, 69u8, 218u8, 0u8, 112u8, 181u8, 160u8, 23u8, 96u8,
							90u8, 3u8, 88u8, 126u8, 22u8, 103u8, 74u8, 64u8, 69u8, 29u8, 247u8,
							18u8, 17u8, 234u8, 143u8, 189u8, 22u8, 247u8, 194u8, 154u8, 249u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
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
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Any liquidity locks on some account balances."]
				#[doc = " NOTE: Should only be accessed when setting, changing and freeing a lock."]
				pub fn locks_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec<
						runtime_types::pallet_balances::types::BalanceLock<::core::primitive::u128>,
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
							10u8, 223u8, 55u8, 0u8, 249u8, 69u8, 168u8, 41u8, 75u8, 35u8, 120u8,
							167u8, 18u8, 132u8, 9u8, 20u8, 91u8, 51u8, 27u8, 69u8, 136u8, 187u8,
							13u8, 220u8, 163u8, 122u8, 26u8, 141u8, 174u8, 249u8, 85u8, 37u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
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
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Named reserves on some account balances."]
				pub fn reserves_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::ReserveData<
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
							112u8, 10u8, 241u8, 77u8, 64u8, 187u8, 106u8, 159u8, 13u8, 153u8,
							140u8, 178u8, 182u8, 50u8, 1u8, 55u8, 149u8, 92u8, 196u8, 229u8, 170u8,
							106u8, 193u8, 88u8, 255u8, 244u8, 2u8, 193u8, 62u8, 235u8, 204u8, 91u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::gargantua_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Holds",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							37u8, 176u8, 2u8, 18u8, 109u8, 26u8, 66u8, 81u8, 28u8, 104u8, 149u8,
							117u8, 119u8, 114u8, 196u8, 35u8, 172u8, 155u8, 66u8, 195u8, 98u8,
							37u8, 134u8, 22u8, 106u8, 221u8, 215u8, 97u8, 25u8, 28u8, 21u8, 206u8,
						],
					)
				}
				#[doc = " Holds on account balances."]
				pub fn holds_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							runtime_types::gargantua_runtime::RuntimeHoldReason,
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Holds",
						Vec::new(),
						[
							37u8, 176u8, 2u8, 18u8, 109u8, 26u8, 66u8, 81u8, 28u8, 104u8, 149u8,
							117u8, 119u8, 114u8, 196u8, 35u8, 172u8, 155u8, 66u8, 195u8, 98u8,
							37u8, 134u8, 22u8, 106u8, 221u8, 215u8, 97u8, 25u8, 28u8, 21u8, 206u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							(),
							::core::primitive::u128,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Freezes",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							69u8, 49u8, 165u8, 76u8, 135u8, 142u8, 179u8, 118u8, 50u8, 109u8, 53u8,
							112u8, 110u8, 94u8, 30u8, 93u8, 173u8, 38u8, 27u8, 142u8, 19u8, 5u8,
							163u8, 4u8, 68u8, 218u8, 179u8, 224u8, 118u8, 218u8, 115u8, 64u8,
						],
					)
				}
				#[doc = " Freeze locks on account balances."]
				pub fn freezes_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_balances::types::IdAmount<
							(),
							::core::primitive::u128,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Balances",
						"Freezes",
						Vec::new(),
						[
							69u8, 49u8, 165u8, 76u8, 135u8, 142u8, 179u8, 118u8, 50u8, 109u8, 53u8,
							112u8, 110u8, 94u8, 30u8, 93u8, 173u8, 38u8, 27u8, 142u8, 19u8, 5u8,
							163u8, 4u8, 68u8, 218u8, 179u8, 224u8, 118u8, 218u8, 115u8, 64u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!"]
				#[doc = ""]
				#[doc = " If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for"]
				#[doc = " this pallet. However, you do so at your own risk: this will open up a major DoS vector."]
				#[doc = " In case you have multiple sources of provider references, you may also get unexpected"]
				#[doc = " behaviour if you set this to zero."]
				#[doc = ""]
				#[doc = " Bottom line: Do yourself a favour and make it at least one!"]
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
				#[doc = " The maximum number of locks that should exist on an account."]
				#[doc = " Not strictly enforced, but used for weight estimation."]
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
				#[doc = " The maximum number of named reserves that can exist on an account."]
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
				#[doc = " The maximum number of holds that can exist on an account at any time."]
				pub fn max_holds(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Balances",
						"MaxHolds",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of individual freeze locks that can exist on an account at any time."]
				pub fn max_freezes(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Balances",
						"MaxFreezes",
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
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_transaction_payment::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
			#[doc = "has been paid by `who`."]
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
							247u8, 39u8, 81u8, 170u8, 225u8, 226u8, 82u8, 147u8, 34u8, 113u8,
							147u8, 213u8, 59u8, 80u8, 139u8, 35u8, 36u8, 196u8, 152u8, 19u8, 9u8,
							159u8, 176u8, 79u8, 249u8, 201u8, 170u8, 1u8, 129u8, 79u8, 146u8,
							197u8,
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
							105u8, 243u8, 158u8, 241u8, 159u8, 231u8, 253u8, 6u8, 4u8, 32u8, 85u8,
							178u8, 126u8, 31u8, 203u8, 134u8, 154u8, 38u8, 122u8, 155u8, 150u8,
							251u8, 174u8, 15u8, 74u8, 134u8, 216u8, 244u8, 168u8, 175u8, 158u8,
							144u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " A fee multiplier for `Operational` extrinsics to compute \"virtual tip\" to boost their"]
				#[doc = " `priority`"]
				#[doc = ""]
				#[doc = " This value is multiplied by the `final_fee` to obtain a \"virtual tip\" that is later"]
				#[doc = " added to a tip component in regular `priority` calculations."]
				#[doc = " It means that a `Normal` transaction can front-run a similarly-sized `Operational`"]
				#[doc = " extrinsic (with no tip), by including a tip value greater than the virtual tip."]
				#[doc = ""]
				#[doc = " ```rust,ignore"]
				#[doc = " // For `Normal`"]
				#[doc = " let priority = priority_calc(tip);"]
				#[doc = ""]
				#[doc = " // For `Operational`"]
				#[doc = " let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;"]
				#[doc = " let priority = priority_calc(tip + virtual_tip);"]
				#[doc = " ```"]
				#[doc = ""]
				#[doc = " Note that since we use `final_fee` the multiplier applies also to the regular `tip`"]
				#[doc = " sent with the transaction. So, not only does the transaction get a priority bump based"]
				#[doc = " on the `inclusion_fee`, but we also amplify the impact of tips applied to `Operational`"]
				#[doc = " transactions."]
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
				#[doc = " Author of current block."]
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
							247u8, 192u8, 118u8, 227u8, 47u8, 20u8, 203u8, 199u8, 216u8, 87u8,
							220u8, 50u8, 166u8, 61u8, 168u8, 213u8, 253u8, 62u8, 202u8, 199u8,
							61u8, 192u8, 237u8, 53u8, 22u8, 148u8, 164u8, 245u8, 99u8, 24u8, 146u8,
							18u8,
						],
					)
				}
			}
		}
	}
	pub mod collator_selection {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_collator_selection::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_collator_selection::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetInvulnerables {
					pub new: ::std::vec::Vec<::subxt::utils::AccountId32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetInvulnerables {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "set_invulnerables";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetDesiredCandidates {
					pub max: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetDesiredCandidates {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "set_desired_candidates";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetCandidacyBond {
					pub bond: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetCandidacyBond {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "set_candidacy_bond";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RegisterAsCandidate;
				impl ::subxt::blocks::StaticExtrinsic for RegisterAsCandidate {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "register_as_candidate";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct LeaveIntent;
				impl ::subxt::blocks::StaticExtrinsic for LeaveIntent {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "leave_intent";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AddInvulnerable {
					pub who: ::subxt::utils::AccountId32,
				}
				impl ::subxt::blocks::StaticExtrinsic for AddInvulnerable {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "add_invulnerable";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemoveInvulnerable {
					pub who: ::subxt::utils::AccountId32,
				}
				impl ::subxt::blocks::StaticExtrinsic for RemoveInvulnerable {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "remove_invulnerable";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateBond {
					pub new_deposit: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateBond {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "update_bond";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TakeCandidateSlot {
					pub deposit: ::core::primitive::u128,
					pub target: ::subxt::utils::AccountId32,
				}
				impl ::subxt::blocks::StaticExtrinsic for TakeCandidateSlot {
					const PALLET: &'static str = "CollatorSelection";
					const CALL: &'static str = "take_candidate_slot";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_invulnerables`]."]
				pub fn set_invulnerables(
					&self,
					new: ::std::vec::Vec<::subxt::utils::AccountId32>,
				) -> ::subxt::tx::Payload<types::SetInvulnerables> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"set_invulnerables",
						types::SetInvulnerables { new },
						[
							113u8, 217u8, 14u8, 48u8, 6u8, 198u8, 8u8, 170u8, 8u8, 237u8, 230u8,
							184u8, 17u8, 181u8, 15u8, 126u8, 117u8, 3u8, 208u8, 215u8, 40u8, 16u8,
							150u8, 162u8, 37u8, 196u8, 235u8, 36u8, 247u8, 24u8, 187u8, 17u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_desired_candidates`]."]
				pub fn set_desired_candidates(
					&self,
					max: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::SetDesiredCandidates> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"set_desired_candidates",
						types::SetDesiredCandidates { max },
						[
							174u8, 44u8, 232u8, 155u8, 228u8, 219u8, 239u8, 75u8, 86u8, 150u8,
							135u8, 214u8, 58u8, 9u8, 25u8, 133u8, 245u8, 101u8, 85u8, 246u8, 15u8,
							248u8, 165u8, 87u8, 88u8, 28u8, 10u8, 196u8, 86u8, 89u8, 28u8, 165u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_candidacy_bond`]."]
				pub fn set_candidacy_bond(
					&self,
					bond: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::SetCandidacyBond> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"set_candidacy_bond",
						types::SetCandidacyBond { bond },
						[
							250u8, 4u8, 185u8, 228u8, 101u8, 223u8, 49u8, 44u8, 172u8, 148u8,
							216u8, 242u8, 192u8, 88u8, 228u8, 59u8, 225u8, 222u8, 171u8, 40u8,
							23u8, 1u8, 46u8, 183u8, 189u8, 191u8, 156u8, 12u8, 218u8, 116u8, 76u8,
							59u8,
						],
					)
				}
				#[doc = "See [`Pallet::register_as_candidate`]."]
				pub fn register_as_candidate(
					&self,
				) -> ::subxt::tx::Payload<types::RegisterAsCandidate> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"register_as_candidate",
						types::RegisterAsCandidate {},
						[
							69u8, 222u8, 214u8, 106u8, 105u8, 168u8, 82u8, 239u8, 158u8, 117u8,
							224u8, 89u8, 228u8, 51u8, 221u8, 244u8, 88u8, 63u8, 72u8, 119u8, 224u8,
							111u8, 93u8, 39u8, 18u8, 66u8, 72u8, 105u8, 70u8, 66u8, 178u8, 173u8,
						],
					)
				}
				#[doc = "See [`Pallet::leave_intent`]."]
				pub fn leave_intent(&self) -> ::subxt::tx::Payload<types::LeaveIntent> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"leave_intent",
						types::LeaveIntent {},
						[
							126u8, 57u8, 10u8, 67u8, 120u8, 229u8, 70u8, 23u8, 154u8, 215u8, 226u8,
							178u8, 203u8, 152u8, 195u8, 177u8, 157u8, 158u8, 40u8, 17u8, 93u8,
							225u8, 253u8, 217u8, 48u8, 165u8, 55u8, 79u8, 43u8, 123u8, 193u8,
							147u8,
						],
					)
				}
				#[doc = "See [`Pallet::add_invulnerable`]."]
				pub fn add_invulnerable(
					&self,
					who: ::subxt::utils::AccountId32,
				) -> ::subxt::tx::Payload<types::AddInvulnerable> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"add_invulnerable",
						types::AddInvulnerable { who },
						[
							115u8, 109u8, 38u8, 19u8, 81u8, 194u8, 124u8, 140u8, 239u8, 23u8, 85u8,
							62u8, 241u8, 83u8, 11u8, 241u8, 14u8, 34u8, 206u8, 63u8, 104u8, 78u8,
							96u8, 182u8, 173u8, 198u8, 230u8, 107u8, 102u8, 6u8, 164u8, 75u8,
						],
					)
				}
				#[doc = "See [`Pallet::remove_invulnerable`]."]
				pub fn remove_invulnerable(
					&self,
					who: ::subxt::utils::AccountId32,
				) -> ::subxt::tx::Payload<types::RemoveInvulnerable> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"remove_invulnerable",
						types::RemoveInvulnerable { who },
						[
							103u8, 146u8, 23u8, 136u8, 61u8, 65u8, 172u8, 157u8, 216u8, 200u8,
							119u8, 28u8, 189u8, 215u8, 13u8, 100u8, 102u8, 13u8, 94u8, 12u8, 78u8,
							156u8, 149u8, 74u8, 126u8, 118u8, 127u8, 49u8, 129u8, 2u8, 12u8, 118u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_bond`]."]
				pub fn update_bond(
					&self,
					new_deposit: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::UpdateBond> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"update_bond",
						types::UpdateBond { new_deposit },
						[
							47u8, 184u8, 193u8, 220u8, 160u8, 1u8, 253u8, 203u8, 8u8, 142u8, 43u8,
							151u8, 190u8, 138u8, 201u8, 174u8, 233u8, 112u8, 200u8, 247u8, 251u8,
							94u8, 23u8, 224u8, 150u8, 179u8, 190u8, 140u8, 199u8, 50u8, 2u8, 249u8,
						],
					)
				}
				#[doc = "See [`Pallet::take_candidate_slot`]."]
				pub fn take_candidate_slot(
					&self,
					deposit: ::core::primitive::u128,
					target: ::subxt::utils::AccountId32,
				) -> ::subxt::tx::Payload<types::TakeCandidateSlot> {
					::subxt::tx::Payload::new_static(
						"CollatorSelection",
						"take_candidate_slot",
						types::TakeCandidateSlot { deposit, target },
						[
							48u8, 150u8, 189u8, 206u8, 199u8, 196u8, 173u8, 3u8, 206u8, 10u8, 50u8,
							160u8, 15u8, 53u8, 189u8, 126u8, 154u8, 36u8, 90u8, 66u8, 235u8, 12u8,
							107u8, 44u8, 117u8, 33u8, 207u8, 194u8, 251u8, 194u8, 224u8, 80u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_collator_selection::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "New Invulnerables were set."]
			pub struct NewInvulnerables {
				pub invulnerables: ::std::vec::Vec<::subxt::utils::AccountId32>,
			}
			impl ::subxt::events::StaticEvent for NewInvulnerables {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "NewInvulnerables";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new Invulnerable was added."]
			pub struct InvulnerableAdded {
				pub account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for InvulnerableAdded {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "InvulnerableAdded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An Invulnerable was removed."]
			pub struct InvulnerableRemoved {
				pub account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for InvulnerableRemoved {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "InvulnerableRemoved";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The number of desired candidates was set."]
			pub struct NewDesiredCandidates {
				pub desired_candidates: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for NewDesiredCandidates {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "NewDesiredCandidates";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The candidacy bond was set."]
			pub struct NewCandidacyBond {
				pub bond_amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for NewCandidacyBond {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "NewCandidacyBond";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new candidate joined."]
			pub struct CandidateAdded {
				pub account_id: ::subxt::utils::AccountId32,
				pub deposit: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for CandidateAdded {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "CandidateAdded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Bond of a candidate updated."]
			pub struct CandidateBondUpdated {
				pub account_id: ::subxt::utils::AccountId32,
				pub deposit: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for CandidateBondUpdated {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "CandidateBondUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A candidate was removed."]
			pub struct CandidateRemoved {
				pub account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for CandidateRemoved {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "CandidateRemoved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was replaced in the candidate list by another one."]
			pub struct CandidateReplaced {
				pub old: ::subxt::utils::AccountId32,
				pub new: ::subxt::utils::AccountId32,
				pub deposit: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for CandidateReplaced {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "CandidateReplaced";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An account was unable to be added to the Invulnerables because they did not have keys"]
			#[doc = "registered. Other Invulnerables may have been set."]
			pub struct InvalidInvulnerableSkipped {
				pub account_id: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for InvalidInvulnerableSkipped {
				const PALLET: &'static str = "CollatorSelection";
				const EVENT: &'static str = "InvalidInvulnerableSkipped";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The invulnerable, permissioned collators. This list must be sorted."]
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
							109u8, 180u8, 25u8, 41u8, 152u8, 158u8, 186u8, 214u8, 89u8, 222u8,
							103u8, 14u8, 91u8, 3u8, 65u8, 6u8, 255u8, 62u8, 47u8, 255u8, 132u8,
							164u8, 217u8, 200u8, 130u8, 29u8, 168u8, 23u8, 81u8, 217u8, 35u8,
							123u8,
						],
					)
				}
				#[doc = " The (community, limited) collation candidates. `Candidates` and `Invulnerables` should be"]
				#[doc = " mutually exclusive."]
				#[doc = ""]
				#[doc = " This list is sorted in ascending order by deposit and when the deposits are equal, the least"]
				#[doc = " recently updated is considered greater."]
				pub fn candidate_list(
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
						"CandidateList",
						vec![],
						[
							77u8, 195u8, 89u8, 139u8, 79u8, 111u8, 151u8, 215u8, 19u8, 152u8, 67u8,
							49u8, 74u8, 76u8, 3u8, 60u8, 51u8, 140u8, 6u8, 134u8, 159u8, 55u8,
							196u8, 57u8, 189u8, 31u8, 219u8, 218u8, 164u8, 189u8, 196u8, 60u8,
						],
					)
				}
				#[doc = " Last block authored by collator."]
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
							176u8, 170u8, 165u8, 244u8, 101u8, 126u8, 24u8, 132u8, 228u8, 138u8,
							72u8, 241u8, 144u8, 100u8, 79u8, 112u8, 9u8, 46u8, 210u8, 80u8, 12u8,
							126u8, 32u8, 214u8, 26u8, 171u8, 155u8, 3u8, 233u8, 22u8, 164u8, 25u8,
						],
					)
				}
				#[doc = " Last block authored by collator."]
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
							176u8, 170u8, 165u8, 244u8, 101u8, 126u8, 24u8, 132u8, 228u8, 138u8,
							72u8, 241u8, 144u8, 100u8, 79u8, 112u8, 9u8, 46u8, 210u8, 80u8, 12u8,
							126u8, 32u8, 214u8, 26u8, 171u8, 155u8, 3u8, 233u8, 22u8, 164u8, 25u8,
						],
					)
				}
				#[doc = " Desired number of candidates."]
				#[doc = ""]
				#[doc = " This should ideally always be less than [`Config::MaxCandidates`] for weights to be correct."]
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
							69u8, 199u8, 130u8, 132u8, 10u8, 127u8, 204u8, 220u8, 59u8, 107u8,
							96u8, 180u8, 42u8, 235u8, 14u8, 126u8, 231u8, 242u8, 162u8, 126u8,
							63u8, 223u8, 15u8, 250u8, 22u8, 210u8, 54u8, 34u8, 235u8, 191u8, 250u8,
							21u8,
						],
					)
				}
				#[doc = " Fixed amount to deposit to become a collator."]
				#[doc = ""]
				#[doc = " When a collator calls `leave_intent` they immediately receive the deposit back."]
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
							71u8, 134u8, 156u8, 102u8, 201u8, 83u8, 240u8, 251u8, 189u8, 213u8,
							211u8, 182u8, 126u8, 122u8, 41u8, 174u8, 105u8, 29u8, 216u8, 23u8,
							255u8, 55u8, 245u8, 187u8, 234u8, 234u8, 178u8, 155u8, 145u8, 49u8,
							196u8, 214u8,
						],
					)
				}
			}
		}
	}
	pub mod session {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the session pallet."]
		pub type Error = runtime_types::pallet_session::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_session::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetKeys {
					pub keys: runtime_types::gargantua_runtime::SessionKeys,
					pub proof: ::std::vec::Vec<::core::primitive::u8>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetKeys {
					const PALLET: &'static str = "Session";
					const CALL: &'static str = "set_keys";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PurgeKeys;
				impl ::subxt::blocks::StaticExtrinsic for PurgeKeys {
					const PALLET: &'static str = "Session";
					const CALL: &'static str = "purge_keys";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_keys`]."]
				pub fn set_keys(
					&self,
					keys: runtime_types::gargantua_runtime::SessionKeys,
					proof: ::std::vec::Vec<::core::primitive::u8>,
				) -> ::subxt::tx::Payload<types::SetKeys> {
					::subxt::tx::Payload::new_static(
						"Session",
						"set_keys",
						types::SetKeys { keys, proof },
						[
							10u8, 183u8, 202u8, 82u8, 236u8, 202u8, 212u8, 220u8, 51u8, 217u8,
							229u8, 169u8, 238u8, 141u8, 129u8, 231u8, 203u8, 176u8, 97u8, 148u8,
							240u8, 87u8, 177u8, 245u8, 33u8, 109u8, 243u8, 52u8, 46u8, 118u8,
							164u8, 35u8,
						],
					)
				}
				#[doc = "See [`Pallet::purge_keys`]."]
				pub fn purge_keys(&self) -> ::subxt::tx::Payload<types::PurgeKeys> {
					::subxt::tx::Payload::new_static(
						"Session",
						"purge_keys",
						types::PurgeKeys {},
						[
							215u8, 204u8, 146u8, 236u8, 32u8, 78u8, 198u8, 79u8, 85u8, 214u8, 15u8,
							151u8, 158u8, 31u8, 146u8, 119u8, 119u8, 204u8, 151u8, 169u8, 226u8,
							67u8, 217u8, 39u8, 241u8, 245u8, 203u8, 240u8, 203u8, 172u8, 16u8,
							209u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_session::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "New session has happened. Note that the argument is the session index, not the"]
			#[doc = "block number as the type might suggest."]
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
				#[doc = " The current set of validators."]
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
							50u8, 86u8, 154u8, 222u8, 249u8, 209u8, 156u8, 22u8, 155u8, 25u8,
							133u8, 194u8, 210u8, 50u8, 38u8, 28u8, 139u8, 201u8, 90u8, 139u8,
							115u8, 12u8, 12u8, 141u8, 4u8, 178u8, 201u8, 241u8, 223u8, 234u8, 6u8,
							86u8,
						],
					)
				}
				#[doc = " Current index of the session."]
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
							167u8, 151u8, 125u8, 150u8, 159u8, 21u8, 78u8, 217u8, 237u8, 183u8,
							135u8, 65u8, 187u8, 114u8, 188u8, 206u8, 16u8, 32u8, 69u8, 208u8,
							134u8, 159u8, 232u8, 224u8, 243u8, 27u8, 31u8, 166u8, 145u8, 44u8,
							221u8, 230u8,
						],
					)
				}
				#[doc = " True if the underlying economic identities or weighting behind the validators"]
				#[doc = " has changed in the queued validator set."]
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
							184u8, 137u8, 224u8, 137u8, 31u8, 236u8, 95u8, 164u8, 102u8, 225u8,
							198u8, 227u8, 140u8, 37u8, 113u8, 57u8, 59u8, 4u8, 202u8, 102u8, 117u8,
							36u8, 226u8, 64u8, 113u8, 141u8, 199u8, 111u8, 99u8, 144u8, 198u8,
							153u8,
						],
					)
				}
				#[doc = " The queued keys for the next session. When the next session begins, these keys"]
				#[doc = " will be used to determine the validator's session keys."]
				pub fn queued_keys(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::std::vec::Vec<(
						::subxt::utils::AccountId32,
						runtime_types::gargantua_runtime::SessionKeys,
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
							3u8, 214u8, 191u8, 168u8, 90u8, 94u8, 107u8, 111u8, 170u8, 31u8, 78u8,
							61u8, 240u8, 184u8, 170u8, 104u8, 178u8, 229u8, 159u8, 89u8, 207u8,
							37u8, 49u8, 209u8, 131u8, 165u8, 14u8, 169u8, 13u8, 68u8, 151u8, 144u8,
						],
					)
				}
				#[doc = " Indices of disabled validators."]
				#[doc = ""]
				#[doc = " The vec is always kept sorted so that we can find whether a given validator is"]
				#[doc = " disabled using binary search. It gets cleared when `on_session_ending` returns"]
				#[doc = " a new set of identities."]
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
							213u8, 19u8, 168u8, 234u8, 187u8, 200u8, 180u8, 97u8, 234u8, 189u8,
							36u8, 233u8, 158u8, 184u8, 45u8, 35u8, 129u8, 213u8, 133u8, 8u8, 104u8,
							183u8, 46u8, 68u8, 154u8, 240u8, 132u8, 22u8, 247u8, 11u8, 54u8, 221u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::gargantua_runtime::SessionKeys,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							193u8, 216u8, 53u8, 103u8, 143u8, 241u8, 201u8, 54u8, 108u8, 149u8,
							241u8, 42u8, 3u8, 151u8, 223u8, 246u8, 30u8, 6u8, 239u8, 206u8, 27u8,
							172u8, 43u8, 226u8, 177u8, 111u8, 203u8, 78u8, 49u8, 34u8, 200u8, 6u8,
						],
					)
				}
				#[doc = " The next session keys for a validator."]
				pub fn next_keys_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::gargantua_runtime::SessionKeys,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Session",
						"NextKeys",
						Vec::new(),
						[
							193u8, 216u8, 53u8, 103u8, 143u8, 241u8, 201u8, 54u8, 108u8, 149u8,
							241u8, 42u8, 3u8, 151u8, 223u8, 246u8, 30u8, 6u8, 239u8, 206u8, 27u8,
							172u8, 43u8, 226u8, 177u8, 111u8, 203u8, 78u8, 49u8, 34u8, 200u8, 6u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
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
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
						],
					)
				}
				#[doc = " The owner of a key. The key is the `KeyTypeId` + the encoded key."]
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
							217u8, 204u8, 21u8, 114u8, 247u8, 129u8, 32u8, 242u8, 93u8, 91u8,
							253u8, 253u8, 248u8, 90u8, 12u8, 202u8, 195u8, 25u8, 18u8, 100u8,
							253u8, 109u8, 88u8, 77u8, 217u8, 140u8, 51u8, 40u8, 118u8, 35u8, 107u8,
							206u8,
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
				#[doc = " The current authority set."]
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
							232u8, 129u8, 167u8, 104u8, 47u8, 188u8, 238u8, 164u8, 6u8, 29u8,
							129u8, 45u8, 64u8, 182u8, 194u8, 47u8, 0u8, 73u8, 63u8, 102u8, 204u8,
							94u8, 111u8, 96u8, 137u8, 7u8, 141u8, 110u8, 180u8, 80u8, 228u8, 16u8,
						],
					)
				}
				#[doc = " The current slot of this block."]
				#[doc = ""]
				#[doc = " This will be set in `on_initialize`."]
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
							112u8, 199u8, 115u8, 248u8, 217u8, 242u8, 45u8, 231u8, 178u8, 53u8,
							236u8, 167u8, 219u8, 238u8, 81u8, 243u8, 39u8, 140u8, 68u8, 19u8,
							201u8, 169u8, 211u8, 133u8, 135u8, 213u8, 150u8, 105u8, 60u8, 252u8,
							43u8, 57u8,
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
				#[doc = " Serves as cache for the authorities."]
				#[doc = ""]
				#[doc = " The authorities in AuRa are overwritten in `on_initialize` when we switch to a new session,"]
				#[doc = " but we require the old authorities to verify the seal when validating a PoV. This will"]
				#[doc = " always be updated to the latest AuRa authorities in `on_finalize`."]
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
							232u8, 129u8, 167u8, 104u8, 47u8, 188u8, 238u8, 164u8, 6u8, 29u8,
							129u8, 45u8, 64u8, 182u8, 194u8, 47u8, 0u8, 73u8, 63u8, 102u8, 204u8,
							94u8, 111u8, 96u8, 137u8, 7u8, 141u8, 110u8, 180u8, 80u8, 228u8, 16u8,
						],
					)
				}
				#[doc = " Current slot paired with a number of authored blocks."]
				#[doc = ""]
				#[doc = " Updated on each block initialization."]
				pub fn slot_info(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(runtime_types::sp_consensus_slots::Slot, ::core::primitive::u32),
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"AuraExt",
						"SlotInfo",
						vec![],
						[
							135u8, 135u8, 71u8, 123u8, 102u8, 223u8, 215u8, 76u8, 183u8, 169u8,
							108u8, 60u8, 122u8, 5u8, 24u8, 201u8, 96u8, 59u8, 132u8, 95u8, 253u8,
							100u8, 148u8, 184u8, 133u8, 146u8, 101u8, 201u8, 91u8, 30u8, 76u8,
							169u8,
						],
					)
				}
			}
		}
	}
	pub mod sudo {
		use super::{root_mod, runtime_types};
		#[doc = "Error for the Sudo pallet."]
		pub type Error = runtime_types::pallet_sudo::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_sudo::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Sudo {
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Sudo {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SudoUncheckedWeight {
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				impl ::subxt::blocks::StaticExtrinsic for SudoUncheckedWeight {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo_unchecked_weight";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetKey {
					pub new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetKey {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "set_key";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SudoAs {
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SudoAs {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "sudo_as";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemoveKey;
				impl ::subxt::blocks::StaticExtrinsic for RemoveKey {
					const PALLET: &'static str = "Sudo";
					const CALL: &'static str = "remove_key";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::sudo`]."]
				pub fn sudo(
					&self,
					call: runtime_types::gargantua_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::Sudo> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo",
						types::Sudo { call: ::std::boxed::Box::new(call) },
						[
							160u8, 65u8, 31u8, 108u8, 97u8, 205u8, 115u8, 4u8, 138u8, 50u8, 114u8,
							237u8, 16u8, 97u8, 244u8, 40u8, 82u8, 101u8, 81u8, 74u8, 107u8, 171u8,
							147u8, 149u8, 255u8, 27u8, 5u8, 147u8, 222u8, 110u8, 229u8, 158u8,
						],
					)
				}
				#[doc = "See [`Pallet::sudo_unchecked_weight`]."]
				pub fn sudo_unchecked_weight(
					&self,
					call: runtime_types::gargantua_runtime::RuntimeCall,
					weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::tx::Payload<types::SudoUncheckedWeight> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_unchecked_weight",
						types::SudoUncheckedWeight { call: ::std::boxed::Box::new(call), weight },
						[
							227u8, 92u8, 49u8, 124u8, 190u8, 213u8, 53u8, 118u8, 188u8, 114u8,
							77u8, 224u8, 235u8, 69u8, 149u8, 131u8, 229u8, 19u8, 185u8, 60u8,
							206u8, 140u8, 131u8, 150u8, 169u8, 149u8, 108u8, 173u8, 123u8, 82u8,
							134u8, 184u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_key`]."]
				pub fn set_key(
					&self,
					new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::SetKey> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"set_key",
						types::SetKey { new },
						[
							9u8, 73u8, 39u8, 205u8, 188u8, 127u8, 143u8, 54u8, 128u8, 94u8, 8u8,
							227u8, 197u8, 44u8, 70u8, 93u8, 228u8, 196u8, 64u8, 165u8, 226u8,
							158u8, 101u8, 192u8, 22u8, 193u8, 102u8, 84u8, 21u8, 35u8, 92u8, 198u8,
						],
					)
				}
				#[doc = "See [`Pallet::sudo_as`]."]
				pub fn sudo_as(
					&self,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					call: runtime_types::gargantua_runtime::RuntimeCall,
				) -> ::subxt::tx::Payload<types::SudoAs> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"sudo_as",
						types::SudoAs { who, call: ::std::boxed::Box::new(call) },
						[
							119u8, 137u8, 94u8, 9u8, 229u8, 85u8, 35u8, 142u8, 141u8, 162u8, 193u8,
							236u8, 180u8, 57u8, 254u8, 184u8, 200u8, 31u8, 205u8, 5u8, 214u8, 70u8,
							54u8, 3u8, 72u8, 143u8, 176u8, 161u8, 97u8, 36u8, 230u8, 229u8,
						],
					)
				}
				#[doc = "See [`Pallet::remove_key`]."]
				pub fn remove_key(&self) -> ::subxt::tx::Payload<types::RemoveKey> {
					::subxt::tx::Payload::new_static(
						"Sudo",
						"remove_key",
						types::RemoveKey {},
						[
							133u8, 253u8, 54u8, 175u8, 202u8, 239u8, 5u8, 198u8, 180u8, 138u8,
							25u8, 28u8, 109u8, 40u8, 30u8, 56u8, 126u8, 100u8, 52u8, 205u8, 250u8,
							191u8, 61u8, 195u8, 172u8, 142u8, 184u8, 239u8, 247u8, 10u8, 211u8,
							79u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_sudo::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A sudo call just took place."]
			pub struct Sudid {
				pub sudo_result:
					::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
			}
			impl ::subxt::events::StaticEvent for Sudid {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "Sudid";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The sudo key has been updated."]
			pub struct KeyChanged {
				pub old: ::core::option::Option<::subxt::utils::AccountId32>,
				pub new: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for KeyChanged {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "KeyChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The key was permanently removed."]
			pub struct KeyRemoved;
			impl ::subxt::events::StaticEvent for KeyRemoved {
				const PALLET: &'static str = "Sudo";
				const EVENT: &'static str = "KeyRemoved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A [sudo_as](Pallet::sudo_as) call just took place."]
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
				#[doc = " The `AccountId` of the sudo key."]
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
							72u8, 14u8, 225u8, 162u8, 205u8, 247u8, 227u8, 105u8, 116u8, 57u8, 4u8,
							31u8, 84u8, 137u8, 227u8, 228u8, 133u8, 245u8, 206u8, 227u8, 117u8,
							36u8, 252u8, 151u8, 107u8, 15u8, 180u8, 4u8, 4u8, 152u8, 195u8, 144u8,
						],
					)
				}
			}
		}
	}
	pub mod xcmp_queue {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::cumulus_pallet_xcmp_queue::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::cumulus_pallet_xcmp_queue::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SuspendXcmExecution;
				impl ::subxt::blocks::StaticExtrinsic for SuspendXcmExecution {
					const PALLET: &'static str = "XcmpQueue";
					const CALL: &'static str = "suspend_xcm_execution";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ResumeXcmExecution;
				impl ::subxt::blocks::StaticExtrinsic for ResumeXcmExecution {
					const PALLET: &'static str = "XcmpQueue";
					const CALL: &'static str = "resume_xcm_execution";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateSuspendThreshold {
					pub new: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateSuspendThreshold {
					const PALLET: &'static str = "XcmpQueue";
					const CALL: &'static str = "update_suspend_threshold";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateDropThreshold {
					pub new: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateDropThreshold {
					const PALLET: &'static str = "XcmpQueue";
					const CALL: &'static str = "update_drop_threshold";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateResumeThreshold {
					pub new: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateResumeThreshold {
					const PALLET: &'static str = "XcmpQueue";
					const CALL: &'static str = "update_resume_threshold";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::suspend_xcm_execution`]."]
				pub fn suspend_xcm_execution(
					&self,
				) -> ::subxt::tx::Payload<types::SuspendXcmExecution> {
					::subxt::tx::Payload::new_static(
						"XcmpQueue",
						"suspend_xcm_execution",
						types::SuspendXcmExecution {},
						[
							54u8, 120u8, 33u8, 251u8, 74u8, 56u8, 29u8, 76u8, 104u8, 218u8, 115u8,
							198u8, 148u8, 237u8, 9u8, 191u8, 241u8, 48u8, 33u8, 24u8, 60u8, 144u8,
							22u8, 78u8, 58u8, 50u8, 26u8, 188u8, 231u8, 42u8, 201u8, 76u8,
						],
					)
				}
				#[doc = "See [`Pallet::resume_xcm_execution`]."]
				pub fn resume_xcm_execution(
					&self,
				) -> ::subxt::tx::Payload<types::ResumeXcmExecution> {
					::subxt::tx::Payload::new_static(
						"XcmpQueue",
						"resume_xcm_execution",
						types::ResumeXcmExecution {},
						[
							173u8, 231u8, 78u8, 253u8, 108u8, 234u8, 199u8, 124u8, 184u8, 154u8,
							95u8, 194u8, 13u8, 77u8, 175u8, 7u8, 7u8, 112u8, 161u8, 72u8, 133u8,
							71u8, 63u8, 218u8, 97u8, 226u8, 133u8, 6u8, 93u8, 177u8, 247u8, 109u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_suspend_threshold`]."]
				pub fn update_suspend_threshold(
					&self,
					new: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::UpdateSuspendThreshold> {
					::subxt::tx::Payload::new_static(
						"XcmpQueue",
						"update_suspend_threshold",
						types::UpdateSuspendThreshold { new },
						[
							64u8, 91u8, 172u8, 51u8, 220u8, 174u8, 54u8, 47u8, 57u8, 89u8, 75u8,
							39u8, 126u8, 198u8, 143u8, 35u8, 70u8, 125u8, 167u8, 14u8, 17u8, 18u8,
							146u8, 222u8, 100u8, 92u8, 81u8, 239u8, 173u8, 43u8, 42u8, 174u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_drop_threshold`]."]
				pub fn update_drop_threshold(
					&self,
					new: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::UpdateDropThreshold> {
					::subxt::tx::Payload::new_static(
						"XcmpQueue",
						"update_drop_threshold",
						types::UpdateDropThreshold { new },
						[
							123u8, 54u8, 12u8, 180u8, 165u8, 198u8, 141u8, 200u8, 149u8, 168u8,
							186u8, 237u8, 162u8, 91u8, 89u8, 242u8, 229u8, 16u8, 32u8, 254u8, 59u8,
							168u8, 31u8, 134u8, 217u8, 251u8, 0u8, 102u8, 113u8, 194u8, 175u8, 9u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_resume_threshold`]."]
				pub fn update_resume_threshold(
					&self,
					new: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::UpdateResumeThreshold> {
					::subxt::tx::Payload::new_static(
						"XcmpQueue",
						"update_resume_threshold",
						types::UpdateResumeThreshold { new },
						[
							172u8, 136u8, 11u8, 106u8, 42u8, 157u8, 167u8, 183u8, 87u8, 62u8,
							182u8, 17u8, 184u8, 59u8, 215u8, 230u8, 18u8, 243u8, 212u8, 34u8, 54u8,
							188u8, 95u8, 119u8, 173u8, 20u8, 91u8, 206u8, 212u8, 57u8, 136u8, 77u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::cumulus_pallet_xcmp_queue::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An HRMP message was sent to a sibling parachain."]
			pub struct XcmpMessageSent {
				pub message_hash: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for XcmpMessageSent {
				const PALLET: &'static str = "XcmpQueue";
				const EVENT: &'static str = "XcmpMessageSent";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The suspended inbound XCMP channels. All others are not suspended."]
				#[doc = ""]
				#[doc = " This is a `StorageValue` instead of a `StorageMap` since we expect multiple reads per block"]
				#[doc = " to different keys with a one byte payload. The access to `BoundedBTreeSet` will be cached"]
				#[doc = " within the block and therefore only included once in the proof size."]
				#[doc = ""]
				#[doc = " NOTE: The PoV benchmarking cannot know this and will over-estimate, but the actual proof"]
				#[doc = " will be smaller."]
				pub fn inbound_xcmp_suspended(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_btree_set::BoundedBTreeSet<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"XcmpQueue",
						"InboundXcmpSuspended",
						vec![],
						[
							110u8, 23u8, 239u8, 104u8, 136u8, 224u8, 179u8, 180u8, 40u8, 159u8,
							54u8, 15u8, 55u8, 111u8, 75u8, 147u8, 131u8, 127u8, 9u8, 57u8, 133u8,
							70u8, 175u8, 181u8, 232u8, 49u8, 13u8, 19u8, 59u8, 151u8, 179u8, 215u8,
						],
					)
				}
				#[doc = " The non-empty XCMP channels in order of becoming non-empty, and the index of the first"]
				#[doc = " and last outbound message. If the two indices are equal, then it indicates an empty"]
				#[doc = " queue and there must be a non-`Ok` `OutboundStatus`. We assume queues grow no greater"]
				#[doc = " than 65535 items. Queue indices for normal messages begin at one; zero is reserved in"]
				#[doc = " case of the need to send a high-priority signal message this block."]
				#[doc = " The bool is true if there is a signal message waiting to be sent."]
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
							181u8, 5u8, 216u8, 176u8, 154u8, 233u8, 116u8, 14u8, 151u8, 1u8, 114u8,
							16u8, 42u8, 20u8, 63u8, 233u8, 79u8, 122u8, 87u8, 255u8, 75u8, 149u8,
							176u8, 106u8, 23u8, 101u8, 228u8, 120u8, 217u8, 167u8, 127u8, 117u8,
						],
					)
				}
				#[doc = " The messages outbound in a given XCMP channel."]
				pub fn outbound_xcmp_messages(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
					>,
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
							156u8, 3u8, 202u8, 175u8, 175u8, 129u8, 38u8, 144u8, 35u8, 59u8, 228u8,
							159u8, 142u8, 25u8, 19u8, 73u8, 73u8, 6u8, 115u8, 19u8, 236u8, 235u8,
							144u8, 172u8, 31u8, 168u8, 24u8, 65u8, 115u8, 95u8, 77u8, 63u8,
						],
					)
				}
				#[doc = " The messages outbound in a given XCMP channel."]
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
							156u8, 3u8, 202u8, 175u8, 175u8, 129u8, 38u8, 144u8, 35u8, 59u8, 228u8,
							159u8, 142u8, 25u8, 19u8, 73u8, 73u8, 6u8, 115u8, 19u8, 236u8, 235u8,
							144u8, 172u8, 31u8, 168u8, 24u8, 65u8, 115u8, 95u8, 77u8, 63u8,
						],
					)
				}
				#[doc = " Any signal messages waiting to be sent."]
				pub fn signal_messages(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
					>,
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
							182u8, 143u8, 233u8, 233u8, 111u8, 137u8, 174u8, 165u8, 166u8, 7u8,
							229u8, 183u8, 99u8, 108u8, 30u8, 162u8, 71u8, 55u8, 122u8, 124u8,
							249u8, 203u8, 142u8, 124u8, 158u8, 213u8, 182u8, 159u8, 206u8, 249u8,
							180u8, 24u8,
						],
					)
				}
				#[doc = " Any signal messages waiting to be sent."]
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
							182u8, 143u8, 233u8, 233u8, 111u8, 137u8, 174u8, 165u8, 166u8, 7u8,
							229u8, 183u8, 99u8, 108u8, 30u8, 162u8, 71u8, 55u8, 122u8, 124u8,
							249u8, 203u8, 142u8, 124u8, 158u8, 213u8, 182u8, 159u8, 206u8, 249u8,
							180u8, 24u8,
						],
					)
				}
				#[doc = " The configuration which controls the dynamics of the outbound queue."]
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
							185u8, 67u8, 247u8, 243u8, 211u8, 232u8, 57u8, 240u8, 237u8, 181u8,
							23u8, 114u8, 215u8, 128u8, 193u8, 1u8, 176u8, 53u8, 110u8, 195u8,
							148u8, 80u8, 187u8, 143u8, 62u8, 30u8, 143u8, 34u8, 248u8, 109u8, 3u8,
							141u8,
						],
					)
				}
				#[doc = " Whether or not the XCMP queue is suspended from executing incoming XCMs or not."]
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
							165u8, 66u8, 105u8, 244u8, 113u8, 43u8, 177u8, 252u8, 212u8, 243u8,
							143u8, 184u8, 87u8, 51u8, 163u8, 104u8, 29u8, 84u8, 119u8, 74u8, 233u8,
							129u8, 203u8, 105u8, 2u8, 101u8, 19u8, 170u8, 69u8, 253u8, 80u8, 132u8,
						],
					)
				}
				#[doc = " The factor to multiply the base delivery fee by."]
				pub fn delivery_fee_factor(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_arithmetic::fixed_point::FixedU128,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"XcmpQueue",
						"DeliveryFeeFactor",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							43u8, 5u8, 63u8, 235u8, 115u8, 155u8, 130u8, 27u8, 75u8, 216u8, 177u8,
							135u8, 203u8, 147u8, 167u8, 95u8, 208u8, 188u8, 25u8, 14u8, 84u8, 63u8,
							116u8, 41u8, 148u8, 110u8, 115u8, 215u8, 196u8, 36u8, 75u8, 102u8,
						],
					)
				}
				#[doc = " The factor to multiply the base delivery fee by."]
				pub fn delivery_fee_factor_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::sp_arithmetic::fixed_point::FixedU128,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"XcmpQueue",
						"DeliveryFeeFactor",
						Vec::new(),
						[
							43u8, 5u8, 63u8, 235u8, 115u8, 155u8, 130u8, 27u8, 75u8, 216u8, 177u8,
							135u8, 203u8, 147u8, 167u8, 95u8, 208u8, 188u8, 25u8, 14u8, 84u8, 63u8,
							116u8, 41u8, 148u8, 110u8, 115u8, 215u8, 196u8, 36u8, 75u8, 102u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The maximum number of inbound XCMP channels that can be suspended simultaneously."]
				#[doc = ""]
				#[doc = " Any further channel suspensions will fail and messages may get dropped without further"]
				#[doc = " notice. Choosing a high value (1000) is okay; the trade-off that is described in"]
				#[doc = " [`InboundXcmpSuspended`] still applies at that scale."]
				pub fn max_inbound_suspended(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"XcmpQueue",
						"MaxInboundSuspended",
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
	pub mod polkadot_xcm {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_xcm::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_xcm::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Send {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Send {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "send";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TeleportAssets {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
					pub fee_asset_item: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for TeleportAssets {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "teleport_assets";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReserveTransferAssets {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
					pub fee_asset_item: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for ReserveTransferAssets {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "reserve_transfer_assets";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Execute {
					pub message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm2>,
					pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				impl ::subxt::blocks::StaticExtrinsic for Execute {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "execute";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceXcmVersion {
					pub location: ::std::boxed::Box<
						runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					>,
					pub version: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceXcmVersion {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "force_xcm_version";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceDefaultXcmVersion {
					pub maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceDefaultXcmVersion {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "force_default_xcm_version";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSubscribeVersionNotify {
					pub location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSubscribeVersionNotify {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "force_subscribe_version_notify";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceUnsubscribeVersionNotify {
					pub location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceUnsubscribeVersionNotify {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "force_unsubscribe_version_notify";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct LimitedReserveTransferAssets {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
					pub fee_asset_item: ::core::primitive::u32,
					pub weight_limit: runtime_types::xcm::v3::WeightLimit,
				}
				impl ::subxt::blocks::StaticExtrinsic for LimitedReserveTransferAssets {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "limited_reserve_transfer_assets";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct LimitedTeleportAssets {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
					pub fee_asset_item: ::core::primitive::u32,
					pub weight_limit: runtime_types::xcm::v3::WeightLimit,
				}
				impl ::subxt::blocks::StaticExtrinsic for LimitedTeleportAssets {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "limited_teleport_assets";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSuspension {
					pub suspended: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSuspension {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "force_suspension";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferAssets {
					pub dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					pub assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
					pub fee_asset_item: ::core::primitive::u32,
					pub weight_limit: runtime_types::xcm::v3::WeightLimit,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferAssets {
					const PALLET: &'static str = "PolkadotXcm";
					const CALL: &'static str = "transfer_assets";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::send`]."]
				pub fn send(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					message: runtime_types::xcm::VersionedXcm,
				) -> ::subxt::tx::Payload<types::Send> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"send",
						types::Send {
							dest: ::std::boxed::Box::new(dest),
							message: ::std::boxed::Box::new(message),
						},
						[
							145u8, 12u8, 109u8, 216u8, 135u8, 198u8, 98u8, 133u8, 12u8, 38u8, 17u8,
							105u8, 67u8, 120u8, 170u8, 96u8, 202u8, 234u8, 96u8, 157u8, 40u8,
							213u8, 72u8, 21u8, 189u8, 220u8, 71u8, 194u8, 227u8, 243u8, 171u8,
							229u8,
						],
					)
				}
				#[doc = "See [`Pallet::teleport_assets`]."]
				pub fn teleport_assets(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					beneficiary: runtime_types::xcm::VersionedMultiLocation,
					assets: runtime_types::xcm::VersionedMultiAssets,
					fee_asset_item: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::TeleportAssets> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"teleport_assets",
						types::TeleportAssets {
							dest: ::std::boxed::Box::new(dest),
							beneficiary: ::std::boxed::Box::new(beneficiary),
							assets: ::std::boxed::Box::new(assets),
							fee_asset_item,
						},
						[
							253u8, 2u8, 191u8, 130u8, 244u8, 135u8, 195u8, 160u8, 160u8, 196u8,
							62u8, 195u8, 157u8, 228u8, 57u8, 59u8, 131u8, 69u8, 125u8, 44u8, 48u8,
							120u8, 176u8, 53u8, 191u8, 69u8, 151u8, 77u8, 245u8, 97u8, 169u8,
							117u8,
						],
					)
				}
				#[doc = "See [`Pallet::reserve_transfer_assets`]."]
				pub fn reserve_transfer_assets(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					beneficiary: runtime_types::xcm::VersionedMultiLocation,
					assets: runtime_types::xcm::VersionedMultiAssets,
					fee_asset_item: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::ReserveTransferAssets> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"reserve_transfer_assets",
						types::ReserveTransferAssets {
							dest: ::std::boxed::Box::new(dest),
							beneficiary: ::std::boxed::Box::new(beneficiary),
							assets: ::std::boxed::Box::new(assets),
							fee_asset_item,
						},
						[
							204u8, 40u8, 10u8, 224u8, 175u8, 80u8, 228u8, 202u8, 250u8, 27u8, 70u8,
							126u8, 166u8, 36u8, 53u8, 125u8, 96u8, 107u8, 121u8, 130u8, 44u8,
							214u8, 174u8, 80u8, 113u8, 45u8, 10u8, 194u8, 179u8, 4u8, 181u8, 70u8,
						],
					)
				}
				#[doc = "See [`Pallet::execute`]."]
				pub fn execute(
					&self,
					message: runtime_types::xcm::VersionedXcm2,
					max_weight: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::tx::Payload<types::Execute> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"execute",
						types::Execute { message: ::std::boxed::Box::new(message), max_weight },
						[
							207u8, 172u8, 34u8, 97u8, 82u8, 80u8, 131u8, 47u8, 27u8, 79u8, 124u8,
							134u8, 97u8, 106u8, 139u8, 230u8, 77u8, 85u8, 109u8, 40u8, 139u8, 56u8,
							234u8, 181u8, 180u8, 31u8, 157u8, 135u8, 75u8, 218u8, 43u8, 157u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_xcm_version`]."]
				pub fn force_xcm_version(
					&self,
					location: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					version: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::ForceXcmVersion> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"force_xcm_version",
						types::ForceXcmVersion {
							location: ::std::boxed::Box::new(location),
							version,
						},
						[
							169u8, 8u8, 18u8, 107u8, 12u8, 180u8, 217u8, 141u8, 188u8, 250u8,
							112u8, 51u8, 122u8, 138u8, 204u8, 44u8, 169u8, 166u8, 36u8, 84u8, 0u8,
							95u8, 138u8, 137u8, 119u8, 226u8, 146u8, 85u8, 53u8, 65u8, 188u8,
							169u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_default_xcm_version`]."]
				pub fn force_default_xcm_version(
					&self,
					maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
				) -> ::subxt::tx::Payload<types::ForceDefaultXcmVersion> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"force_default_xcm_version",
						types::ForceDefaultXcmVersion { maybe_xcm_version },
						[
							43u8, 114u8, 102u8, 104u8, 209u8, 234u8, 108u8, 173u8, 109u8, 188u8,
							94u8, 214u8, 136u8, 43u8, 153u8, 75u8, 161u8, 192u8, 76u8, 12u8, 221u8,
							237u8, 158u8, 247u8, 41u8, 193u8, 35u8, 174u8, 183u8, 207u8, 79u8,
							213u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_subscribe_version_notify`]."]
				pub fn force_subscribe_version_notify(
					&self,
					location: runtime_types::xcm::VersionedMultiLocation,
				) -> ::subxt::tx::Payload<types::ForceSubscribeVersionNotify> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"force_subscribe_version_notify",
						types::ForceSubscribeVersionNotify {
							location: ::std::boxed::Box::new(location),
						},
						[
							79u8, 2u8, 202u8, 173u8, 179u8, 250u8, 48u8, 28u8, 168u8, 183u8, 59u8,
							45u8, 193u8, 132u8, 141u8, 240u8, 65u8, 182u8, 31u8, 85u8, 98u8, 124u8,
							21u8, 169u8, 133u8, 107u8, 221u8, 235u8, 230u8, 56u8, 81u8, 201u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_unsubscribe_version_notify`]."]
				pub fn force_unsubscribe_version_notify(
					&self,
					location: runtime_types::xcm::VersionedMultiLocation,
				) -> ::subxt::tx::Payload<types::ForceUnsubscribeVersionNotify> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"force_unsubscribe_version_notify",
						types::ForceUnsubscribeVersionNotify {
							location: ::std::boxed::Box::new(location),
						},
						[
							144u8, 180u8, 158u8, 178u8, 165u8, 54u8, 105u8, 207u8, 194u8, 105u8,
							190u8, 142u8, 253u8, 133u8, 61u8, 83u8, 199u8, 147u8, 199u8, 69u8,
							144u8, 22u8, 174u8, 138u8, 168u8, 131u8, 241u8, 221u8, 151u8, 148u8,
							129u8, 22u8,
						],
					)
				}
				#[doc = "See [`Pallet::limited_reserve_transfer_assets`]."]
				pub fn limited_reserve_transfer_assets(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					beneficiary: runtime_types::xcm::VersionedMultiLocation,
					assets: runtime_types::xcm::VersionedMultiAssets,
					fee_asset_item: ::core::primitive::u32,
					weight_limit: runtime_types::xcm::v3::WeightLimit,
				) -> ::subxt::tx::Payload<types::LimitedReserveTransferAssets> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"limited_reserve_transfer_assets",
						types::LimitedReserveTransferAssets {
							dest: ::std::boxed::Box::new(dest),
							beneficiary: ::std::boxed::Box::new(beneficiary),
							assets: ::std::boxed::Box::new(assets),
							fee_asset_item,
							weight_limit,
						},
						[
							126u8, 160u8, 23u8, 48u8, 47u8, 7u8, 124u8, 99u8, 32u8, 91u8, 133u8,
							116u8, 204u8, 133u8, 22u8, 216u8, 188u8, 247u8, 10u8, 41u8, 204u8,
							182u8, 38u8, 200u8, 55u8, 117u8, 86u8, 119u8, 225u8, 133u8, 16u8,
							129u8,
						],
					)
				}
				#[doc = "See [`Pallet::limited_teleport_assets`]."]
				pub fn limited_teleport_assets(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					beneficiary: runtime_types::xcm::VersionedMultiLocation,
					assets: runtime_types::xcm::VersionedMultiAssets,
					fee_asset_item: ::core::primitive::u32,
					weight_limit: runtime_types::xcm::v3::WeightLimit,
				) -> ::subxt::tx::Payload<types::LimitedTeleportAssets> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"limited_teleport_assets",
						types::LimitedTeleportAssets {
							dest: ::std::boxed::Box::new(dest),
							beneficiary: ::std::boxed::Box::new(beneficiary),
							assets: ::std::boxed::Box::new(assets),
							fee_asset_item,
							weight_limit,
						},
						[
							58u8, 84u8, 105u8, 56u8, 164u8, 58u8, 78u8, 24u8, 68u8, 83u8, 172u8,
							145u8, 14u8, 85u8, 1u8, 52u8, 188u8, 81u8, 36u8, 60u8, 31u8, 203u8,
							72u8, 195u8, 198u8, 128u8, 116u8, 197u8, 69u8, 15u8, 235u8, 252u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_suspension`]."]
				pub fn force_suspension(
					&self,
					suspended: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::ForceSuspension> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"force_suspension",
						types::ForceSuspension { suspended },
						[
							78u8, 125u8, 93u8, 55u8, 129u8, 44u8, 36u8, 227u8, 75u8, 46u8, 68u8,
							202u8, 81u8, 127u8, 111u8, 92u8, 149u8, 38u8, 225u8, 185u8, 183u8,
							154u8, 89u8, 159u8, 79u8, 10u8, 229u8, 1u8, 226u8, 243u8, 65u8, 238u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_assets`]."]
				pub fn transfer_assets(
					&self,
					dest: runtime_types::xcm::VersionedMultiLocation,
					beneficiary: runtime_types::xcm::VersionedMultiLocation,
					assets: runtime_types::xcm::VersionedMultiAssets,
					fee_asset_item: ::core::primitive::u32,
					weight_limit: runtime_types::xcm::v3::WeightLimit,
				) -> ::subxt::tx::Payload<types::TransferAssets> {
					::subxt::tx::Payload::new_static(
						"PolkadotXcm",
						"transfer_assets",
						types::TransferAssets {
							dest: ::std::boxed::Box::new(dest),
							beneficiary: ::std::boxed::Box::new(beneficiary),
							assets: ::std::boxed::Box::new(assets),
							fee_asset_item,
							weight_limit,
						},
						[
							73u8, 16u8, 172u8, 19u8, 161u8, 78u8, 81u8, 191u8, 123u8, 205u8, 224u8,
							69u8, 27u8, 180u8, 15u8, 230u8, 10u8, 218u8, 61u8, 81u8, 21u8, 192u8,
							16u8, 158u8, 210u8, 186u8, 5u8, 68u8, 24u8, 204u8, 158u8, 171u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_xcm::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Execution of an XCM message was attempted."]
			pub struct Attempted {
				pub outcome: runtime_types::xcm::v3::traits::Outcome,
			}
			impl ::subxt::events::StaticEvent for Attempted {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "Attempted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A XCM message was sent."]
			pub struct Sent {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub message: runtime_types::xcm::v3::Xcm,
				pub message_id: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for Sent {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "Sent";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response received which does not match a registered query. This may be because a"]
			#[doc = "matching query was never registered, it may be because it is a duplicate response, or"]
			#[doc = "because the query timed out."]
			pub struct UnexpectedResponse {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for UnexpectedResponse {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "UnexpectedResponse";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response has been received and is ready for taking with `take_response`. There is"]
			#[doc = "no registered notification call."]
			pub struct ResponseReady {
				pub query_id: ::core::primitive::u64,
				pub response: runtime_types::xcm::v3::Response,
			}
			impl ::subxt::events::StaticEvent for ResponseReady {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "ResponseReady";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response has been received and query is removed. The registered notification has"]
			#[doc = "been dispatched and executed successfully."]
			pub struct Notified {
				pub query_id: ::core::primitive::u64,
				pub pallet_index: ::core::primitive::u8,
				pub call_index: ::core::primitive::u8,
			}
			impl ::subxt::events::StaticEvent for Notified {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "Notified";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response has been received and query is removed. The registered notification"]
			#[doc = "could not be dispatched because the dispatch weight is greater than the maximum weight"]
			#[doc = "originally budgeted by this runtime for the query result."]
			pub struct NotifyOverweight {
				pub query_id: ::core::primitive::u64,
				pub pallet_index: ::core::primitive::u8,
				pub call_index: ::core::primitive::u8,
				pub actual_weight: runtime_types::sp_weights::weight_v2::Weight,
				pub max_budgeted_weight: runtime_types::sp_weights::weight_v2::Weight,
			}
			impl ::subxt::events::StaticEvent for NotifyOverweight {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "NotifyOverweight";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response has been received and query is removed. There was a general error with"]
			#[doc = "dispatching the notification call."]
			pub struct NotifyDispatchError {
				pub query_id: ::core::primitive::u64,
				pub pallet_index: ::core::primitive::u8,
				pub call_index: ::core::primitive::u8,
			}
			impl ::subxt::events::StaticEvent for NotifyDispatchError {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "NotifyDispatchError";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Query response has been received and query is removed. The dispatch was unable to be"]
			#[doc = "decoded into a `Call`; this might be due to dispatch function having a signature which"]
			#[doc = "is not `(origin, QueryId, Response)`."]
			pub struct NotifyDecodeFailed {
				pub query_id: ::core::primitive::u64,
				pub pallet_index: ::core::primitive::u8,
				pub call_index: ::core::primitive::u8,
			}
			impl ::subxt::events::StaticEvent for NotifyDecodeFailed {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "NotifyDecodeFailed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Expected query response has been received but the origin location of the response does"]
			#[doc = "not match that expected. The query remains registered for a later, valid, response to"]
			#[doc = "be received and acted upon."]
			pub struct InvalidResponder {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
				pub expected_location: ::core::option::Option<
					runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				>,
			}
			impl ::subxt::events::StaticEvent for InvalidResponder {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "InvalidResponder";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Expected query response has been received but the expected origin location placed in"]
			#[doc = "storage by this runtime previously cannot be decoded. The query remains registered."]
			#[doc = ""]
			#[doc = "This is unexpected (since a location placed in storage in a previously executing"]
			#[doc = "runtime should be readable prior to query timeout) and dangerous since the possibly"]
			#[doc = "valid response will be dropped. Manual governance intervention is probably going to be"]
			#[doc = "needed."]
			pub struct InvalidResponderVersion {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for InvalidResponderVersion {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "InvalidResponderVersion";
			}
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Received query response has been read and removed."]
			pub struct ResponseTaken {
				pub query_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for ResponseTaken {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "ResponseTaken";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some assets have been placed in an asset trap."]
			pub struct AssetsTrapped {
				pub hash: ::subxt::utils::H256,
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub assets: runtime_types::xcm::VersionedMultiAssets,
			}
			impl ::subxt::events::StaticEvent for AssetsTrapped {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "AssetsTrapped";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An XCM version change notification message has been attempted to be sent."]
			#[doc = ""]
			#[doc = "The cost of sending it (borne by the chain) is included."]
			pub struct VersionChangeNotified {
				pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub result: ::core::primitive::u32,
				pub cost: runtime_types::xcm::v3::multiasset::MultiAssets,
				pub message_id: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for VersionChangeNotified {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "VersionChangeNotified";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The supported version of a location has been changed. This might be through an"]
			#[doc = "automatic notification or a manual intervention."]
			pub struct SupportedVersionChanged {
				pub location: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub version: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for SupportedVersionChanged {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "SupportedVersionChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A given location which had a version change subscription was dropped owing to an error"]
			#[doc = "sending the notification to it."]
			pub struct NotifyTargetSendFail {
				pub location: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
				pub error: runtime_types::xcm::v3::traits::Error,
			}
			impl ::subxt::events::StaticEvent for NotifyTargetSendFail {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "NotifyTargetSendFail";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A given location which had a version change subscription was dropped owing to an error"]
			#[doc = "migrating the location to our new XCM format."]
			pub struct NotifyTargetMigrationFail {
				pub location: runtime_types::xcm::VersionedMultiLocation,
				pub query_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for NotifyTargetMigrationFail {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "NotifyTargetMigrationFail";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Expected query response has been received but the expected querier location placed in"]
			#[doc = "storage by this runtime previously cannot be decoded. The query remains registered."]
			#[doc = ""]
			#[doc = "This is unexpected (since a location placed in storage in a previously executing"]
			#[doc = "runtime should be readable prior to query timeout) and dangerous since the possibly"]
			#[doc = "valid response will be dropped. Manual governance intervention is probably going to be"]
			#[doc = "needed."]
			pub struct InvalidQuerierVersion {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for InvalidQuerierVersion {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "InvalidQuerierVersion";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Expected query response has been received but the querier location of the response does"]
			#[doc = "not match the expected. The query remains registered for a later, valid, response to"]
			#[doc = "be received and acted upon."]
			pub struct InvalidQuerier {
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub query_id: ::core::primitive::u64,
				pub expected_querier: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub maybe_actual_querier: ::core::option::Option<
					runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				>,
			}
			impl ::subxt::events::StaticEvent for InvalidQuerier {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "InvalidQuerier";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A remote has requested XCM version change notification from us and we have honored it."]
			#[doc = "A version information message is sent to them and its cost is included."]
			pub struct VersionNotifyStarted {
				pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub cost: runtime_types::xcm::v3::multiasset::MultiAssets,
				pub message_id: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for VersionNotifyStarted {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "VersionNotifyStarted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "We have requested that a remote chain send us XCM version change notifications."]
			pub struct VersionNotifyRequested {
				pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub cost: runtime_types::xcm::v3::multiasset::MultiAssets,
				pub message_id: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for VersionNotifyRequested {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "VersionNotifyRequested";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "We have requested that a remote chain stops sending us XCM version change"]
			#[doc = "notifications."]
			pub struct VersionNotifyUnrequested {
				pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub cost: runtime_types::xcm::v3::multiasset::MultiAssets,
				pub message_id: [::core::primitive::u8; 32usize],
			}
			impl ::subxt::events::StaticEvent for VersionNotifyUnrequested {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "VersionNotifyUnrequested";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Fees were paid from a location for an operation (often for using `SendXcm`)."]
			pub struct FeesPaid {
				pub paying: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub fees: runtime_types::xcm::v3::multiasset::MultiAssets,
			}
			impl ::subxt::events::StaticEvent for FeesPaid {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "FeesPaid";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some assets have been claimed from an asset trap"]
			pub struct AssetsClaimed {
				pub hash: ::subxt::utils::H256,
				pub origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub assets: runtime_types::xcm::VersionedMultiAssets,
			}
			impl ::subxt::events::StaticEvent for AssetsClaimed {
				const PALLET: &'static str = "PolkadotXcm";
				const EVENT: &'static str = "AssetsClaimed";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The latest available query index."]
				pub fn query_counter(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"QueryCounter",
						vec![],
						[
							216u8, 73u8, 160u8, 232u8, 60u8, 245u8, 218u8, 219u8, 152u8, 68u8,
							146u8, 219u8, 255u8, 7u8, 86u8, 112u8, 83u8, 49u8, 94u8, 173u8, 64u8,
							203u8, 147u8, 226u8, 236u8, 39u8, 129u8, 106u8, 209u8, 113u8, 150u8,
							50u8,
						],
					)
				}
				#[doc = " The ongoing queries."]
				pub fn queries(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u64>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_xcm::pallet::QueryStatus<::core::primitive::u32>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"Queries",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							68u8, 247u8, 165u8, 128u8, 238u8, 192u8, 246u8, 53u8, 248u8, 238u8,
							7u8, 235u8, 89u8, 176u8, 223u8, 130u8, 45u8, 135u8, 235u8, 66u8, 24u8,
							64u8, 116u8, 97u8, 2u8, 139u8, 89u8, 79u8, 87u8, 250u8, 229u8, 46u8,
						],
					)
				}
				#[doc = " The ongoing queries."]
				pub fn queries_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_xcm::pallet::QueryStatus<::core::primitive::u32>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"Queries",
						Vec::new(),
						[
							68u8, 247u8, 165u8, 128u8, 238u8, 192u8, 246u8, 53u8, 248u8, 238u8,
							7u8, 235u8, 89u8, 176u8, 223u8, 130u8, 45u8, 135u8, 235u8, 66u8, 24u8,
							64u8, 116u8, 97u8, 2u8, 139u8, 89u8, 79u8, 87u8, 250u8, 229u8, 46u8,
						],
					)
				}
				#[doc = " The existing asset traps."]
				#[doc = ""]
				#[doc = " Key is the blake2 256 hash of (origin, versioned `MultiAssets`) pair. Value is the number of"]
				#[doc = " times this pair has been trapped (usually just 1 if it exists at all)."]
				pub fn asset_traps(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"AssetTraps",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							148u8, 41u8, 254u8, 134u8, 61u8, 172u8, 126u8, 146u8, 78u8, 178u8,
							50u8, 77u8, 226u8, 8u8, 200u8, 78u8, 77u8, 91u8, 26u8, 133u8, 104u8,
							126u8, 28u8, 28u8, 202u8, 62u8, 87u8, 183u8, 231u8, 191u8, 5u8, 181u8,
						],
					)
				}
				#[doc = " The existing asset traps."]
				#[doc = ""]
				#[doc = " Key is the blake2 256 hash of (origin, versioned `MultiAssets`) pair. Value is the number of"]
				#[doc = " times this pair has been trapped (usually just 1 if it exists at all)."]
				pub fn asset_traps_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"AssetTraps",
						Vec::new(),
						[
							148u8, 41u8, 254u8, 134u8, 61u8, 172u8, 126u8, 146u8, 78u8, 178u8,
							50u8, 77u8, 226u8, 8u8, 200u8, 78u8, 77u8, 91u8, 26u8, 133u8, 104u8,
							126u8, 28u8, 28u8, 202u8, 62u8, 87u8, 183u8, 231u8, 191u8, 5u8, 181u8,
						],
					)
				}
				#[doc = " Default version to encode XCM when latest version of destination is unknown. If `None`,"]
				#[doc = " then the destinations whose XCM version is unknown are considered unreachable."]
				pub fn safe_xcm_version(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"SafeXcmVersion",
						vec![],
						[
							187u8, 8u8, 74u8, 126u8, 80u8, 215u8, 177u8, 60u8, 223u8, 123u8, 196u8,
							155u8, 166u8, 66u8, 25u8, 164u8, 191u8, 66u8, 116u8, 131u8, 116u8,
							188u8, 224u8, 122u8, 75u8, 195u8, 246u8, 188u8, 83u8, 134u8, 49u8,
							143u8,
						],
					)
				}
				#[doc = " The Latest versions that we know various locations support."]
				pub fn supported_version(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<runtime_types::xcm::VersionedMultiLocation>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"SupportedVersion",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							76u8, 134u8, 202u8, 57u8, 10u8, 29u8, 58u8, 88u8, 141u8, 0u8, 189u8,
							133u8, 140u8, 204u8, 126u8, 143u8, 71u8, 186u8, 9u8, 233u8, 188u8,
							74u8, 184u8, 158u8, 40u8, 17u8, 223u8, 129u8, 144u8, 189u8, 211u8,
							194u8,
						],
					)
				}
				#[doc = " The Latest versions that we know various locations support."]
				pub fn supported_version_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"SupportedVersion",
						Vec::new(),
						[
							76u8, 134u8, 202u8, 57u8, 10u8, 29u8, 58u8, 88u8, 141u8, 0u8, 189u8,
							133u8, 140u8, 204u8, 126u8, 143u8, 71u8, 186u8, 9u8, 233u8, 188u8,
							74u8, 184u8, 158u8, 40u8, 17u8, 223u8, 129u8, 144u8, 189u8, 211u8,
							194u8,
						],
					)
				}
				#[doc = " All locations that we have requested version notifications from."]
				pub fn version_notifiers(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<runtime_types::xcm::VersionedMultiLocation>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"VersionNotifiers",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							212u8, 132u8, 8u8, 249u8, 16u8, 112u8, 47u8, 251u8, 73u8, 100u8, 218u8,
							194u8, 197u8, 252u8, 146u8, 31u8, 196u8, 234u8, 187u8, 14u8, 126u8,
							29u8, 7u8, 81u8, 30u8, 11u8, 167u8, 95u8, 46u8, 124u8, 66u8, 184u8,
						],
					)
				}
				#[doc = " All locations that we have requested version notifications from."]
				pub fn version_notifiers_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"VersionNotifiers",
						Vec::new(),
						[
							212u8, 132u8, 8u8, 249u8, 16u8, 112u8, 47u8, 251u8, 73u8, 100u8, 218u8,
							194u8, 197u8, 252u8, 146u8, 31u8, 196u8, 234u8, 187u8, 14u8, 126u8,
							29u8, 7u8, 81u8, 30u8, 11u8, 167u8, 95u8, 46u8, 124u8, 66u8, 184u8,
						],
					)
				}
				#[doc = " The target locations that are subscribed to our version changes, as well as the most recent"]
				#[doc = " of our versions we informed them of."]
				pub fn version_notify_targets(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<runtime_types::xcm::VersionedMultiLocation>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(
						::core::primitive::u64,
						runtime_types::sp_weights::weight_v2::Weight,
						::core::primitive::u32,
					),
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"VersionNotifyTargets",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							147u8, 115u8, 179u8, 91u8, 230u8, 102u8, 121u8, 203u8, 88u8, 26u8,
							69u8, 115u8, 168u8, 176u8, 234u8, 155u8, 200u8, 220u8, 33u8, 49u8,
							199u8, 97u8, 232u8, 9u8, 5u8, 2u8, 144u8, 58u8, 108u8, 236u8, 127u8,
							130u8,
						],
					)
				}
				#[doc = " The target locations that are subscribed to our version changes, as well as the most recent"]
				#[doc = " of our versions we informed them of."]
				pub fn version_notify_targets_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					(
						::core::primitive::u64,
						runtime_types::sp_weights::weight_v2::Weight,
						::core::primitive::u32,
					),
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"VersionNotifyTargets",
						Vec::new(),
						[
							147u8, 115u8, 179u8, 91u8, 230u8, 102u8, 121u8, 203u8, 88u8, 26u8,
							69u8, 115u8, 168u8, 176u8, 234u8, 155u8, 200u8, 220u8, 33u8, 49u8,
							199u8, 97u8, 232u8, 9u8, 5u8, 2u8, 144u8, 58u8, 108u8, 236u8, 127u8,
							130u8,
						],
					)
				}
				#[doc = " Destinations whose latest XCM version we would like to know. Duplicates not allowed, and"]
				#[doc = " the `u32` counter is the number of times that a send to the destination has been attempted,"]
				#[doc = " which is used as a prioritization."]
				pub fn version_discovery_queue(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						runtime_types::xcm::VersionedMultiLocation,
						::core::primitive::u32,
					)>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"VersionDiscoveryQueue",
						vec![],
						[
							139u8, 46u8, 46u8, 247u8, 93u8, 78u8, 193u8, 202u8, 133u8, 178u8,
							213u8, 171u8, 145u8, 167u8, 119u8, 135u8, 255u8, 7u8, 34u8, 90u8,
							208u8, 229u8, 49u8, 119u8, 12u8, 198u8, 46u8, 23u8, 119u8, 192u8,
							255u8, 147u8,
						],
					)
				}
				#[doc = " The current migration's stage, if any."]
				pub fn current_migration(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_xcm::pallet::VersionMigrationStage,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"CurrentMigration",
						vec![],
						[
							74u8, 138u8, 181u8, 162u8, 59u8, 251u8, 37u8, 28u8, 232u8, 51u8, 30u8,
							152u8, 252u8, 133u8, 95u8, 195u8, 47u8, 127u8, 21u8, 44u8, 62u8, 143u8,
							170u8, 234u8, 160u8, 37u8, 131u8, 179u8, 57u8, 241u8, 140u8, 124u8,
						],
					)
				}
				#[doc = " Fungible assets which we know are locked on a remote chain."]
				pub fn remote_locked_fungibles(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
					_1: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
					_2: impl ::std::borrow::Borrow<runtime_types::xcm::VersionedAssetId>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_xcm::pallet::RemoteLockedFungibleRecord<()>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"RemoteLockedFungibles",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_2.borrow()),
						],
						[
							179u8, 227u8, 131u8, 21u8, 47u8, 223u8, 117u8, 21u8, 16u8, 43u8, 225u8,
							132u8, 222u8, 97u8, 158u8, 211u8, 8u8, 26u8, 157u8, 136u8, 92u8, 244u8,
							158u8, 239u8, 94u8, 70u8, 8u8, 132u8, 53u8, 61u8, 212u8, 179u8,
						],
					)
				}
				#[doc = " Fungible assets which we know are locked on a remote chain."]
				pub fn remote_locked_fungibles_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_xcm::pallet::RemoteLockedFungibleRecord<()>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"RemoteLockedFungibles",
						Vec::new(),
						[
							179u8, 227u8, 131u8, 21u8, 47u8, 223u8, 117u8, 21u8, 16u8, 43u8, 225u8,
							132u8, 222u8, 97u8, 158u8, 211u8, 8u8, 26u8, 157u8, 136u8, 92u8, 244u8,
							158u8, 239u8, 94u8, 70u8, 8u8, 132u8, 53u8, 61u8, 212u8, 179u8,
						],
					)
				}
				#[doc = " Fungible assets which we know are locked on this chain."]
				pub fn locked_fungibles(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::core::primitive::u128,
						runtime_types::xcm::VersionedMultiLocation,
					)>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"LockedFungibles",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							209u8, 60u8, 254u8, 70u8, 62u8, 118u8, 99u8, 28u8, 153u8, 68u8, 40u8,
							239u8, 30u8, 90u8, 201u8, 178u8, 219u8, 37u8, 34u8, 153u8, 115u8, 99u8,
							100u8, 240u8, 36u8, 110u8, 8u8, 2u8, 45u8, 58u8, 131u8, 137u8,
						],
					)
				}
				#[doc = " Fungible assets which we know are locked on this chain."]
				pub fn locked_fungibles_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						::core::primitive::u128,
						runtime_types::xcm::VersionedMultiLocation,
					)>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"LockedFungibles",
						Vec::new(),
						[
							209u8, 60u8, 254u8, 70u8, 62u8, 118u8, 99u8, 28u8, 153u8, 68u8, 40u8,
							239u8, 30u8, 90u8, 201u8, 178u8, 219u8, 37u8, 34u8, 153u8, 115u8, 99u8,
							100u8, 240u8, 36u8, 110u8, 8u8, 2u8, 45u8, 58u8, 131u8, 137u8,
						],
					)
				}
				#[doc = " Global suspension state of the XCM executor."]
				pub fn xcm_execution_suspended(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"PolkadotXcm",
						"XcmExecutionSuspended",
						vec![],
						[
							182u8, 54u8, 69u8, 68u8, 78u8, 76u8, 103u8, 79u8, 47u8, 136u8, 99u8,
							104u8, 128u8, 129u8, 249u8, 54u8, 214u8, 136u8, 97u8, 48u8, 178u8,
							42u8, 26u8, 27u8, 82u8, 24u8, 33u8, 77u8, 33u8, 27u8, 20u8, 127u8,
						],
					)
				}
			}
		}
	}
	pub mod cumulus_xcm {
		use super::{root_mod, runtime_types};
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::cumulus_pallet_xcm::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Downward message is invalid XCM."]
			#[doc = "\\[ id \\]"]
			pub struct InvalidFormat(pub [::core::primitive::u8; 32usize]);
			impl ::subxt::events::StaticEvent for InvalidFormat {
				const PALLET: &'static str = "CumulusXcm";
				const EVENT: &'static str = "InvalidFormat";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Downward message is unsupported version of XCM."]
			#[doc = "\\[ id \\]"]
			pub struct UnsupportedVersion(pub [::core::primitive::u8; 32usize]);
			impl ::subxt::events::StaticEvent for UnsupportedVersion {
				const PALLET: &'static str = "CumulusXcm";
				const EVENT: &'static str = "UnsupportedVersion";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Downward message executed with the given outcome."]
			#[doc = "\\[ id, outcome \\]"]
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
	pub mod mmr {
		use super::{root_mod, runtime_types};
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Latest MMR Root hash."]
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
						"Mmr",
						"RootHash",
						vec![],
						[
							111u8, 206u8, 173u8, 92u8, 67u8, 49u8, 150u8, 113u8, 90u8, 245u8, 38u8,
							254u8, 76u8, 250u8, 167u8, 66u8, 130u8, 129u8, 251u8, 220u8, 172u8,
							229u8, 162u8, 251u8, 36u8, 227u8, 43u8, 189u8, 7u8, 106u8, 23u8, 13u8,
						],
					)
				}
				#[doc = " Current size of the MMR (number of leaves)."]
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
						"Mmr",
						"NumberOfLeaves",
						vec![],
						[
							123u8, 58u8, 149u8, 174u8, 85u8, 45u8, 20u8, 115u8, 241u8, 0u8, 51u8,
							174u8, 234u8, 60u8, 230u8, 59u8, 237u8, 144u8, 170u8, 32u8, 4u8, 0u8,
							34u8, 163u8, 238u8, 205u8, 93u8, 208u8, 53u8, 38u8, 141u8, 195u8,
						],
					)
				}
				#[doc = " Height at which the pallet started inserting leaves into offchain storage."]
				pub fn initial_height(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mmr",
						"InitialHeight",
						vec![],
						[
							67u8, 88u8, 206u8, 192u8, 83u8, 144u8, 233u8, 210u8, 232u8, 59u8,
							203u8, 131u8, 215u8, 66u8, 14u8, 157u8, 152u8, 58u8, 100u8, 175u8,
							140u8, 211u8, 14u8, 72u8, 164u8, 116u8, 68u8, 19u8, 182u8, 39u8, 6u8,
							172u8,
						],
					)
				}
				#[doc = " Temporary leaf storage for while the block is still executing."]
				pub fn intermediate_leaves(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u64>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_ismp::mmr::Leaf,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Mmr",
						"IntermediateLeaves",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							222u8, 45u8, 165u8, 55u8, 14u8, 85u8, 78u8, 242u8, 11u8, 44u8, 255u8,
							47u8, 112u8, 59u8, 26u8, 87u8, 194u8, 209u8, 223u8, 199u8, 39u8, 122u8,
							177u8, 100u8, 112u8, 167u8, 54u8, 66u8, 180u8, 127u8, 104u8, 122u8,
						],
					)
				}
				#[doc = " Temporary leaf storage for while the block is still executing."]
				pub fn intermediate_leaves_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_ismp::mmr::Leaf,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Mmr",
						"IntermediateLeaves",
						Vec::new(),
						[
							222u8, 45u8, 165u8, 55u8, 14u8, 85u8, 78u8, 242u8, 11u8, 44u8, 255u8,
							47u8, 112u8, 59u8, 26u8, 87u8, 194u8, 209u8, 223u8, 199u8, 39u8, 122u8,
							177u8, 100u8, 112u8, 167u8, 54u8, 66u8, 180u8, 127u8, 104u8, 122u8,
						],
					)
				}
				#[doc = "Counter for the related counted storage map"]
				pub fn counter_for_intermediate_leaves(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mmr",
						"CounterForIntermediateLeaves",
						vec![],
						[
							5u8, 41u8, 150u8, 31u8, 198u8, 235u8, 232u8, 184u8, 57u8, 45u8, 107u8,
							139u8, 204u8, 104u8, 99u8, 171u8, 148u8, 230u8, 234u8, 104u8, 61u8,
							159u8, 87u8, 200u8, 138u8, 40u8, 123u8, 108u8, 248u8, 226u8, 14u8,
							185u8,
						],
					)
				}
				#[doc = " Hashes of the nodes in the MMR."]
				#[doc = ""]
				#[doc = " Note this collection only contains MMR peaks, the inner nodes (and leaves)"]
				#[doc = " are pruned and only stored in the Offchain DB."]
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
						"Mmr",
						"Nodes",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							27u8, 84u8, 41u8, 195u8, 146u8, 81u8, 211u8, 189u8, 63u8, 125u8, 173u8,
							206u8, 69u8, 198u8, 202u8, 213u8, 89u8, 31u8, 89u8, 177u8, 76u8, 154u8,
							249u8, 197u8, 133u8, 78u8, 142u8, 71u8, 183u8, 3u8, 132u8, 25u8,
						],
					)
				}
				#[doc = " Hashes of the nodes in the MMR."]
				#[doc = ""]
				#[doc = " Note this collection only contains MMR peaks, the inner nodes (and leaves)"]
				#[doc = " are pruned and only stored in the Offchain DB."]
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
						"Mmr",
						"Nodes",
						Vec::new(),
						[
							27u8, 84u8, 41u8, 195u8, 146u8, 81u8, 211u8, 189u8, 63u8, 125u8, 173u8,
							206u8, 69u8, 198u8, 202u8, 213u8, 89u8, 31u8, 89u8, 177u8, 76u8, 154u8,
							249u8, 197u8, 133u8, 78u8, 142u8, 71u8, 183u8, 3u8, 132u8, 25u8,
						],
					)
				}
				#[doc = "Counter for the related counted storage map"]
				pub fn counter_for_nodes(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u32,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Mmr",
						"CounterForNodes",
						vec![],
						[
							43u8, 207u8, 89u8, 208u8, 61u8, 32u8, 136u8, 156u8, 10u8, 31u8, 25u8,
							125u8, 208u8, 48u8, 145u8, 54u8, 155u8, 28u8, 243u8, 159u8, 214u8,
							195u8, 13u8, 219u8, 158u8, 46u8, 174u8, 61u8, 85u8, 48u8, 182u8, 35u8,
						],
					)
				}
			}
		}
	}
	pub mod ismp {
		use super::{root_mod, runtime_types};
		#[doc = "Pallet errors"]
		pub type Error = runtime_types::pallet_ismp::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_ismp::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct HandleUnsigned {
					pub messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message>,
				}
				impl ::subxt::blocks::StaticExtrinsic for HandleUnsigned {
					const PALLET: &'static str = "Ismp";
					const CALL: &'static str = "handle_unsigned";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateConsensusClient {
					pub message: runtime_types::ismp::messaging::CreateConsensusState,
				}
				impl ::subxt::blocks::StaticExtrinsic for CreateConsensusClient {
					const PALLET: &'static str = "Ismp";
					const CALL: &'static str = "create_consensus_client";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateConsensusState {
					pub message: runtime_types::pallet_ismp::utils::UpdateConsensusState,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateConsensusState {
					const PALLET: &'static str = "Ismp";
					const CALL: &'static str = "update_consensus_state";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FundMessage {
					pub message: runtime_types::pallet_ismp::utils::FundMessageParams<
						::core::primitive::u128,
					>,
				}
				impl ::subxt::blocks::StaticExtrinsic for FundMessage {
					const PALLET: &'static str = "Ismp";
					const CALL: &'static str = "fund_message";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::handle_unsigned`]."]
				pub fn handle_unsigned(
					&self,
					messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message>,
				) -> ::subxt::tx::Payload<types::HandleUnsigned> {
					::subxt::tx::Payload::new_static(
						"Ismp",
						"handle_unsigned",
						types::HandleUnsigned { messages },
						[
							197u8, 89u8, 197u8, 57u8, 62u8, 108u8, 239u8, 217u8, 36u8, 190u8, 86u8,
							48u8, 164u8, 4u8, 114u8, 184u8, 30u8, 114u8, 98u8, 88u8, 53u8, 42u8,
							104u8, 20u8, 81u8, 179u8, 211u8, 187u8, 76u8, 130u8, 94u8, 121u8,
						],
					)
				}
				#[doc = "See [`Pallet::create_consensus_client`]."]
				pub fn create_consensus_client(
					&self,
					message: runtime_types::ismp::messaging::CreateConsensusState,
				) -> ::subxt::tx::Payload<types::CreateConsensusClient> {
					::subxt::tx::Payload::new_static(
						"Ismp",
						"create_consensus_client",
						types::CreateConsensusClient { message },
						[
							25u8, 24u8, 99u8, 172u8, 178u8, 235u8, 121u8, 24u8, 122u8, 12u8, 137u8,
							190u8, 255u8, 37u8, 221u8, 253u8, 14u8, 102u8, 165u8, 108u8, 63u8,
							215u8, 176u8, 39u8, 136u8, 148u8, 28u8, 67u8, 230u8, 200u8, 173u8,
							223u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_consensus_state`]."]
				pub fn update_consensus_state(
					&self,
					message: runtime_types::pallet_ismp::utils::UpdateConsensusState,
				) -> ::subxt::tx::Payload<types::UpdateConsensusState> {
					::subxt::tx::Payload::new_static(
						"Ismp",
						"update_consensus_state",
						types::UpdateConsensusState { message },
						[
							136u8, 203u8, 19u8, 127u8, 243u8, 197u8, 201u8, 231u8, 224u8, 255u8,
							148u8, 220u8, 134u8, 114u8, 5u8, 196u8, 145u8, 251u8, 6u8, 246u8,
							245u8, 215u8, 172u8, 124u8, 30u8, 12u8, 42u8, 151u8, 79u8, 140u8, 49u8,
							207u8,
						],
					)
				}
				#[doc = "See [`Pallet::fund_message`]."]
				pub fn fund_message(
					&self,
					message: runtime_types::pallet_ismp::utils::FundMessageParams<
						::core::primitive::u128,
					>,
				) -> ::subxt::tx::Payload<types::FundMessage> {
					::subxt::tx::Payload::new_static(
						"Ismp",
						"fund_message",
						types::FundMessage { message },
						[
							61u8, 68u8, 29u8, 251u8, 220u8, 212u8, 80u8, 45u8, 65u8, 205u8, 111u8,
							91u8, 88u8, 95u8, 61u8, 160u8, 27u8, 153u8, 38u8, 7u8, 247u8, 94u8,
							214u8, 216u8, 204u8, 237u8, 208u8, 216u8, 3u8, 178u8, 53u8, 6u8,
						],
					)
				}
			}
		}
		#[doc = "Pallet Events"]
		pub type Event = runtime_types::pallet_ismp::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Emitted when a state machine is successfully updated to a new height"]
			pub struct StateMachineUpdated {
				pub state_machine_id: runtime_types::ismp::consensus::StateMachineId,
				pub latest_height: ::core::primitive::u64,
			}
			impl ::subxt::events::StaticEvent for StateMachineUpdated {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "StateMachineUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Emitted when a state commitment is vetoed by a fisherman"]
			pub struct StateCommitmentVetoed {
				pub height: runtime_types::ismp::consensus::StateMachineHeight,
				pub fisherman: runtime_types::bounded_collections::bounded_vec::BoundedVec<
					::core::primitive::u8,
				>,
			}
			impl ::subxt::events::StaticEvent for StateCommitmentVetoed {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "StateCommitmentVetoed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Indicates that a consensus client has been created"]
			pub struct ConsensusClientCreated {
				pub consensus_client_id: [::core::primitive::u8; 4usize],
			}
			impl ::subxt::events::StaticEvent for ConsensusClientCreated {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "ConsensusClientCreated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Indicates that a consensus client has been created"]
			pub struct ConsensusClientFrozen {
				pub consensus_client_id: [::core::primitive::u8; 4usize],
			}
			impl ::subxt::events::StaticEvent for ConsensusClientFrozen {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "ConsensusClientFrozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An Outgoing Response has been deposited"]
			pub struct Response {
				pub dest_chain: runtime_types::ismp::host::StateMachine,
				pub source_chain: runtime_types::ismp::host::StateMachine,
				pub request_nonce: ::core::primitive::u64,
				pub commitment: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for Response {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "Response";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An Outgoing Request has been deposited"]
			pub struct Request {
				pub dest_chain: runtime_types::ismp::host::StateMachine,
				pub source_chain: runtime_types::ismp::host::StateMachine,
				pub request_nonce: ::core::primitive::u64,
				pub commitment: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for Request {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "Request";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some errors handling some ismp messages"]
			pub struct Errors {
				pub errors: ::std::vec::Vec<runtime_types::pallet_ismp::errors::HandlingError>,
			}
			impl ::subxt::events::StaticEvent for Errors {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "Errors";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Post Request Handled"]
			pub struct PostRequestHandled(pub runtime_types::ismp::events::RequestResponseHandled);
			impl ::subxt::events::StaticEvent for PostRequestHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "PostRequestHandled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Post Response Handled"]
			pub struct PostResponseHandled(pub runtime_types::ismp::events::RequestResponseHandled);
			impl ::subxt::events::StaticEvent for PostResponseHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "PostResponseHandled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Get Response Handled"]
			pub struct GetRequestHandled(pub runtime_types::ismp::events::RequestResponseHandled);
			impl ::subxt::events::StaticEvent for GetRequestHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "GetRequestHandled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Post request timeout handled"]
			pub struct PostRequestTimeoutHandled(pub runtime_types::ismp::events::TimeoutHandled);
			impl ::subxt::events::StaticEvent for PostRequestTimeoutHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "PostRequestTimeoutHandled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Post response timeout handled"]
			pub struct PostResponseTimeoutHandled(pub runtime_types::ismp::events::TimeoutHandled);
			impl ::subxt::events::StaticEvent for PostResponseTimeoutHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "PostResponseTimeoutHandled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Get request timeout handled"]
			pub struct GetRequestTimeoutHandled(pub runtime_types::ismp::events::TimeoutHandled);
			impl ::subxt::events::StaticEvent for GetRequestTimeoutHandled {
				const PALLET: &'static str = "Ismp";
				const EVENT: &'static str = "GetRequestTimeoutHandled";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Holds a map of state machine heights to their verified state commitments. These state"]
				#[doc = " commitments end up here after they are successfully verified by a `ConsensusClient`"]
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
							3u8, 93u8, 41u8, 168u8, 235u8, 142u8, 6u8, 38u8, 114u8, 69u8, 207u8,
							195u8, 37u8, 193u8, 128u8, 226u8, 6u8, 175u8, 6u8, 220u8, 234u8, 130u8,
							232u8, 157u8, 143u8, 225u8, 111u8, 211u8, 184u8, 65u8, 82u8, 188u8,
						],
					)
				}
				#[doc = " Holds a map of state machine heights to their verified state commitments. These state"]
				#[doc = " commitments end up here after they are successfully verified by a `ConsensusClient`"]
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
							3u8, 93u8, 41u8, 168u8, 235u8, 142u8, 6u8, 38u8, 114u8, 69u8, 207u8,
							195u8, 37u8, 193u8, 128u8, 226u8, 6u8, 175u8, 6u8, 220u8, 234u8, 130u8,
							232u8, 157u8, 143u8, 225u8, 111u8, 211u8, 184u8, 65u8, 82u8, 188u8,
						],
					)
				}
				#[doc = " Holds a map of consensus state identifiers to their consensus state."]
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
							93u8, 68u8, 6u8, 50u8, 68u8, 143u8, 143u8, 137u8, 62u8, 219u8, 174u8,
							84u8, 44u8, 166u8, 180u8, 168u8, 8u8, 120u8, 199u8, 50u8, 79u8, 33u8,
							35u8, 90u8, 101u8, 246u8, 125u8, 197u8, 18u8, 116u8, 110u8, 178u8,
						],
					)
				}
				#[doc = " Holds a map of consensus state identifiers to their consensus state."]
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
							93u8, 68u8, 6u8, 50u8, 68u8, 143u8, 143u8, 137u8, 62u8, 219u8, 174u8,
							84u8, 44u8, 166u8, 180u8, 168u8, 8u8, 120u8, 199u8, 50u8, 79u8, 33u8,
							35u8, 90u8, 101u8, 246u8, 125u8, 197u8, 18u8, 116u8, 110u8, 178u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifier to it's associated consensus client identifier"]
				pub fn consensus_state_client(
					&self,
					_0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					[::core::primitive::u8; 4usize],
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"ConsensusStateClient",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							63u8, 119u8, 17u8, 2u8, 193u8, 194u8, 243u8, 241u8, 152u8, 164u8,
							250u8, 200u8, 176u8, 51u8, 213u8, 116u8, 198u8, 216u8, 25u8, 7u8, 31u8,
							254u8, 100u8, 157u8, 144u8, 239u8, 89u8, 14u8, 160u8, 194u8, 0u8, 21u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifier to it's associated consensus client identifier"]
				pub fn consensus_state_client_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					[::core::primitive::u8; 4usize],
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"ConsensusStateClient",
						Vec::new(),
						[
							63u8, 119u8, 17u8, 2u8, 193u8, 194u8, 243u8, 241u8, 152u8, 164u8,
							250u8, 200u8, 176u8, 51u8, 213u8, 116u8, 198u8, 216u8, 25u8, 7u8, 31u8,
							254u8, 100u8, 157u8, 144u8, 239u8, 89u8, 14u8, 160u8, 194u8, 0u8, 21u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifiers to their unbonding periods"]
				pub fn unbonding_period(
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
						"UnbondingPeriod",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							47u8, 119u8, 19u8, 162u8, 154u8, 45u8, 45u8, 73u8, 200u8, 98u8, 171u8,
							157u8, 161u8, 23u8, 201u8, 49u8, 30u8, 123u8, 127u8, 187u8, 212u8,
							220u8, 121u8, 120u8, 94u8, 16u8, 20u8, 28u8, 105u8, 22u8, 57u8, 103u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifiers to their unbonding periods"]
				pub fn unbonding_period_root(
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
						"UnbondingPeriod",
						Vec::new(),
						[
							47u8, 119u8, 19u8, 162u8, 154u8, 45u8, 45u8, 73u8, 200u8, 98u8, 171u8,
							157u8, 161u8, 23u8, 201u8, 49u8, 30u8, 123u8, 127u8, 187u8, 212u8,
							220u8, 121u8, 120u8, 94u8, 16u8, 20u8, 28u8, 105u8, 22u8, 57u8, 103u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifiers to their challenge periods"]
				pub fn challenge_period(
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
						"ChallengePeriod",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							75u8, 63u8, 55u8, 10u8, 160u8, 71u8, 235u8, 8u8, 108u8, 134u8, 55u8,
							34u8, 243u8, 61u8, 11u8, 68u8, 118u8, 188u8, 89u8, 221u8, 246u8, 196u8,
							216u8, 110u8, 62u8, 40u8, 239u8, 50u8, 122u8, 35u8, 93u8, 245u8,
						],
					)
				}
				#[doc = " A mapping of consensus state identifiers to their challenge periods"]
				pub fn challenge_period_root(
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
						"ChallengePeriod",
						Vec::new(),
						[
							75u8, 63u8, 55u8, 10u8, 160u8, 71u8, 235u8, 8u8, 108u8, 134u8, 55u8,
							34u8, 243u8, 61u8, 11u8, 68u8, 118u8, 188u8, 89u8, 221u8, 246u8, 196u8,
							216u8, 110u8, 62u8, 40u8, 239u8, 50u8, 122u8, 35u8, 93u8, 245u8,
						],
					)
				}
				#[doc = " Holds a map of consensus clients frozen due to byzantine"]
				#[doc = " behaviour"]
				pub fn frozen_consensus_clients(
					&self,
					_0: impl ::std::borrow::Borrow<[::core::primitive::u8; 4usize]>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"FrozenConsensusClients",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							91u8, 246u8, 143u8, 73u8, 69u8, 255u8, 61u8, 108u8, 130u8, 177u8,
							160u8, 25u8, 77u8, 135u8, 2u8, 137u8, 36u8, 57u8, 44u8, 86u8, 124u8,
							111u8, 153u8, 170u8, 73u8, 22u8, 16u8, 169u8, 218u8, 157u8, 146u8,
							143u8,
						],
					)
				}
				#[doc = " Holds a map of consensus clients frozen due to byzantine"]
				#[doc = " behaviour"]
				pub fn frozen_consensus_clients_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"FrozenConsensusClients",
						Vec::new(),
						[
							91u8, 246u8, 143u8, 73u8, 69u8, 255u8, 61u8, 108u8, 130u8, 177u8,
							160u8, 25u8, 77u8, 135u8, 2u8, 137u8, 36u8, 57u8, 44u8, 86u8, 124u8,
							111u8, 153u8, 170u8, 73u8, 22u8, 16u8, 169u8, 218u8, 157u8, 146u8,
							143u8,
						],
					)
				}
				#[doc = " The latest verified height for a state machine"]
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
							179u8, 207u8, 243u8, 232u8, 166u8, 136u8, 69u8, 151u8, 78u8, 27u8,
							56u8, 203u8, 95u8, 26u8, 137u8, 235u8, 95u8, 135u8, 77u8, 80u8, 219u8,
							141u8, 70u8, 17u8, 85u8, 15u8, 237u8, 203u8, 3u8, 52u8, 239u8, 140u8,
						],
					)
				}
				#[doc = " The latest verified height for a state machine"]
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
							179u8, 207u8, 243u8, 232u8, 166u8, 136u8, 69u8, 151u8, 78u8, 27u8,
							56u8, 203u8, 95u8, 26u8, 137u8, 235u8, 95u8, 135u8, 77u8, 80u8, 219u8,
							141u8, 70u8, 17u8, 85u8, 15u8, 237u8, 203u8, 3u8, 52u8, 239u8, 140u8,
						],
					)
				}
				#[doc = " Holds the timestamp at which a consensus client was recently updated."]
				#[doc = " Used in ensuring that the configured challenge period elapses."]
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
							87u8, 226u8, 222u8, 152u8, 112u8, 144u8, 222u8, 120u8, 37u8, 135u8,
							245u8, 229u8, 180u8, 162u8, 244u8, 167u8, 123u8, 190u8, 80u8, 99u8,
							234u8, 205u8, 118u8, 196u8, 21u8, 20u8, 222u8, 87u8, 144u8, 83u8,
							154u8, 102u8,
						],
					)
				}
				#[doc = " Holds the timestamp at which a consensus client was recently updated."]
				#[doc = " Used in ensuring that the configured challenge period elapses."]
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
							87u8, 226u8, 222u8, 152u8, 112u8, 144u8, 222u8, 120u8, 37u8, 135u8,
							245u8, 229u8, 180u8, 162u8, 244u8, 167u8, 123u8, 190u8, 80u8, 99u8,
							234u8, 205u8, 118u8, 196u8, 21u8, 20u8, 222u8, 87u8, 144u8, 83u8,
							154u8, 102u8,
						],
					)
				}
				#[doc = " Holds the timestamp at which a state machine height was updated."]
				#[doc = " Used in ensuring that the configured challenge period elapses."]
				pub fn state_machine_update_time(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::ismp::consensus::StateMachineHeight>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"StateMachineUpdateTime",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							24u8, 223u8, 143u8, 164u8, 183u8, 133u8, 255u8, 23u8, 120u8, 254u8,
							107u8, 164u8, 72u8, 169u8, 127u8, 46u8, 58u8, 249u8, 110u8, 82u8,
							200u8, 239u8, 6u8, 172u8, 135u8, 217u8, 250u8, 192u8, 165u8, 172u8,
							219u8, 199u8,
						],
					)
				}
				#[doc = " Holds the timestamp at which a state machine height was updated."]
				#[doc = " Used in ensuring that the configured challenge period elapses."]
				pub fn state_machine_update_time_root(
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
						"StateMachineUpdateTime",
						Vec::new(),
						[
							24u8, 223u8, 143u8, 164u8, 183u8, 133u8, 255u8, 23u8, 120u8, 254u8,
							107u8, 164u8, 72u8, 169u8, 127u8, 46u8, 58u8, 249u8, 110u8, 82u8,
							200u8, 239u8, 6u8, 172u8, 135u8, 217u8, 250u8, 192u8, 165u8, 172u8,
							219u8, 199u8,
						],
					)
				}
				#[doc = " Tracks requests that have been responded to"]
				#[doc = " The key is the request commitment"]
				pub fn responded(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"Responded",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							151u8, 204u8, 21u8, 237u8, 146u8, 5u8, 22u8, 175u8, 101u8, 164u8,
							203u8, 66u8, 248u8, 97u8, 70u8, 11u8, 20u8, 219u8, 9u8, 164u8, 145u8,
							66u8, 83u8, 157u8, 34u8, 19u8, 127u8, 16u8, 252u8, 59u8, 194u8, 24u8,
						],
					)
				}
				#[doc = " Tracks requests that have been responded to"]
				#[doc = " The key is the request commitment"]
				pub fn responded_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Ismp",
						"Responded",
						Vec::new(),
						[
							151u8, 204u8, 21u8, 237u8, 146u8, 5u8, 22u8, 175u8, 101u8, 164u8,
							203u8, 66u8, 248u8, 97u8, 70u8, 11u8, 20u8, 219u8, 9u8, 164u8, 145u8,
							66u8, 83u8, 157u8, 34u8, 19u8, 127u8, 16u8, 252u8, 59u8, 194u8, 24u8,
						],
					)
				}
				#[doc = " Latest nonce for messages sent from this chain"]
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
							47u8, 101u8, 89u8, 252u8, 98u8, 25u8, 178u8, 154u8, 17u8, 57u8, 185u8,
							10u8, 133u8, 94u8, 73u8, 160u8, 137u8, 150u8, 97u8, 119u8, 8u8, 146u8,
							149u8, 146u8, 212u8, 60u8, 141u8, 24u8, 124u8, 28u8, 57u8, 19u8,
						],
					)
				}
				#[doc = " The child trie root of messages"]
				pub fn child_trie_root(
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
						"ChildTrieRoot",
						vec![],
						[
							34u8, 16u8, 226u8, 194u8, 6u8, 122u8, 213u8, 244u8, 234u8, 160u8,
							196u8, 243u8, 17u8, 59u8, 90u8, 186u8, 107u8, 66u8, 22u8, 129u8, 161u8,
							226u8, 111u8, 122u8, 207u8, 173u8, 11u8, 207u8, 202u8, 187u8, 140u8,
							116u8,
						],
					)
				}
			}
		}
	}
	pub mod message_queue {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_message_queue::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_message_queue::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReapPage {
					pub message_origin:
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					pub page_index: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for ReapPage {
					const PALLET: &'static str = "MessageQueue";
					const CALL: &'static str = "reap_page";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExecuteOverweight {
					pub message_origin:
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					pub page: ::core::primitive::u32,
					pub index: ::core::primitive::u32,
					pub weight_limit: runtime_types::sp_weights::weight_v2::Weight,
				}
				impl ::subxt::blocks::StaticExtrinsic for ExecuteOverweight {
					const PALLET: &'static str = "MessageQueue";
					const CALL: &'static str = "execute_overweight";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::reap_page`]."]
				pub fn reap_page(
					&self,
					message_origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					page_index: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::ReapPage> {
					::subxt::tx::Payload::new_static(
						"MessageQueue",
						"reap_page",
						types::ReapPage { message_origin, page_index },
						[
							116u8, 17u8, 120u8, 238u8, 117u8, 222u8, 10u8, 166u8, 132u8, 181u8,
							114u8, 150u8, 242u8, 202u8, 31u8, 143u8, 212u8, 65u8, 145u8, 249u8,
							27u8, 204u8, 137u8, 133u8, 220u8, 187u8, 137u8, 90u8, 112u8, 55u8,
							104u8, 163u8,
						],
					)
				}
				#[doc = "See [`Pallet::execute_overweight`]."]
				pub fn execute_overweight(
					&self,
					message_origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					page: ::core::primitive::u32,
					index: ::core::primitive::u32,
					weight_limit: runtime_types::sp_weights::weight_v2::Weight,
				) -> ::subxt::tx::Payload<types::ExecuteOverweight> {
					::subxt::tx::Payload::new_static(
						"MessageQueue",
						"execute_overweight",
						types::ExecuteOverweight { message_origin, page, index, weight_limit },
						[
							177u8, 54u8, 82u8, 58u8, 94u8, 125u8, 241u8, 172u8, 52u8, 7u8, 236u8,
							80u8, 66u8, 99u8, 42u8, 199u8, 38u8, 195u8, 65u8, 118u8, 166u8, 246u8,
							239u8, 195u8, 144u8, 153u8, 155u8, 8u8, 224u8, 56u8, 106u8, 135u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_message_queue::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Message discarded due to an error in the `MessageProcessor` (usually a format error)."]
			pub struct ProcessingFailed {
				pub id: ::subxt::utils::H256,
				pub origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
				pub error: runtime_types::frame_support::traits::messages::ProcessMessageError,
			}
			impl ::subxt::events::StaticEvent for ProcessingFailed {
				const PALLET: &'static str = "MessageQueue";
				const EVENT: &'static str = "ProcessingFailed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Message is processed."]
			pub struct Processed {
				pub id: ::subxt::utils::H256,
				pub origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
				pub weight_used: runtime_types::sp_weights::weight_v2::Weight,
				pub success: ::core::primitive::bool,
			}
			impl ::subxt::events::StaticEvent for Processed {
				const PALLET: &'static str = "MessageQueue";
				const EVENT: &'static str = "Processed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Message placed in overweight queue."]
			pub struct OverweightEnqueued {
				pub id: [::core::primitive::u8; 32usize],
				pub origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
				pub page_index: ::core::primitive::u32,
				pub message_index: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for OverweightEnqueued {
				const PALLET: &'static str = "MessageQueue";
				const EVENT: &'static str = "OverweightEnqueued";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "This page was reaped."]
			pub struct PageReaped {
				pub origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
				pub index: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for PageReaped {
				const PALLET: &'static str = "MessageQueue";
				const EVENT: &'static str = "PageReaped";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " The index of the first and last (non-empty) pages."]
				pub fn book_state_for(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_message_queue::BookState<
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MessageQueue",
						"BookStateFor",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							33u8, 240u8, 235u8, 59u8, 150u8, 42u8, 91u8, 248u8, 235u8, 52u8, 170u8,
							52u8, 195u8, 129u8, 6u8, 174u8, 57u8, 242u8, 30u8, 220u8, 232u8, 4u8,
							246u8, 218u8, 162u8, 174u8, 102u8, 95u8, 210u8, 92u8, 133u8, 143u8,
						],
					)
				}
				#[doc = " The index of the first and last (non-empty) pages."]
				pub fn book_state_for_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_message_queue::BookState<
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MessageQueue",
						"BookStateFor",
						Vec::new(),
						[
							33u8, 240u8, 235u8, 59u8, 150u8, 42u8, 91u8, 248u8, 235u8, 52u8, 170u8,
							52u8, 195u8, 129u8, 6u8, 174u8, 57u8, 242u8, 30u8, 220u8, 232u8, 4u8,
							246u8, 218u8, 162u8, 174u8, 102u8, 95u8, 210u8, 92u8, 133u8, 143u8,
						],
					)
				}
				#[doc = " The origin at which we should begin servicing."]
				pub fn service_head(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"MessageQueue",
						"ServiceHead",
						vec![],
						[
							104u8, 146u8, 240u8, 41u8, 171u8, 68u8, 20u8, 147u8, 212u8, 155u8,
							59u8, 39u8, 174u8, 186u8, 97u8, 250u8, 41u8, 247u8, 67u8, 190u8, 252u8,
							167u8, 234u8, 36u8, 124u8, 239u8, 163u8, 72u8, 223u8, 82u8, 82u8,
							171u8,
						],
					)
				}
				#[doc = " The map of page indices to pages."]
				pub fn pages(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
					>,
					_1: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_message_queue::Page<::core::primitive::u32>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MessageQueue",
						"Pages",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							45u8, 202u8, 18u8, 128u8, 31u8, 194u8, 175u8, 173u8, 99u8, 81u8, 161u8,
							44u8, 32u8, 183u8, 238u8, 181u8, 110u8, 240u8, 203u8, 12u8, 152u8,
							58u8, 239u8, 190u8, 144u8, 168u8, 210u8, 33u8, 121u8, 250u8, 137u8,
							142u8,
						],
					)
				}
				#[doc = " The map of page indices to pages."]
				pub fn pages_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_message_queue::Page<::core::primitive::u32>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"MessageQueue",
						"Pages",
						Vec::new(),
						[
							45u8, 202u8, 18u8, 128u8, 31u8, 194u8, 175u8, 173u8, 99u8, 81u8, 161u8,
							44u8, 32u8, 183u8, 238u8, 181u8, 110u8, 240u8, 203u8, 12u8, 152u8,
							58u8, 239u8, 190u8, 144u8, 168u8, 210u8, 33u8, 121u8, 250u8, 137u8,
							142u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The size of the page; this implies the maximum message size which can be sent."]
				#[doc = ""]
				#[doc = " A good value depends on the expected message sizes, their weights, the weight that is"]
				#[doc = " available for processing them and the maximal needed message size. The maximal message"]
				#[doc = " size is slightly lower than this as defined by [`MaxMessageLenOf`]."]
				pub fn heap_size(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MessageQueue",
						"HeapSize",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The maximum number of stale pages (i.e. of overweight messages) allowed before culling"]
				#[doc = " can happen. Once there are more stale pages than this, then historical pages may be"]
				#[doc = " dropped, even if they contain unprocessed overweight messages."]
				pub fn max_stale(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"MessageQueue",
						"MaxStale",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The amount of weight (if any) which should be provided to the message queue for"]
				#[doc = " servicing enqueued items."]
				#[doc = ""]
				#[doc = " This may be legitimately `None` in the case that you will call"]
				#[doc = " `ServiceQueues::service_queues` manually."]
				pub fn service_weight(
					&self,
				) -> ::subxt::constants::Address<
					::core::option::Option<runtime_types::sp_weights::weight_v2::Weight>,
				> {
					::subxt::constants::Address::new_static(
						"MessageQueue",
						"ServiceWeight",
						[
							204u8, 140u8, 63u8, 167u8, 49u8, 8u8, 148u8, 163u8, 190u8, 224u8, 15u8,
							103u8, 86u8, 153u8, 248u8, 117u8, 223u8, 117u8, 210u8, 80u8, 205u8,
							155u8, 40u8, 11u8, 59u8, 63u8, 129u8, 156u8, 17u8, 83u8, 177u8, 250u8,
						],
					)
				}
			}
		}
	}
	pub mod ismp_parachain {
		use super::{root_mod, runtime_types};
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::ismp_parachain::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateParachainConsensus {
					pub data: runtime_types::ismp::messaging::ConsensusMessage,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateParachainConsensus {
					const PALLET: &'static str = "IsmpParachain";
					const CALL: &'static str = "update_parachain_consensus";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AddParachain {
					pub para_ids: ::std::vec::Vec<runtime_types::ismp_parachain::ParachainData>,
				}
				impl ::subxt::blocks::StaticExtrinsic for AddParachain {
					const PALLET: &'static str = "IsmpParachain";
					const CALL: &'static str = "add_parachain";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemoveParachain {
					pub para_ids: ::std::vec::Vec<::core::primitive::u32>,
				}
				impl ::subxt::blocks::StaticExtrinsic for RemoveParachain {
					const PALLET: &'static str = "IsmpParachain";
					const CALL: &'static str = "remove_parachain";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::update_parachain_consensus`]."]
				pub fn update_parachain_consensus(
					&self,
					data: runtime_types::ismp::messaging::ConsensusMessage,
				) -> ::subxt::tx::Payload<types::UpdateParachainConsensus> {
					::subxt::tx::Payload::new_static(
						"IsmpParachain",
						"update_parachain_consensus",
						types::UpdateParachainConsensus { data },
						[
							189u8, 35u8, 112u8, 253u8, 157u8, 106u8, 129u8, 183u8, 196u8, 21u8,
							182u8, 18u8, 179u8, 158u8, 51u8, 86u8, 226u8, 154u8, 88u8, 154u8,
							144u8, 119u8, 40u8, 167u8, 72u8, 138u8, 96u8, 160u8, 201u8, 122u8,
							106u8, 161u8,
						],
					)
				}
				#[doc = "See [`Pallet::add_parachain`]."]
				pub fn add_parachain(
					&self,
					para_ids: ::std::vec::Vec<runtime_types::ismp_parachain::ParachainData>,
				) -> ::subxt::tx::Payload<types::AddParachain> {
					::subxt::tx::Payload::new_static(
						"IsmpParachain",
						"add_parachain",
						types::AddParachain { para_ids },
						[
							145u8, 240u8, 18u8, 43u8, 64u8, 18u8, 105u8, 2u8, 17u8, 60u8, 198u8,
							25u8, 236u8, 175u8, 203u8, 53u8, 11u8, 99u8, 16u8, 67u8, 108u8, 106u8,
							145u8, 32u8, 194u8, 206u8, 221u8, 122u8, 19u8, 121u8, 60u8, 227u8,
						],
					)
				}
				#[doc = "See [`Pallet::remove_parachain`]."]
				pub fn remove_parachain(
					&self,
					para_ids: ::std::vec::Vec<::core::primitive::u32>,
				) -> ::subxt::tx::Payload<types::RemoveParachain> {
					::subxt::tx::Payload::new_static(
						"IsmpParachain",
						"remove_parachain",
						types::RemoveParachain { para_ids },
						[
							232u8, 67u8, 140u8, 13u8, 186u8, 43u8, 235u8, 5u8, 68u8, 177u8, 119u8,
							5u8, 237u8, 165u8, 167u8, 143u8, 185u8, 62u8, 111u8, 125u8, 12u8, 34u8,
							113u8, 239u8, 99u8, 108u8, 173u8, 120u8, 170u8, 33u8, 138u8, 88u8,
						],
					)
				}
			}
		}
		#[doc = "Events emitted by this pallet"]
		pub type Event = runtime_types::ismp_parachain::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Parachains with the `para_ids` have been added to the whitelist"]
			pub struct ParachainsAdded {
				pub para_ids: ::std::vec::Vec<runtime_types::ismp_parachain::ParachainData>,
			}
			impl ::subxt::events::StaticEvent for ParachainsAdded {
				const PALLET: &'static str = "IsmpParachain";
				const EVENT: &'static str = "ParachainsAdded";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Parachains with the `para_ids` have been removed from the whitelist"]
			pub struct ParachainsRemoved {
				pub para_ids: ::std::vec::Vec<::core::primitive::u32>,
			}
			impl ::subxt::events::StaticEvent for ParachainsRemoved {
				const PALLET: &'static str = "IsmpParachain";
				const EVENT: &'static str = "ParachainsRemoved";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Mapping of relay chain heights to it's state commitment. The state commitment of the parent"]
				#[doc = " relay block is inserted at every block in `on_finalize`. This commitment is gotten from"]
				#[doc = " parachain-system."]
				pub fn relay_chain_state_commitments(
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
						"RelayChainStateCommitments",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							249u8, 69u8, 23u8, 170u8, 108u8, 67u8, 49u8, 103u8, 12u8, 225u8, 254u8,
							251u8, 198u8, 53u8, 74u8, 194u8, 215u8, 100u8, 226u8, 114u8, 249u8,
							79u8, 87u8, 135u8, 65u8, 253u8, 65u8, 155u8, 121u8, 21u8, 159u8, 37u8,
						],
					)
				}
				#[doc = " Mapping of relay chain heights to it's state commitment. The state commitment of the parent"]
				#[doc = " relay block is inserted at every block in `on_finalize`. This commitment is gotten from"]
				#[doc = " parachain-system."]
				pub fn relay_chain_state_commitments_root(
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
						"RelayChainStateCommitments",
						Vec::new(),
						[
							249u8, 69u8, 23u8, 170u8, 108u8, 67u8, 49u8, 103u8, 12u8, 225u8, 254u8,
							251u8, 198u8, 53u8, 74u8, 194u8, 215u8, 100u8, 226u8, 114u8, 249u8,
							79u8, 87u8, 135u8, 65u8, 253u8, 65u8, 155u8, 121u8, 21u8, 159u8, 37u8,
						],
					)
				}
				#[doc = " Tracks whether we've already seen the `update_parachain_consensus` inherent"]
				pub fn consensus_updated(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"IsmpParachain",
						"ConsensusUpdated",
						vec![],
						[
							81u8, 206u8, 228u8, 179u8, 185u8, 62u8, 232u8, 189u8, 100u8, 115u8,
							38u8, 154u8, 240u8, 31u8, 155u8, 196u8, 27u8, 47u8, 16u8, 151u8, 31u8,
							201u8, 248u8, 145u8, 118u8, 233u8, 157u8, 98u8, 6u8, 140u8, 21u8, 84u8,
						],
					)
				}
				#[doc = " List of parachains that this state machine is interested in."]
				pub fn parachains(
					&self,
					_0: impl ::std::borrow::Borrow<::core::primitive::u32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ismp_parachain::ParachainData,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"IsmpParachain",
						"Parachains",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							49u8, 183u8, 232u8, 105u8, 52u8, 240u8, 181u8, 18u8, 94u8, 149u8,
							170u8, 118u8, 80u8, 104u8, 113u8, 137u8, 5u8, 47u8, 1u8, 69u8, 70u8,
							175u8, 18u8, 206u8, 148u8, 172u8, 30u8, 0u8, 251u8, 65u8, 224u8, 0u8,
						],
					)
				}
				#[doc = " List of parachains that this state machine is interested in."]
				pub fn parachains_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::ismp_parachain::ParachainData,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"IsmpParachain",
						"Parachains",
						Vec::new(),
						[
							49u8, 183u8, 232u8, 105u8, 52u8, 240u8, 181u8, 18u8, 94u8, 149u8,
							170u8, 118u8, 80u8, 104u8, 113u8, 137u8, 5u8, 47u8, 1u8, 69u8, 70u8,
							175u8, 18u8, 206u8, 148u8, 172u8, 30u8, 0u8, 251u8, 65u8, 224u8, 0u8,
						],
					)
				}
			}
		}
	}
	pub mod ismp_sync_committee {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::ismp_sync_committee::pallet::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::ismp_sync_committee::pallet::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AddIsmpAddress {
					pub contract_address: ::subxt::utils::H160,
					pub state_machine_id: runtime_types::ismp::consensus::StateMachineId,
				}
				impl ::subxt::blocks::StaticExtrinsic for AddIsmpAddress {
					const PALLET: &'static str = "IsmpSyncCommittee";
					const CALL: &'static str = "add_ismp_address";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AddL2Consensus {
					pub state_machine_id: runtime_types::ismp::consensus::StateMachineId,
					pub l2_consensus: runtime_types::ismp_sync_committee::types::L2Consensus,
				}
				impl ::subxt::blocks::StaticExtrinsic for AddL2Consensus {
					const PALLET: &'static str = "IsmpSyncCommittee";
					const CALL: &'static str = "add_l2_consensus";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::add_ismp_address`]."]
				pub fn add_ismp_address(
					&self,
					contract_address: ::subxt::utils::H160,
					state_machine_id: runtime_types::ismp::consensus::StateMachineId,
				) -> ::subxt::tx::Payload<types::AddIsmpAddress> {
					::subxt::tx::Payload::new_static(
						"IsmpSyncCommittee",
						"add_ismp_address",
						types::AddIsmpAddress { contract_address, state_machine_id },
						[
							36u8, 248u8, 112u8, 126u8, 83u8, 192u8, 94u8, 200u8, 252u8, 187u8,
							98u8, 142u8, 77u8, 167u8, 233u8, 202u8, 18u8, 99u8, 3u8, 70u8, 169u8,
							251u8, 22u8, 12u8, 185u8, 174u8, 239u8, 17u8, 65u8, 22u8, 196u8, 58u8,
						],
					)
				}
				#[doc = "See [`Pallet::add_l2_consensus`]."]
				pub fn add_l2_consensus(
					&self,
					state_machine_id: runtime_types::ismp::consensus::StateMachineId,
					l2_consensus: runtime_types::ismp_sync_committee::types::L2Consensus,
				) -> ::subxt::tx::Payload<types::AddL2Consensus> {
					::subxt::tx::Payload::new_static(
						"IsmpSyncCommittee",
						"add_l2_consensus",
						types::AddL2Consensus { state_machine_id, l2_consensus },
						[
							120u8, 182u8, 156u8, 130u8, 194u8, 95u8, 88u8, 254u8, 75u8, 233u8,
							108u8, 76u8, 150u8, 118u8, 218u8, 67u8, 30u8, 190u8, 153u8, 111u8,
							39u8, 232u8, 134u8, 29u8, 190u8, 119u8, 245u8, 253u8, 193u8, 101u8,
							138u8, 118u8,
						],
					)
				}
			}
		}
	}
	pub mod ismp_demo {
		use super::{root_mod, runtime_types};
		#[doc = "Pallet Errors"]
		pub type Error = runtime_types::pallet_ismp_demo::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_ismp_demo::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Transfer {
					pub params: runtime_types::pallet_ismp_demo::pallet::TransferParams<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Transfer {
					const PALLET: &'static str = "IsmpDemo";
					const CALL: &'static str = "transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GetRequest {
					pub params: runtime_types::pallet_ismp_demo::pallet::GetRequest,
				}
				impl ::subxt::blocks::StaticExtrinsic for GetRequest {
					const PALLET: &'static str = "IsmpDemo";
					const CALL: &'static str = "get_request";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DispatchToEvm {
					pub params: runtime_types::pallet_ismp_demo::pallet::EvmParams,
				}
				impl ::subxt::blocks::StaticExtrinsic for DispatchToEvm {
					const PALLET: &'static str = "IsmpDemo";
					const CALL: &'static str = "dispatch_to_evm";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::transfer`]."]
				pub fn transfer(
					&self,
					params: runtime_types::pallet_ismp_demo::pallet::TransferParams<
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
				) -> ::subxt::tx::Payload<types::Transfer> {
					::subxt::tx::Payload::new_static(
						"IsmpDemo",
						"transfer",
						types::Transfer { params },
						[
							218u8, 118u8, 134u8, 170u8, 56u8, 171u8, 160u8, 214u8, 245u8, 140u8,
							250u8, 11u8, 180u8, 224u8, 180u8, 59u8, 221u8, 47u8, 77u8, 145u8,
							131u8, 108u8, 165u8, 247u8, 46u8, 70u8, 206u8, 191u8, 175u8, 31u8,
							167u8, 16u8,
						],
					)
				}
				#[doc = "See [`Pallet::get_request`]."]
				pub fn get_request(
					&self,
					params: runtime_types::pallet_ismp_demo::pallet::GetRequest,
				) -> ::subxt::tx::Payload<types::GetRequest> {
					::subxt::tx::Payload::new_static(
						"IsmpDemo",
						"get_request",
						types::GetRequest { params },
						[
							245u8, 61u8, 107u8, 34u8, 240u8, 238u8, 158u8, 125u8, 148u8, 86u8,
							226u8, 142u8, 95u8, 4u8, 36u8, 84u8, 192u8, 186u8, 198u8, 181u8, 196u8,
							139u8, 185u8, 68u8, 218u8, 144u8, 86u8, 234u8, 206u8, 12u8, 140u8,
							160u8,
						],
					)
				}
				#[doc = "See [`Pallet::dispatch_to_evm`]."]
				pub fn dispatch_to_evm(
					&self,
					params: runtime_types::pallet_ismp_demo::pallet::EvmParams,
				) -> ::subxt::tx::Payload<types::DispatchToEvm> {
					::subxt::tx::Payload::new_static(
						"IsmpDemo",
						"dispatch_to_evm",
						types::DispatchToEvm { params },
						[
							34u8, 164u8, 88u8, 73u8, 202u8, 39u8, 238u8, 235u8, 133u8, 20u8, 77u8,
							61u8, 166u8, 77u8, 245u8, 93u8, 161u8, 212u8, 39u8, 223u8, 183u8,
							197u8, 30u8, 101u8, 126u8, 4u8, 186u8, 200u8, 236u8, 250u8, 171u8,
							42u8,
						],
					)
				}
			}
		}
		#[doc = "Pallet events"]
		pub type Event = runtime_types::pallet_ismp_demo::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance has been transferred"]
			pub struct BalanceTransferred {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub dest_chain: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for BalanceTransferred {
				const PALLET: &'static str = "IsmpDemo";
				const EVENT: &'static str = "BalanceTransferred";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some balance has been received"]
			pub struct BalanceReceived {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub source_chain: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for BalanceReceived {
				const PALLET: &'static str = "IsmpDemo";
				const EVENT: &'static str = "BalanceReceived";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Request data receieved"]
			pub struct Request {
				pub source: runtime_types::ismp::host::StateMachine,
				pub data: ::std::string::String,
			}
			impl ::subxt::events::StaticEvent for Request {
				const PALLET: &'static str = "IsmpDemo";
				const EVENT: &'static str = "Request";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Get response recieved"]
			pub struct GetResponse(
				pub ::std::vec::Vec<::core::option::Option<::std::vec::Vec<::core::primitive::u8>>>,
			);
			impl ::subxt::events::StaticEvent for GetResponse {
				const PALLET: &'static str = "IsmpDemo";
				const EVENT: &'static str = "GetResponse";
			}
		}
	}
	pub mod relayer {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_ismp_relayer::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_ismp_relayer::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccumulateFees {
					pub withdrawal_proof:
						runtime_types::pallet_ismp_relayer::withdrawal::WithdrawalProof,
				}
				impl ::subxt::blocks::StaticExtrinsic for AccumulateFees {
					const PALLET: &'static str = "Relayer";
					const CALL: &'static str = "accumulate_fees";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WithdrawFees {
					pub withdrawal_data:
						runtime_types::pallet_ismp_relayer::withdrawal::WithdrawalInputData,
				}
				impl ::subxt::blocks::StaticExtrinsic for WithdrawFees {
					const PALLET: &'static str = "Relayer";
					const CALL: &'static str = "withdraw_fees";
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetMinimumWithdrawal {
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetMinimumWithdrawal {
					const PALLET: &'static str = "Relayer";
					const CALL: &'static str = "set_minimum_withdrawal";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::accumulate_fees`]."]
				pub fn accumulate_fees(
					&self,
					withdrawal_proof : runtime_types :: pallet_ismp_relayer :: withdrawal :: WithdrawalProof,
				) -> ::subxt::tx::Payload<types::AccumulateFees> {
					::subxt::tx::Payload::new_static(
						"Relayer",
						"accumulate_fees",
						types::AccumulateFees { withdrawal_proof },
						[
							11u8, 191u8, 178u8, 152u8, 252u8, 239u8, 197u8, 81u8, 144u8, 208u8,
							42u8, 102u8, 43u8, 3u8, 187u8, 144u8, 106u8, 9u8, 3u8, 146u8, 227u8,
							25u8, 109u8, 174u8, 154u8, 195u8, 245u8, 77u8, 227u8, 99u8, 197u8,
							192u8,
						],
					)
				}
				#[doc = "See [`Pallet::withdraw_fees`]."]
				pub fn withdraw_fees(
					&self,
					withdrawal_data : runtime_types :: pallet_ismp_relayer :: withdrawal :: WithdrawalInputData,
				) -> ::subxt::tx::Payload<types::WithdrawFees> {
					::subxt::tx::Payload::new_static(
						"Relayer",
						"withdraw_fees",
						types::WithdrawFees { withdrawal_data },
						[
							243u8, 31u8, 150u8, 164u8, 123u8, 101u8, 4u8, 109u8, 222u8, 64u8, 40u8,
							82u8, 29u8, 254u8, 88u8, 11u8, 36u8, 107u8, 240u8, 87u8, 132u8, 235u8,
							1u8, 83u8, 48u8, 238u8, 219u8, 178u8, 215u8, 32u8, 165u8, 212u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_minimum_withdrawal`]."]
				pub fn set_minimum_withdrawal(
					&self,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::SetMinimumWithdrawal> {
					::subxt::tx::Payload::new_static(
						"Relayer",
						"set_minimum_withdrawal",
						types::SetMinimumWithdrawal { amount },
						[
							60u8, 193u8, 238u8, 187u8, 73u8, 216u8, 202u8, 71u8, 226u8, 40u8,
							230u8, 42u8, 84u8, 156u8, 180u8, 172u8, 38u8, 151u8, 227u8, 241u8,
							53u8, 219u8, 10u8, 151u8, 92u8, 46u8, 106u8, 235u8, 124u8, 239u8, 99u8,
							123u8,
						],
					)
				}
			}
		}
		#[doc = "Events emiited by the relayer pallet"]
		pub type Event = runtime_types::pallet_ismp_relayer::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A relayer with the `address` has accumulated some fees on the `state_machine`"]
			pub struct AccumulateFees {
				pub address: runtime_types::bounded_collections::bounded_vec::BoundedVec<
					::core::primitive::u8,
				>,
				pub state_machine: runtime_types::ismp::host::StateMachine,
				pub amount: runtime_types::primitive_types::U256,
			}
			impl ::subxt::events::StaticEvent for AccumulateFees {
				const PALLET: &'static str = "Relayer";
				const EVENT: &'static str = "AccumulateFees";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A relayer with the the `address` has initiated a withdrawal on the `state_machine`"]
			pub struct Withdraw {
				pub address: runtime_types::bounded_collections::bounded_vec::BoundedVec<
					::core::primitive::u8,
				>,
				pub state_machine: runtime_types::ismp::host::StateMachine,
				pub amount: runtime_types::primitive_types::U256,
			}
			impl ::subxt::events::StaticEvent for Withdraw {
				const PALLET: &'static str = "Relayer";
				const EVENT: &'static str = "Withdraw";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " double map of address to source chain, which holds the amount of the relayer address"]
				pub fn fees(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
					_1: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::primitive_types::U256,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Relayer",
						"Fees",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							183u8, 252u8, 230u8, 142u8, 231u8, 87u8, 14u8, 48u8, 98u8, 97u8, 12u8,
							54u8, 82u8, 252u8, 107u8, 7u8, 234u8, 154u8, 230u8, 23u8, 235u8, 154u8,
							216u8, 254u8, 150u8, 33u8, 110u8, 54u8, 157u8, 172u8, 116u8, 204u8,
						],
					)
				}
				#[doc = " double map of address to source chain, which holds the amount of the relayer address"]
				pub fn fees_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::primitive_types::U256,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Relayer",
						"Fees",
						Vec::new(),
						[
							183u8, 252u8, 230u8, 142u8, 231u8, 87u8, 14u8, 48u8, 98u8, 97u8, 12u8,
							54u8, 82u8, 252u8, 107u8, 7u8, 234u8, 154u8, 230u8, 23u8, 235u8, 154u8,
							216u8, 254u8, 150u8, 33u8, 110u8, 54u8, 157u8, 172u8, 116u8, 204u8,
						],
					)
				}
				#[doc = " Latest nonce for each address and the state machine they want to withdraw from"]
				pub fn nonce(
					&self,
					_0: impl ::std::borrow::Borrow<[::core::primitive::u8]>,
					_1: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Relayer",
						"Nonce",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							181u8, 129u8, 255u8, 254u8, 37u8, 62u8, 32u8, 90u8, 101u8, 188u8, 0u8,
							95u8, 231u8, 211u8, 171u8, 60u8, 116u8, 2u8, 101u8, 171u8, 112u8, 59u8,
							107u8, 144u8, 25u8, 79u8, 91u8, 183u8, 138u8, 233u8, 66u8, 139u8,
						],
					)
				}
				#[doc = " Latest nonce for each address and the state machine they want to withdraw from"]
				pub fn nonce_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::u64,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Relayer",
						"Nonce",
						Vec::new(),
						[
							181u8, 129u8, 255u8, 254u8, 37u8, 62u8, 32u8, 90u8, 101u8, 188u8, 0u8,
							95u8, 231u8, 211u8, 171u8, 60u8, 116u8, 2u8, 101u8, 171u8, 112u8, 59u8,
							107u8, 144u8, 25u8, 79u8, 91u8, 183u8, 138u8, 233u8, 66u8, 139u8,
						],
					)
				}
				#[doc = " Minimum withdrawal amount"]
				pub fn minimum_withdrawal_amount(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::primitive_types::U256,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Relayer",
						"MinimumWithdrawalAmount",
						vec![],
						[
							58u8, 166u8, 160u8, 225u8, 93u8, 112u8, 167u8, 9u8, 44u8, 65u8, 183u8,
							140u8, 14u8, 144u8, 186u8, 191u8, 67u8, 89u8, 78u8, 153u8, 131u8, 68u8,
							101u8, 43u8, 189u8, 116u8, 80u8, 158u8, 54u8, 209u8, 89u8, 17u8,
						],
					)
				}
			}
		}
	}
	pub mod host_executive {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_ismp_host_executive::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_ismp_host_executive::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetHostParams {
					pub params: ::subxt::utils::KeyedVec<
						runtime_types::ismp::host::StateMachine,
						runtime_types::pallet_ismp_host_executive::params::HostParam<
							::core::primitive::u128,
						>,
					>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetHostParams {
					const PALLET: &'static str = "HostExecutive";
					const CALL: &'static str = "set_host_params";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateHostParams {
					pub state_machine: runtime_types::ismp::host::StateMachine,
					pub update: runtime_types::pallet_ismp_host_executive::params::HostParamUpdate<
						::core::primitive::u128,
					>,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateHostParams {
					const PALLET: &'static str = "HostExecutive";
					const CALL: &'static str = "update_host_params";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_host_params`]."]
				pub fn set_host_params(
					&self,
					params: ::subxt::utils::KeyedVec<
						runtime_types::ismp::host::StateMachine,
						runtime_types::pallet_ismp_host_executive::params::HostParam<
							::core::primitive::u128,
						>,
					>,
				) -> ::subxt::tx::Payload<types::SetHostParams> {
					::subxt::tx::Payload::new_static(
						"HostExecutive",
						"set_host_params",
						types::SetHostParams { params },
						[
							91u8, 77u8, 33u8, 15u8, 57u8, 22u8, 80u8, 100u8, 112u8, 201u8, 117u8,
							179u8, 33u8, 54u8, 138u8, 110u8, 183u8, 96u8, 195u8, 36u8, 110u8,
							251u8, 107u8, 95u8, 79u8, 133u8, 42u8, 174u8, 88u8, 241u8, 92u8, 31u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_host_params`]."]
				pub fn update_host_params(
					&self,
					state_machine: runtime_types::ismp::host::StateMachine,
					update: runtime_types::pallet_ismp_host_executive::params::HostParamUpdate<
						::core::primitive::u128,
					>,
				) -> ::subxt::tx::Payload<types::UpdateHostParams> {
					::subxt::tx::Payload::new_static(
						"HostExecutive",
						"update_host_params",
						types::UpdateHostParams { state_machine, update },
						[
							52u8, 1u8, 225u8, 131u8, 83u8, 117u8, 165u8, 75u8, 246u8, 30u8, 87u8,
							4u8, 200u8, 132u8, 41u8, 102u8, 184u8, 162u8, 216u8, 65u8, 59u8, 123u8,
							160u8, 161u8, 210u8, 24u8, 29u8, 241u8, 239u8, 171u8, 23u8, 144u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_ismp_host_executive::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "`AdminOrigin` has initiated a host parameter update to the mentioned state"]
			#[doc = "machine"]
			pub struct HostParamsUpdated {
				pub state_machine: runtime_types::ismp::host::StateMachine,
				pub old: runtime_types::pallet_ismp_host_executive::params::HostParam<
					::core::primitive::u128,
				>,
				pub new: runtime_types::pallet_ismp_host_executive::params::HostParam<
					::core::primitive::u128,
				>,
			}
			impl ::subxt::events::StaticEvent for HostParamsUpdated {
				const PALLET: &'static str = "HostExecutive";
				const EVENT: &'static str = "HostParamsUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "`AdminOrigin` has set the initial host parameters for the mentioned state"]
			#[doc = "machine"]
			pub struct HostParamsSet {
				pub state_machine: runtime_types::ismp::host::StateMachine,
				pub params: runtime_types::pallet_ismp_host_executive::params::HostParam<
					::core::primitive::u128,
				>,
			}
			impl ::subxt::events::StaticEvent for HostParamsSet {
				const PALLET: &'static str = "HostExecutive";
				const EVENT: &'static str = "HostParamsSet";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Host Manager Addresses on different chains"]
				pub fn host_params(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_ismp_host_executive::params::HostParam<
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"HostExecutive",
						"HostParams",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							244u8, 208u8, 199u8, 246u8, 81u8, 196u8, 138u8, 57u8, 73u8, 61u8,
							192u8, 123u8, 227u8, 106u8, 237u8, 190u8, 29u8, 240u8, 60u8, 112u8,
							212u8, 70u8, 182u8, 105u8, 119u8, 89u8, 204u8, 232u8, 205u8, 198u8,
							157u8, 212u8,
						],
					)
				}
				#[doc = " Host Manager Addresses on different chains"]
				pub fn host_params_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_ismp_host_executive::params::HostParam<
						::core::primitive::u128,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"HostExecutive",
						"HostParams",
						Vec::new(),
						[
							244u8, 208u8, 199u8, 246u8, 81u8, 196u8, 138u8, 57u8, 73u8, 61u8,
							192u8, 123u8, 227u8, 106u8, 237u8, 190u8, 29u8, 240u8, 60u8, 112u8,
							212u8, 70u8, 182u8, 105u8, 119u8, 89u8, 204u8, 232u8, 205u8, 198u8,
							157u8, 212u8,
						],
					)
				}
			}
		}
	}
	pub mod call_decompressor {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_call_decompressor::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_call_decompressor::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DecompressCall {
					pub compressed: ::std::vec::Vec<::core::primitive::u8>,
					pub encoded_call_size: ::core::primitive::u32,
				}
				impl ::subxt::blocks::StaticExtrinsic for DecompressCall {
					const PALLET: &'static str = "CallDecompressor";
					const CALL: &'static str = "decompress_call";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::decompress_call`]."]
				pub fn decompress_call(
					&self,
					compressed: ::std::vec::Vec<::core::primitive::u8>,
					encoded_call_size: ::core::primitive::u32,
				) -> ::subxt::tx::Payload<types::DecompressCall> {
					::subxt::tx::Payload::new_static(
						"CallDecompressor",
						"decompress_call",
						types::DecompressCall { compressed, encoded_call_size },
						[
							100u8, 30u8, 167u8, 61u8, 94u8, 92u8, 222u8, 182u8, 88u8, 56u8, 109u8,
							253u8, 127u8, 136u8, 170u8, 157u8, 91u8, 200u8, 60u8, 117u8, 86u8,
							184u8, 215u8, 97u8, 179u8, 67u8, 207u8, 96u8, 168u8, 36u8, 8u8, 238u8,
						],
					)
				}
			}
		}
	}
	pub mod gateway {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_asset_gateway::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_asset_gateway::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetParams {
					pub update:
						runtime_types::pallet_asset_gateway::pallet::TokenGatewayParamsUpdate,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetParams {
					const PALLET: &'static str = "Gateway";
					const CALL: &'static str = "set_params";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::set_params`]."]
				pub fn set_params(
					&self,
					update: runtime_types::pallet_asset_gateway::pallet::TokenGatewayParamsUpdate,
				) -> ::subxt::tx::Payload<types::SetParams> {
					::subxt::tx::Payload::new_static(
						"Gateway",
						"set_params",
						types::SetParams { update },
						[
							203u8, 148u8, 4u8, 104u8, 228u8, 43u8, 135u8, 22u8, 170u8, 240u8,
							106u8, 166u8, 5u8, 161u8, 170u8, 72u8, 168u8, 247u8, 44u8, 222u8,
							178u8, 106u8, 34u8, 32u8, 56u8, 99u8, 9u8, 171u8, 22u8, 127u8, 250u8,
							108u8,
						],
					)
				}
			}
		}
		#[doc = "Events emiited by the relayer pallet"]
		pub type Event = runtime_types::pallet_asset_gateway::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An XCM transfer from the relay chain has been transformed into a crosschain message"]
			pub struct AssetTeleported {
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::H160,
				pub amount: ::core::primitive::u128,
				pub dest: runtime_types::ismp::host::StateMachine,
				pub commitment: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for AssetTeleported {
				const PALLET: &'static str = "Gateway";
				const EVENT: &'static str = "AssetTeleported";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An asset has been received and transferred to the beneficiary's account on the"]
			#[doc = "relaychain"]
			pub struct AssetReceived {
				pub beneficiary: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub source: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for AssetReceived {
				const PALLET: &'static str = "Gateway";
				const EVENT: &'static str = "AssetReceived";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An asset has been refunded and transferred to the beneficiary's account on the"]
			#[doc = "relaychain"]
			pub struct AssetRefunded {
				pub beneficiary: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
				pub source: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for AssetRefunded {
				const PALLET: &'static str = "Gateway";
				const EVENT: &'static str = "AssetRefunded";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				pub fn params(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_asset_gateway::pallet::TokenGatewayParams,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"Gateway",
						"Params",
						vec![],
						[
							138u8, 241u8, 128u8, 10u8, 222u8, 234u8, 248u8, 124u8, 197u8, 43u8,
							44u8, 34u8, 221u8, 43u8, 66u8, 184u8, 6u8, 229u8, 239u8, 133u8, 226u8,
							143u8, 253u8, 72u8, 21u8, 49u8, 177u8, 67u8, 214u8, 228u8, 236u8,
							239u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " The asset tranfer's pallet id, used for deriving its sovereign account ID."]
				#[doc = " All escrowed funds will be custodied by this account"]
				pub fn pallet_id(
					&self,
				) -> ::subxt::constants::Address<runtime_types::frame_support::PalletId> {
					::subxt::constants::Address::new_static(
						"Gateway",
						"PalletId",
						[
							56u8, 243u8, 53u8, 83u8, 154u8, 179u8, 170u8, 80u8, 133u8, 173u8, 61u8,
							161u8, 47u8, 225u8, 146u8, 21u8, 50u8, 229u8, 248u8, 27u8, 104u8, 58u8,
							129u8, 197u8, 102u8, 160u8, 168u8, 205u8, 154u8, 42u8, 217u8, 53u8,
						],
					)
				}
				#[doc = " Pallet parameters"]
				pub fn params(
					&self,
				) -> ::subxt::constants::Address<
					runtime_types::pallet_asset_gateway::pallet::TokenGatewayParams,
				> {
					::subxt::constants::Address::new_static(
						"Gateway",
						"Params",
						[
							162u8, 27u8, 191u8, 64u8, 121u8, 81u8, 89u8, 173u8, 71u8, 185u8, 216u8,
							241u8, 144u8, 220u8, 17u8, 204u8, 42u8, 97u8, 241u8, 178u8, 214u8,
							248u8, 18u8, 126u8, 37u8, 209u8, 111u8, 171u8, 198u8, 145u8, 95u8,
							180u8,
						],
					)
				}
			}
		}
	}
	pub mod assets {
		use super::{root_mod, runtime_types};
		#[doc = "The `Error` enum of this pallet."]
		pub type Error = runtime_types::pallet_assets::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_assets::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Create {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub min_balance: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for Create {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "create";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceCreate {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub is_sufficient: ::core::primitive::bool,
					#[codec(compact)]
					pub min_balance: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceCreate {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_create";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StartDestroy {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for StartDestroy {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "start_destroy";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DestroyAccounts {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for DestroyAccounts {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "destroy_accounts";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DestroyApprovals {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for DestroyApprovals {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "destroy_approvals";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FinishDestroy {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for FinishDestroy {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "finish_destroy";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Mint {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub beneficiary: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for Mint {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "mint";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Burn {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for Burn {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "burn";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Transfer {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for Transfer {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferKeepAlive {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferKeepAlive {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "transfer_keep_alive";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceTransfer {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceTransfer {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Freeze {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Freeze {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "freeze";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Thaw {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Thaw {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "thaw";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FreezeAsset {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for FreezeAsset {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "freeze_asset";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ThawAsset {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for ThawAsset {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "thaw_asset";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferOwnership {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferOwnership {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "transfer_ownership";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetTeam {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetTeam {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "set_team";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetMetadata {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub name: ::std::vec::Vec<::core::primitive::u8>,
					pub symbol: ::std::vec::Vec<::core::primitive::u8>,
					pub decimals: ::core::primitive::u8,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetMetadata {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "set_metadata";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ClearMetadata {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for ClearMetadata {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "clear_metadata";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceSetMetadata {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub name: ::std::vec::Vec<::core::primitive::u8>,
					pub symbol: ::std::vec::Vec<::core::primitive::u8>,
					pub decimals: ::core::primitive::u8,
					pub is_frozen: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceSetMetadata {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_set_metadata";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceClearMetadata {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceClearMetadata {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_clear_metadata";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceAssetStatus {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub min_balance: ::core::primitive::u128,
					pub is_sufficient: ::core::primitive::bool,
					pub is_frozen: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceAssetStatus {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_asset_status";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ApproveTransfer {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for ApproveTransfer {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "approve_transfer";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CancelApproval {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for CancelApproval {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "cancel_approval";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ForceCancelApproval {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for ForceCancelApproval {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "force_cancel_approval";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferApproved {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					pub destination: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					#[codec(compact)]
					pub amount: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for TransferApproved {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "transfer_approved";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Touch {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				}
				impl ::subxt::blocks::StaticExtrinsic for Touch {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "touch";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Refund {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub allow_burn: ::core::primitive::bool,
				}
				impl ::subxt::blocks::StaticExtrinsic for Refund {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "refund";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SetMinBalance {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub min_balance: ::core::primitive::u128,
				}
				impl ::subxt::blocks::StaticExtrinsic for SetMinBalance {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "set_min_balance";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TouchOther {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for TouchOther {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "touch_other";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RefundOther {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for RefundOther {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "refund_other";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Block {
					pub id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					pub who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				}
				impl ::subxt::blocks::StaticExtrinsic for Block {
					const PALLET: &'static str = "Assets";
					const CALL: &'static str = "block";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::create`]."]
				pub fn create(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					min_balance: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::Create> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"create",
						types::Create { id, admin, min_balance },
						[
							164u8, 148u8, 246u8, 221u8, 212u8, 31u8, 7u8, 209u8, 153u8, 128u8,
							170u8, 21u8, 202u8, 246u8, 180u8, 124u8, 187u8, 137u8, 19u8, 29u8,
							199u8, 126u8, 137u8, 153u8, 122u8, 57u8, 70u8, 214u8, 202u8, 152u8,
							11u8, 15u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_create`]."]
				pub fn force_create(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					is_sufficient: ::core::primitive::bool,
					min_balance: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceCreate> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_create",
						types::ForceCreate { id, owner, is_sufficient, min_balance },
						[
							54u8, 163u8, 60u8, 192u8, 118u8, 181u8, 233u8, 43u8, 82u8, 230u8, 71u8,
							159u8, 232u8, 68u8, 205u8, 18u8, 200u8, 117u8, 220u8, 200u8, 6u8,
							203u8, 239u8, 201u8, 18u8, 106u8, 106u8, 200u8, 94u8, 27u8, 120u8,
							197u8,
						],
					)
				}
				#[doc = "See [`Pallet::start_destroy`]."]
				pub fn start_destroy(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::StartDestroy> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"start_destroy",
						types::StartDestroy { id },
						[
							206u8, 209u8, 96u8, 116u8, 63u8, 254u8, 29u8, 162u8, 35u8, 11u8, 115u8,
							82u8, 144u8, 45u8, 82u8, 5u8, 229u8, 177u8, 145u8, 112u8, 252u8, 118u8,
							142u8, 237u8, 198u8, 6u8, 193u8, 66u8, 95u8, 214u8, 114u8, 206u8,
						],
					)
				}
				#[doc = "See [`Pallet::destroy_accounts`]."]
				pub fn destroy_accounts(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::DestroyAccounts> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"destroy_accounts",
						types::DestroyAccounts { id },
						[
							252u8, 18u8, 12u8, 126u8, 214u8, 14u8, 28u8, 24u8, 53u8, 233u8, 172u8,
							57u8, 46u8, 204u8, 174u8, 224u8, 42u8, 120u8, 103u8, 236u8, 23u8,
							235u8, 183u8, 135u8, 36u8, 156u8, 167u8, 29u8, 144u8, 69u8, 11u8,
							215u8,
						],
					)
				}
				#[doc = "See [`Pallet::destroy_approvals`]."]
				pub fn destroy_approvals(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::DestroyApprovals> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"destroy_approvals",
						types::DestroyApprovals { id },
						[
							233u8, 160u8, 197u8, 89u8, 231u8, 234u8, 78u8, 50u8, 7u8, 74u8, 69u8,
							131u8, 146u8, 201u8, 67u8, 252u8, 22u8, 228u8, 93u8, 77u8, 217u8, 92u8,
							119u8, 145u8, 84u8, 194u8, 248u8, 160u8, 25u8, 144u8, 28u8, 98u8,
						],
					)
				}
				#[doc = "See [`Pallet::finish_destroy`]."]
				pub fn finish_destroy(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::FinishDestroy> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"finish_destroy",
						types::FinishDestroy { id },
						[
							48u8, 136u8, 163u8, 47u8, 202u8, 253u8, 7u8, 5u8, 38u8, 233u8, 136u8,
							24u8, 125u8, 230u8, 199u8, 1u8, 192u8, 179u8, 2u8, 190u8, 60u8, 118u8,
							97u8, 187u8, 219u8, 16u8, 238u8, 181u8, 62u8, 183u8, 174u8, 79u8,
						],
					)
				}
				#[doc = "See [`Pallet::mint`]."]
				pub fn mint(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					beneficiary: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::Mint> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"mint",
						types::Mint { id, beneficiary, amount },
						[
							215u8, 42u8, 109u8, 162u8, 94u8, 207u8, 140u8, 105u8, 0u8, 82u8, 73u8,
							84u8, 4u8, 136u8, 44u8, 155u8, 30u8, 77u8, 201u8, 110u8, 114u8, 215u8,
							147u8, 133u8, 240u8, 235u8, 81u8, 240u8, 116u8, 52u8, 122u8, 94u8,
						],
					)
				}
				#[doc = "See [`Pallet::burn`]."]
				pub fn burn(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::Burn> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"burn",
						types::Burn { id, who, amount },
						[
							229u8, 211u8, 246u8, 225u8, 53u8, 177u8, 244u8, 107u8, 143u8, 190u8,
							24u8, 229u8, 181u8, 198u8, 48u8, 96u8, 203u8, 171u8, 178u8, 186u8,
							169u8, 7u8, 251u8, 146u8, 139u8, 95u8, 178u8, 67u8, 192u8, 136u8,
							151u8, 224u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer`]."]
				pub fn transfer(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::Transfer> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"transfer",
						types::Transfer { id, target, amount },
						[
							217u8, 93u8, 1u8, 20u8, 190u8, 12u8, 153u8, 43u8, 159u8, 95u8, 41u8,
							135u8, 5u8, 191u8, 157u8, 187u8, 173u8, 32u8, 44u8, 123u8, 55u8, 88u8,
							8u8, 184u8, 160u8, 208u8, 96u8, 164u8, 34u8, 29u8, 208u8, 150u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_keep_alive`]."]
				pub fn transfer_keep_alive(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferKeepAlive> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"transfer_keep_alive",
						types::TransferKeepAlive { id, target, amount },
						[
							30u8, 113u8, 41u8, 253u8, 39u8, 184u8, 86u8, 201u8, 65u8, 222u8, 162u8,
							24u8, 73u8, 129u8, 242u8, 40u8, 41u8, 169u8, 241u8, 137u8, 13u8, 174u8,
							175u8, 15u8, 85u8, 172u8, 156u8, 32u8, 111u8, 106u8, 33u8, 5u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_transfer`]."]
				pub fn force_transfer(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ForceTransfer> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_transfer",
						types::ForceTransfer { id, source, dest, amount },
						[
							22u8, 160u8, 197u8, 220u8, 138u8, 148u8, 247u8, 50u8, 2u8, 236u8,
							189u8, 176u8, 227u8, 62u8, 69u8, 113u8, 34u8, 190u8, 91u8, 211u8,
							120u8, 100u8, 86u8, 81u8, 108u8, 174u8, 131u8, 93u8, 70u8, 21u8, 8u8,
							73u8,
						],
					)
				}
				#[doc = "See [`Pallet::freeze`]."]
				pub fn freeze(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::Freeze> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"freeze",
						types::Freeze { id, who },
						[
							95u8, 31u8, 200u8, 187u8, 138u8, 164u8, 10u8, 55u8, 143u8, 190u8,
							248u8, 147u8, 67u8, 99u8, 4u8, 213u8, 248u8, 150u8, 97u8, 42u8, 154u8,
							106u8, 1u8, 194u8, 229u8, 214u8, 119u8, 101u8, 50u8, 135u8, 184u8,
							125u8,
						],
					)
				}
				#[doc = "See [`Pallet::thaw`]."]
				pub fn thaw(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::Thaw> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"thaw",
						types::Thaw { id, who },
						[
							45u8, 219u8, 36u8, 117u8, 64u8, 89u8, 113u8, 159u8, 228u8, 3u8, 250u8,
							210u8, 40u8, 144u8, 11u8, 155u8, 11u8, 181u8, 113u8, 91u8, 130u8,
							230u8, 62u8, 190u8, 180u8, 191u8, 218u8, 171u8, 137u8, 76u8, 109u8,
							249u8,
						],
					)
				}
				#[doc = "See [`Pallet::freeze_asset`]."]
				pub fn freeze_asset(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::FreezeAsset> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"freeze_asset",
						types::FreezeAsset { id },
						[
							146u8, 248u8, 184u8, 9u8, 247u8, 29u8, 250u8, 16u8, 160u8, 55u8, 152u8,
							6u8, 177u8, 75u8, 89u8, 55u8, 234u8, 247u8, 32u8, 153u8, 187u8, 27u8,
							205u8, 166u8, 233u8, 64u8, 212u8, 187u8, 172u8, 250u8, 191u8, 51u8,
						],
					)
				}
				#[doc = "See [`Pallet::thaw_asset`]."]
				pub fn thaw_asset(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::ThawAsset> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"thaw_asset",
						types::ThawAsset { id },
						[
							71u8, 107u8, 116u8, 11u8, 163u8, 95u8, 70u8, 244u8, 160u8, 92u8, 48u8,
							39u8, 43u8, 15u8, 93u8, 31u8, 83u8, 106u8, 248u8, 135u8, 219u8, 159u8,
							231u8, 94u8, 247u8, 107u8, 184u8, 195u8, 124u8, 238u8, 152u8, 4u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_ownership`]."]
				pub fn transfer_ownership(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::TransferOwnership> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"transfer_ownership",
						types::TransferOwnership { id, owner },
						[
							100u8, 47u8, 5u8, 118u8, 162u8, 68u8, 248u8, 108u8, 160u8, 27u8, 158u8,
							135u8, 233u8, 56u8, 69u8, 215u8, 121u8, 112u8, 167u8, 109u8, 234u8,
							230u8, 14u8, 161u8, 72u8, 5u8, 181u8, 187u8, 156u8, 41u8, 206u8, 214u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_team`]."]
				pub fn set_team(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::SetTeam> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"set_team",
						types::SetTeam { id, issuer, admin, freezer },
						[
							87u8, 196u8, 115u8, 44u8, 208u8, 80u8, 46u8, 221u8, 184u8, 190u8,
							194u8, 191u8, 181u8, 243u8, 68u8, 250u8, 122u8, 192u8, 69u8, 23u8,
							70u8, 7u8, 48u8, 137u8, 29u8, 57u8, 201u8, 252u8, 112u8, 210u8, 212u8,
							165u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_metadata`]."]
				pub fn set_metadata(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					name: ::std::vec::Vec<::core::primitive::u8>,
					symbol: ::std::vec::Vec<::core::primitive::u8>,
					decimals: ::core::primitive::u8,
				) -> ::subxt::tx::Payload<types::SetMetadata> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"set_metadata",
						types::SetMetadata { id, name, symbol, decimals },
						[
							211u8, 156u8, 93u8, 165u8, 146u8, 65u8, 178u8, 206u8, 137u8, 139u8,
							247u8, 172u8, 160u8, 158u8, 58u8, 66u8, 9u8, 187u8, 190u8, 57u8, 244u8,
							39u8, 223u8, 203u8, 4u8, 15u8, 203u8, 40u8, 73u8, 183u8, 32u8, 2u8,
						],
					)
				}
				#[doc = "See [`Pallet::clear_metadata`]."]
				pub fn clear_metadata(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::ClearMetadata> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"clear_metadata",
						types::ClearMetadata { id },
						[
							216u8, 174u8, 37u8, 121u8, 74u8, 223u8, 116u8, 152u8, 59u8, 143u8,
							173u8, 129u8, 84u8, 46u8, 106u8, 172u8, 242u8, 181u8, 144u8, 198u8,
							184u8, 251u8, 75u8, 139u8, 242u8, 169u8, 52u8, 67u8, 239u8, 131u8,
							214u8, 6u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_set_metadata`]."]
				pub fn force_set_metadata(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					name: ::std::vec::Vec<::core::primitive::u8>,
					symbol: ::std::vec::Vec<::core::primitive::u8>,
					decimals: ::core::primitive::u8,
					is_frozen: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::ForceSetMetadata> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_set_metadata",
						types::ForceSetMetadata { id, name, symbol, decimals, is_frozen },
						[
							82u8, 65u8, 114u8, 208u8, 93u8, 64u8, 7u8, 33u8, 104u8, 221u8, 70u8,
							90u8, 83u8, 53u8, 40u8, 242u8, 208u8, 186u8, 169u8, 49u8, 233u8, 27u8,
							65u8, 194u8, 27u8, 13u8, 157u8, 211u8, 69u8, 167u8, 167u8, 45u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_clear_metadata`]."]
				pub fn force_clear_metadata(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::ForceClearMetadata> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_clear_metadata",
						types::ForceClearMetadata { id },
						[
							202u8, 54u8, 234u8, 190u8, 164u8, 128u8, 229u8, 135u8, 206u8, 150u8,
							64u8, 187u8, 37u8, 198u8, 244u8, 203u8, 25u8, 208u8, 95u8, 216u8, 67u8,
							64u8, 7u8, 96u8, 231u8, 9u8, 10u8, 202u8, 110u8, 173u8, 30u8, 194u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_asset_status`]."]
				pub fn force_asset_status(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					min_balance: ::core::primitive::u128,
					is_sufficient: ::core::primitive::bool,
					is_frozen: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::ForceAssetStatus> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_asset_status",
						types::ForceAssetStatus {
							id,
							owner,
							issuer,
							admin,
							freezer,
							min_balance,
							is_sufficient,
							is_frozen,
						},
						[
							117u8, 86u8, 248u8, 120u8, 210u8, 204u8, 127u8, 187u8, 136u8, 144u8,
							138u8, 177u8, 43u8, 195u8, 185u8, 46u8, 85u8, 116u8, 37u8, 59u8, 131u8,
							244u8, 174u8, 15u8, 56u8, 145u8, 223u8, 123u8, 36u8, 161u8, 242u8,
							42u8,
						],
					)
				}
				#[doc = "See [`Pallet::approve_transfer`]."]
				pub fn approve_transfer(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::ApproveTransfer> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"approve_transfer",
						types::ApproveTransfer { id, delegate, amount },
						[
							71u8, 8u8, 186u8, 118u8, 182u8, 19u8, 140u8, 190u8, 28u8, 8u8, 239u8,
							105u8, 181u8, 241u8, 86u8, 177u8, 39u8, 26u8, 65u8, 90u8, 13u8, 81u8,
							220u8, 44u8, 247u8, 179u8, 141u8, 136u8, 204u8, 160u8, 5u8, 209u8,
						],
					)
				}
				#[doc = "See [`Pallet::cancel_approval`]."]
				pub fn cancel_approval(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::CancelApproval> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"cancel_approval",
						types::CancelApproval { id, delegate },
						[
							153u8, 114u8, 130u8, 154u8, 51u8, 255u8, 27u8, 91u8, 36u8, 23u8, 128u8,
							22u8, 58u8, 8u8, 188u8, 60u8, 171u8, 131u8, 99u8, 35u8, 155u8, 238u8,
							52u8, 231u8, 192u8, 131u8, 17u8, 97u8, 227u8, 16u8, 254u8, 249u8,
						],
					)
				}
				#[doc = "See [`Pallet::force_cancel_approval`]."]
				pub fn force_cancel_approval(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::ForceCancelApproval> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"force_cancel_approval",
						types::ForceCancelApproval { id, owner, delegate },
						[
							47u8, 204u8, 7u8, 144u8, 234u8, 135u8, 62u8, 47u8, 162u8, 8u8, 35u8,
							43u8, 238u8, 242u8, 181u8, 112u8, 229u8, 243u8, 44u8, 126u8, 69u8,
							53u8, 238u8, 234u8, 215u8, 81u8, 117u8, 235u8, 136u8, 179u8, 194u8,
							137u8,
						],
					)
				}
				#[doc = "See [`Pallet::transfer_approved`]."]
				pub fn transfer_approved(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					destination: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					amount: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::TransferApproved> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"transfer_approved",
						types::TransferApproved { id, owner, destination, amount },
						[
							54u8, 164u8, 26u8, 42u8, 226u8, 154u8, 102u8, 156u8, 216u8, 24u8, 11u8,
							34u8, 93u8, 126u8, 58u8, 139u8, 194u8, 43u8, 208u8, 203u8, 166u8,
							177u8, 99u8, 86u8, 111u8, 58u8, 200u8, 105u8, 49u8, 41u8, 143u8, 121u8,
						],
					)
				}
				#[doc = "See [`Pallet::touch`]."]
				pub fn touch(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				) -> ::subxt::tx::Payload<types::Touch> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"touch",
						types::Touch { id },
						[
							42u8, 151u8, 196u8, 52u8, 233u8, 90u8, 70u8, 15u8, 191u8, 203u8, 173u8,
							225u8, 251u8, 200u8, 209u8, 187u8, 180u8, 238u8, 73u8, 136u8, 232u8,
							89u8, 32u8, 121u8, 192u8, 115u8, 248u8, 240u8, 44u8, 24u8, 61u8, 15u8,
						],
					)
				}
				#[doc = "See [`Pallet::refund`]."]
				pub fn refund(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					allow_burn: ::core::primitive::bool,
				) -> ::subxt::tx::Payload<types::Refund> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"refund",
						types::Refund { id, allow_burn },
						[
							146u8, 138u8, 19u8, 215u8, 33u8, 157u8, 99u8, 109u8, 30u8, 206u8,
							140u8, 34u8, 232u8, 222u8, 40u8, 202u8, 225u8, 145u8, 52u8, 78u8, 85u8,
							82u8, 227u8, 170u8, 186u8, 211u8, 50u8, 178u8, 153u8, 220u8, 18u8,
							92u8,
						],
					)
				}
				#[doc = "See [`Pallet::set_min_balance`]."]
				pub fn set_min_balance(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					min_balance: ::core::primitive::u128,
				) -> ::subxt::tx::Payload<types::SetMinBalance> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"set_min_balance",
						types::SetMinBalance { id, min_balance },
						[
							84u8, 1u8, 220u8, 65u8, 248u8, 40u8, 231u8, 57u8, 81u8, 181u8, 184u8,
							100u8, 107u8, 150u8, 231u8, 252u8, 108u8, 66u8, 214u8, 213u8, 75u8,
							102u8, 48u8, 76u8, 93u8, 127u8, 210u8, 111u8, 93u8, 219u8, 252u8,
							173u8,
						],
					)
				}
				#[doc = "See [`Pallet::touch_other`]."]
				pub fn touch_other(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::TouchOther> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"touch_other",
						types::TouchOther { id, who },
						[
							137u8, 73u8, 220u8, 107u8, 20u8, 207u8, 2u8, 189u8, 79u8, 114u8, 37u8,
							126u8, 147u8, 195u8, 169u8, 193u8, 21u8, 78u8, 209u8, 73u8, 121u8,
							43u8, 102u8, 196u8, 231u8, 98u8, 128u8, 188u8, 194u8, 104u8, 20u8,
							175u8,
						],
					)
				}
				#[doc = "See [`Pallet::refund_other`]."]
				pub fn refund_other(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::RefundOther> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"refund_other",
						types::RefundOther { id, who },
						[
							41u8, 235u8, 20u8, 27u8, 26u8, 242u8, 236u8, 245u8, 125u8, 98u8, 2u8,
							162u8, 20u8, 86u8, 74u8, 25u8, 138u8, 209u8, 205u8, 54u8, 145u8, 48u8,
							152u8, 96u8, 154u8, 108u8, 5u8, 51u8, 113u8, 188u8, 166u8, 51u8,
						],
					)
				}
				#[doc = "See [`Pallet::block`]."]
				pub fn block(
					&self,
					id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
				) -> ::subxt::tx::Payload<types::Block> {
					::subxt::tx::Payload::new_static(
						"Assets",
						"block",
						types::Block { id, who },
						[
							66u8, 192u8, 226u8, 215u8, 140u8, 35u8, 214u8, 45u8, 86u8, 247u8, 43u8,
							247u8, 99u8, 154u8, 17u8, 224u8, 124u8, 75u8, 69u8, 15u8, 137u8, 85u8,
							196u8, 8u8, 116u8, 226u8, 203u8, 25u8, 125u8, 251u8, 16u8, 79u8,
						],
					)
				}
			}
		}
		#[doc = "The `Event` enum of this pallet"]
		pub type Event = runtime_types::pallet_assets::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some asset class was created."]
			pub struct Created {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub creator: ::subxt::utils::AccountId32,
				pub owner: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Created {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Created";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some assets were issued."]
			pub struct Issued {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Issued {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Issued";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some assets were transferred."]
			pub struct Transferred {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub from: ::subxt::utils::AccountId32,
				pub to: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Transferred {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Transferred";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some assets were destroyed."]
			pub struct Burned {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
				pub balance: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for Burned {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Burned";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The management team changed."]
			pub struct TeamChanged {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub issuer: ::subxt::utils::AccountId32,
				pub admin: ::subxt::utils::AccountId32,
				pub freezer: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for TeamChanged {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "TeamChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The owner changed."]
			pub struct OwnerChanged {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for OwnerChanged {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "OwnerChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some account `who` was frozen."]
			pub struct Frozen {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Frozen {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Frozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some account `who` was thawed."]
			pub struct Thawed {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Thawed {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Thawed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some asset `asset_id` was frozen."]
			pub struct AssetFrozen {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for AssetFrozen {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "AssetFrozen";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some asset `asset_id` was thawed."]
			pub struct AssetThawed {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for AssetThawed {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "AssetThawed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Accounts were destroyed for given asset."]
			pub struct AccountsDestroyed {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub accounts_destroyed: ::core::primitive::u32,
				pub accounts_remaining: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for AccountsDestroyed {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "AccountsDestroyed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Approvals were destroyed for given asset."]
			pub struct ApprovalsDestroyed {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub approvals_destroyed: ::core::primitive::u32,
				pub approvals_remaining: ::core::primitive::u32,
			}
			impl ::subxt::events::StaticEvent for ApprovalsDestroyed {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "ApprovalsDestroyed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An asset class is in the process of being destroyed."]
			pub struct DestructionStarted {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for DestructionStarted {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "DestructionStarted";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An asset class was destroyed."]
			pub struct Destroyed {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for Destroyed {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Destroyed";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some asset class was force-created."]
			pub struct ForceCreated {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for ForceCreated {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "ForceCreated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "New metadata has been set for an asset."]
			pub struct MetadataSet {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub name: ::std::vec::Vec<::core::primitive::u8>,
				pub symbol: ::std::vec::Vec<::core::primitive::u8>,
				pub decimals: ::core::primitive::u8,
				pub is_frozen: ::core::primitive::bool,
			}
			impl ::subxt::events::StaticEvent for MetadataSet {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "MetadataSet";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Metadata has been cleared for an asset."]
			pub struct MetadataCleared {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for MetadataCleared {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "MetadataCleared";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "(Additional) funds have been approved for transfer to a destination account."]
			pub struct ApprovedTransfer {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub source: ::subxt::utils::AccountId32,
				pub delegate: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for ApprovedTransfer {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "ApprovedTransfer";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An approval for account `delegate` was cancelled by `owner`."]
			pub struct ApprovalCancelled {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
				pub delegate: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for ApprovalCancelled {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "ApprovalCancelled";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An `amount` was transferred in its entirety from `owner` to `destination` by"]
			#[doc = "the approved `delegate`."]
			pub struct TransferredApproved {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub owner: ::subxt::utils::AccountId32,
				pub delegate: ::subxt::utils::AccountId32,
				pub destination: ::subxt::utils::AccountId32,
				pub amount: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for TransferredApproved {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "TransferredApproved";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "An asset has had its attributes changed by the `Force` origin."]
			pub struct AssetStatusChanged {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
			}
			impl ::subxt::events::StaticEvent for AssetStatusChanged {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "AssetStatusChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The min_balance of an asset has been updated by the asset owner."]
			pub struct AssetMinBalanceChanged {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub new_min_balance: ::core::primitive::u128,
			}
			impl ::subxt::events::StaticEvent for AssetMinBalanceChanged {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "AssetMinBalanceChanged";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some account `who` was created with a deposit from `depositor`."]
			pub struct Touched {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub who: ::subxt::utils::AccountId32,
				pub depositor: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Touched {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Touched";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "Some account `who` was blocked."]
			pub struct Blocked {
				pub asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
				pub who: ::subxt::utils::AccountId32,
			}
			impl ::subxt::events::StaticEvent for Blocked {
				const PALLET: &'static str = "Assets";
				const EVENT: &'static str = "Blocked";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Details of an asset."]
				pub fn asset(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetDetails<
						::core::primitive::u128,
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Asset",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							112u8, 239u8, 115u8, 203u8, 164u8, 251u8, 180u8, 224u8, 110u8, 236u8,
							14u8, 48u8, 236u8, 62u8, 237u8, 92u8, 48u8, 28u8, 183u8, 214u8, 89u8,
							246u8, 62u8, 78u8, 135u8, 217u8, 84u8, 211u8, 52u8, 198u8, 55u8, 142u8,
						],
					)
				}
				#[doc = " Details of an asset."]
				pub fn asset_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetDetails<
						::core::primitive::u128,
						::subxt::utils::AccountId32,
						::core::primitive::u128,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Asset",
						Vec::new(),
						[
							112u8, 239u8, 115u8, 203u8, 164u8, 251u8, 180u8, 224u8, 110u8, 236u8,
							14u8, 48u8, 236u8, 62u8, 237u8, 92u8, 48u8, 28u8, 183u8, 214u8, 89u8,
							246u8, 62u8, 78u8, 135u8, 217u8, 84u8, 211u8, 52u8, 198u8, 55u8, 142u8,
						],
					)
				}
				#[doc = " The holdings of a specific account for a specific asset."]
				pub fn account(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					>,
					_1: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetAccount<
						::core::primitive::u128,
						::core::primitive::u128,
						(),
						::subxt::utils::AccountId32,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Account",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							33u8, 209u8, 210u8, 175u8, 24u8, 163u8, 246u8, 28u8, 57u8, 151u8, 19u8,
							173u8, 106u8, 155u8, 141u8, 240u8, 102u8, 208u8, 72u8, 61u8, 29u8,
							203u8, 218u8, 191u8, 72u8, 204u8, 35u8, 129u8, 3u8, 190u8, 20u8, 164u8,
						],
					)
				}
				#[doc = " The holdings of a specific account for a specific asset."]
				pub fn account_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetAccount<
						::core::primitive::u128,
						::core::primitive::u128,
						(),
						::subxt::utils::AccountId32,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Account",
						Vec::new(),
						[
							33u8, 209u8, 210u8, 175u8, 24u8, 163u8, 246u8, 28u8, 57u8, 151u8, 19u8,
							173u8, 106u8, 155u8, 141u8, 240u8, 102u8, 208u8, 72u8, 61u8, 29u8,
							203u8, 218u8, 191u8, 72u8, 204u8, 35u8, 129u8, 3u8, 190u8, 20u8, 164u8,
						],
					)
				}
				#[doc = " Approved balance transfers. First balance is the amount approved for transfer. Second"]
				#[doc = " is the amount of `T::Currency` reserved for storing this."]
				#[doc = " First key is the asset ID, second key is the owner and third key is the delegate."]
				pub fn approvals(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					>,
					_1: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
					_2: impl ::std::borrow::Borrow<::subxt::utils::AccountId32>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::Approval<
						::core::primitive::u128,
						::core::primitive::u128,
					>,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Approvals",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_2.borrow()),
						],
						[
							142u8, 228u8, 80u8, 196u8, 155u8, 70u8, 48u8, 80u8, 182u8, 157u8,
							124u8, 50u8, 25u8, 204u8, 78u8, 90u8, 198u8, 59u8, 26u8, 78u8, 113u8,
							142u8, 110u8, 25u8, 133u8, 92u8, 8u8, 200u8, 234u8, 221u8, 223u8,
							188u8,
						],
					)
				}
				#[doc = " Approved balance transfers. First balance is the amount approved for transfer. Second"]
				#[doc = " is the amount of `T::Currency` reserved for storing this."]
				#[doc = " First key is the asset ID, second key is the owner and third key is the delegate."]
				pub fn approvals_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::Approval<
						::core::primitive::u128,
						::core::primitive::u128,
					>,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Approvals",
						Vec::new(),
						[
							142u8, 228u8, 80u8, 196u8, 155u8, 70u8, 48u8, 80u8, 182u8, 157u8,
							124u8, 50u8, 25u8, 204u8, 78u8, 90u8, 198u8, 59u8, 26u8, 78u8, 113u8,
							142u8, 110u8, 25u8, 133u8, 92u8, 8u8, 200u8, 234u8, 221u8, 223u8,
							188u8,
						],
					)
				}
				#[doc = " Metadata of an asset."]
				pub fn metadata(
					&self,
					_0: impl ::std::borrow::Borrow<
						runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetMetadata<
						::core::primitive::u128,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Metadata",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							134u8, 126u8, 216u8, 12u8, 39u8, 11u8, 197u8, 142u8, 106u8, 159u8,
							30u8, 25u8, 200u8, 100u8, 95u8, 81u8, 13u8, 139u8, 2u8, 50u8, 91u8,
							249u8, 166u8, 96u8, 198u8, 104u8, 102u8, 155u8, 113u8, 7u8, 112u8,
							116u8,
						],
					)
				}
				#[doc = " Metadata of an asset."]
				pub fn metadata_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_assets::types::AssetMetadata<
						::core::primitive::u128,
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
					(),
					::subxt::storage::address::Yes,
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"Assets",
						"Metadata",
						Vec::new(),
						[
							134u8, 126u8, 216u8, 12u8, 39u8, 11u8, 197u8, 142u8, 106u8, 159u8,
							30u8, 25u8, 200u8, 100u8, 95u8, 81u8, 13u8, 139u8, 2u8, 50u8, 91u8,
							249u8, 166u8, 96u8, 198u8, 104u8, 102u8, 155u8, 113u8, 7u8, 112u8,
							116u8,
						],
					)
				}
			}
		}
		pub mod constants {
			use super::runtime_types;
			pub struct ConstantsApi;
			impl ConstantsApi {
				#[doc = " Max number of items to destroy per `destroy_accounts` and `destroy_approvals` call."]
				#[doc = ""]
				#[doc = " Must be configured to result in a weight that makes each call fit in a block."]
				pub fn remove_items_limit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Assets",
						"RemoveItemsLimit",
						[
							98u8, 252u8, 116u8, 72u8, 26u8, 180u8, 225u8, 83u8, 200u8, 157u8,
							125u8, 151u8, 53u8, 76u8, 168u8, 26u8, 10u8, 9u8, 98u8, 68u8, 9u8,
							178u8, 197u8, 113u8, 31u8, 79u8, 200u8, 90u8, 203u8, 100u8, 41u8,
							145u8,
						],
					)
				}
				#[doc = " The basic amount of funds that must be reserved for an asset."]
				pub fn asset_deposit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Assets",
						"AssetDeposit",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The amount of funds that must be reserved for a non-provider asset account to be"]
				#[doc = " maintained."]
				pub fn asset_account_deposit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Assets",
						"AssetAccountDeposit",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The basic amount of funds that must be reserved when adding metadata to your asset."]
				pub fn metadata_deposit_base(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Assets",
						"MetadataDepositBase",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The additional funds that must be reserved for the number of bytes you store in your"]
				#[doc = " metadata."]
				pub fn metadata_deposit_per_byte(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Assets",
						"MetadataDepositPerByte",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The amount of funds that must be reserved when creating a new approval."]
				pub fn approval_deposit(
					&self,
				) -> ::subxt::constants::Address<::core::primitive::u128> {
					::subxt::constants::Address::new_static(
						"Assets",
						"ApprovalDeposit",
						[
							84u8, 157u8, 140u8, 4u8, 93u8, 57u8, 29u8, 133u8, 105u8, 200u8, 214u8,
							27u8, 144u8, 208u8, 218u8, 160u8, 130u8, 109u8, 101u8, 54u8, 210u8,
							136u8, 71u8, 63u8, 49u8, 237u8, 234u8, 15u8, 178u8, 98u8, 148u8, 156u8,
						],
					)
				}
				#[doc = " The maximum length of a name or symbol stored on-chain."]
				pub fn string_limit(&self) -> ::subxt::constants::Address<::core::primitive::u32> {
					::subxt::constants::Address::new_static(
						"Assets",
						"StringLimit",
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
	pub mod token_governor {
		use super::{root_mod, runtime_types};
		#[doc = "Errors that can be returned by this pallet."]
		pub type Error = runtime_types::pallet_token_governor::pallet::Error;
		#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
		pub type Call = runtime_types::pallet_token_governor::pallet::Call;
		pub mod calls {
			use super::{root_mod, runtime_types};
			type DispatchError = runtime_types::sp_runtime::DispatchError;
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateErc6160Asset {
					pub asset:
						runtime_types::pallet_token_governor::types::ERC6160AssetRegistration,
				}
				impl ::subxt::blocks::StaticExtrinsic for CreateErc6160Asset {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "create_erc6160_asset";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateErc6160AssetUnsigned { pub registration : runtime_types :: pallet_token_governor :: types :: UnsignedERC6160AssetRegistration < :: subxt :: utils :: AccountId32 > , }
				impl ::subxt::blocks::StaticExtrinsic for CreateErc6160AssetUnsigned {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "create_erc6160_asset_unsigned";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateRegistrarParams {
					pub update: runtime_types::pallet_token_governor::types::RegistrarParamsUpdate,
					pub state_machine: runtime_types::ismp::host::StateMachine,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateRegistrarParams {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "update_registrar_params";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateGatewayParams {
					pub update:
						runtime_types::pallet_token_governor::types::TokenGatewayParamsUpdate,
					pub state_machine: runtime_types::ismp::host::StateMachine,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateGatewayParams {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "update_gateway_params";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateParams {
					pub update: runtime_types::pallet_token_governor::types::ParamsUpdate<
						::core::primitive::u128,
					>,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateParams {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "update_params";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateErc6160Asset {
					pub update: runtime_types::pallet_token_governor::types::ERC6160AssetUpdate,
				}
				impl ::subxt::blocks::StaticExtrinsic for UpdateErc6160Asset {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "update_erc6160_asset";
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateErc20Asset {
					pub asset: runtime_types::pallet_token_governor::types::ERC20AssetRegistration,
				}
				impl ::subxt::blocks::StaticExtrinsic for CreateErc20Asset {
					const PALLET: &'static str = "TokenGovernor";
					const CALL: &'static str = "create_erc20_asset";
				}
			}
			pub struct TransactionApi;
			impl TransactionApi {
				#[doc = "See [`Pallet::create_erc6160_asset`]."]
				pub fn create_erc6160_asset(
					&self,
					asset: runtime_types::pallet_token_governor::types::ERC6160AssetRegistration,
				) -> ::subxt::tx::Payload<types::CreateErc6160Asset> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"create_erc6160_asset",
						types::CreateErc6160Asset { asset },
						[
							67u8, 36u8, 47u8, 51u8, 25u8, 173u8, 1u8, 19u8, 11u8, 57u8, 238u8,
							98u8, 163u8, 134u8, 237u8, 13u8, 244u8, 72u8, 160u8, 107u8, 250u8,
							238u8, 60u8, 20u8, 255u8, 103u8, 254u8, 207u8, 51u8, 48u8, 124u8,
							203u8,
						],
					)
				}
				#[doc = "See [`Pallet::create_erc6160_asset_unsigned`]."]
				pub fn create_erc6160_asset_unsigned(
					&self,
					registration : runtime_types :: pallet_token_governor :: types :: UnsignedERC6160AssetRegistration < :: subxt :: utils :: AccountId32 >,
				) -> ::subxt::tx::Payload<types::CreateErc6160AssetUnsigned> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"create_erc6160_asset_unsigned",
						types::CreateErc6160AssetUnsigned { registration },
						[
							219u8, 99u8, 229u8, 59u8, 222u8, 101u8, 204u8, 228u8, 7u8, 7u8, 84u8,
							108u8, 100u8, 6u8, 23u8, 162u8, 123u8, 112u8, 182u8, 83u8, 135u8, 89u8,
							237u8, 161u8, 75u8, 41u8, 162u8, 29u8, 254u8, 207u8, 224u8, 223u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_registrar_params`]."]
				pub fn update_registrar_params(
					&self,
					update: runtime_types::pallet_token_governor::types::RegistrarParamsUpdate,
					state_machine: runtime_types::ismp::host::StateMachine,
				) -> ::subxt::tx::Payload<types::UpdateRegistrarParams> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"update_registrar_params",
						types::UpdateRegistrarParams { update, state_machine },
						[
							51u8, 62u8, 50u8, 183u8, 204u8, 88u8, 36u8, 12u8, 148u8, 181u8, 30u8,
							22u8, 133u8, 80u8, 30u8, 39u8, 0u8, 146u8, 150u8, 76u8, 208u8, 26u8,
							8u8, 111u8, 223u8, 61u8, 95u8, 110u8, 104u8, 99u8, 34u8, 141u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_gateway_params`]."]
				pub fn update_gateway_params(
					&self,
					update: runtime_types::pallet_token_governor::types::TokenGatewayParamsUpdate,
					state_machine: runtime_types::ismp::host::StateMachine,
				) -> ::subxt::tx::Payload<types::UpdateGatewayParams> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"update_gateway_params",
						types::UpdateGatewayParams { update, state_machine },
						[
							217u8, 70u8, 200u8, 128u8, 216u8, 245u8, 80u8, 187u8, 61u8, 75u8, 85u8,
							40u8, 126u8, 68u8, 152u8, 100u8, 205u8, 176u8, 136u8, 125u8, 213u8,
							17u8, 119u8, 141u8, 77u8, 10u8, 139u8, 170u8, 19u8, 154u8, 131u8, 48u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_params`]."]
				pub fn update_params(
					&self,
					update: runtime_types::pallet_token_governor::types::ParamsUpdate<
						::core::primitive::u128,
					>,
				) -> ::subxt::tx::Payload<types::UpdateParams> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"update_params",
						types::UpdateParams { update },
						[
							75u8, 79u8, 195u8, 173u8, 123u8, 209u8, 215u8, 5u8, 229u8, 70u8, 23u8,
							85u8, 106u8, 167u8, 208u8, 101u8, 112u8, 146u8, 148u8, 187u8, 192u8,
							225u8, 124u8, 139u8, 254u8, 179u8, 180u8, 112u8, 243u8, 77u8, 124u8,
							182u8,
						],
					)
				}
				#[doc = "See [`Pallet::update_erc6160_asset`]."]
				pub fn update_erc6160_asset(
					&self,
					update: runtime_types::pallet_token_governor::types::ERC6160AssetUpdate,
				) -> ::subxt::tx::Payload<types::UpdateErc6160Asset> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"update_erc6160_asset",
						types::UpdateErc6160Asset { update },
						[
							90u8, 142u8, 58u8, 163u8, 10u8, 133u8, 200u8, 14u8, 17u8, 255u8, 85u8,
							35u8, 72u8, 155u8, 175u8, 95u8, 112u8, 5u8, 51u8, 50u8, 220u8, 185u8,
							105u8, 219u8, 112u8, 73u8, 193u8, 209u8, 62u8, 255u8, 174u8, 179u8,
						],
					)
				}
				#[doc = "See [`Pallet::create_erc20_asset`]."]
				pub fn create_erc20_asset(
					&self,
					asset: runtime_types::pallet_token_governor::types::ERC20AssetRegistration,
				) -> ::subxt::tx::Payload<types::CreateErc20Asset> {
					::subxt::tx::Payload::new_static(
						"TokenGovernor",
						"create_erc20_asset",
						types::CreateErc20Asset { asset },
						[
							50u8, 47u8, 222u8, 221u8, 255u8, 229u8, 76u8, 96u8, 77u8, 105u8, 250u8,
							89u8, 187u8, 176u8, 184u8, 180u8, 81u8, 221u8, 123u8, 233u8, 168u8,
							45u8, 20u8, 127u8, 217u8, 184u8, 16u8, 79u8, 77u8, 255u8, 133u8, 202u8,
						],
					)
				}
			}
		}
		#[doc = "Pallet events that functions in this pallet can emit."]
		pub type Event = runtime_types::pallet_token_governor::pallet::Event;
		pub mod events {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new asset has been registered"]
			pub struct AssetRegistered {
				pub asset_id: ::subxt::utils::H256,
			}
			impl ::subxt::events::StaticEvent for AssetRegistered {
				const PALLET: &'static str = "TokenGovernor";
				const EVENT: &'static str = "AssetRegistered";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "A new pending asset has been registered"]
			pub struct NewPendingAsset {
				pub asset_id: ::subxt::utils::H256,
				pub owner: ::subxt::utils::H160,
			}
			impl ::subxt::events::StaticEvent for NewPendingAsset {
				const PALLET: &'static str = "TokenGovernor";
				const EVENT: &'static str = "NewPendingAsset";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The TokenRegistrar params have been updated for a state machine"]
			pub struct RegistrarParamsUpdated {
				pub old: runtime_types::pallet_token_governor::types::RegistrarParams,
				pub new: runtime_types::pallet_token_governor::types::RegistrarParams,
				pub state_machine: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for RegistrarParamsUpdated {
				const PALLET: &'static str = "TokenGovernor";
				const EVENT: &'static str = "RegistrarParamsUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The TokenGateway params have been updated for a state machine"]
			pub struct GatewayParamsUpdated {
				pub old: runtime_types::pallet_token_governor::types::GatewayParams,
				pub new: runtime_types::pallet_token_governor::types::GatewayParams,
				pub state_machine: runtime_types::ismp::host::StateMachine,
			}
			impl ::subxt::events::StaticEvent for GatewayParamsUpdated {
				const PALLET: &'static str = "TokenGovernor";
				const EVENT: &'static str = "GatewayParamsUpdated";
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			#[doc = "The TokenGovernor parameters have been updated"]
			pub struct ParamsUpdated {
				pub old:
					runtime_types::pallet_token_governor::types::Params<::core::primitive::u128>,
				pub new:
					runtime_types::pallet_token_governor::types::Params<::core::primitive::u128>,
			}
			impl ::subxt::events::StaticEvent for ParamsUpdated {
				const PALLET: &'static str = "TokenGovernor";
				const EVENT: &'static str = "ParamsUpdated";
			}
		}
		pub mod storage {
			use super::runtime_types;
			pub struct StorageApi;
			impl StorageApi {
				#[doc = " Maps a pending asset to it's owner. Enables asset registration without the native token by"]
				#[doc = " prviding a single-use execution ticket for asset creation through an unsigned transaction."]
				pub fn pending_asset(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H160,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"PendingAsset",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							30u8, 44u8, 145u8, 81u8, 77u8, 192u8, 25u8, 54u8, 115u8, 1u8, 107u8,
							181u8, 231u8, 140u8, 41u8, 221u8, 139u8, 53u8, 131u8, 162u8, 241u8,
							91u8, 76u8, 28u8, 118u8, 47u8, 238u8, 217u8, 237u8, 144u8, 155u8,
							211u8,
						],
					)
				}
				#[doc = " Maps a pending asset to it's owner. Enables asset registration without the native token by"]
				#[doc = " prviding a single-use execution ticket for asset creation through an unsigned transaction."]
				pub fn pending_asset_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::H160,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"PendingAsset",
						Vec::new(),
						[
							30u8, 44u8, 145u8, 81u8, 77u8, 192u8, 25u8, 54u8, 115u8, 1u8, 107u8,
							181u8, 231u8, 140u8, 41u8, 221u8, 139u8, 53u8, 131u8, 162u8, 241u8,
							91u8, 76u8, 28u8, 118u8, 47u8, 238u8, 217u8, 237u8, 144u8, 155u8,
							211u8,
						],
					)
				}
				#[doc = " Tracks which assets have been deployed to which chains. Uses bool to ensure its persisted to"]
				#[doc = " the underlying trie"]
				pub fn supported_chains(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
					_1: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"SupportedChains",
						vec![
							::subxt::storage::address::make_static_storage_map_key(_0.borrow()),
							::subxt::storage::address::make_static_storage_map_key(_1.borrow()),
						],
						[
							251u8, 252u8, 44u8, 194u8, 78u8, 84u8, 99u8, 165u8, 98u8, 0u8, 63u8,
							61u8, 179u8, 90u8, 186u8, 136u8, 216u8, 43u8, 236u8, 26u8, 145u8,
							239u8, 48u8, 124u8, 189u8, 10u8, 133u8, 102u8, 98u8, 181u8, 254u8,
							19u8,
						],
					)
				}
				#[doc = " Tracks which assets have been deployed to which chains. Uses bool to ensure its persisted to"]
				#[doc = " the underlying trie"]
				pub fn supported_chains_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::core::primitive::bool,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"SupportedChains",
						Vec::new(),
						[
							251u8, 252u8, 44u8, 194u8, 78u8, 84u8, 99u8, 165u8, 98u8, 0u8, 63u8,
							61u8, 179u8, 90u8, 186u8, 136u8, 216u8, 43u8, 236u8, 26u8, 145u8,
							239u8, 48u8, 124u8, 189u8, 10u8, 133u8, 102u8, 98u8, 181u8, 254u8,
							19u8,
						],
					)
				}
				#[doc = " Mapping of AssetId to their metadata"]
				pub fn asset_metadatas(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::AssetMetadata,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"AssetMetadatas",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							188u8, 166u8, 178u8, 194u8, 165u8, 118u8, 246u8, 165u8, 140u8, 235u8,
							209u8, 21u8, 213u8, 43u8, 48u8, 227u8, 211u8, 151u8, 143u8, 36u8,
							231u8, 2u8, 153u8, 106u8, 97u8, 89u8, 135u8, 191u8, 200u8, 53u8, 28u8,
							9u8,
						],
					)
				}
				#[doc = " Mapping of AssetId to their metadata"]
				pub fn asset_metadatas_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::AssetMetadata,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"AssetMetadatas",
						Vec::new(),
						[
							188u8, 166u8, 178u8, 194u8, 165u8, 118u8, 246u8, 165u8, 140u8, 235u8,
							209u8, 21u8, 213u8, 43u8, 48u8, 227u8, 211u8, 151u8, 143u8, 36u8,
							231u8, 2u8, 153u8, 106u8, 97u8, 89u8, 135u8, 191u8, 200u8, 53u8, 28u8,
							9u8,
						],
					)
				}
				#[doc = " Mapping of AssetIds to their owners"]
				pub fn asset_owners(
					&self,
					_0: impl ::std::borrow::Borrow<::subxt::utils::H256>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"AssetOwners",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							206u8, 188u8, 20u8, 204u8, 58u8, 192u8, 21u8, 106u8, 194u8, 205u8,
							38u8, 214u8, 20u8, 133u8, 121u8, 208u8, 102u8, 114u8, 146u8, 98u8,
							20u8, 55u8, 174u8, 153u8, 88u8, 4u8, 5u8, 48u8, 201u8, 111u8, 49u8,
							210u8,
						],
					)
				}
				#[doc = " Mapping of AssetIds to their owners"]
				pub fn asset_owners_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					::subxt::utils::AccountId32,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"AssetOwners",
						Vec::new(),
						[
							206u8, 188u8, 20u8, 204u8, 58u8, 192u8, 21u8, 106u8, 194u8, 205u8,
							38u8, 214u8, 20u8, 133u8, 121u8, 208u8, 102u8, 114u8, 146u8, 98u8,
							20u8, 55u8, 174u8, 153u8, 88u8, 4u8, 5u8, 48u8, 201u8, 111u8, 49u8,
							210u8,
						],
					)
				}
				#[doc = " TokenGovernor protocol parameters."]
				pub fn protocol_params(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::Params<::core::primitive::u128>,
					::subxt::storage::address::Yes,
					(),
					(),
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"ProtocolParams",
						vec![],
						[
							149u8, 32u8, 176u8, 85u8, 230u8, 81u8, 178u8, 187u8, 228u8, 91u8,
							151u8, 80u8, 195u8, 77u8, 163u8, 176u8, 122u8, 255u8, 238u8, 2u8,
							194u8, 144u8, 41u8, 165u8, 57u8, 250u8, 191u8, 128u8, 222u8, 14u8,
							83u8, 201u8,
						],
					)
				}
				#[doc = " TokenRegistrar protocol parameters."]
				pub fn token_registrar_params(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::RegistrarParams,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"TokenRegistrarParams",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							205u8, 100u8, 12u8, 102u8, 117u8, 8u8, 56u8, 61u8, 91u8, 233u8, 51u8,
							108u8, 156u8, 244u8, 63u8, 148u8, 186u8, 51u8, 141u8, 149u8, 115u8,
							55u8, 80u8, 198u8, 30u8, 12u8, 19u8, 92u8, 117u8, 182u8, 48u8, 218u8,
						],
					)
				}
				#[doc = " TokenRegistrar protocol parameters."]
				pub fn token_registrar_params_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::RegistrarParams,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"TokenRegistrarParams",
						Vec::new(),
						[
							205u8, 100u8, 12u8, 102u8, 117u8, 8u8, 56u8, 61u8, 91u8, 233u8, 51u8,
							108u8, 156u8, 244u8, 63u8, 148u8, 186u8, 51u8, 141u8, 149u8, 115u8,
							55u8, 80u8, 198u8, 30u8, 12u8, 19u8, 92u8, 117u8, 182u8, 48u8, 218u8,
						],
					)
				}
				#[doc = " TokenGateway protocol parameters."]
				pub fn token_gateway_params(
					&self,
					_0: impl ::std::borrow::Borrow<runtime_types::ismp::host::StateMachine>,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::GatewayParams,
					::subxt::storage::address::Yes,
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"TokenGatewayParams",
						vec![::subxt::storage::address::make_static_storage_map_key(_0.borrow())],
						[
							8u8, 85u8, 14u8, 105u8, 6u8, 39u8, 173u8, 164u8, 198u8, 7u8, 127u8,
							104u8, 182u8, 20u8, 197u8, 222u8, 149u8, 91u8, 34u8, 158u8, 254u8,
							204u8, 248u8, 37u8, 229u8, 222u8, 49u8, 39u8, 190u8, 162u8, 211u8,
							228u8,
						],
					)
				}
				#[doc = " TokenGateway protocol parameters."]
				pub fn token_gateway_params_root(
					&self,
				) -> ::subxt::storage::address::Address<
					::subxt::storage::address::StaticStorageMapKey,
					runtime_types::pallet_token_governor::types::GatewayParams,
					(),
					(),
					::subxt::storage::address::Yes,
				> {
					::subxt::storage::address::Address::new_static(
						"TokenGovernor",
						"TokenGatewayParams",
						Vec::new(),
						[
							8u8, 85u8, 14u8, 105u8, 6u8, 39u8, 173u8, 164u8, 198u8, 7u8, 127u8,
							104u8, 182u8, 20u8, 197u8, 222u8, 149u8, 91u8, 34u8, 158u8, 254u8,
							204u8, 248u8, 37u8, 229u8, 222u8, 49u8, 39u8, 190u8, 162u8, 211u8,
							228u8,
						],
					)
				}
			}
		}
	}
	pub mod runtime_types {
		use super::runtime_types;
		pub mod bounded_collections {
			use super::runtime_types;
			pub mod bounded_btree_set {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BoundedBTreeSet<_0>(pub ::std::vec::Vec<_0>);
			}
			pub mod bounded_vec {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BoundedVec<_0>(pub ::std::vec::Vec<_0>);
			}
			pub mod weak_bounded_vec {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WeakBoundedVec<_0>(pub ::std::vec::Vec<_0>);
			}
		}
		pub mod cumulus_pallet_parachain_system {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					# [codec (index = 0)] # [doc = "See [`Pallet::set_validation_data`]."] set_validation_data { data : runtime_types :: cumulus_primitives_parachain_inherent :: ParachainInherentData , } , # [codec (index = 1)] # [doc = "See [`Pallet::sudo_send_upward_message`]."] sudo_send_upward_message { message : :: std :: vec :: Vec < :: core :: primitive :: u8 > , } , # [codec (index = 2)] # [doc = "See [`Pallet::authorize_upgrade`]."] authorize_upgrade { code_hash : :: subxt :: utils :: H256 , check_version : :: core :: primitive :: bool , } , # [codec (index = 3)] # [doc = "See [`Pallet::enact_authorized_upgrade`]."] enact_authorized_upgrade { code : :: std :: vec :: Vec < :: core :: primitive :: u8 > , } , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Attempt to upgrade validation function while existing upgrade pending."]
					OverlappingUpgrades,
					#[codec(index = 1)]
					#[doc = "Polkadot currently prohibits this parachain from upgrading its validation function."]
					ProhibitedByPolkadot,
					#[codec(index = 2)]
					#[doc = "The supplied validation function has compiled into a blob larger than Polkadot is"]
					#[doc = "willing to run."]
					TooBig,
					#[codec(index = 3)]
					#[doc = "The inherent which supplies the validation data did not run this block."]
					ValidationDataNotAvailable,
					#[codec(index = 4)]
					#[doc = "The inherent which supplies the host configuration did not run this block."]
					HostConfigurationNotAvailable,
					#[codec(index = 5)]
					#[doc = "No validation function upgrade is currently scheduled."]
					NotScheduled,
					#[codec(index = 6)]
					#[doc = "No code upgrade has been authorized."]
					NothingAuthorized,
					#[codec(index = 7)]
					#[doc = "The given code upgrade has not been authorized."]
					Unauthorized,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "The validation function has been scheduled to apply."]
					ValidationFunctionStored,
					#[codec(index = 1)]
					#[doc = "The validation function was applied as of the contained relay chain block number."]
					ValidationFunctionApplied { relay_chain_block_num: ::core::primitive::u32 },
					#[codec(index = 2)]
					#[doc = "The relay-chain aborted the upgrade process."]
					ValidationFunctionDiscarded,
					#[codec(index = 3)]
					#[doc = "Some downward messages have been received and will be processed."]
					DownwardMessagesReceived { count: ::core::primitive::u32 },
					#[codec(index = 4)]
					#[doc = "Downward messages were processed using the given weight."]
					DownwardMessagesProcessed {
						weight_used: runtime_types::sp_weights::weight_v2::Weight,
						dmq_head: ::subxt::utils::H256,
					},
					#[codec(index = 5)]
					#[doc = "An upward message was sent to the relay chain."]
					UpwardMessageSent {
						message_hash: ::core::option::Option<[::core::primitive::u8; 32usize]>,
					},
				}
			}
			pub mod relay_state_snapshot {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct MessagingStateSnapshot { pub dmq_mqc_head : :: subxt :: utils :: H256 , pub relay_dispatch_queue_remaining_capacity : runtime_types :: cumulus_pallet_parachain_system :: relay_state_snapshot :: RelayDispatchQueueRemainingCapacity , pub ingress_channels : :: std :: vec :: Vec < (runtime_types :: polkadot_parachain_primitives :: primitives :: Id , runtime_types :: polkadot_primitives :: v6 :: AbridgedHrmpChannel ,) > , pub egress_channels : :: std :: vec :: Vec < (runtime_types :: polkadot_parachain_primitives :: primitives :: Id , runtime_types :: polkadot_primitives :: v6 :: AbridgedHrmpChannel ,) > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RelayDispatchQueueRemainingCapacity {
					pub remaining_count: ::core::primitive::u32,
					pub remaining_size: ::core::primitive::u32,
				}
			}
			pub mod unincluded_segment {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Ancestor < _0 > { pub used_bandwidth : runtime_types :: cumulus_pallet_parachain_system :: unincluded_segment :: UsedBandwidth , pub para_head_hash : :: core :: option :: Option < _0 > , pub consumed_go_ahead_signal : :: core :: option :: Option < runtime_types :: polkadot_primitives :: v6 :: UpgradeGoAhead > , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct HrmpChannelUpdate {
					pub msg_count: ::core::primitive::u32,
					pub total_bytes: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct SegmentTracker < _0 > { pub used_bandwidth : runtime_types :: cumulus_pallet_parachain_system :: unincluded_segment :: UsedBandwidth , pub hrmp_watermark : :: core :: option :: Option < :: core :: primitive :: u32 > , pub consumed_go_ahead_signal : :: core :: option :: Option < runtime_types :: polkadot_primitives :: v6 :: UpgradeGoAhead > , # [codec (skip)] pub __subxt_unused_type_params : :: core :: marker :: PhantomData < _0 > }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UsedBandwidth { pub ump_msg_count : :: core :: primitive :: u32 , pub ump_total_bytes : :: core :: primitive :: u32 , pub hrmp_outgoing : :: subxt :: utils :: KeyedVec < runtime_types :: polkadot_parachain_primitives :: primitives :: Id , runtime_types :: cumulus_pallet_parachain_system :: unincluded_segment :: HrmpChannelUpdate > , }
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RelayChainState {
				pub number: ::core::primitive::u32,
				pub state_root: ::subxt::utils::H256,
			}
		}
		pub mod cumulus_pallet_xcm {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Downward message is invalid XCM."]
					#[doc = "\\[ id \\]"]
					InvalidFormat([::core::primitive::u8; 32usize]),
					#[codec(index = 1)]
					#[doc = "Downward message is unsupported version of XCM."]
					#[doc = "\\[ id \\]"]
					UnsupportedVersion([::core::primitive::u8; 32usize]),
					#[codec(index = 2)]
					#[doc = "Downward message executed with the given outcome."]
					#[doc = "\\[ id, outcome \\]"]
					ExecutedDownward(
						[::core::primitive::u8; 32usize],
						runtime_types::xcm::v3::traits::Outcome,
					),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Origin {
					#[codec(index = 0)]
					Relay,
					#[codec(index = 1)]
					SiblingParachain(runtime_types::polkadot_parachain_primitives::primitives::Id),
				}
			}
		}
		pub mod cumulus_pallet_xcmp_queue {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 1)]
					#[doc = "See [`Pallet::suspend_xcm_execution`]."]
					suspend_xcm_execution,
					#[codec(index = 2)]
					#[doc = "See [`Pallet::resume_xcm_execution`]."]
					resume_xcm_execution,
					#[codec(index = 3)]
					#[doc = "See [`Pallet::update_suspend_threshold`]."]
					update_suspend_threshold { new: ::core::primitive::u32 },
					#[codec(index = 4)]
					#[doc = "See [`Pallet::update_drop_threshold`]."]
					update_drop_threshold { new: ::core::primitive::u32 },
					#[codec(index = 5)]
					#[doc = "See [`Pallet::update_resume_threshold`]."]
					update_resume_threshold { new: ::core::primitive::u32 },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Setting the queue config failed since one of its values was invalid."]
					BadQueueConfig,
					#[codec(index = 1)]
					#[doc = "The execution is already suspended."]
					AlreadySuspended,
					#[codec(index = 2)]
					#[doc = "The execution is already resumed."]
					AlreadyResumed,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An HRMP message was sent to a sibling parachain."]
					XcmpMessageSent { message_hash: [::core::primitive::u8; 32usize] },
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OutboundChannelDetails {
				pub recipient: runtime_types::polkadot_parachain_primitives::primitives::Id,
				pub state: runtime_types::cumulus_pallet_xcmp_queue::OutboundState,
				pub signals_exist: ::core::primitive::bool,
				pub first_index: ::core::primitive::u16,
				pub last_index: ::core::primitive::u16,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum OutboundState {
				#[codec(index = 0)]
				Ok,
				#[codec(index = 1)]
				Suspended,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct QueueConfigData {
				pub suspend_threshold: ::core::primitive::u32,
				pub drop_threshold: ::core::primitive::u32,
				pub resume_threshold: ::core::primitive::u32,
			}
		}
		pub mod cumulus_primitives_core {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum AggregateMessageOrigin {
				#[codec(index = 0)]
				Here,
				#[codec(index = 1)]
				Parent,
				#[codec(index = 2)]
				Sibling(runtime_types::polkadot_parachain_primitives::primitives::Id),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct CollationInfo {
				pub upward_messages: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				pub horizontal_messages: ::std::vec::Vec<
					runtime_types::polkadot_core_primitives::OutboundHrmpMessage<
						runtime_types::polkadot_parachain_primitives::primitives::Id,
					>,
				>,
				pub new_validation_code: ::core::option::Option<
					runtime_types::polkadot_parachain_primitives::primitives::ValidationCode,
				>,
				pub processed_downward_messages: ::core::primitive::u32,
				pub hrmp_watermark: ::core::primitive::u32,
				pub head_data: runtime_types::polkadot_parachain_primitives::primitives::HeadData,
			}
		}
		pub mod cumulus_primitives_parachain_inherent {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct MessageQueueChain(pub ::subxt::utils::H256);
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ParachainInherentData {
				pub validation_data:
					runtime_types::polkadot_primitives::v6::PersistedValidationData<
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
					runtime_types::polkadot_parachain_primitives::primitives::Id,
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DispatchInfo {
					pub weight: runtime_types::sp_weights::weight_v2::Weight,
					pub class: runtime_types::frame_support::dispatch::DispatchClass,
					pub pays_fee: runtime_types::frame_support::dispatch::Pays,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Pays {
					#[codec(index = 0)]
					Yes,
					#[codec(index = 1)]
					No,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PerDispatchClass<_0> {
					pub normal: _0,
					pub operational: _0,
					pub mandatory: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum RawOrigin<_0> {
					#[codec(index = 0)]
					Root,
					#[codec(index = 1)]
					Signed(_0),
					#[codec(index = 2)]
					None,
				}
			}
			pub mod traits {
				use super::runtime_types;
				pub mod messages {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum ProcessMessageError {
						#[codec(index = 0)]
						BadFormat,
						#[codec(index = 1)]
						Corrupt,
						#[codec(index = 2)]
						Unsupported,
						#[codec(index = 3)]
						Overweight(runtime_types::sp_weights::weight_v2::Weight),
						#[codec(index = 4)]
						Yield,
					}
				}
				pub mod tokens {
					use super::runtime_types;
					pub mod misc {
						use super::runtime_types;
						#[derive(
							:: subxt :: ext :: codec :: Decode,
							:: subxt :: ext :: codec :: Encode,
							:: subxt :: ext :: scale_decode :: DecodeAsType,
							:: subxt :: ext :: scale_encode :: EncodeAsType,
							Clone,
							Debug,
							Eq,
							PartialEq,
						)]
						# [codec (crate = :: subxt :: ext :: codec)]
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
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct PalletId(pub [::core::primitive::u8; 8usize]);
		}
		pub mod frame_system {
			use super::runtime_types;
			pub mod extensions {
				use super::runtime_types;
				pub mod check_genesis {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckGenesis;
				}
				pub mod check_mortality {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckMortality(pub runtime_types::sp_runtime::generic::era::Era);
				}
				pub mod check_non_zero_sender {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckNonZeroSender;
				}
				pub mod check_nonce {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckNonce(#[codec(compact)] pub ::core::primitive::u32);
				}
				pub mod check_spec_version {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckSpecVersion;
				}
				pub mod check_tx_version {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckTxVersion;
				}
				pub mod check_weight {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct CheckWeight;
				}
			}
			pub mod limits {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BlockLength {
					pub max: runtime_types::frame_support::dispatch::PerDispatchClass<
						::core::primitive::u32,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::remark`]."]
					remark { remark: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::set_heap_pages`]."]
					set_heap_pages { pages: ::core::primitive::u64 },
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_code`]."]
					set_code { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 3)]
					#[doc = "See [`Pallet::set_code_without_checks`]."]
					set_code_without_checks { code: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 4)]
					#[doc = "See [`Pallet::set_storage`]."]
					set_storage {
						items: ::std::vec::Vec<(
							::std::vec::Vec<::core::primitive::u8>,
							::std::vec::Vec<::core::primitive::u8>,
						)>,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::kill_storage`]."]
					kill_storage { keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>> },
					#[codec(index = 6)]
					#[doc = "See [`Pallet::kill_prefix`]."]
					kill_prefix {
						prefix: ::std::vec::Vec<::core::primitive::u8>,
						subkeys: ::core::primitive::u32,
					},
					#[codec(index = 7)]
					#[doc = "See [`Pallet::remark_with_event`]."]
					remark_with_event { remark: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 9)]
					#[doc = "See [`Pallet::authorize_upgrade`]."]
					authorize_upgrade { code_hash: ::subxt::utils::H256 },
					#[codec(index = 10)]
					#[doc = "See [`Pallet::authorize_upgrade_without_checks`]."]
					authorize_upgrade_without_checks { code_hash: ::subxt::utils::H256 },
					#[codec(index = 11)]
					#[doc = "See [`Pallet::apply_authorized_upgrade`]."]
					apply_authorized_upgrade { code: ::std::vec::Vec<::core::primitive::u8> },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the System pallet"]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The name of specification does not match between the current runtime"]
					#[doc = "and the new runtime."]
					InvalidSpecName,
					#[codec(index = 1)]
					#[doc = "The specification version is not allowed to decrease between the current runtime"]
					#[doc = "and the new runtime."]
					SpecVersionNeedsToIncrease,
					#[codec(index = 2)]
					#[doc = "Failed to extract the runtime version from the new runtime."]
					#[doc = ""]
					#[doc = "Either calling `Core_version` or decoding `RuntimeVersion` failed."]
					FailedToExtractRuntimeVersion,
					#[codec(index = 3)]
					#[doc = "Suicide called when the account has non-default composite data."]
					NonDefaultComposite,
					#[codec(index = 4)]
					#[doc = "There is a non-zero reference count preventing the account from being purged."]
					NonZeroRefCount,
					#[codec(index = 5)]
					#[doc = "The origin filter prevent the call to be dispatched."]
					CallFiltered,
					#[codec(index = 6)]
					#[doc = "No upgrade authorized."]
					NothingAuthorized,
					#[codec(index = 7)]
					#[doc = "The submitted code is not authorized."]
					Unauthorized,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Event for the System pallet."]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An extrinsic completed successfully."]
					ExtrinsicSuccess {
						dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
					},
					#[codec(index = 1)]
					#[doc = "An extrinsic failed."]
					ExtrinsicFailed {
						dispatch_error: runtime_types::sp_runtime::DispatchError,
						dispatch_info: runtime_types::frame_support::dispatch::DispatchInfo,
					},
					#[codec(index = 2)]
					#[doc = "`:code` was updated."]
					CodeUpdated,
					#[codec(index = 3)]
					#[doc = "A new account was created."]
					NewAccount { account: ::subxt::utils::AccountId32 },
					#[codec(index = 4)]
					#[doc = "An account was reaped."]
					KilledAccount { account: ::subxt::utils::AccountId32 },
					#[codec(index = 5)]
					#[doc = "On on-chain remark happened."]
					Remarked { sender: ::subxt::utils::AccountId32, hash: ::subxt::utils::H256 },
					#[codec(index = 6)]
					#[doc = "An upgrade was authorized."]
					UpgradeAuthorized {
						code_hash: ::subxt::utils::H256,
						check_version: ::core::primitive::bool,
					},
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct AccountInfo<_0, _1> {
				pub nonce: _0,
				pub consumers: ::core::primitive::u32,
				pub providers: ::core::primitive::u32,
				pub sufficients: ::core::primitive::u32,
				pub data: _1,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct CodeUpgradeAuthorization {
				pub code_hash: ::subxt::utils::H256,
				pub check_version: ::core::primitive::bool,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct EventRecord<_0, _1> {
				pub phase: runtime_types::frame_system::Phase,
				pub event: _0,
				pub topics: ::std::vec::Vec<_1>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct LastRuntimeUpgradeInfo {
				#[codec(compact)]
				pub spec_version: ::core::primitive::u32,
				pub spec_name: ::std::string::String,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
		pub mod gargantua_runtime {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum OriginCaller {
				#[codec(index = 0)]
				system(
					runtime_types::frame_support::dispatch::RawOrigin<::subxt::utils::AccountId32>,
				),
				#[codec(index = 31)]
				PolkadotXcm(runtime_types::pallet_xcm::pallet::Origin),
				#[codec(index = 32)]
				CumulusXcm(runtime_types::cumulus_pallet_xcm::pallet::Origin),
				#[codec(index = 3)]
				Void(runtime_types::sp_core::Void),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Runtime;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeCall {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Call),
				#[codec(index = 1)]
				Timestamp(runtime_types::pallet_timestamp::pallet::Call),
				#[codec(index = 2)]
				ParachainSystem(runtime_types::cumulus_pallet_parachain_system::pallet::Call),
				#[codec(index = 3)]
				ParachainInfo(runtime_types::staging_parachain_info::pallet::Call),
				#[codec(index = 4)]
				Utility(runtime_types::pallet_utility::pallet::Call),
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
				#[codec(index = 41)]
				Ismp(runtime_types::pallet_ismp::pallet::Call),
				#[codec(index = 42)]
				MessageQueue(runtime_types::pallet_message_queue::pallet::Call),
				#[codec(index = 50)]
				IsmpParachain(runtime_types::ismp_parachain::pallet::Call),
				#[codec(index = 51)]
				IsmpSyncCommittee(runtime_types::ismp_sync_committee::pallet::pallet::Call),
				#[codec(index = 52)]
				IsmpDemo(runtime_types::pallet_ismp_demo::pallet::Call),
				#[codec(index = 53)]
				Relayer(runtime_types::pallet_ismp_relayer::pallet::Call),
				#[codec(index = 55)]
				HostExecutive(runtime_types::pallet_ismp_host_executive::pallet::Call),
				#[codec(index = 56)]
				CallDecompressor(runtime_types::pallet_call_decompressor::pallet::Call),
				#[codec(index = 57)]
				Gateway(runtime_types::pallet_asset_gateway::pallet::Call),
				#[codec(index = 58)]
				Assets(runtime_types::pallet_assets::pallet::Call),
				#[codec(index = 59)]
				TokenGovernor(runtime_types::pallet_token_governor::pallet::Call),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeError {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Error),
				#[codec(index = 2)]
				ParachainSystem(runtime_types::cumulus_pallet_parachain_system::pallet::Error),
				#[codec(index = 4)]
				Utility(runtime_types::pallet_utility::pallet::Error),
				#[codec(index = 10)]
				Balances(runtime_types::pallet_balances::pallet::Error),
				#[codec(index = 21)]
				CollatorSelection(runtime_types::pallet_collator_selection::pallet::Error),
				#[codec(index = 22)]
				Session(runtime_types::pallet_session::pallet::Error),
				#[codec(index = 25)]
				Sudo(runtime_types::pallet_sudo::pallet::Error),
				#[codec(index = 30)]
				XcmpQueue(runtime_types::cumulus_pallet_xcmp_queue::pallet::Error),
				#[codec(index = 31)]
				PolkadotXcm(runtime_types::pallet_xcm::pallet::Error),
				#[codec(index = 41)]
				Ismp(runtime_types::pallet_ismp::pallet::Error),
				#[codec(index = 42)]
				MessageQueue(runtime_types::pallet_message_queue::pallet::Error),
				#[codec(index = 51)]
				IsmpSyncCommittee(runtime_types::ismp_sync_committee::pallet::pallet::Error),
				#[codec(index = 52)]
				IsmpDemo(runtime_types::pallet_ismp_demo::pallet::Error),
				#[codec(index = 53)]
				Relayer(runtime_types::pallet_ismp_relayer::pallet::Error),
				#[codec(index = 55)]
				HostExecutive(runtime_types::pallet_ismp_host_executive::pallet::Error),
				#[codec(index = 56)]
				CallDecompressor(runtime_types::pallet_call_decompressor::pallet::Error),
				#[codec(index = 57)]
				Gateway(runtime_types::pallet_asset_gateway::pallet::Error),
				#[codec(index = 58)]
				Assets(runtime_types::pallet_assets::pallet::Error),
				#[codec(index = 59)]
				TokenGovernor(runtime_types::pallet_token_governor::pallet::Error),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeEvent {
				#[codec(index = 0)]
				System(runtime_types::frame_system::pallet::Event),
				#[codec(index = 2)]
				ParachainSystem(runtime_types::cumulus_pallet_parachain_system::pallet::Event),
				#[codec(index = 4)]
				Utility(runtime_types::pallet_utility::pallet::Event),
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
				#[codec(index = 41)]
				Ismp(runtime_types::pallet_ismp::pallet::Event),
				#[codec(index = 42)]
				MessageQueue(runtime_types::pallet_message_queue::pallet::Event),
				#[codec(index = 50)]
				IsmpParachain(runtime_types::ismp_parachain::pallet::Event),
				#[codec(index = 52)]
				IsmpDemo(runtime_types::pallet_ismp_demo::pallet::Event),
				#[codec(index = 53)]
				Relayer(runtime_types::pallet_ismp_relayer::pallet::Event),
				#[codec(index = 55)]
				HostExecutive(runtime_types::pallet_ismp_host_executive::pallet::Event),
				#[codec(index = 57)]
				Gateway(runtime_types::pallet_asset_gateway::pallet::Event),
				#[codec(index = 58)]
				Assets(runtime_types::pallet_assets::pallet::Event),
				#[codec(index = 59)]
				TokenGovernor(runtime_types::pallet_token_governor::pallet::Event),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum RuntimeHoldReason {}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateCommitment {
					pub timestamp: ::core::primitive::u64,
					pub overlay_root: ::core::option::Option<::subxt::utils::H256>,
					pub state_root: ::subxt::utils::H256,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateMachineHeight {
					pub id: runtime_types::ismp::consensus::StateMachineId,
					pub height: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateMachineId {
					pub state_id: runtime_types::ismp::host::StateMachine,
					pub consensus_state_id: [::core::primitive::u8; 4usize],
				}
			}
			pub mod events {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Event {
					#[codec(index = 0)]
					StateMachineUpdated(runtime_types::ismp::events::StateMachineUpdated),
					#[codec(index = 1)]
					StateCommitmentVetoed(runtime_types::ismp::events::StateCommitmentVetoed),
					#[codec(index = 2)]
					PostRequest(runtime_types::ismp::router::PostRequest),
					#[codec(index = 3)]
					PostResponse(runtime_types::ismp::router::PostResponse),
					#[codec(index = 4)]
					GetRequest(runtime_types::ismp::router::GetRequest),
					#[codec(index = 5)]
					PostRequestHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 6)]
					PostResponseHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 7)]
					PostRequestTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
					#[codec(index = 8)]
					PostResponseTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
					#[codec(index = 9)]
					GetRequestHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 10)]
					GetRequestTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Meta {
					pub source: runtime_types::ismp::host::StateMachine,
					pub dest: runtime_types::ismp::host::StateMachine,
					pub nonce: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RequestResponseHandled {
					pub commitment: ::subxt::utils::H256,
					pub relayer: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateCommitmentVetoed {
					pub height: runtime_types::ismp::consensus::StateMachineHeight,
					pub fisherman: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateMachineUpdated {
					pub state_machine_id: runtime_types::ismp::consensus::StateMachineId,
					pub latest_height: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TimeoutHandled {
					pub commitment: ::subxt::utils::H256,
					pub source: runtime_types::ismp::host::StateMachine,
					pub dest: runtime_types::ismp::host::StateMachine,
				}
			}
			pub mod host {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Ethereum {
					#[codec(index = 0)]
					ExecutionLayer,
					#[codec(index = 1)]
					Optimism,
					#[codec(index = 2)]
					Arbitrum,
					#[codec(index = 3)]
					Base,
					#[codec(index = 4)]
					Blast,
					#[codec(index = 5)]
					Mantle,
					#[codec(index = 6)]
					Manta,
					#[codec(index = 7)]
					Bob,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum StateMachine {
					#[codec(index = 0)]
					Ethereum(runtime_types::ismp::host::Ethereum),
					#[codec(index = 1)]
					Polkadot(::core::primitive::u32),
					#[codec(index = 2)]
					Kusama(::core::primitive::u32),
					#[codec(index = 3)]
					Grandpa([::core::primitive::u8; 4usize]),
					#[codec(index = 4)]
					Beefy([::core::primitive::u8; 4usize]),
					#[codec(index = 5)]
					Polygon,
					#[codec(index = 6)]
					Bsc,
				}
			}
			pub mod messaging {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ConsensusMessage {
					pub consensus_proof: ::std::vec::Vec<::core::primitive::u8>,
					pub consensus_state_id: [::core::primitive::u8; 4usize],
					pub signer: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CreateConsensusState {
					pub consensus_state: ::std::vec::Vec<::core::primitive::u8>,
					pub consensus_client_id: [::core::primitive::u8; 4usize],
					pub consensus_state_id: [::core::primitive::u8; 4usize],
					pub unbonding_period: ::core::primitive::u64,
					pub challenge_period: ::core::primitive::u64,
					pub state_machine_commitments: ::std::vec::Vec<(
						runtime_types::ismp::consensus::StateMachineId,
						runtime_types::ismp::messaging::StateCommitmentHeight,
					)>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FraudProofMessage {
					pub proof_1: ::std::vec::Vec<::core::primitive::u8>,
					pub proof_2: ::std::vec::Vec<::core::primitive::u8>,
					pub consensus_state_id: [::core::primitive::u8; 4usize],
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Message {
					#[codec(index = 0)]
					Consensus(runtime_types::ismp::messaging::ConsensusMessage),
					#[codec(index = 1)]
					FraudProof(runtime_types::ismp::messaging::FraudProofMessage),
					#[codec(index = 2)]
					Request(runtime_types::ismp::messaging::RequestMessage),
					#[codec(index = 3)]
					Response(runtime_types::ismp::messaging::ResponseMessage),
					#[codec(index = 4)]
					Timeout(runtime_types::ismp::messaging::TimeoutMessage),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Proof {
					pub height: runtime_types::ismp::consensus::StateMachineHeight,
					pub proof: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RequestMessage {
					pub requests: ::std::vec::Vec<runtime_types::ismp::router::PostRequest>,
					pub proof: runtime_types::ismp::messaging::Proof,
					pub signer: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ResponseMessage {
					pub datagram: runtime_types::ismp::router::RequestResponse,
					pub proof: runtime_types::ismp::messaging::Proof,
					pub signer: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct StateCommitmentHeight {
					pub commitment: runtime_types::ismp::consensus::StateCommitment,
					pub height: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum TimeoutMessage {
					#[codec(index = 0)]
					Post {
						requests: ::std::vec::Vec<runtime_types::ismp::router::Request>,
						timeout_proof: runtime_types::ismp::messaging::Proof,
					},
					#[codec(index = 1)]
					PostResponse {
						responses: ::std::vec::Vec<runtime_types::ismp::router::PostResponse>,
						timeout_proof: runtime_types::ismp::messaging::Proof,
					},
					#[codec(index = 2)]
					Get { requests: ::std::vec::Vec<runtime_types::ismp::router::Request> },
				}
			}
			pub mod router {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GetRequest {
					pub source: runtime_types::ismp::host::StateMachine,
					pub dest: runtime_types::ismp::host::StateMachine,
					pub nonce: ::core::primitive::u64,
					pub from: ::std::vec::Vec<::core::primitive::u8>,
					pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
					pub height: ::core::primitive::u64,
					pub timeout_timestamp: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GetResponse {
					pub get: runtime_types::ismp::router::GetRequest,
					pub values: ::subxt::utils::KeyedVec<
						::std::vec::Vec<::core::primitive::u8>,
						::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PostRequest {
					pub source: runtime_types::ismp::host::StateMachine,
					pub dest: runtime_types::ismp::host::StateMachine,
					pub nonce: ::core::primitive::u64,
					pub from: ::std::vec::Vec<::core::primitive::u8>,
					pub to: ::std::vec::Vec<::core::primitive::u8>,
					pub timeout_timestamp: ::core::primitive::u64,
					pub body: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PostResponse {
					pub post: runtime_types::ismp::router::PostRequest,
					pub response: ::std::vec::Vec<::core::primitive::u8>,
					pub timeout_timestamp: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Request {
					#[codec(index = 0)]
					Post(runtime_types::ismp::router::PostRequest),
					#[codec(index = 1)]
					Get(runtime_types::ismp::router::GetRequest),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum RequestResponse {
					#[codec(index = 0)]
					Request(::std::vec::Vec<runtime_types::ismp::router::Request>),
					#[codec(index = 1)]
					Response(::std::vec::Vec<runtime_types::ismp::router::Response>),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Response {
					#[codec(index = 0)]
					Post(runtime_types::ismp::router::PostResponse),
					#[codec(index = 1)]
					Get(runtime_types::ismp::router::GetResponse),
				}
			}
		}
		pub mod ismp_parachain {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::update_parachain_consensus`]."]
					update_parachain_consensus {
						data: runtime_types::ismp::messaging::ConsensusMessage,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::add_parachain`]."]
					add_parachain {
						para_ids: ::std::vec::Vec<runtime_types::ismp_parachain::ParachainData>,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::remove_parachain`]."]
					remove_parachain { para_ids: ::std::vec::Vec<::core::primitive::u32> },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Events emitted by this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Parachains with the `para_ids` have been added to the whitelist"]
					ParachainsAdded {
						para_ids: ::std::vec::Vec<runtime_types::ismp_parachain::ParachainData>,
					},
					#[codec(index = 1)]
					#[doc = "Parachains with the `para_ids` have been removed from the whitelist"]
					ParachainsRemoved { para_ids: ::std::vec::Vec<::core::primitive::u32> },
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ParachainData {
				pub id: ::core::primitive::u32,
				pub slot_duration: ::core::primitive::u64,
			}
		}
		pub mod ismp_sync_committee {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				pub mod pallet {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
					pub enum Call {
						#[codec(index = 0)]
						#[doc = "See [`Pallet::add_ismp_address`]."]
						add_ismp_address {
							contract_address: ::subxt::utils::H160,
							state_machine_id: runtime_types::ismp::consensus::StateMachineId,
						},
						#[codec(index = 1)]
						#[doc = "See [`Pallet::add_l2_consensus`]."]
						add_l2_consensus {
							state_machine_id: runtime_types::ismp::consensus::StateMachineId,
							l2_consensus: runtime_types::ismp_sync_committee::types::L2Consensus,
						},
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					#[doc = "The `Error` enum of this pallet."]
					pub enum Error {
						#[codec(index = 0)]
						#[doc = "Contract Address Already Exists"]
						ContractAddressAlreadyExists,
						#[codec(index = 1)]
						#[doc = "Error fetching consensus state"]
						ErrorFetchingConsensusState,
						#[codec(index = 2)]
						#[doc = "Error decoding consensus state"]
						ErrorDecodingConsensusState,
						#[codec(index = 3)]
						#[doc = "Error storing consensus state"]
						ErrorStoringConsensusState,
					}
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum L2Consensus {
					#[codec(index = 0)]
					ArbitrumOrbit(::subxt::utils::H160),
					#[codec(index = 1)]
					OpL2Oracle(::subxt::utils::H160),
					#[codec(index = 2)]
					OpFaultProofs(::subxt::utils::H160),
				}
			}
		}
		pub mod pallet_asset_gateway {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set_params`]."]
					set_params {
						update:
							runtime_types::pallet_asset_gateway::pallet::TokenGatewayParamsUpdate,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Error encountered while dispatching post request"]
					DispatchPostError,
					#[codec(index = 1)]
					#[doc = "Pallet has not been initialized"]
					NotInitialized,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Events emiited by the relayer pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An XCM transfer from the relay chain has been transformed into a crosschain message"]
					AssetTeleported {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::H160,
						amount: ::core::primitive::u128,
						dest: runtime_types::ismp::host::StateMachine,
						commitment: ::subxt::utils::H256,
					},
					#[codec(index = 1)]
					#[doc = "An asset has been received and transferred to the beneficiary's account on the"]
					#[doc = "relaychain"]
					AssetReceived {
						beneficiary: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						source: runtime_types::ismp::host::StateMachine,
					},
					#[codec(index = 2)]
					#[doc = "An asset has been refunded and transferred to the beneficiary's account on the"]
					#[doc = "relaychain"]
					AssetRefunded {
						beneficiary: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						source: runtime_types::ismp::host::StateMachine,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TokenGatewayParams {
					pub protocol_fee_percentage: runtime_types::sp_arithmetic::per_things::Permill,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TokenGatewayParamsUpdate {
					pub protocol_fee_percentage:
						::core::option::Option<runtime_types::sp_arithmetic::per_things::Permill>,
				}
			}
		}
		pub mod pallet_assets {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::create`]."]
					create {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						min_balance: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::force_create`]."]
					force_create {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						is_sufficient: ::core::primitive::bool,
						#[codec(compact)]
						min_balance: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::start_destroy`]."]
					start_destroy {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::destroy_accounts`]."]
					destroy_accounts {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::destroy_approvals`]."]
					destroy_approvals {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::finish_destroy`]."]
					finish_destroy {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 6)]
					#[doc = "See [`Pallet::mint`]."]
					mint {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						beneficiary: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 7)]
					#[doc = "See [`Pallet::burn`]."]
					burn {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 8)]
					#[doc = "See [`Pallet::transfer`]."]
					transfer {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 9)]
					#[doc = "See [`Pallet::transfer_keep_alive`]."]
					transfer_keep_alive {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						target: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 10)]
					#[doc = "See [`Pallet::force_transfer`]."]
					force_transfer {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 11)]
					#[doc = "See [`Pallet::freeze`]."]
					freeze {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 12)]
					#[doc = "See [`Pallet::thaw`]."]
					thaw {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 13)]
					#[doc = "See [`Pallet::freeze_asset`]."]
					freeze_asset {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 14)]
					#[doc = "See [`Pallet::thaw_asset`]."]
					thaw_asset { id: runtime_types::staging_xcm::v3::multilocation::MultiLocation },
					#[codec(index = 15)]
					#[doc = "See [`Pallet::transfer_ownership`]."]
					transfer_ownership {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 16)]
					#[doc = "See [`Pallet::set_team`]."]
					set_team {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 17)]
					#[doc = "See [`Pallet::set_metadata`]."]
					set_metadata {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						name: ::std::vec::Vec<::core::primitive::u8>,
						symbol: ::std::vec::Vec<::core::primitive::u8>,
						decimals: ::core::primitive::u8,
					},
					#[codec(index = 18)]
					#[doc = "See [`Pallet::clear_metadata`]."]
					clear_metadata {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 19)]
					#[doc = "See [`Pallet::force_set_metadata`]."]
					force_set_metadata {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						name: ::std::vec::Vec<::core::primitive::u8>,
						symbol: ::std::vec::Vec<::core::primitive::u8>,
						decimals: ::core::primitive::u8,
						is_frozen: ::core::primitive::bool,
					},
					#[codec(index = 20)]
					#[doc = "See [`Pallet::force_clear_metadata`]."]
					force_clear_metadata {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 21)]
					#[doc = "See [`Pallet::force_asset_status`]."]
					force_asset_status {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						issuer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						admin: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						freezer: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						min_balance: ::core::primitive::u128,
						is_sufficient: ::core::primitive::bool,
						is_frozen: ::core::primitive::bool,
					},
					#[codec(index = 22)]
					#[doc = "See [`Pallet::approve_transfer`]."]
					approve_transfer {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 23)]
					#[doc = "See [`Pallet::cancel_approval`]."]
					cancel_approval {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 24)]
					#[doc = "See [`Pallet::force_cancel_approval`]."]
					force_cancel_approval {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						delegate: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 25)]
					#[doc = "See [`Pallet::transfer_approved`]."]
					transfer_approved {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						destination: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						amount: ::core::primitive::u128,
					},
					#[codec(index = 26)]
					#[doc = "See [`Pallet::touch`]."]
					touch { id: runtime_types::staging_xcm::v3::multilocation::MultiLocation },
					#[codec(index = 27)]
					#[doc = "See [`Pallet::refund`]."]
					refund {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						allow_burn: ::core::primitive::bool,
					},
					#[codec(index = 28)]
					#[doc = "See [`Pallet::set_min_balance`]."]
					set_min_balance {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						min_balance: ::core::primitive::u128,
					},
					#[codec(index = 29)]
					#[doc = "See [`Pallet::touch_other`]."]
					touch_other {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 30)]
					#[doc = "See [`Pallet::refund_other`]."]
					refund_other {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
					#[codec(index = 31)]
					#[doc = "See [`Pallet::block`]."]
					block {
						id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Account balance must be greater than or equal to the transfer amount."]
					BalanceLow,
					#[codec(index = 1)]
					#[doc = "The account to alter does not exist."]
					NoAccount,
					#[codec(index = 2)]
					#[doc = "The signing account has no permission to do the operation."]
					NoPermission,
					#[codec(index = 3)]
					#[doc = "The given asset ID is unknown."]
					Unknown,
					#[codec(index = 4)]
					#[doc = "The origin account is frozen."]
					Frozen,
					#[codec(index = 5)]
					#[doc = "The asset ID is already taken."]
					InUse,
					#[codec(index = 6)]
					#[doc = "Invalid witness data given."]
					BadWitness,
					#[codec(index = 7)]
					#[doc = "Minimum balance should be non-zero."]
					MinBalanceZero,
					#[codec(index = 8)]
					#[doc = "Unable to increment the consumer reference counters on the account. Either no provider"]
					#[doc = "reference exists to allow a non-zero balance of a non-self-sufficient asset, or one"]
					#[doc = "fewer then the maximum number of consumers has been reached."]
					UnavailableConsumer,
					#[codec(index = 9)]
					#[doc = "Invalid metadata given."]
					BadMetadata,
					#[codec(index = 10)]
					#[doc = "No approval exists that would allow the transfer."]
					Unapproved,
					#[codec(index = 11)]
					#[doc = "The source account would not survive the transfer and it needs to stay alive."]
					WouldDie,
					#[codec(index = 12)]
					#[doc = "The asset-account already exists."]
					AlreadyExists,
					#[codec(index = 13)]
					#[doc = "The asset-account doesn't have an associated deposit."]
					NoDeposit,
					#[codec(index = 14)]
					#[doc = "The operation would result in funds being burned."]
					WouldBurn,
					#[codec(index = 15)]
					#[doc = "The asset is a live asset and is actively being used. Usually emit for operations such"]
					#[doc = "as `start_destroy` which require the asset to be in a destroying state."]
					LiveAsset,
					#[codec(index = 16)]
					#[doc = "The asset is not live, and likely being destroyed."]
					AssetNotLive,
					#[codec(index = 17)]
					#[doc = "The asset status is not the expected status."]
					IncorrectStatus,
					#[codec(index = 18)]
					#[doc = "The asset should be frozen before the given operation."]
					NotFrozen,
					#[codec(index = 19)]
					#[doc = "Callback action resulted in error"]
					CallbackFailed,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Some asset class was created."]
					Created {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						creator: ::subxt::utils::AccountId32,
						owner: ::subxt::utils::AccountId32,
					},
					#[codec(index = 1)]
					#[doc = "Some assets were issued."]
					Issued {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Some assets were transferred."]
					Transferred {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "Some assets were destroyed."]
					Burned {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
						balance: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "The management team changed."]
					TeamChanged {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						issuer: ::subxt::utils::AccountId32,
						admin: ::subxt::utils::AccountId32,
						freezer: ::subxt::utils::AccountId32,
					},
					#[codec(index = 5)]
					#[doc = "The owner changed."]
					OwnerChanged {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
					},
					#[codec(index = 6)]
					#[doc = "Some account `who` was frozen."]
					Frozen {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::AccountId32,
					},
					#[codec(index = 7)]
					#[doc = "Some account `who` was thawed."]
					Thawed {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::AccountId32,
					},
					#[codec(index = 8)]
					#[doc = "Some asset `asset_id` was frozen."]
					AssetFrozen {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 9)]
					#[doc = "Some asset `asset_id` was thawed."]
					AssetThawed {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 10)]
					#[doc = "Accounts were destroyed for given asset."]
					AccountsDestroyed {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						accounts_destroyed: ::core::primitive::u32,
						accounts_remaining: ::core::primitive::u32,
					},
					#[codec(index = 11)]
					#[doc = "Approvals were destroyed for given asset."]
					ApprovalsDestroyed {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						approvals_destroyed: ::core::primitive::u32,
						approvals_remaining: ::core::primitive::u32,
					},
					#[codec(index = 12)]
					#[doc = "An asset class is in the process of being destroyed."]
					DestructionStarted {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 13)]
					#[doc = "An asset class was destroyed."]
					Destroyed {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 14)]
					#[doc = "Some asset class was force-created."]
					ForceCreated {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
					},
					#[codec(index = 15)]
					#[doc = "New metadata has been set for an asset."]
					MetadataSet {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						name: ::std::vec::Vec<::core::primitive::u8>,
						symbol: ::std::vec::Vec<::core::primitive::u8>,
						decimals: ::core::primitive::u8,
						is_frozen: ::core::primitive::bool,
					},
					#[codec(index = 16)]
					#[doc = "Metadata has been cleared for an asset."]
					MetadataCleared {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 17)]
					#[doc = "(Additional) funds have been approved for transfer to a destination account."]
					ApprovedTransfer {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						source: ::subxt::utils::AccountId32,
						delegate: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 18)]
					#[doc = "An approval for account `delegate` was cancelled by `owner`."]
					ApprovalCancelled {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
						delegate: ::subxt::utils::AccountId32,
					},
					#[codec(index = 19)]
					#[doc = "An `amount` was transferred in its entirety from `owner` to `destination` by"]
					#[doc = "the approved `delegate`."]
					TransferredApproved {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						owner: ::subxt::utils::AccountId32,
						delegate: ::subxt::utils::AccountId32,
						destination: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 20)]
					#[doc = "An asset has had its attributes changed by the `Force` origin."]
					AssetStatusChanged {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 21)]
					#[doc = "The min_balance of an asset has been updated by the asset owner."]
					AssetMinBalanceChanged {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						new_min_balance: ::core::primitive::u128,
					},
					#[codec(index = 22)]
					#[doc = "Some account `who` was created with a deposit from `depositor`."]
					Touched {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::AccountId32,
						depositor: ::subxt::utils::AccountId32,
					},
					#[codec(index = 23)]
					#[doc = "Some account `who` was blocked."]
					Blocked {
						asset_id: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						who: ::subxt::utils::AccountId32,
					},
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum AccountStatus {
					#[codec(index = 0)]
					Liquid,
					#[codec(index = 1)]
					Frozen,
					#[codec(index = 2)]
					Blocked,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Approval<_0, _1> {
					pub amount: _0,
					pub deposit: _1,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AssetAccount<_0, _1, _2, _3> {
					pub balance: _0,
					pub status: runtime_types::pallet_assets::types::AccountStatus,
					pub reason: runtime_types::pallet_assets::types::ExistenceReason<_0, _3>,
					pub extra: _2,
					#[codec(skip)]
					pub __subxt_unused_type_params: ::core::marker::PhantomData<_1>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AssetDetails<_0, _1, _2> {
					pub owner: _1,
					pub issuer: _1,
					pub admin: _1,
					pub freezer: _1,
					pub supply: _0,
					pub deposit: _2,
					pub min_balance: _0,
					pub is_sufficient: ::core::primitive::bool,
					pub accounts: ::core::primitive::u32,
					pub sufficients: ::core::primitive::u32,
					pub approvals: ::core::primitive::u32,
					pub status: runtime_types::pallet_assets::types::AssetStatus,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AssetMetadata<_0, _1> {
					pub deposit: _0,
					pub name: _1,
					pub symbol: _1,
					pub decimals: ::core::primitive::u8,
					pub is_frozen: ::core::primitive::bool,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum AssetStatus {
					#[codec(index = 0)]
					Live,
					#[codec(index = 1)]
					Frozen,
					#[codec(index = 2)]
					Destroying,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum ExistenceReason<_0, _1> {
					#[codec(index = 0)]
					Consumer,
					#[codec(index = 1)]
					Sufficient,
					#[codec(index = 2)]
					DepositHeld(_0),
					#[codec(index = 3)]
					DepositRefunded,
					#[codec(index = 4)]
					DepositFrom(_1, _0),
				}
			}
		}
		pub mod pallet_balances {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::transfer_allow_death`]."]
					transfer_allow_death {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::force_transfer`]."]
					force_transfer {
						source: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::transfer_keep_alive`]."]
					transfer_keep_alive {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						value: ::core::primitive::u128,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::transfer_all`]."]
					transfer_all {
						dest: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						keep_alive: ::core::primitive::bool,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::force_unreserve`]."]
					force_unreserve {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "See [`Pallet::upgrade_accounts`]."]
					upgrade_accounts { who: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 8)]
					#[doc = "See [`Pallet::force_set_balance`]."]
					force_set_balance {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						#[codec(compact)]
						new_free: ::core::primitive::u128,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Vesting balance too high to send value."]
					VestingBalance,
					#[codec(index = 1)]
					#[doc = "Account liquidity restrictions prevent withdrawal."]
					LiquidityRestrictions,
					#[codec(index = 2)]
					#[doc = "Balance too low to send value."]
					InsufficientBalance,
					#[codec(index = 3)]
					#[doc = "Value too low to create account due to existential deposit."]
					ExistentialDeposit,
					#[codec(index = 4)]
					#[doc = "Transfer/payment would kill account."]
					Expendability,
					#[codec(index = 5)]
					#[doc = "A vesting schedule already exists for this account."]
					ExistingVestingSchedule,
					#[codec(index = 6)]
					#[doc = "Beneficiary account must pre-exist."]
					DeadAccount,
					#[codec(index = 7)]
					#[doc = "Number of named reserves exceed `MaxReserves`."]
					TooManyReserves,
					#[codec(index = 8)]
					#[doc = "Number of holds exceed `MaxHolds`."]
					TooManyHolds,
					#[codec(index = 9)]
					#[doc = "Number of freezes exceed `MaxFreezes`."]
					TooManyFreezes,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "An account was created with some free balance."]
					Endowed {
						account: ::subxt::utils::AccountId32,
						free_balance: ::core::primitive::u128,
					},
					#[codec(index = 1)]
					#[doc = "An account was removed whose balance was non-zero but below ExistentialDeposit,"]
					#[doc = "resulting in an outright loss."]
					DustLost {
						account: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 2)]
					#[doc = "Transfer succeeded."]
					Transfer {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
					},
					#[codec(index = 3)]
					#[doc = "A balance was set by root."]
					BalanceSet { who: ::subxt::utils::AccountId32, free: ::core::primitive::u128 },
					#[codec(index = 4)]
					#[doc = "Some balance was reserved (moved from free to reserved)."]
					Reserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 5)]
					#[doc = "Some balance was unreserved (moved from reserved to free)."]
					Unreserved { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 6)]
					#[doc = "Some balance was moved from the reserve of the first account to the second account."]
					#[doc = "Final argument indicates the destination balance type."]
					ReserveRepatriated {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						destination_status:
							runtime_types::frame_support::traits::tokens::misc::BalanceStatus,
					},
					#[codec(index = 7)]
					#[doc = "Some amount was deposited (e.g. for transaction fees)."]
					Deposit { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 8)]
					#[doc = "Some amount was withdrawn from the account (e.g. for transaction fees)."]
					Withdraw { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 9)]
					#[doc = "Some amount was removed from the account (e.g. for misbehavior)."]
					Slashed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 10)]
					#[doc = "Some amount was minted into an account."]
					Minted { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 11)]
					#[doc = "Some amount was burned from an account."]
					Burned { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 12)]
					#[doc = "Some amount was suspended from an account (it can be restored later)."]
					Suspended { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 13)]
					#[doc = "Some amount was restored into an account."]
					Restored { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 14)]
					#[doc = "An account was upgraded."]
					Upgraded { who: ::subxt::utils::AccountId32 },
					#[codec(index = 15)]
					#[doc = "Total issuance was increased by `amount`, creating a credit to be balanced."]
					Issued { amount: ::core::primitive::u128 },
					#[codec(index = 16)]
					#[doc = "Total issuance was decreased by `amount`, creating a debt to be balanced."]
					Rescinded { amount: ::core::primitive::u128 },
					#[codec(index = 17)]
					#[doc = "Some balance was locked."]
					Locked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 18)]
					#[doc = "Some balance was unlocked."]
					Unlocked { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 19)]
					#[doc = "Some balance was frozen."]
					Frozen { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
					#[codec(index = 20)]
					#[doc = "Some balance was thawed."]
					Thawed { who: ::subxt::utils::AccountId32, amount: ::core::primitive::u128 },
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AccountData<_0> {
					pub free: _0,
					pub reserved: _0,
					pub frozen: _0,
					pub flags: runtime_types::pallet_balances::types::ExtraFlags,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct BalanceLock<_0> {
					pub id: [::core::primitive::u8; 8usize],
					pub amount: _0,
					pub reasons: runtime_types::pallet_balances::types::Reasons,
				}
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ExtraFlags(pub ::core::primitive::u128);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct IdAmount<_0, _1> {
					pub id: _0,
					pub amount: _1,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ReserveData<_0, _1> {
					pub id: _0,
					pub amount: _1,
				}
			}
		}
		pub mod pallet_call_decompressor {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::decompress_call`]."]
					decompress_call {
						compressed: ::std::vec::Vec<::core::primitive::u8>,
						encoded_call_size: ::core::primitive::u32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Unsupported Call"]
					CallNotSupported,
					#[codec(index = 1)]
					#[doc = "Error executing Call"]
					ErrorExecutingCall,
					#[codec(index = 2)]
					#[doc = "Decompression Failed"]
					DecompressionFailed,
					#[codec(index = 3)]
					#[doc = "Error Decoding Call"]
					ErrorDecodingCall,
					#[codec(index = 4)]
					#[doc = "Call Size Out Of Bound"]
					CallSizeOutOfBound,
				}
			}
		}
		pub mod pallet_collator_selection {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set_invulnerables`]."]
					set_invulnerables { new: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::set_desired_candidates`]."]
					set_desired_candidates { max: ::core::primitive::u32 },
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_candidacy_bond`]."]
					set_candidacy_bond { bond: ::core::primitive::u128 },
					#[codec(index = 3)]
					#[doc = "See [`Pallet::register_as_candidate`]."]
					register_as_candidate,
					#[codec(index = 4)]
					#[doc = "See [`Pallet::leave_intent`]."]
					leave_intent,
					#[codec(index = 5)]
					#[doc = "See [`Pallet::add_invulnerable`]."]
					add_invulnerable { who: ::subxt::utils::AccountId32 },
					#[codec(index = 6)]
					#[doc = "See [`Pallet::remove_invulnerable`]."]
					remove_invulnerable { who: ::subxt::utils::AccountId32 },
					#[codec(index = 7)]
					#[doc = "See [`Pallet::update_bond`]."]
					update_bond { new_deposit: ::core::primitive::u128 },
					#[codec(index = 8)]
					#[doc = "See [`Pallet::take_candidate_slot`]."]
					take_candidate_slot {
						deposit: ::core::primitive::u128,
						target: ::subxt::utils::AccountId32,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct CandidateInfo<_0, _1> {
					pub who: _0,
					pub deposit: _1,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The pallet has too many candidates."]
					TooManyCandidates,
					#[codec(index = 1)]
					#[doc = "Leaving would result in too few candidates."]
					TooFewEligibleCollators,
					#[codec(index = 2)]
					#[doc = "Account is already a candidate."]
					AlreadyCandidate,
					#[codec(index = 3)]
					#[doc = "Account is not a candidate."]
					NotCandidate,
					#[codec(index = 4)]
					#[doc = "There are too many Invulnerables."]
					TooManyInvulnerables,
					#[codec(index = 5)]
					#[doc = "Account is already an Invulnerable."]
					AlreadyInvulnerable,
					#[codec(index = 6)]
					#[doc = "Account is not an Invulnerable."]
					NotInvulnerable,
					#[codec(index = 7)]
					#[doc = "Account has no associated validator ID."]
					NoAssociatedValidatorId,
					#[codec(index = 8)]
					#[doc = "Validator ID is not yet registered."]
					ValidatorNotRegistered,
					#[codec(index = 9)]
					#[doc = "Could not insert in the candidate list."]
					InsertToCandidateListFailed,
					#[codec(index = 10)]
					#[doc = "Could not remove from the candidate list."]
					RemoveFromCandidateListFailed,
					#[codec(index = 11)]
					#[doc = "New deposit amount would be below the minimum candidacy bond."]
					DepositTooLow,
					#[codec(index = 12)]
					#[doc = "Could not update the candidate list."]
					UpdateCandidateListFailed,
					#[codec(index = 13)]
					#[doc = "Deposit amount is too low to take the target's slot in the candidate list."]
					InsufficientBond,
					#[codec(index = 14)]
					#[doc = "The target account to be replaced in the candidate list is not a candidate."]
					TargetIsNotCandidate,
					#[codec(index = 15)]
					#[doc = "The updated deposit amount is equal to the amount already reserved."]
					IdenticalDeposit,
					#[codec(index = 16)]
					#[doc = "Cannot lower candidacy bond while occupying a future collator slot in the list."]
					InvalidUnreserve,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "New Invulnerables were set."]
					NewInvulnerables { invulnerables: ::std::vec::Vec<::subxt::utils::AccountId32> },
					#[codec(index = 1)]
					#[doc = "A new Invulnerable was added."]
					InvulnerableAdded { account_id: ::subxt::utils::AccountId32 },
					#[codec(index = 2)]
					#[doc = "An Invulnerable was removed."]
					InvulnerableRemoved { account_id: ::subxt::utils::AccountId32 },
					#[codec(index = 3)]
					#[doc = "The number of desired candidates was set."]
					NewDesiredCandidates { desired_candidates: ::core::primitive::u32 },
					#[codec(index = 4)]
					#[doc = "The candidacy bond was set."]
					NewCandidacyBond { bond_amount: ::core::primitive::u128 },
					#[codec(index = 5)]
					#[doc = "A new candidate joined."]
					CandidateAdded {
						account_id: ::subxt::utils::AccountId32,
						deposit: ::core::primitive::u128,
					},
					#[codec(index = 6)]
					#[doc = "Bond of a candidate updated."]
					CandidateBondUpdated {
						account_id: ::subxt::utils::AccountId32,
						deposit: ::core::primitive::u128,
					},
					#[codec(index = 7)]
					#[doc = "A candidate was removed."]
					CandidateRemoved { account_id: ::subxt::utils::AccountId32 },
					#[codec(index = 8)]
					#[doc = "An account was replaced in the candidate list by another one."]
					CandidateReplaced {
						old: ::subxt::utils::AccountId32,
						new: ::subxt::utils::AccountId32,
						deposit: ::core::primitive::u128,
					},
					#[codec(index = 9)]
					#[doc = "An account was unable to be added to the Invulnerables because they did not have keys"]
					#[doc = "registered. Other Invulnerables may have been set."]
					InvalidInvulnerableSkipped { account_id: ::subxt::utils::AccountId32 },
				}
			}
		}
		pub mod pallet_hyperbridge {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedHostParams<_0> {
				#[codec(index = 0)]
				V1(_0),
			}
		}
		pub mod pallet_ismp {
			use super::runtime_types;
			pub mod errors {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					FrozenStateMachine { id: runtime_types::ismp::consensus::StateMachineId },
					#[codec(index = 5)]
					RequestCommitmentNotFound { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 6)]
					RequestVerificationFailed { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 7)]
					ResponseVerificationFailed { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 8)]
					ConsensusProofVerificationFailed { id: [::core::primitive::u8; 4usize] },
					#[codec(index = 9)]
					ExpiredConsensusClient { id: [::core::primitive::u8; 4usize] },
					#[codec(index = 10)]
					CannotHandleMessage,
					#[codec(index = 11)]
					Custom { msg: ::std::vec::Vec<::core::primitive::u8> },
					#[codec(index = 12)]
					UnbondingPeriodElapsed { id: [::core::primitive::u8; 4usize] },
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
						meta: runtime_types::ismp::events::Meta,
						timeout_timestamp: ::core::primitive::u64,
						state_machine_time: ::core::primitive::u64,
					},
					#[codec(index = 17)]
					RequestTimeoutVerificationFailed { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 18)]
					InsufficientProofHeight,
					#[codec(index = 19)]
					ModuleNotFound(::std::vec::Vec<::core::primitive::u8>),
					#[codec(index = 20)]
					ModuleDispatchError {
						msg: ::std::vec::Vec<::core::primitive::u8>,
						meta: runtime_types::ismp::events::Meta,
					},
					#[codec(index = 21)]
					UnsolicitedResponse { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 22)]
					RequestTimeout { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 23)]
					ResponseTimeout { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 24)]
					DuplicateRequest { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 25)]
					DuplicateResponse { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 26)]
					RequestProofMetadataNotValid { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 27)]
					ResponseProofMetadataNotValid { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 28)]
					RequestProxyProhibited { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 29)]
					ResponseProxyProhibited { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 30)]
					InvalidRequestDestination { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 31)]
					InvalidResponseDestination { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 32)]
					InvalidResponseType { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 33)]
					UnknownRequest { meta: runtime_types::ismp::events::Meta },
					#[codec(index = 34)]
					UnknownResponse { meta: runtime_types::ismp::events::Meta },
				}
			}
			pub mod mmr {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Leaf {
					#[codec(index = 0)]
					Request(runtime_types::ismp::router::Request),
					#[codec(index = 1)]
					Response(runtime_types::ismp::router::Response),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct LeafIndexAndPos {
					pub leaf_index: ::core::primitive::u64,
					pub pos: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Proof<_0> {
					pub leaf_indices_and_pos:
						::std::vec::Vec<runtime_types::pallet_ismp::mmr::LeafIndexAndPos>,
					pub leaf_count: ::core::primitive::u64,
					pub items: ::std::vec::Vec<_0>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum ProofKeys {
					#[codec(index = 0)]
					Requests(::std::vec::Vec<::subxt::utils::H256>),
					#[codec(index = 1)]
					Responses(::std::vec::Vec<::subxt::utils::H256>),
				}
			}
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::handle_unsigned`]."]
					handle_unsigned {
						messages: ::std::vec::Vec<runtime_types::ismp::messaging::Message>,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::create_consensus_client`]."]
					create_consensus_client {
						message: runtime_types::ismp::messaging::CreateConsensusState,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::update_consensus_state`]."]
					update_consensus_state {
						message: runtime_types::pallet_ismp::utils::UpdateConsensusState,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::fund_message`]."]
					fund_message {
						message: runtime_types::pallet_ismp::utils::FundMessageParams<
							::core::primitive::u128,
						>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Pallet errors"]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Invalid ISMP message"]
					InvalidMessage,
					#[codec(index = 1)]
					#[doc = "Requested message was not found"]
					MessageNotFound,
					#[codec(index = 2)]
					#[doc = "Encountered an error while creating the consensus client."]
					ConsensusClientCreationFailed,
					#[codec(index = 3)]
					#[doc = "Couldn't update unbonding period"]
					UnbondingPeriodUpdateFailed,
					#[codec(index = 4)]
					#[doc = "Couldn't update challenge period"]
					ChallengePeriodUpdateFailed,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Pallet Events"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Emitted when a state machine is successfully updated to a new height"]
					StateMachineUpdated {
						state_machine_id: runtime_types::ismp::consensus::StateMachineId,
						latest_height: ::core::primitive::u64,
					},
					#[codec(index = 1)]
					#[doc = "Emitted when a state commitment is vetoed by a fisherman"]
					StateCommitmentVetoed {
						height: runtime_types::ismp::consensus::StateMachineHeight,
						fisherman: runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					},
					#[codec(index = 2)]
					#[doc = "Indicates that a consensus client has been created"]
					ConsensusClientCreated { consensus_client_id: [::core::primitive::u8; 4usize] },
					#[codec(index = 3)]
					#[doc = "Indicates that a consensus client has been created"]
					ConsensusClientFrozen { consensus_client_id: [::core::primitive::u8; 4usize] },
					#[codec(index = 4)]
					#[doc = "An Outgoing Response has been deposited"]
					Response {
						dest_chain: runtime_types::ismp::host::StateMachine,
						source_chain: runtime_types::ismp::host::StateMachine,
						request_nonce: ::core::primitive::u64,
						commitment: ::subxt::utils::H256,
					},
					#[codec(index = 5)]
					#[doc = "An Outgoing Request has been deposited"]
					Request {
						dest_chain: runtime_types::ismp::host::StateMachine,
						source_chain: runtime_types::ismp::host::StateMachine,
						request_nonce: ::core::primitive::u64,
						commitment: ::subxt::utils::H256,
					},
					#[codec(index = 6)]
					#[doc = "Some errors handling some ismp messages"]
					Errors {
						errors: ::std::vec::Vec<runtime_types::pallet_ismp::errors::HandlingError>,
					},
					#[codec(index = 7)]
					#[doc = "Post Request Handled"]
					PostRequestHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 8)]
					#[doc = "Post Response Handled"]
					PostResponseHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 9)]
					#[doc = "Get Response Handled"]
					GetRequestHandled(runtime_types::ismp::events::RequestResponseHandled),
					#[codec(index = 10)]
					#[doc = "Post request timeout handled"]
					PostRequestTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
					#[codec(index = 11)]
					#[doc = "Post response timeout handled"]
					PostResponseTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
					#[codec(index = 12)]
					#[doc = "Get request timeout handled"]
					GetRequestTimeoutHandled(runtime_types::ismp::events::TimeoutHandled),
				}
			}
			pub mod utils {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FundMessageParams<_0> {
					pub commitment: runtime_types::pallet_ismp::utils::MessageCommitment,
					pub amount: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum MessageCommitment {
					#[codec(index = 0)]
					Request(::subxt::utils::H256),
					#[codec(index = 1)]
					Response(::subxt::utils::H256),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UpdateConsensusState {
					pub consensus_state_id: [::core::primitive::u8; 4usize],
					pub unbonding_period: ::core::option::Option<::core::primitive::u64>,
					pub challenge_period: ::core::option::Option<::core::primitive::u64>,
				}
			}
		}
		pub mod pallet_ismp_demo {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::transfer`]."]
					transfer {
						params: runtime_types::pallet_ismp_demo::pallet::TransferParams<
							::subxt::utils::AccountId32,
							::core::primitive::u128,
						>,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::get_request`]."]
					get_request { params: runtime_types::pallet_ismp_demo::pallet::GetRequest },
					#[codec(index = 2)]
					#[doc = "See [`Pallet::dispatch_to_evm`]."]
					dispatch_to_evm { params: runtime_types::pallet_ismp_demo::pallet::EvmParams },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Pallet Errors"]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Error encountered when initializing transfer"]
					TransferFailed,
					#[codec(index = 1)]
					#[doc = "Failed to dispatch get request"]
					GetDispatchFailed,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Pallet events"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Some balance has been transferred"]
					BalanceTransferred {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						dest_chain: runtime_types::ismp::host::StateMachine,
					},
					#[codec(index = 1)]
					#[doc = "Some balance has been received"]
					BalanceReceived {
						from: ::subxt::utils::AccountId32,
						to: ::subxt::utils::AccountId32,
						amount: ::core::primitive::u128,
						source_chain: runtime_types::ismp::host::StateMachine,
					},
					#[codec(index = 2)]
					#[doc = "Request data receieved"]
					Request {
						source: runtime_types::ismp::host::StateMachine,
						data: ::std::string::String,
					},
					#[codec(index = 3)]
					#[doc = "Get response recieved"]
					GetResponse(
						::std::vec::Vec<
							::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
						>,
					),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct EvmParams {
					pub module: ::subxt::utils::H160,
					pub destination: runtime_types::ismp::host::Ethereum,
					pub timeout: ::core::primitive::u64,
					pub count: ::core::primitive::u64,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GetRequest {
					pub para_id: ::core::primitive::u32,
					pub height: ::core::primitive::u32,
					pub timeout: ::core::primitive::u64,
					pub keys: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TransferParams<_0, _1> {
					pub to: _0,
					pub amount: _1,
					pub para_id: ::core::primitive::u32,
					pub timeout: ::core::primitive::u64,
				}
			}
		}
		pub mod pallet_ismp_host_executive {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set_host_params`]."]
					set_host_params {
						params: ::subxt::utils::KeyedVec<
							runtime_types::ismp::host::StateMachine,
							runtime_types::pallet_ismp_host_executive::params::HostParam<
								::core::primitive::u128,
							>,
						>,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::update_host_params`]."]
					update_host_params {
						state_machine: runtime_types::ismp::host::StateMachine,
						update: runtime_types::pallet_ismp_host_executive::params::HostParamUpdate<
							::core::primitive::u128,
						>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Could not commit the outgoing request"]
					DispatchFailed,
					#[codec(index = 1)]
					#[doc = "The requested state machine was unrecognized"]
					UnknownStateMachine,
					#[codec(index = 2)]
					#[doc = "Mismatched state machine and HostParams"]
					MismatchedHostParams,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "`AdminOrigin` has initiated a host parameter update to the mentioned state"]
					#[doc = "machine"]
					HostParamsUpdated {
						state_machine: runtime_types::ismp::host::StateMachine,
						old: runtime_types::pallet_ismp_host_executive::params::HostParam<
							::core::primitive::u128,
						>,
						new: runtime_types::pallet_ismp_host_executive::params::HostParam<
							::core::primitive::u128,
						>,
					},
					#[codec(index = 1)]
					#[doc = "`AdminOrigin` has set the initial host parameters for the mentioned state"]
					#[doc = "machine"]
					HostParamsSet {
						state_machine: runtime_types::ismp::host::StateMachine,
						params: runtime_types::pallet_ismp_host_executive::params::HostParam<
							::core::primitive::u128,
						>,
					},
				}
			}
			pub mod params {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct EvmHostParam {
					pub default_timeout: ::core::primitive::u128,
					pub per_byte_fee: runtime_types::primitive_types::U256,
					pub state_commitment_fee: runtime_types::primitive_types::U256,
					pub fee_token: ::subxt::utils::H160,
					pub admin: ::subxt::utils::H160,
					pub handler: ::subxt::utils::H160,
					pub host_manager: ::subxt::utils::H160,
					pub un_staking_period: ::core::primitive::u128,
					pub challenge_period: ::core::primitive::u128,
					pub consensus_client: ::subxt::utils::H160,
					pub state_machines: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u32,
					>,
					pub fishermen: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::subxt::utils::H160,
					>,
					pub hyperbridge: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct EvmHostParamUpdate {
					pub default_timeout: ::core::option::Option<::core::primitive::u128>,
					pub per_byte_fee: ::core::option::Option<runtime_types::primitive_types::U256>,
					pub state_commitment_fee:
						::core::option::Option<runtime_types::primitive_types::U256>,
					pub fee_token: ::core::option::Option<::subxt::utils::H160>,
					pub admin: ::core::option::Option<::subxt::utils::H160>,
					pub handler: ::core::option::Option<::subxt::utils::H160>,
					pub host_manager: ::core::option::Option<::subxt::utils::H160>,
					pub un_staking_period: ::core::option::Option<::core::primitive::u128>,
					pub challenge_period: ::core::option::Option<::core::primitive::u128>,
					pub consensus_client: ::core::option::Option<::subxt::utils::H160>,
					pub state_machines: ::core::option::Option<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u32,
						>,
					>,
					pub fishermen: ::core::option::Option<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::subxt::utils::H160,
						>,
					>,
					pub hyperbridge: ::core::option::Option<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum HostParam<_0> {
					#[codec(index = 0)]
					SubstrateHostParam(runtime_types::pallet_hyperbridge::VersionedHostParams<_0>),
					#[codec(index = 1)]
					EvmHostParam(runtime_types::pallet_ismp_host_executive::params::EvmHostParam),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum HostParamUpdate<_0> {
					#[codec(index = 0)]
					SubstrateHostParam(runtime_types::pallet_hyperbridge::VersionedHostParams<_0>),
					#[codec(index = 1)]
					EvmHostParam(
						runtime_types::pallet_ismp_host_executive::params::EvmHostParamUpdate,
					),
				}
			}
		}
		pub mod pallet_ismp_relayer {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::accumulate_fees`]."]
					accumulate_fees {
						withdrawal_proof:
							runtime_types::pallet_ismp_relayer::withdrawal::WithdrawalProof,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::withdraw_fees`]."]
					withdraw_fees {
						withdrawal_data:
							runtime_types::pallet_ismp_relayer::withdrawal::WithdrawalInputData,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_minimum_withdrawal`]."]
					set_minimum_withdrawal { amount: ::core::primitive::u128 },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Withdrawal Proof Validation Error"]
					ProofValidationError,
					#[codec(index = 1)]
					#[doc = "Invalid Public Key"]
					InvalidPublicKey,
					#[codec(index = 2)]
					#[doc = "Invalid Withdrawal signature"]
					InvalidSignature,
					#[codec(index = 3)]
					#[doc = "Empty balance"]
					EmptyBalance,
					#[codec(index = 4)]
					#[doc = "Invalid Amount"]
					NotEnoughBalance,
					#[codec(index = 5)]
					#[doc = "Encountered a mis-match in the requested state machine"]
					MismatchedStateMachine,
					#[codec(index = 6)]
					#[doc = "Relayer Manager Address on Dest chain not set"]
					MissingMangerAddress,
					#[codec(index = 7)]
					#[doc = "Failed to dispatch request"]
					DispatchFailed,
					#[codec(index = 8)]
					#[doc = "Error"]
					ErrorCompletingCall,
					#[codec(index = 9)]
					#[doc = "Missing commitments"]
					MissingCommitments,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Events emiited by the relayer pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A relayer with the `address` has accumulated some fees on the `state_machine`"]
					AccumulateFees {
						address: runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						state_machine: runtime_types::ismp::host::StateMachine,
						amount: runtime_types::primitive_types::U256,
					},
					#[codec(index = 1)]
					#[doc = "A relayer with the the `address` has initiated a withdrawal on the `state_machine`"]
					Withdraw {
						address: runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
						state_machine: runtime_types::ismp::host::StateMachine,
						amount: runtime_types::primitive_types::U256,
					},
				}
			}
			pub mod withdrawal {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Key {
					#[codec(index = 0)]
					Request(::subxt::utils::H256),
					#[codec(index = 1)]
					Response {
						request_commitment: ::subxt::utils::H256,
						response_commitment: ::subxt::utils::H256,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Signature {
					#[codec(index = 0)]
					Ethereum {
						address: ::std::vec::Vec<::core::primitive::u8>,
						signature: ::std::vec::Vec<::core::primitive::u8>,
					},
					#[codec(index = 1)]
					Sr25519 {
						public_key: ::std::vec::Vec<::core::primitive::u8>,
						signature: ::std::vec::Vec<::core::primitive::u8>,
					},
					#[codec(index = 2)]
					Ed25519 {
						public_key: ::std::vec::Vec<::core::primitive::u8>,
						signature: ::std::vec::Vec<::core::primitive::u8>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WithdrawalInputData {
					pub signature: runtime_types::pallet_ismp_relayer::withdrawal::Signature,
					pub dest_chain: runtime_types::ismp::host::StateMachine,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct WithdrawalProof {
					pub commitments:
						::std::vec::Vec<runtime_types::pallet_ismp_relayer::withdrawal::Key>,
					pub source_proof: runtime_types::ismp::messaging::Proof,
					pub dest_proof: runtime_types::ismp::messaging::Proof,
				}
			}
		}
		pub mod pallet_message_queue {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::reap_page`]."]
					reap_page {
						message_origin:
							runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						page_index: ::core::primitive::u32,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::execute_overweight`]."]
					execute_overweight {
						message_origin:
							runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						page: ::core::primitive::u32,
						index: ::core::primitive::u32,
						weight_limit: runtime_types::sp_weights::weight_v2::Weight,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Page is not reapable because it has items remaining to be processed and is not old"]
					#[doc = "enough."]
					NotReapable,
					#[codec(index = 1)]
					#[doc = "Page to be reaped does not exist."]
					NoPage,
					#[codec(index = 2)]
					#[doc = "The referenced message could not be found."]
					NoMessage,
					#[codec(index = 3)]
					#[doc = "The message was already processed and cannot be processed again."]
					AlreadyProcessed,
					#[codec(index = 4)]
					#[doc = "The message is queued for future execution."]
					Queued,
					#[codec(index = 5)]
					#[doc = "There is temporarily not enough weight to continue servicing messages."]
					InsufficientWeight,
					#[codec(index = 6)]
					#[doc = "This message is temporarily unprocessable."]
					#[doc = ""]
					#[doc = "Such errors are expected, but not guaranteed, to resolve themselves eventually through"]
					#[doc = "retrying."]
					TemporarilyUnprocessable,
					#[codec(index = 7)]
					#[doc = "The queue is paused and no message can be executed from it."]
					#[doc = ""]
					#[doc = "This can change at any time and may resolve in the future by re-trying."]
					QueuePaused,
					#[codec(index = 8)]
					#[doc = "Another call is in progress and needs to finish before this call can happen."]
					RecursiveDisallowed,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Message discarded due to an error in the `MessageProcessor` (usually a format error)."]
					ProcessingFailed {
						id: ::subxt::utils::H256,
						origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						error: runtime_types::frame_support::traits::messages::ProcessMessageError,
					},
					#[codec(index = 1)]
					#[doc = "Message is processed."]
					Processed {
						id: ::subxt::utils::H256,
						origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						weight_used: runtime_types::sp_weights::weight_v2::Weight,
						success: ::core::primitive::bool,
					},
					#[codec(index = 2)]
					#[doc = "Message placed in overweight queue."]
					OverweightEnqueued {
						id: [::core::primitive::u8; 32usize],
						origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						page_index: ::core::primitive::u32,
						message_index: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "This page was reaped."]
					PageReaped {
						origin: runtime_types::cumulus_primitives_core::AggregateMessageOrigin,
						index: ::core::primitive::u32,
					},
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct BookState<_0> {
				pub begin: ::core::primitive::u32,
				pub end: ::core::primitive::u32,
				pub count: ::core::primitive::u32,
				pub ready_neighbours:
					::core::option::Option<runtime_types::pallet_message_queue::Neighbours<_0>>,
				pub message_count: ::core::primitive::u64,
				pub size: ::core::primitive::u64,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Neighbours<_0> {
				pub prev: _0,
				pub next: _0,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Page<_0> {
				pub remaining: _0,
				pub remaining_size: _0,
				pub first_index: _0,
				pub first: _0,
				pub last: _0,
				pub heap: runtime_types::bounded_collections::bounded_vec::BoundedVec<
					::core::primitive::u8,
				>,
			}
		}
		pub mod pallet_session {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set_keys`]."]
					set_keys {
						keys: runtime_types::gargantua_runtime::SessionKeys,
						proof: ::std::vec::Vec<::core::primitive::u8>,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::purge_keys`]."]
					purge_keys,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the session pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Invalid ownership proof."]
					InvalidProof,
					#[codec(index = 1)]
					#[doc = "No associated validator ID for account."]
					NoAssociatedValidatorId,
					#[codec(index = 2)]
					#[doc = "Registered duplicate key."]
					DuplicatedKey,
					#[codec(index = 3)]
					#[doc = "No keys are associated with this account."]
					NoKeys,
					#[codec(index = 4)]
					#[doc = "Key setting account is not live, so it's impossible to associate keys."]
					NoAccount,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "New session has happened. Note that the argument is the session index, not the"]
					#[doc = "block number as the type might suggest."]
					NewSession { session_index: ::core::primitive::u32 },
				}
			}
		}
		pub mod pallet_sudo {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::sudo`]."]
					sudo { call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::sudo_unchecked_weight`]."]
					sudo_unchecked_weight {
						call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
						weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::set_key`]."]
					set_key { new: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()> },
					#[codec(index = 3)]
					#[doc = "See [`Pallet::sudo_as`]."]
					sudo_as {
						who: ::subxt::utils::MultiAddress<::subxt::utils::AccountId32, ()>,
						call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::remove_key`]."]
					remove_key,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Error for the Sudo pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Sender must be the Sudo account."]
					RequireSudo,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A sudo call just took place."]
					Sudid {
						sudo_result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
					#[codec(index = 1)]
					#[doc = "The sudo key has been updated."]
					KeyChanged {
						old: ::core::option::Option<::subxt::utils::AccountId32>,
						new: ::subxt::utils::AccountId32,
					},
					#[codec(index = 2)]
					#[doc = "The key was permanently removed."]
					KeyRemoved,
					#[codec(index = 3)]
					#[doc = "A [sudo_as](Pallet::sudo_as) call just took place."]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::set`]."]
					set {
						#[codec(compact)]
						now: ::core::primitive::u64,
					},
				}
			}
		}
		pub mod pallet_token_governor {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					# [codec (index = 0)] # [doc = "See [`Pallet::create_erc6160_asset`]."] create_erc6160_asset { asset : runtime_types :: pallet_token_governor :: types :: ERC6160AssetRegistration , } , # [codec (index = 1)] # [doc = "See [`Pallet::create_erc6160_asset_unsigned`]."] create_erc6160_asset_unsigned { registration : runtime_types :: pallet_token_governor :: types :: UnsignedERC6160AssetRegistration < :: subxt :: utils :: AccountId32 > , } , # [codec (index = 2)] # [doc = "See [`Pallet::update_registrar_params`]."] update_registrar_params { update : runtime_types :: pallet_token_governor :: types :: RegistrarParamsUpdate , state_machine : runtime_types :: ismp :: host :: StateMachine , } , # [codec (index = 3)] # [doc = "See [`Pallet::update_gateway_params`]."] update_gateway_params { update : runtime_types :: pallet_token_governor :: types :: TokenGatewayParamsUpdate , state_machine : runtime_types :: ismp :: host :: StateMachine , } , # [codec (index = 4)] # [doc = "See [`Pallet::update_params`]."] update_params { update : runtime_types :: pallet_token_governor :: types :: ParamsUpdate < :: core :: primitive :: u128 > , } , # [codec (index = 5)] # [doc = "See [`Pallet::update_erc6160_asset`]."] update_erc6160_asset { update : runtime_types :: pallet_token_governor :: types :: ERC6160AssetUpdate , } , # [codec (index = 6)] # [doc = "See [`Pallet::create_erc20_asset`]."] create_erc20_asset { asset : runtime_types :: pallet_token_governor :: types :: ERC20AssetRegistration , } , }
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Errors that can be returned by this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "An asset with the same identifier already exists"]
					AssetAlreadyExists,
					#[codec(index = 1)]
					#[doc = "The pallet has not yet been initialized"]
					NotInitialized,
					#[codec(index = 2)]
					#[doc = "Failed to dispatch a request"]
					DispatchFailed,
					#[codec(index = 3)]
					#[doc = "Provided name or symbol isn't valid utf-8"]
					InvalidUtf8,
					#[codec(index = 4)]
					#[doc = "Provided asset identifier was unknown"]
					UnknownAsset,
					#[codec(index = 5)]
					#[doc = "The account signer is not authorized to perform this action"]
					NotAssetOwner,
					#[codec(index = 6)]
					#[doc = "Provided signature was invalid"]
					InvalidSignature,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Pallet events that functions in this pallet can emit."]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A new asset has been registered"]
					AssetRegistered { asset_id: ::subxt::utils::H256 },
					#[codec(index = 1)]
					#[doc = "A new pending asset has been registered"]
					NewPendingAsset { asset_id: ::subxt::utils::H256, owner: ::subxt::utils::H160 },
					#[codec(index = 2)]
					#[doc = "The TokenRegistrar params have been updated for a state machine"]
					RegistrarParamsUpdated {
						old: runtime_types::pallet_token_governor::types::RegistrarParams,
						new: runtime_types::pallet_token_governor::types::RegistrarParams,
						state_machine: runtime_types::ismp::host::StateMachine,
					},
					#[codec(index = 3)]
					#[doc = "The TokenGateway params have been updated for a state machine"]
					GatewayParamsUpdated {
						old: runtime_types::pallet_token_governor::types::GatewayParams,
						new: runtime_types::pallet_token_governor::types::GatewayParams,
						state_machine: runtime_types::ismp::host::StateMachine,
					},
					#[codec(index = 4)]
					#[doc = "The TokenGovernor parameters have been updated"]
					ParamsUpdated {
						old: runtime_types::pallet_token_governor::types::Params<
							::core::primitive::u128,
						>,
						new: runtime_types::pallet_token_governor::types::Params<
							::core::primitive::u128,
						>,
					},
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AssetMetadata {
					pub name: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub symbol: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub logo: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct AssetRegistration {
					pub chain: runtime_types::ismp::host::StateMachine,
					pub erc20: ::core::option::Option<::subxt::utils::H160>,
					pub erc6160: ::core::option::Option<::subxt::utils::H160>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ChainWithSupply {
					pub chain: runtime_types::ismp::host::StateMachine,
					pub supply: ::core::option::Option<
						runtime_types::pallet_token_governor::types::InitialSupply,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ERC20AssetRegistration {
					pub name: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub symbol: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub logo: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub chains: ::std::vec::Vec<
						runtime_types::pallet_token_governor::types::AssetRegistration,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ERC6160AssetRegistration {
					pub name: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub symbol: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub logo: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub chains: ::std::vec::Vec<
						runtime_types::pallet_token_governor::types::ChainWithSupply,
					>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ERC6160AssetUpdate {
					pub asset_id: ::subxt::utils::H256,
					pub logo: ::core::option::Option<
						runtime_types::bounded_collections::bounded_vec::BoundedVec<
							::core::primitive::u8,
						>,
					>,
					pub add_chains: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::pallet_token_governor::types::ChainWithSupply,
					>,
					pub remove_chains: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						runtime_types::ismp::host::StateMachine,
					>,
					pub new_admins: runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						runtime_types::ismp::host::StateMachine,
						::subxt::utils::H160,
					)>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct GatewayParams {
					pub host: ::subxt::utils::H160,
					pub uniswap_v2: ::subxt::utils::H160,
					pub call_dispatcher: ::subxt::utils::H160,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InitialSupply {
					pub beneficiary: ::subxt::utils::H160,
					pub initial_supply: runtime_types::primitive_types::U256,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Params<_0> {
					pub token_gateway_address: ::subxt::utils::H160,
					pub token_registrar_address: ::subxt::utils::H160,
					pub registration_fee: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ParamsUpdate<_0> {
					pub token_gateway_address: ::core::option::Option<::subxt::utils::H160>,
					pub token_registrar_address: ::core::option::Option<::subxt::utils::H160>,
					pub registration_fee: ::core::option::Option<_0>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RegistrarParams {
					pub erc20_native_token: ::subxt::utils::H160,
					pub host: ::subxt::utils::H160,
					pub uniswap_v2: ::subxt::utils::H160,
					pub base_fee: runtime_types::primitive_types::U256,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RegistrarParamsUpdate {
					pub erc20_native_token: ::core::option::Option<::subxt::utils::H160>,
					pub host: ::core::option::Option<::subxt::utils::H160>,
					pub uniswap_v2: ::core::option::Option<::subxt::utils::H160>,
					pub base_fee: ::core::option::Option<runtime_types::primitive_types::U256>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct TokenGatewayParamsUpdate {
					pub host: ::core::option::Option<::subxt::utils::H160>,
					pub uniswap_v2: ::core::option::Option<::subxt::utils::H160>,
					pub call_dispatcher: ::core::option::Option<::subxt::utils::H160>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct UnsignedERC6160AssetRegistration<_0> {
					pub asset:
						runtime_types::pallet_token_governor::types::ERC6160AssetRegistration,
					pub signature: runtime_types::bounded_collections::bounded_vec::BoundedVec<
						::core::primitive::u8,
					>,
					pub owner: _0,
				}
			}
		}
		pub mod pallet_transaction_payment {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,"]
					#[doc = "has been paid by `who`."]
					TransactionFeePaid {
						who: ::subxt::utils::AccountId32,
						actual_fee: ::core::primitive::u128,
						tip: ::core::primitive::u128,
					},
				}
			}
			pub mod types {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FeeDetails<_0> {
					pub inclusion_fee: ::core::option::Option<
						runtime_types::pallet_transaction_payment::types::InclusionFee<_0>,
					>,
					pub tip: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct InclusionFee<_0> {
					pub base_fee: _0,
					pub len_fee: _0,
					pub adjusted_weight_fee: _0,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RuntimeDispatchInfo<_0, _1> {
					pub weight: _1,
					pub class: runtime_types::frame_support::dispatch::DispatchClass,
					pub partial_fee: _0,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ChargeTransactionPayment(#[codec(compact)] pub ::core::primitive::u128);
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Releases {
				#[codec(index = 0)]
				V1Ancient,
				#[codec(index = 1)]
				V2,
			}
		}
		pub mod pallet_utility {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::batch`]."]
					batch { calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall> },
					#[codec(index = 1)]
					#[doc = "See [`Pallet::as_derivative`]."]
					as_derivative {
						index: ::core::primitive::u16,
						call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::batch_all`]."]
					batch_all {
						calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::dispatch_as`]."]
					dispatch_as {
						as_origin:
							::std::boxed::Box<runtime_types::gargantua_runtime::OriginCaller>,
						call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::force_batch`]."]
					force_batch {
						calls: ::std::vec::Vec<runtime_types::gargantua_runtime::RuntimeCall>,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::with_weight`]."]
					with_weight {
						call: ::std::boxed::Box<runtime_types::gargantua_runtime::RuntimeCall>,
						weight: runtime_types::sp_weights::weight_v2::Weight,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "Too many calls batched."]
					TooManyCalls,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Batch of dispatches did not complete fully. Index of first failing dispatch given, as"]
					#[doc = "well as the error."]
					BatchInterrupted {
						index: ::core::primitive::u32,
						error: runtime_types::sp_runtime::DispatchError,
					},
					#[codec(index = 1)]
					#[doc = "Batch of dispatches completed fully with no error."]
					BatchCompleted,
					#[codec(index = 2)]
					#[doc = "Batch of dispatches completed but has errors."]
					BatchCompletedWithErrors,
					#[codec(index = 3)]
					#[doc = "A single item within a Batch of dispatches has completed with no error."]
					ItemCompleted,
					#[codec(index = 4)]
					#[doc = "A single item within a Batch of dispatches has completed with error."]
					ItemFailed { error: runtime_types::sp_runtime::DispatchError },
					#[codec(index = 5)]
					#[doc = "A call was dispatched."]
					DispatchedAs {
						result:
							::core::result::Result<(), runtime_types::sp_runtime::DispatchError>,
					},
				}
			}
		}
		pub mod pallet_xcm {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {
					#[codec(index = 0)]
					#[doc = "See [`Pallet::send`]."]
					send {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm>,
					},
					#[codec(index = 1)]
					#[doc = "See [`Pallet::teleport_assets`]."]
					teleport_assets {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
						fee_asset_item: ::core::primitive::u32,
					},
					#[codec(index = 2)]
					#[doc = "See [`Pallet::reserve_transfer_assets`]."]
					reserve_transfer_assets {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
						fee_asset_item: ::core::primitive::u32,
					},
					#[codec(index = 3)]
					#[doc = "See [`Pallet::execute`]."]
					execute {
						message: ::std::boxed::Box<runtime_types::xcm::VersionedXcm2>,
						max_weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 4)]
					#[doc = "See [`Pallet::force_xcm_version`]."]
					force_xcm_version {
						location: ::std::boxed::Box<
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
						version: ::core::primitive::u32,
					},
					#[codec(index = 5)]
					#[doc = "See [`Pallet::force_default_xcm_version`]."]
					force_default_xcm_version {
						maybe_xcm_version: ::core::option::Option<::core::primitive::u32>,
					},
					#[codec(index = 6)]
					#[doc = "See [`Pallet::force_subscribe_version_notify`]."]
					force_subscribe_version_notify {
						location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					},
					#[codec(index = 7)]
					#[doc = "See [`Pallet::force_unsubscribe_version_notify`]."]
					force_unsubscribe_version_notify {
						location: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
					},
					#[codec(index = 8)]
					#[doc = "See [`Pallet::limited_reserve_transfer_assets`]."]
					limited_reserve_transfer_assets {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
						fee_asset_item: ::core::primitive::u32,
						weight_limit: runtime_types::xcm::v3::WeightLimit,
					},
					#[codec(index = 9)]
					#[doc = "See [`Pallet::limited_teleport_assets`]."]
					limited_teleport_assets {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
						fee_asset_item: ::core::primitive::u32,
						weight_limit: runtime_types::xcm::v3::WeightLimit,
					},
					#[codec(index = 10)]
					#[doc = "See [`Pallet::force_suspension`]."]
					force_suspension { suspended: ::core::primitive::bool },
					#[codec(index = 11)]
					#[doc = "See [`Pallet::transfer_assets`]."]
					transfer_assets {
						dest: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						beneficiary: ::std::boxed::Box<runtime_types::xcm::VersionedMultiLocation>,
						assets: ::std::boxed::Box<runtime_types::xcm::VersionedMultiAssets>,
						fee_asset_item: ::core::primitive::u32,
						weight_limit: runtime_types::xcm::v3::WeightLimit,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Error` enum of this pallet."]
				pub enum Error {
					#[codec(index = 0)]
					#[doc = "The desired destination was unreachable, generally because there is a no way of routing"]
					#[doc = "to it."]
					Unreachable,
					#[codec(index = 1)]
					#[doc = "There was some other issue (i.e. not to do with routing) in sending the message."]
					#[doc = "Perhaps a lack of space for buffering the message."]
					SendFailure,
					#[codec(index = 2)]
					#[doc = "The message execution fails the filter."]
					Filtered,
					#[codec(index = 3)]
					#[doc = "The message's weight could not be determined."]
					UnweighableMessage,
					#[codec(index = 4)]
					#[doc = "The destination `MultiLocation` provided cannot be inverted."]
					DestinationNotInvertible,
					#[codec(index = 5)]
					#[doc = "The assets to be sent are empty."]
					Empty,
					#[codec(index = 6)]
					#[doc = "Could not re-anchor the assets to declare the fees for the destination chain."]
					CannotReanchor,
					#[codec(index = 7)]
					#[doc = "Too many assets have been attempted for transfer."]
					TooManyAssets,
					#[codec(index = 8)]
					#[doc = "Origin is invalid for sending."]
					InvalidOrigin,
					#[codec(index = 9)]
					#[doc = "The version of the `Versioned` value used is not able to be interpreted."]
					BadVersion,
					#[codec(index = 10)]
					#[doc = "The given location could not be used (e.g. because it cannot be expressed in the"]
					#[doc = "desired version of XCM)."]
					BadLocation,
					#[codec(index = 11)]
					#[doc = "The referenced subscription could not be found."]
					NoSubscription,
					#[codec(index = 12)]
					#[doc = "The location is invalid since it already has a subscription from us."]
					AlreadySubscribed,
					#[codec(index = 13)]
					#[doc = "Could not check-out the assets for teleportation to the destination chain."]
					CannotCheckOutTeleport,
					#[codec(index = 14)]
					#[doc = "The owner does not own (all) of the asset that they wish to do the operation on."]
					LowBalance,
					#[codec(index = 15)]
					#[doc = "The asset owner has too many locks on the asset."]
					TooManyLocks,
					#[codec(index = 16)]
					#[doc = "The given account is not an identifiable sovereign account for any location."]
					AccountNotSovereign,
					#[codec(index = 17)]
					#[doc = "The operation required fees to be paid which the initiator could not meet."]
					FeesNotMet,
					#[codec(index = 18)]
					#[doc = "A remote lock with the corresponding data could not be found."]
					LockNotFound,
					#[codec(index = 19)]
					#[doc = "The unlock operation cannot succeed because there are still consumers of the lock."]
					InUse,
					#[codec(index = 20)]
					#[doc = "Invalid non-concrete asset."]
					InvalidAssetNotConcrete,
					#[codec(index = 21)]
					#[doc = "Invalid asset, reserve chain could not be determined for it."]
					InvalidAssetUnknownReserve,
					#[codec(index = 22)]
					#[doc = "Invalid asset, do not support remote asset reserves with different fees reserves."]
					InvalidAssetUnsupportedReserve,
					#[codec(index = 23)]
					#[doc = "Too many assets with different reserve locations have been attempted for transfer."]
					TooManyReserves,
					#[codec(index = 24)]
					#[doc = "Local XCM execution incomplete."]
					LocalExecutionIncomplete,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "The `Event` enum of this pallet"]
				pub enum Event {
					#[codec(index = 0)]
					#[doc = "Execution of an XCM message was attempted."]
					Attempted { outcome: runtime_types::xcm::v3::traits::Outcome },
					#[codec(index = 1)]
					#[doc = "A XCM message was sent."]
					Sent {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						message: runtime_types::xcm::v3::Xcm,
						message_id: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 2)]
					#[doc = "Query response received which does not match a registered query. This may be because a"]
					#[doc = "matching query was never registered, it may be because it is a duplicate response, or"]
					#[doc = "because the query timed out."]
					UnexpectedResponse {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
					},
					#[codec(index = 3)]
					#[doc = "Query response has been received and is ready for taking with `take_response`. There is"]
					#[doc = "no registered notification call."]
					ResponseReady {
						query_id: ::core::primitive::u64,
						response: runtime_types::xcm::v3::Response,
					},
					#[codec(index = 4)]
					#[doc = "Query response has been received and query is removed. The registered notification has"]
					#[doc = "been dispatched and executed successfully."]
					Notified {
						query_id: ::core::primitive::u64,
						pallet_index: ::core::primitive::u8,
						call_index: ::core::primitive::u8,
					},
					#[codec(index = 5)]
					#[doc = "Query response has been received and query is removed. The registered notification"]
					#[doc = "could not be dispatched because the dispatch weight is greater than the maximum weight"]
					#[doc = "originally budgeted by this runtime for the query result."]
					NotifyOverweight {
						query_id: ::core::primitive::u64,
						pallet_index: ::core::primitive::u8,
						call_index: ::core::primitive::u8,
						actual_weight: runtime_types::sp_weights::weight_v2::Weight,
						max_budgeted_weight: runtime_types::sp_weights::weight_v2::Weight,
					},
					#[codec(index = 6)]
					#[doc = "Query response has been received and query is removed. There was a general error with"]
					#[doc = "dispatching the notification call."]
					NotifyDispatchError {
						query_id: ::core::primitive::u64,
						pallet_index: ::core::primitive::u8,
						call_index: ::core::primitive::u8,
					},
					#[codec(index = 7)]
					#[doc = "Query response has been received and query is removed. The dispatch was unable to be"]
					#[doc = "decoded into a `Call`; this might be due to dispatch function having a signature which"]
					#[doc = "is not `(origin, QueryId, Response)`."]
					NotifyDecodeFailed {
						query_id: ::core::primitive::u64,
						pallet_index: ::core::primitive::u8,
						call_index: ::core::primitive::u8,
					},
					#[codec(index = 8)]
					#[doc = "Expected query response has been received but the origin location of the response does"]
					#[doc = "not match that expected. The query remains registered for a later, valid, response to"]
					#[doc = "be received and acted upon."]
					InvalidResponder {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
						expected_location: ::core::option::Option<
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
					#[codec(index = 9)]
					#[doc = "Expected query response has been received but the expected origin location placed in"]
					#[doc = "storage by this runtime previously cannot be decoded. The query remains registered."]
					#[doc = ""]
					#[doc = "This is unexpected (since a location placed in storage in a previously executing"]
					#[doc = "runtime should be readable prior to query timeout) and dangerous since the possibly"]
					#[doc = "valid response will be dropped. Manual governance intervention is probably going to be"]
					#[doc = "needed."]
					InvalidResponderVersion {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
					},
					#[codec(index = 10)]
					#[doc = "Received query response has been read and removed."]
					ResponseTaken { query_id: ::core::primitive::u64 },
					#[codec(index = 11)]
					#[doc = "Some assets have been placed in an asset trap."]
					AssetsTrapped {
						hash: ::subxt::utils::H256,
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						assets: runtime_types::xcm::VersionedMultiAssets,
					},
					#[codec(index = 12)]
					#[doc = "An XCM version change notification message has been attempted to be sent."]
					#[doc = ""]
					#[doc = "The cost of sending it (borne by the chain) is included."]
					VersionChangeNotified {
						destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						result: ::core::primitive::u32,
						cost: runtime_types::xcm::v3::multiasset::MultiAssets,
						message_id: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 13)]
					#[doc = "The supported version of a location has been changed. This might be through an"]
					#[doc = "automatic notification or a manual intervention."]
					SupportedVersionChanged {
						location: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						version: ::core::primitive::u32,
					},
					#[codec(index = 14)]
					#[doc = "A given location which had a version change subscription was dropped owing to an error"]
					#[doc = "sending the notification to it."]
					NotifyTargetSendFail {
						location: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
						error: runtime_types::xcm::v3::traits::Error,
					},
					#[codec(index = 15)]
					#[doc = "A given location which had a version change subscription was dropped owing to an error"]
					#[doc = "migrating the location to our new XCM format."]
					NotifyTargetMigrationFail {
						location: runtime_types::xcm::VersionedMultiLocation,
						query_id: ::core::primitive::u64,
					},
					#[codec(index = 16)]
					#[doc = "Expected query response has been received but the expected querier location placed in"]
					#[doc = "storage by this runtime previously cannot be decoded. The query remains registered."]
					#[doc = ""]
					#[doc = "This is unexpected (since a location placed in storage in a previously executing"]
					#[doc = "runtime should be readable prior to query timeout) and dangerous since the possibly"]
					#[doc = "valid response will be dropped. Manual governance intervention is probably going to be"]
					#[doc = "needed."]
					InvalidQuerierVersion {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
					},
					#[codec(index = 17)]
					#[doc = "Expected query response has been received but the querier location of the response does"]
					#[doc = "not match the expected. The query remains registered for a later, valid, response to"]
					#[doc = "be received and acted upon."]
					InvalidQuerier {
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						query_id: ::core::primitive::u64,
						expected_querier:
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						maybe_actual_querier: ::core::option::Option<
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
					#[codec(index = 18)]
					#[doc = "A remote has requested XCM version change notification from us and we have honored it."]
					#[doc = "A version information message is sent to them and its cost is included."]
					VersionNotifyStarted {
						destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						cost: runtime_types::xcm::v3::multiasset::MultiAssets,
						message_id: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 19)]
					#[doc = "We have requested that a remote chain send us XCM version change notifications."]
					VersionNotifyRequested {
						destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						cost: runtime_types::xcm::v3::multiasset::MultiAssets,
						message_id: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 20)]
					#[doc = "We have requested that a remote chain stops sending us XCM version change"]
					#[doc = "notifications."]
					VersionNotifyUnrequested {
						destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						cost: runtime_types::xcm::v3::multiasset::MultiAssets,
						message_id: [::core::primitive::u8; 32usize],
					},
					#[codec(index = 21)]
					#[doc = "Fees were paid from a location for an operation (often for using `SendXcm`)."]
					FeesPaid {
						paying: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						fees: runtime_types::xcm::v3::multiasset::MultiAssets,
					},
					#[codec(index = 22)]
					#[doc = "Some assets have been claimed from an asset trap"]
					AssetsClaimed {
						hash: ::subxt::utils::H256,
						origin: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						assets: runtime_types::xcm::VersionedMultiAssets,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Origin {
					#[codec(index = 0)]
					Xcm(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
					#[codec(index = 1)]
					Response(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum QueryStatus<_0> {
					#[codec(index = 0)]
					Pending {
						responder: runtime_types::xcm::VersionedMultiLocation,
						maybe_match_querier:
							::core::option::Option<runtime_types::xcm::VersionedMultiLocation>,
						maybe_notify:
							::core::option::Option<(::core::primitive::u8, ::core::primitive::u8)>,
						timeout: _0,
					},
					#[codec(index = 1)]
					VersionNotifier {
						origin: runtime_types::xcm::VersionedMultiLocation,
						is_active: ::core::primitive::bool,
					},
					#[codec(index = 2)]
					Ready { response: runtime_types::xcm::VersionedResponse, at: _0 },
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct RemoteLockedFungibleRecord<_0> {
					pub amount: ::core::primitive::u128,
					pub owner: runtime_types::xcm::VersionedMultiLocation,
					pub locker: runtime_types::xcm::VersionedMultiLocation,
					pub consumers: runtime_types::bounded_collections::bounded_vec::BoundedVec<(
						_0,
						::core::primitive::u128,
					)>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum VersionMigrationStage {
					#[codec(index = 0)]
					MigrateSupportedVersion,
					#[codec(index = 1)]
					MigrateVersionNotifiers,
					#[codec(index = 2)]
					NotifyCurrentTargets(
						::core::option::Option<::std::vec::Vec<::core::primitive::u8>>,
					),
					#[codec(index = 3)]
					MigrateAndNotifyOldTargets,
				}
			}
		}
		pub mod polkadot_core_primitives {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct InboundDownwardMessage<_0> {
				pub sent_at: _0,
				pub msg: ::std::vec::Vec<::core::primitive::u8>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct InboundHrmpMessage<_0> {
				pub sent_at: _0,
				pub data: ::std::vec::Vec<::core::primitive::u8>,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OutboundHrmpMessage<_0> {
				pub recipient: _0,
				pub data: ::std::vec::Vec<::core::primitive::u8>,
			}
		}
		pub mod polkadot_parachain_primitives {
			use super::runtime_types;
			pub mod primitives {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct HeadData(pub ::std::vec::Vec<::core::primitive::u8>);
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Id(pub ::core::primitive::u32);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ValidationCode(pub ::std::vec::Vec<::core::primitive::u8>);
			}
		}
		pub mod polkadot_primitives {
			use super::runtime_types;
			pub mod v6 {
				use super::runtime_types;
				pub mod async_backing {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct AsyncBackingParams {
						pub max_candidate_depth: ::core::primitive::u32,
						pub allowed_ancestry_len: ::core::primitive::u32,
					}
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					pub async_backing_params:
						runtime_types::polkadot_primitives::v6::async_backing::AsyncBackingParams,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct PersistedValidationData<_0, _1> {
					pub parent_head:
						runtime_types::polkadot_parachain_primitives::primitives::HeadData,
					pub relay_parent_number: _1,
					pub relay_parent_storage_root: _0,
					pub max_pov_size: ::core::primitive::u32,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum UpgradeGoAhead {
					#[codec(index = 0)]
					Abort,
					#[codec(index = 1)]
					GoAhead,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum UpgradeRestriction {
					#[codec(index = 0)]
					Present,
				}
			}
		}
		pub mod primitive_types {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct U256(pub [::core::primitive::u64; 4usize]);
		}
		pub mod sp_arithmetic {
			use super::runtime_types;
			pub mod fixed_point {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct FixedU128(pub ::core::primitive::u128);
			}
			pub mod per_things {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: CompactAs,
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Permill(pub ::core::primitive::u32);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Public(pub runtime_types::sp_core::sr25519::Public);
				}
			}
		}
		pub mod sp_consensus_slots {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct Slot(pub ::core::primitive::u64);
			#[derive(
				:: subxt :: ext :: codec :: CompactAs,
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct SlotDuration(pub ::core::primitive::u64);
		}
		pub mod sp_core {
			use super::runtime_types;
			pub mod crypto {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct KeyTypeId(pub [::core::primitive::u8; 4usize]);
			}
			pub mod ecdsa {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 65usize]);
			}
			pub mod ed25519 {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 64usize]);
			}
			pub mod sr25519 {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Public(pub [::core::primitive::u8; 32usize]);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Signature(pub [::core::primitive::u8; 64usize]);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct OpaqueMetadata(pub ::std::vec::Vec<::core::primitive::u8>);
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Void {}
		}
		pub mod sp_inherents {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct CheckInherentsResult {
				pub okay: ::core::primitive::bool,
				pub fatal_error: ::core::primitive::bool,
				pub errors: runtime_types::sp_inherents::InherentData,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct InherentData {
				pub data: ::subxt::utils::KeyedVec<
					[::core::primitive::u8; 8usize],
					::std::vec::Vec<::core::primitive::u8>,
				>,
			}
		}
		pub mod sp_mmr_primitives {
			use super::runtime_types;
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum Error {
				#[codec(index = 0)]
				InvalidNumericOp,
				#[codec(index = 1)]
				Push,
				#[codec(index = 2)]
				GetRoot,
				#[codec(index = 3)]
				Commit,
				#[codec(index = 4)]
				GenerateProof,
				#[codec(index = 5)]
				Verify,
				#[codec(index = 6)]
				LeafNotFound,
				#[codec(index = 7)]
				PalletNotIncluded,
				#[codec(index = 8)]
				InvalidLeafIndex,
				#[codec(index = 9)]
				InvalidBestKnownBlock,
			}
		}
		pub mod sp_runtime {
			use super::runtime_types;
			pub mod generic {
				use super::runtime_types;
				pub mod block {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Block<_0, _1> {
						pub header: _0,
						pub extrinsics: ::std::vec::Vec<_1>,
					}
				}
				pub mod digest {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Digest {
						pub logs:
							::std::vec::Vec<runtime_types::sp_runtime::generic::digest::DigestItem>,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
				pub mod header {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct Header<_0> {
						pub parent_hash: ::subxt::utils::H256,
						#[codec(compact)]
						pub number: _0,
						pub state_root: ::subxt::utils::H256,
						pub extrinsics_root: ::subxt::utils::H256,
						pub digest: runtime_types::sp_runtime::generic::digest::Digest,
					}
				}
			}
			pub mod transaction_validity {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum InvalidTransaction {
					#[codec(index = 0)]
					Call,
					#[codec(index = 1)]
					Payment,
					#[codec(index = 2)]
					Future,
					#[codec(index = 3)]
					Stale,
					#[codec(index = 4)]
					BadProof,
					#[codec(index = 5)]
					AncientBirthBlock,
					#[codec(index = 6)]
					ExhaustsResources,
					#[codec(index = 7)]
					Custom(::core::primitive::u8),
					#[codec(index = 8)]
					BadMandatory,
					#[codec(index = 9)]
					MandatoryValidation,
					#[codec(index = 10)]
					BadSigner,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum TransactionSource {
					#[codec(index = 0)]
					InBlock,
					#[codec(index = 1)]
					Local,
					#[codec(index = 2)]
					External,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum TransactionValidityError {
					#[codec(index = 0)]
					Invalid(runtime_types::sp_runtime::transaction_validity::InvalidTransaction),
					#[codec(index = 1)]
					Unknown(runtime_types::sp_runtime::transaction_validity::UnknownTransaction),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum UnknownTransaction {
					#[codec(index = 0)]
					CannotLookup,
					#[codec(index = 1)]
					NoUnsignedValidator,
					#[codec(index = 2)]
					Custom(::core::primitive::u8),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct ValidTransaction {
					pub priority: ::core::primitive::u64,
					pub requires: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
					pub provides: ::std::vec::Vec<::std::vec::Vec<::core::primitive::u8>>,
					pub longevity: ::core::primitive::u64,
					pub propagate: ::core::primitive::bool,
				}
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
				#[codec(index = 13)]
				RootNotAllowed,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct ModuleError {
				pub index: ::core::primitive::u8,
				pub error: [::core::primitive::u8; 4usize],
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum TokenError {
				#[codec(index = 0)]
				FundsUnavailable,
				#[codec(index = 1)]
				OnlyProvider,
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
				#[codec(index = 7)]
				CannotCreateHold,
				#[codec(index = 8)]
				NotExpendable,
				#[codec(index = 9)]
				Blocked,
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub struct RuntimeDbWeight {
				pub read: ::core::primitive::u64,
				pub write: ::core::primitive::u64,
			}
		}
		pub mod staging_parachain_info {
			use super::runtime_types;
			pub mod pallet {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				#[doc = "Contains a variant per dispatchable extrinsic that this pallet has."]
				pub enum Call {}
			}
		}
		pub mod staging_xcm {
			use super::runtime_types;
			pub mod v3 {
				use super::runtime_types;
				pub mod multilocation {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct MultiLocation {
						pub parents: ::core::primitive::u8,
						pub interior: runtime_types::xcm::v3::junctions::Junctions,
					}
				}
			}
		}
		pub mod xcm {
			use super::runtime_types;
			pub mod double_encoded {
				use super::runtime_types;
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DoubleEncoded {
					pub encoded: ::std::vec::Vec<::core::primitive::u8>,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct DoubleEncoded2 {
					pub encoded: ::std::vec::Vec<::core::primitive::u8>,
				}
			}
			pub mod v2 {
				use super::runtime_types;
				pub mod junction {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum AssetId {
						#[codec(index = 0)]
						Concrete(runtime_types::xcm::v2::multilocation::MultiLocation),
						#[codec(index = 1)]
						Abstract(::std::vec::Vec<::core::primitive::u8>),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum Fungibility {
						#[codec(index = 0)]
						Fungible(#[codec(compact)] ::core::primitive::u128),
						#[codec(index = 1)]
						NonFungible(runtime_types::xcm::v2::multiasset::AssetInstance),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct MultiAsset {
						pub id: runtime_types::xcm::v2::multiasset::AssetId,
						pub fun: runtime_types::xcm::v2::multiasset::Fungibility,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum MultiAssetFilter {
						#[codec(index = 0)]
						Definite(runtime_types::xcm::v2::multiasset::MultiAssets),
						#[codec(index = 1)]
						Wild(runtime_types::xcm::v2::multiasset::WildMultiAsset),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct MultiAssets(
						pub ::std::vec::Vec<runtime_types::xcm::v2::multiasset::MultiAsset>,
					);
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum WildFungibility {
						#[codec(index = 0)]
						Fungible,
						#[codec(index = 1)]
						NonFungible,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Instruction2 {
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
						call: runtime_types::xcm::double_encoded::DoubleEncoded2,
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
					SetErrorHandler(runtime_types::xcm::v2::Xcm2),
					#[codec(index = 22)]
					SetAppendix(runtime_types::xcm::v2::Xcm2),
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum WeightLimit {
					#[codec(index = 0)]
					Unlimited,
					#[codec(index = 1)]
					Limited(#[codec(compact)] ::core::primitive::u64),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Xcm(pub ::std::vec::Vec<runtime_types::xcm::v2::Instruction>);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Xcm2(pub ::std::vec::Vec<runtime_types::xcm::v2::Instruction2>);
			}
			pub mod v3 {
				use super::runtime_types;
				pub mod junction {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						#[codec(index = 10)]
						PolkadotBulletin,
					}
				}
				pub mod junctions {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum AssetId {
						#[codec(index = 0)]
						Concrete(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
						#[codec(index = 1)]
						Abstract([::core::primitive::u8; 32usize]),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum Fungibility {
						#[codec(index = 0)]
						Fungible(#[codec(compact)] ::core::primitive::u128),
						#[codec(index = 1)]
						NonFungible(runtime_types::xcm::v3::multiasset::AssetInstance),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct MultiAsset {
						pub id: runtime_types::xcm::v3::multiasset::AssetId,
						pub fun: runtime_types::xcm::v3::multiasset::Fungibility,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum MultiAssetFilter {
						#[codec(index = 0)]
						Definite(runtime_types::xcm::v3::multiasset::MultiAssets),
						#[codec(index = 1)]
						Wild(runtime_types::xcm::v3::multiasset::WildMultiAsset),
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub struct MultiAssets(
						pub ::std::vec::Vec<runtime_types::xcm::v3::multiasset::MultiAsset>,
					);
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
					#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
					#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
					pub enum WildFungibility {
						#[codec(index = 0)]
						Fungible,
						#[codec(index = 1)]
						NonFungible,
					}
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
				pub mod traits {
					use super::runtime_types;
					#[derive(
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
						:: subxt :: ext :: codec :: Decode,
						:: subxt :: ext :: codec :: Encode,
						:: subxt :: ext :: scale_decode :: DecodeAsType,
						:: subxt :: ext :: scale_encode :: EncodeAsType,
						Clone,
						Debug,
						Eq,
						PartialEq,
					)]
					# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
					#[codec(index = 4)]
					TransferAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssets,
						beneficiary: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 5)]
					TransferReserveAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssets,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						beneficiary: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 14)]
					DepositReserveAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						reserve: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						xcm: runtime_types::xcm::v3::Xcm,
					},
					#[codec(index = 17)]
					InitiateTeleport {
						assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						ticket: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						unlocker: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 40)]
					UnlockAsset {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						target: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 41)]
					NoteUnlockable {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						owner: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 42)]
					RequestUnlock {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						locker: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 43)]
					SetFeesMode { jit_withdraw: ::core::primitive::bool },
					#[codec(index = 44)]
					SetTopic([::core::primitive::u8; 32usize]),
					#[codec(index = 45)]
					ClearTopic,
					#[codec(index = 46)]
					AliasOrigin(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
					#[codec(index = 47)]
					UnpaidExecution {
						weight_limit: runtime_types::xcm::v3::WeightLimit,
						check_origin: ::core::option::Option<
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum Instruction2 {
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
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
					#[codec(index = 4)]
					TransferAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssets,
						beneficiary: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 5)]
					TransferReserveAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssets,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						xcm: runtime_types::xcm::v3::Xcm,
					},
					#[codec(index = 6)]
					Transact {
						origin_kind: runtime_types::xcm::v2::OriginKind,
						require_weight_at_most: runtime_types::sp_weights::weight_v2::Weight,
						call: runtime_types::xcm::double_encoded::DoubleEncoded2,
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
						beneficiary: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 14)]
					DepositReserveAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						reserve: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						xcm: runtime_types::xcm::v3::Xcm,
					},
					#[codec(index = 17)]
					InitiateTeleport {
						assets: runtime_types::xcm::v3::multiasset::MultiAssetFilter,
						dest: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
					SetErrorHandler(runtime_types::xcm::v3::Xcm2),
					#[codec(index = 22)]
					SetAppendix(runtime_types::xcm::v3::Xcm2),
					#[codec(index = 23)]
					ClearError,
					#[codec(index = 24)]
					ClaimAsset {
						assets: runtime_types::xcm::v3::multiasset::MultiAssets,
						ticket: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
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
						unlocker: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 40)]
					UnlockAsset {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						target: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 41)]
					NoteUnlockable {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						owner: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 42)]
					RequestUnlock {
						asset: runtime_types::xcm::v3::multiasset::MultiAsset,
						locker: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					},
					#[codec(index = 43)]
					SetFeesMode { jit_withdraw: ::core::primitive::bool },
					#[codec(index = 44)]
					SetTopic([::core::primitive::u8; 32usize]),
					#[codec(index = 45)]
					ClearTopic,
					#[codec(index = 46)]
					AliasOrigin(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
					#[codec(index = 47)]
					UnpaidExecution {
						weight_limit: runtime_types::xcm::v3::WeightLimit,
						check_origin: ::core::option::Option<
							runtime_types::staging_xcm::v3::multilocation::MultiLocation,
						>,
					},
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct QueryResponseInfo {
					pub destination: runtime_types::staging_xcm::v3::multilocation::MultiLocation,
					#[codec(compact)]
					pub query_id: ::core::primitive::u64,
					pub max_weight: runtime_types::sp_weights::weight_v2::Weight,
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
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
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub enum WeightLimit {
					#[codec(index = 0)]
					Unlimited,
					#[codec(index = 1)]
					Limited(runtime_types::sp_weights::weight_v2::Weight),
				}
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Xcm(pub ::std::vec::Vec<runtime_types::xcm::v3::Instruction>);
				#[derive(
					:: subxt :: ext :: codec :: Decode,
					:: subxt :: ext :: codec :: Encode,
					:: subxt :: ext :: scale_decode :: DecodeAsType,
					:: subxt :: ext :: scale_encode :: EncodeAsType,
					Clone,
					Debug,
					Eq,
					PartialEq,
				)]
				# [codec (crate = :: subxt :: ext :: codec)]
				#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
				#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
				pub struct Xcm2(pub ::std::vec::Vec<runtime_types::xcm::v3::Instruction2>);
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedAssetId {
				#[codec(index = 3)]
				V3(runtime_types::xcm::v3::multiasset::AssetId),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedMultiAssets {
				#[codec(index = 1)]
				V2(runtime_types::xcm::v2::multiasset::MultiAssets),
				#[codec(index = 3)]
				V3(runtime_types::xcm::v3::multiasset::MultiAssets),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedMultiLocation {
				#[codec(index = 1)]
				V2(runtime_types::xcm::v2::multilocation::MultiLocation),
				#[codec(index = 3)]
				V3(runtime_types::staging_xcm::v3::multilocation::MultiLocation),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedResponse {
				#[codec(index = 2)]
				V2(runtime_types::xcm::v2::Response),
				#[codec(index = 3)]
				V3(runtime_types::xcm::v3::Response),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedXcm {
				#[codec(index = 2)]
				V2(runtime_types::xcm::v2::Xcm),
				#[codec(index = 3)]
				V3(runtime_types::xcm::v3::Xcm),
			}
			#[derive(
				:: subxt :: ext :: codec :: Decode,
				:: subxt :: ext :: codec :: Encode,
				:: subxt :: ext :: scale_decode :: DecodeAsType,
				:: subxt :: ext :: scale_encode :: EncodeAsType,
				Clone,
				Debug,
				Eq,
				PartialEq,
			)]
			# [codec (crate = :: subxt :: ext :: codec)]
			#[decode_as_type(crate_path = ":: subxt :: ext :: scale_decode")]
			#[encode_as_type(crate_path = ":: subxt :: ext :: scale_encode")]
			pub enum VersionedXcm2 {
				#[codec(index = 2)]
				V2(runtime_types::xcm::v2::Xcm2),
				#[codec(index = 3)]
				V3(runtime_types::xcm::v3::Xcm2),
			}
		}
	}
}
