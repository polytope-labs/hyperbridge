// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::EnsureOrigin;
use frame_system::RawOrigin;
use polkadot_sdk::*;

#[benchmarks(
	where
	T::AdminOrigin: EnsureOrigin<T::RuntimeOrigin>
)]
mod benchmarks {
	use super::*;
	use cumulus_primitives_core::{relay_chain::HeadData, PersistedValidationData};
	use ismp::messaging::ConsensusMessage;
	use primitive_types::H256;

	/// Benchmark for add_parachain extrinsic
	/// The benchmark creates n parachains and measures the time to add them
	/// to the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of parachains to add in a single call
	#[benchmark]
	fn add_parachain(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let parachains: Vec<ParachainData> =
			(0..n).map(|i| ParachainData { id: i, slot_duration: 6000u64 }).collect();

		#[block]
		{
			Pallet::<T>::add_parachain(origin, parachains)?;
		}

		Ok(())
	}

	/// Benchmark for remove_parachain extrinsic
	/// The benchmark first adds n parachains, then measures the time to remove them
	/// from the whitelist.
	///
	/// Parameters:
	/// - `n`: Number of parachains to remove in a single call
	#[benchmark]
	fn remove_parachain(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
		let origin =
			T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let parachains: Vec<ParachainData> =
			(0..n).map(|i| ParachainData { id: i, slot_duration: 6000u64 }).collect();

		Pallet::<T>::add_parachain(RawOrigin::Root.into(), parachains)?;

		#[block]
		{
			Pallet::<T>::remove_parachain(origin, vec![0, 1, 2, 3, 4])?;
		}

		Ok(())
	}

