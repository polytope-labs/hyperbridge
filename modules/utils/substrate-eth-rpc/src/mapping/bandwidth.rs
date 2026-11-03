use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

const TIER_INDEX: &[(&str, u8)] =
	&[("TierOne", 1), ("TierTwo", 2), ("TierThree", 3), ("TierFour", 4)];

pub fn spec() -> PalletSpec {
	PalletSpec {
		key: "bandwidth",
		metadata_name: "Bandwidth",
		events: vec![
			EventSpec::new(
				"BandwidthCredited",
				vec![
					FieldSpec::new("appChain", "app_chain", AbiType::StateMachine),
					FieldSpec::new("app", "app", AbiType::Bytes),
					FieldSpec::new("paidFrom", "paid_from", AbiType::StateMachine),
					FieldSpec::new("tier", "tier", AbiType::Enum(TIER_INDEX)),
					FieldSpec::new("byteCount", "bytes", AbiType::Uint(128)),
					FieldSpec::new("expiresAt", "expires_at", AbiType::Uint(64)),
				],
			),
			EventSpec::new(
				"BandwidthConsumed",
				vec![
					FieldSpec::new("source", "source", AbiType::StateMachine),
					FieldSpec::new("app", "app", AbiType::Bytes),
					FieldSpec::new("byteCount", "bytes", AbiType::Uint(128)),
					FieldSpec::new("remaining", "remaining", AbiType::Uint(128)),
				],
			),
			EventSpec::new(
				"SubscriptionEvicted",
				vec![
					FieldSpec::new("appChain", "app_chain", AbiType::StateMachine),
					FieldSpec::new("app", "app", AbiType::Bytes),
					FieldSpec::new("tier", "tier", AbiType::Enum(TIER_INDEX)),
					FieldSpec::new("lostBytes", "lost_bytes", AbiType::Uint(128)),
				],
			),
			EventSpec::new(
				"ForceCredited",
				vec![
					FieldSpec::new("appChain", "app_chain", AbiType::StateMachine),
					FieldSpec::new("app", "app", AbiType::Bytes),
					FieldSpec::new("tier", "tier", AbiType::Enum(TIER_INDEX)),
					FieldSpec::new("byteCount", "bytes", AbiType::Uint(128)),
					FieldSpec::new("expiresAt", "expires_at", AbiType::Uint(64)),
				],
			),
			EventSpec::new(
				"TierSet",
				vec![
					FieldSpec::new("tier", "tier", AbiType::Enum(TIER_INDEX)),
					FieldSpec::new(
						"config",
						"config",
						AbiType::Option(Box::new(AbiType::Tuple(vec![
							FieldSpec::new("byteCount", "bytes", AbiType::Uint(128)),
							FieldSpec::new("durationSecs", "duration_secs", AbiType::Uint(64)),
						]))),
					),
				],
			),
		],
	}
}
