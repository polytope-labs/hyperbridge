use ismp::host::StateMachine;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

/// Message type to the telemetry server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
	/// ecdsa signature of the metadata
	pub signature: Vec<u8>,
	/// metadata about the supported state machine and their signer
	pub metadata: Vec<(StateMachine, H160)>,
}
