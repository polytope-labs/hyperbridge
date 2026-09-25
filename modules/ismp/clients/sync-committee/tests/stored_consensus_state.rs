//! Decode a `ConsensusState` taken verbatim from the live chain.
//!
//! `Root` changed representation during the ssz migration, and its SCALE encoding is what every
//! stored consensus state and every relayer already speaks. A unit test can show the new type
//! encodes to 32 raw bytes; only real stored bytes show that what is on chain still decodes.
//!
//! The fixture is the raw `ETH0` entry of `Ismp::ConsensusStates` on Gargantua, fetched with
//! `state_getStorage`. The pallet stores consensus states as opaque `Vec<u8>`, so the value on the
//! wire is a SCALE compact length followed by the encoded `ConsensusState`.

use codec::{Decode, Encode};
use ismp_sync_committee::types::ConsensusState;

const STORED: &[u8] = include_bytes!("fixtures/eth0_consensus_state.bin");

#[test]
fn the_live_ethereum_consensus_state_still_decodes() {
	// Unwrap the pallet's `Vec<u8>` first, then read the consensus state out of it.
	let inner =
		<Vec<u8>>::decode(&mut &STORED[..]).expect("the storage value is an opaque byte vector");

	let state = ConsensusState::decode(&mut &inner[..])
		.expect("a consensus state written by the current runtime must still decode");

	// The three roots that would have broken had `Root` gained a length prefix.
	let header = &state.light_client_state.finalized_header;
	assert_eq!(header.state_root.as_ref().len(), 32);
	assert_eq!(header.parent_root.as_ref().len(), 32);
	assert_eq!(header.body_root.as_ref().len(), 32);

	// Re-encoding must reproduce the stored bytes exactly, or a later write would corrupt storage.
	assert_eq!(state.encode(), inner, "re-encoding changed the stored bytes");
}
