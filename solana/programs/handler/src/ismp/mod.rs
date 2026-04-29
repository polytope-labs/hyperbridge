//! Solana-specific impls for the `ismp-core` trait surface.

pub mod consensus_client;
pub mod host_facade;
pub mod router;
pub mod state_machine_client;

pub use consensus_client::Sp1BeefyConsensusClient;
pub use host_facade::{CommitmentSnapshot, SolanaHostFacade, SOLANA_STATE_MACHINE};
pub use router::{SolanaCpiModule, SolanaRouter};
pub use state_machine_client::SubstrateStateMachineClient;
