use core::fmt::{Display, Formatter};
use light_client_primitives::types::{LightClientState, LightClientUpdate};

#[derive(Debug)]
pub enum Error {
    SyncCommitteeParticiapntsTooLow,
    InvalidUpdate,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SyncCommitteeParticiapntsTooLow => {
                write!(f, "Sync committee participants are too low")
            }
            Error::InvalidUpdate => write!(f, "Invalid update"),
        }
    }
}

struct EthLightClient {}

impl EthLightClient {
    pub fn verify_sync_committee_attestation<const SYNC_COMMITTEE_SIZE: usize>(
        trusted_state: LightClientState<SYNC_COMMITTEE_SIZE>,
        update: LightClientUpdate<SYNC_COMMITTEE_SIZE>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
