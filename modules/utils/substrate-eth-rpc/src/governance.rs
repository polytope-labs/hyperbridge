//! The `governance` logs: every event of the pallets OpenGov, the technical committee and the
//! treasury are made of, built from the same decode the pallet logs come from.

use subxt_core::Metadata;

use crate::{
	abi::encode_values,
	explorer::event_values,
	json::Encoder,
	mapping::{explorer, governance, MappedLog},
	types::{pallet_address, DecodedEvent, Phase},
};

/** The pallets whose events a governance forum follows. */
const PALLETS: &[&str] = &[
	"Referenda",
	"ConvictionVoting",
	"Scheduler",
	"TechnicalCollective",
	"Whitelist",
	"Preimage",
	"Treasury",
];

/** An `Event` log for each event of a governance pallet, with the phase it belongs to. */
pub fn logs(metadata: &Metadata, events: &[DecodedEvent]) -> Vec<(MappedLog, Phase)> {
	let encoder = Encoder::ss58(metadata);
	let address = pallet_address(governance::KEY);
	let topic = explorer::event().topic0();
	events
		.iter()
		.filter(|event| PALLETS.contains(&event.pallet.as_str()))
		.map(|event| {
			let log = MappedLog {
				address,
				topics: vec![topic],
				data: encode_values(event_values(&encoder, event)),
			};
			(log, event.phase.clone())
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use codec::{Compact, Encode};
	use sp_crypto_hashing::{twox_128, twox_64};

	use crate::{
		blocks::storage_type,
		decode::{decode_metadata, testdata::NEXUS},
		json::{ss58, Encoder},
	};

	const ALICE: [u8; 32] = [7; 32];

	fn voting_for(account: &[u8; 32], track: u16) -> Vec<u8> {
		[
			&twox_128(b"ConvictionVoting")[..],
			&twox_128(b"VotingFor"),
			&twox_64(account),
			account,
			&twox_64(&track.encode()),
			&track.encode(),
		]
		.concat()
	}

	#[test]
	fn storage_decodes_by_its_entry_with_accounts_in_ss58() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);
		let ty = storage_type(&metadata, &voting_for(&ALICE, 0)).unwrap();

		// Casting { votes: [(3, Standard { aye with 2x conviction, balance })], delegations, prior
		// }
		let casting = (
			0u8,
			vec![(3u32, 0u8, 0x82u8, 5_000_000_000_000_000_000u128)],
			7u128,
			8u128,
			9u32,
			10u128,
		);
		let casting = encoder.decode(ty, &casting.encode()).unwrap();
		assert_eq!(
			casting["casting"]["votes"][0],
			serde_json::json!([3, { "standard": { "vote": 0x82, "balance": "5000000000000000000" } }])
		);
		assert_eq!(casting["casting"]["delegations"]["votes"], 7);

		let delegating = (1u8, 11u128, ALICE, 3u8, 7u128, 8u128, 9u32, 10u128);
		let delegating = encoder.decode(ty, &delegating.encode()).unwrap();
		assert_eq!(delegating["delegating"]["target"], ss58(&ALICE, 0));
		assert_eq!(delegating["delegating"]["conviction"], "Locked3x");

		let unknown = [twox_128(b"Nothing"), twox_128(b"Here")].concat();
		assert_eq!(storage_type(&metadata, &unknown), None);
	}

	#[test]
	fn calls_and_constants_decode_with_the_runtime() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);

		let balances = metadata.pallet_by_name("Balances").unwrap();
		let transfer = balances.call_variant_by_name("transfer_keep_alive").unwrap();
		let call = (balances.index(), transfer.index, 0u8, ALICE, Compact(5u128)).encode();
		let call = encoder.decode(metadata.outer_enums().call_enum_ty(), &call).unwrap();
		assert_eq!(
			(call["section"].as_str(), call["method"].as_str()),
			(Some("balances"), Some("transferKeepAlive"))
		);
		assert_eq!(call["args"][0]["value"]["id"], ss58(&ALICE, 0));
		assert_eq!(call["args"][1]["value"], 5);
		assert_eq!(encoder.decode(metadata.outer_enums().call_enum_ty(), &[0xff, 0xff]), None);

		let tracks = metadata
			.pallet_by_name("Referenda")
			.unwrap()
			.constant_by_name("Tracks")
			.unwrap();
		let tracks = encoder.decode(tracks.ty(), tracks.value()).unwrap();
		let root = &tracks[0];
		assert_eq!(root[0], 0);
		assert!(root[1]["name"].as_str().unwrap().starts_with("root"));
		assert_eq!(root[1]["decisionDeposit"], "1000000000000000000");
		assert!(root[1]["minSupport"]["reciprocal"]["factor"].is_u64());
	}
}
