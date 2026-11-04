//! `ProtocolRevenueService`: dust the gateway collects and sweeps, per chain and token. Dust
//! isn't valued. SubQuery priced it with Uniswap quotes, which run in pools anyone can create,
//! and what reads the dust prices the raw amounts itself.

use alloy::primitives::{Address, U256};
use tokio_postgres::Transaction;

use super::orders::lower_hex;
use crate::{
	chains::Chain,
	errors::is_transient,
	store::{numeric, timestamp_to_date},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dust {
	Collected,
	Swept,
}

impl Dust {
	fn table(&self) -> &'static str {
		match self {
			Dust::Collected => "protocol_dust_collecteds",
			Dust::Swept => "protocol_dust_swepts",
		}
	}
}

/**
 * `recordDustCollected` / `recordDustSwept`: adds `amount` to `<chain>-<token>`. The symbol is
 * only written with the row's first amount.
 */
pub async fn record_dust(
	db: &Transaction<'_>,
	dust: Dust,
	state_machine: &str,
	token: Address,
	amount: U256,
	timestamp: u64,
	symbol: &str,
) -> anyhow::Result<()> {
	let table = dust.table();
	db.execute(
		&format!(
			"INSERT INTO app.{table} (id, chain, token_symbol, amount, last_updated) VALUES ($1, $2, $3, $4, $5)
			 ON CONFLICT (id) DO UPDATE SET amount = {table}.amount + EXCLUDED.amount,
			   last_updated = EXCLUDED.last_updated"
		),
		&[
			&format!("{state_machine}-{}", lower_hex(token)),
			&state_machine,
			&symbol,
			&numeric(amount),
			&timestamp_to_date(timestamp),
		],
	)
	.await?;
	Ok(())
}

/**
 * The token's symbol for dust: `eth` for the native token, and `UNKNOWN` when its `symbol()` or
 * `decimals()` fails for good. A transient failure is an error, so the batch retries.
 */
pub async fn dust_symbol(chain: &Chain, token: Address, block: u64) -> anyhow::Result<String> {
	if token == Address::ZERO {
		return Ok("eth".into());
	}
	match crate::services::prices::token_metadata(chain, token, block).await {
		Ok((symbol, _)) => Ok(symbol),
		Err(err) if is_transient(&err) => Err(err),
		Err(err) => {
			tracing::warn!("dust token {token} metadata: {err:#}");
			Ok("UNKNOWN".into())
		},
	}
}
