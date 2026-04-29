pub mod handle_consensus;
pub mod handle_post_requests;
pub mod initialize_handler;

// Glob re-exports needed by Anchor's `#[program]` macro for the
// auto-generated `__client_accounts_*` modules. Each handler is
// `pub(crate)` so the glob doesn't reach it.
pub use handle_consensus::*;
pub use handle_post_requests::*;
pub use initialize_handler::*;
