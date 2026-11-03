//! Solver discovery, shared by the intent-gateway fill handlers and the solver watchlist.

use tokio_postgres::Transaction;

use crate::{
	config::ChainConfig,
	store::{numeric, timestamp_secs, SolverDiscoveryTrigger, TrackedSolverStatus},
};

pub struct Discovery<'a> {
	pub by: SolverDiscoveryTrigger,
	/** The fill's transaction hash, lowercase, for `FILL`. */
	pub fill_transaction: Option<&'a str>,
	pub block_number: u64,
	/** Block time in seconds. */
	pub block_timestamp: u64,
}

/**
 * `queueSolver`: tracks `solver` on `chain` (state machine id) as `PENDING` so the inventory
 * hook seeds it. The first discovery wins.
 */
pub async fn queue_solver(
	db: &Transaction<'_>,
	chain: &str,
	solver: &str,
	discovery: Discovery<'_>,
) -> anyhow::Result<()> {
	let solver = solver.to_lowercase();
	db.execute(
		"INSERT INTO app.tracked_solvers
		   (id, chain, solver, status, discovered_by, discovered_by_fill, discovery_block, discovered_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		 ON CONFLICT (id) DO NOTHING",
		&[
			&format!("{chain}-{solver}"),
			&chain,
			&solver,
			&TrackedSolverStatus::Pending,
			&discovery.by,
			&discovery.fill_transaction,
			&numeric(discovery.block_number),
			&timestamp_secs(discovery.block_timestamp),
		],
	)
	.await?;
	Ok(())
}

/** `discoverSolverFromFill`: a filler becomes a tracked solver on chains with supported tokens. */
pub async fn discover_solver_from_fill(
	db: &Transaction<'_>,
	chain: &ChainConfig,
	filler: &str,
	transaction_hash: &str,
	block_number: u64,
	block_timestamp: u64,
) -> anyhow::Result<()> {
	if chain.contracts.yield_vaults.is_empty() {
		return Ok(());
	}
	let fill_transaction = transaction_hash.to_lowercase();
	queue_solver(
		db,
		&chain.state_machine_id,
		filler,
		Discovery {
			by: SolverDiscoveryTrigger::Fill,
			fill_transaction: Some(&fill_transaction),
			block_number,
			block_timestamp,
		},
	)
	.await
}
