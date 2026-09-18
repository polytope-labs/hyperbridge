use ssz_types::typenum::Unsigned;
use sync_committee_primitives::beacon_state::{BeaconState, BeaconStateElectra, BeaconStateGloas};

/// A beacon state response, deserialized into whichever shape the node says it sent.
///
/// The beacon api answers `/eth/v2/debug/beacon/states/{id}` with `{"version": "...", "data":
/// {...}}`, which is exactly serde's adjacently tagged enum. So the fork is read off the wire
/// rather than chosen when the binary was built, and one relayer serves both sides of Gloas.
///
/// Every pre-Gloas version maps to the same shape: the state layout this client cares about last
/// changed at Electra, and Fulu did not move it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version", content = "data", rename_all = "lowercase")]
#[serde(bound = "")]
pub enum Response<ETH1_DATA_VOTES_BOUND: Unsigned, PROPOSER_LOOK_AHEAD_LIMIT: Unsigned> {
	#[serde(
		alias = "phase0",
		alias = "altair",
		alias = "bellatrix",
		alias = "capella",
		alias = "deneb",
		alias = "fulu"
	)]
	Electra(
		BeaconStateElectra<
			sync_committee_primitives::constants::SLOTS_PER_HISTORICAL_ROOT,
			sync_committee_primitives::constants::HISTORICAL_ROOTS_LIMIT,
			ETH1_DATA_VOTES_BOUND,
			sync_committee_primitives::constants::VALIDATOR_REGISTRY_LIMIT,
			sync_committee_primitives::constants::EPOCHS_PER_HISTORICAL_VECTOR,
			sync_committee_primitives::constants::EPOCHS_PER_SLASHINGS_VECTOR,
			sync_committee_primitives::constants::SYNC_COMMITTEE_SIZE,
			sync_committee_primitives::constants::BYTES_PER_LOGS_BLOOM,
			sync_committee_primitives::constants::MAX_EXTRA_DATA_BYTES,
			sync_committee_primitives::constants::PENDING_DEPOSITS_LIMIT,
			sync_committee_primitives::constants::PENDING_CONSOLIDATIONS_LIMIT,
			sync_committee_primitives::constants::PENDING_PARTIAL_WITHDRAWALS_LIMIT,
			PROPOSER_LOOK_AHEAD_LIMIT,
		>,
	),
	Gloas(
		BeaconStateGloas<
			sync_committee_primitives::constants::SLOTS_PER_HISTORICAL_ROOT,
			sync_committee_primitives::constants::HISTORICAL_ROOTS_LIMIT,
			ETH1_DATA_VOTES_BOUND,
			sync_committee_primitives::constants::EPOCHS_PER_HISTORICAL_VECTOR,
			sync_committee_primitives::constants::EPOCHS_PER_SLASHINGS_VECTOR,
			sync_committee_primitives::constants::SYNC_COMMITTEE_SIZE,
			PROPOSER_LOOK_AHEAD_LIMIT,
		>,
	),
}

impl<ETH1_DATA_VOTES_BOUND: Unsigned, PROPOSER_LOOK_AHEAD_LIMIT: Unsigned>
	Response<ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>
{
	/// The state, tagged with the fork the node reported.
	pub fn into_state(
		self,
	) -> BeaconState<
		sync_committee_primitives::constants::SLOTS_PER_HISTORICAL_ROOT,
		sync_committee_primitives::constants::HISTORICAL_ROOTS_LIMIT,
		ETH1_DATA_VOTES_BOUND,
		sync_committee_primitives::constants::VALIDATOR_REGISTRY_LIMIT,
		sync_committee_primitives::constants::EPOCHS_PER_HISTORICAL_VECTOR,
		sync_committee_primitives::constants::EPOCHS_PER_SLASHINGS_VECTOR,
		sync_committee_primitives::constants::SYNC_COMMITTEE_SIZE,
		sync_committee_primitives::constants::BYTES_PER_LOGS_BLOOM,
		sync_committee_primitives::constants::MAX_EXTRA_DATA_BYTES,
		sync_committee_primitives::constants::PENDING_DEPOSITS_LIMIT,
		sync_committee_primitives::constants::PENDING_CONSOLIDATIONS_LIMIT,
		sync_committee_primitives::constants::PENDING_PARTIAL_WITHDRAWALS_LIMIT,
		PROPOSER_LOOK_AHEAD_LIMIT,
	> {
		match self {
			Response::Electra(state) => BeaconState::Electra(state),
			Response::Gloas(state) => BeaconState::Gloas(state),
		}
	}
}
