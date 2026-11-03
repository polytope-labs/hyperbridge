//! Every handler, grouped by the area of the TypeScript indexer it was ported from.

pub mod abi;
pub mod ismp;
pub mod pallets;
pub mod solver;

use std::sync::Arc;

use crate::{
	registry::{BlockHook, Subscription},
	Indexer,
};

/** What a handler area contributes. */
#[derive(Default)]
pub struct Handlers {
	pub subscriptions: Vec<Subscription>,
	pub hooks: Vec<Arc<dyn BlockHook>>,
}

/** Every event and block hook the indexer runs. */
pub fn all() -> Handlers {
	let mut handlers = Handlers::default();
	ismp::register(&mut handlers);
	pallets::register(&mut handlers);
	solver::register(&mut handlers);
	handlers
}

/** Background work that isn't tied to a chain's blocks, e.g. polling an HTTP endpoint. */
pub fn spawn_tasks(indexer: &Arc<Indexer>) {
	crate::services::pending::spawn_flusher(indexer.clone());
	solver::spawn_tasks(indexer);
}
