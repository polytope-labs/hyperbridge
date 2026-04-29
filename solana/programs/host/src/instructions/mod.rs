// Admin / governance
pub mod initialize_host;
pub mod set_consensus_state;
pub mod set_frozen_state;
pub mod set_handler;
pub mod update_host_params;
pub mod veto_state_commitment;

// Handler-gated state primitives (only the configured handler program's
// `[b"handler_authority"]` PDA can satisfy the signer constraint).
pub mod dispatch_incoming;
pub mod store_consensus_state;
pub mod store_state_commitment;

// Permissionless ops
pub mod close_expired_receipt;
pub mod withdraw_fees;

// Glob re-exports for Anchor's `#[program]` macro: it expects each
// instruction module's auto-generated `__client_accounts_*` to be
// reachable at `crate::*`. Each module's `handler` is `pub(crate)` so it
// doesn't get pulled in here, avoiding ambiguous-glob warnings.
pub use close_expired_receipt::*;
pub use dispatch_incoming::*;
pub use initialize_host::*;
pub use set_consensus_state::*;
pub use set_frozen_state::*;
pub use set_handler::*;
pub use store_consensus_state::*;
pub use store_state_commitment::*;
pub use update_host_params::*;
pub use veto_state_commitment::*;
pub use withdraw_fees::*;