	/// Benchmark for update_parachain_consensus extrinsic
	/// The benchmark first inserts a Parachain, then sets the ValidationData,
	/// afterward proceed to update the parachain consensus.
	#[benchmark]
	fn update_parachain_consensus() -> Result<(), BenchmarkError> {
		let host = T::IsmpHost::default();

		let host_state_machine = host.host_state_machine();
		let consensus_message = ConsensusMessage {
			consensus_proof: hex::decode("825343001cb036004131e62a7368ee07000037803937450aba30ede02c0df7ca997ec9000e89fb68bd32e01097975eadc3ff55054d05a562fce78ebc2a99bdc809124eea7a220517e57ac517a5dc804e251d785c3c51ee293a00638b15d26969a945491f035648a811ef6d6c1d84864891b8ad8990c89f8aa27b7c11fbb62a93db8c10b4f79b5bcb838330340bd1e92342247c8c6f02ec74449314066175726120fe4f9e0800000000045250535290db6f2927773c5e1103907d99844d0626453a5b2987a4e31d5a7dec0c015cb545024e0d010466726f6e8801640c2728044dbb747004ccd03ca02b6e8bef4fa0bde33aea8bbe90716858134f000449534d5001010000000000000000000000000000000000000000000000000000000000000000471985d4945bc6c8f3e23fd146e1d24170624024afa2f37f0201a113c8ed76c705617572610101008e484c50e614d075f0972602a9c96145d33b480c161d1ce79fae577d27ac6647d4f6dae51b6c1485ce68f52f6d29fb0196d2269ea45989af1236d85192f98a250380029d80224a52f114ab1a88145a96d316f019bd92042c383a6683ba8c65fff6152dfe20807016a756665964a1c18722c71dc2a88b61a2170e370bede79faf3aeae794b06980d4860999463a8d9e4e1f544d10bf596569efff6f7065dcf0de0cb72be6032fc880828a9f395e1bb0f79aaab17e6f610b3c235b1a7e466505d82164ab2581b36d3e80739c15f45dcbf244ae9ff6f4916222923564c1488582b421ac88ebc137734d91803fe7f37b6bc215f7b755b0b62237e926dd1e35cacb43c78adc762b0dd88d797e250380546480805b68d75c109f29b589a2e933d3add750b832ab14118b0a47172a1295a00d028063fa3443f4e8b71a14886166ad03f5747914d180a4f9766ef2f500531e14c60880911e015f455657fe02ab564bf226adfe0e0e93969e95c8f397eb78b2b950d2e380f6f6801e4b41e2e6d8ec194dba122bfb9eb33feb2545ef5144cea79551f7cc528072ef2653224e6f7bf59a1181469f9d02f2ec2c34501b2e28c368f43e3b669b8880faebd742d9a812ad819a7711c13059b1f2935e1205a4be45dc1a1c4615b493a54d0880ffff80ef834bee8b63ba8e8421c76f0763441b585cd8fe6bfa585a4043b811be43dd4e8008a3ea5a26aae30824f158c904d9684299b656d5a7d53cf5f15ccd5ca9b9c1c48044be9dff9da3546926d6e0b20f98ec609bc946970d7de805d6f3303db479957e806e6c423ca9c6f1e78b810f8c099d8aa2f7baea14b7b369621fdb1932bf88807e80d2c5f53b89e4e7a674312d0748a86d57eb58b67fdf9dedf9ae9f5c33171aa0d880bf6de02e054df87c04079fef2ff587704ce08c1b500a4b5679932b4d28f8db6d8034f3837bf803d3e07f7b03050cc2253cb9098c591377380eae894dcabec596c680928c7aeef0329500c4d687a62ae81057c17275e80b557c71634915348cb1caa080d9abbab39a3f73d9c44bd6b56b120485807306537c2df7f2e3a8401d5ca5c32180e2134710da0547dc7680141c08754dbc001fcbaec563eac6db500f24da61691f80f8917b0e3e0e427dbe722050be4ce1ad98c4aa04aa4a2e3f73550e5c661d72da807e664e0a08c5369a06b24c3039af5e0eed4c37cb070f180012a1751e9d10a71480e0f820145eaa4a95faad6f3b5394865e986d0a3b64b2898c2b28516cb1fa9f5780bf56c7d6af42a099a109d994cb94c75b948f631ce3033d163864a8ec93f8773580bf479f4f4ac201e91cceb1b80befce19798a70ed572e5f4c40129120b4e45a72801251547e9228098a010647a0b22e580236a9a7ec52d96190f1e0b97956ef027fd9049e710b30bd2eab0352ddcc26417aa1945fc380df23617fe84624ee3a237f090565501628c634ab465cdaf2cebe390e88949f578005b66a1717fa59769d977e6d1e82c7cd39ccf06601a95c477ce3a98b0cce657580dffa5030e4d00bbb5ec8e94fd84c1f849e1d747a7c0b1cee3e4594c20e3ebf358039183678fbf0917679e88304ea4d576455847f85ba3019b62146a76fdcded5c4505f0e7b9012096b41c4eb3aaf947f6ea4290800004c5f03c716fb8fff3de61a883bb76adb34a2040080ffabf0036468f09381d8388a0a228b5a20ef364292d8a08f6563ef0df5aceb8b4c5f0f4993f016e2d2f8e5f43be7bb259486040080bcbcd3e31c129afddb25dc7c6b6fa05ab9456897d4bff9740db5b54dab7847dc8084e4102db06857917315d395e1bba01496489df5c65ef1144a08081e60fab0488d089f0b3c252fcb29d88eff4f3de5de4476c3ffff80889ed5e0c08c9d3c179bc7a389c4a06f884d1b11255720fc99408feef530992f80c8c97dd0a133cae1cc139da9fad4b7608eec2a285615b54df938314ed2d61519809399f4484a565428bd2e157c4b50bbe2b8062a6bed251ecd9075be2fae13014f80cf1ad2da790e25108a23dce568bf85b07024c1d6a0cd9901aca2ae54d7857fed8067b4da94af41d1439b7c82bfe2626fa2f175e6bcb936dca137891e623cd3083680eabd62d710962690f51ddeac0845b0debf5b047033ebdaf61ad9c8bc1a78d7e68023ae4ac98f3e4e4da887153440a36a28c864e2e727a77bec702fa50a1b00637a8065a82c283839f68d533a3dc5a77fe4b1dcaab35cfa8c68c68bcc788a43b3e4f280f8594eec587bfef5b35e96e5b05d46edda21d499dcb0659326335fd57d8fc8d58012035bd14aa2c5ffd0b1baa28b515884808b2cd73aa97fc7075d8154596fe432801128f943a07fd2692adfa15df9b9fb4c374161b4b9da11d0c7a0851f524794aa80de0c1d5a68a34f6d6ae6f03effd9a761dfef83c97c2b50bc7b6130734fa36aa58061ae531244939ee9211cf6b24156cd2ae4f6f3324b025eefd8d55aae423035f8809f2ff8e9f7a13866525d3de985d1bc1646a798e05314f0a0d9bf414cf0eaa963807e5f808ca32a79d5dfbc58970fe6720ece7f3f41d62f69ec929175c8f70a7f088030ffbafb038ce795acca0ed1eb852161a7a8ed46113f443f7501532a39572642").expect("Decoding failed"),
			consensus_state_id: parachain_consensus_state_id(host_state_machine),
			signer: vec![],
		};

		Parachains::<T>::insert(2030, 12);

		let parent_head = HeadData::from(hex::decode("b4aee1453eb98d6e40d8754ebca2e552993c2d1e9965e55b524d82df5e1b16d1ce2bb8004f5573167fd4535931d9466a2109ae9c91d5a41dee393e331175dd40bdf786a9b837e8a44d492e771362234ea009e55e750f8d23c4d6381e8ba613c95045ba7d10066175726120fd9f3c110000000004525053529014935157381177eb2f0751116bebf1fb03f89bc461b7d1918780c05ed8cc2314064e0d010449534d500101f3a6810d97b4d483145e5be8f80bb5ae9a7d51c46a433c268b818489e0215ac0dd0376457139ce8bfb5346339bc162037f71eae7bcfb940024b7393387469bf0056175726101011227ed222d82e6374f3db9f40cc05875c4cca770021b64e1c051c30d1f2fc672676c57349e617f85c3de816d780ad509049cb876a32ca013eb2949882a501c8b").expect("Invalid parent head"));

		let validation_data = PersistedValidationData {
			parent_head,
			relay_parent_number: 4412290,
			max_pov_size: 5242880,
			relay_parent_storage_root: H256::from_slice(
				&hex::decode("70af557e788862e2cb91eb6a79e828e8b067ab02082ef72e1dcec55972111f7e")
					.expect("Invalid root"),
			),
		};

		cumulus_pallet_parachain_system::ValidationData::<T>::put(validation_data);

		#[block]
		{
			Pallet::<T>::update_parachain_consensus(RawOrigin::None.into(), consensus_message)?;
		}

		Ok(())
	}
}
