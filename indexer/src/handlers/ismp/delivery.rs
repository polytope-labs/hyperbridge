//! What a dispatch or delivery transaction adds beyond the request's status: relayer delivery
//! stats (`HyperBridgeService.handleRequestHandledEvent`) and the "transfer sweep" every host
//! handler ran over the transaction's ERC-20 transfers.

use alloy::{
	primitives::{Address, Bytes, B256, U256},
	providers::Provider,
};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::OnceCell;
use tokio_postgres::Transaction;

use super::{
	calldata::{handler_message_modules, MessageFunction},
	Retry,
};
use crate::{
	registry::EventContext,
	services::{
		decimal::amount_value_in_usd,
		points::{award_points, Award},
		prices::{native_currency_price, token_metadata, token_price},
		relayers::{
			add_chain_stats, add_relayer_stats, touch_relayer_activity, ChainStatsDelta,
			RelayerStatsDelta,
		},
		volume::update_volume,
	},
	store::{numeric, PointsActivityType, ProtocolParticipantType},
};

/** `Transfer(address,address,uint256)`. */
const ERC20_TRANSFER_TOPIC: B256 =
	alloy::primitives::b256!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
	#[serde(default)]
	pub logs: Vec<ReceiptLog>,
	pub gas_used: Option<U256>,
	pub effective_gas_price: Option<U256>,
	/** OP-stack chains' L1 data fee. */
	pub l1_fee: Option<U256>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLog {
	pub address: Address,
	pub topics: Vec<B256>,
	pub data: Bytes,
	pub transaction_hash: Option<B256>,
}

/** A transaction's receipt and calldata, each fetched on first use. */
pub struct TxData {
	hash: B256,
	receipt: OnceCell<Receipt>,
	input: OnceCell<Bytes>,
}

impl TxData {
	pub fn new(hash: B256) -> Self {
		Self { hash, receipt: OnceCell::new(), input: OnceCell::new() }
	}

	pub async fn receipt(&self, ctx: &EventContext<'_>) -> anyhow::Result<&Receipt> {
		self.receipt
			.get_or_try_init(|| async {
				let value: Value = ctx
					.chain
					.provider
					.raw_request("eth_getTransactionReceipt".into(), (self.hash,))
					.await?;
				if value.is_null() {
					return Err(anyhow!(Retry(format!("no receipt yet for {:#x}", self.hash))));
				}
				serde_json::from_value(value).context("decoding a receipt")
			})
			.await
	}

	pub async fn input(&self, ctx: &EventContext<'_>) -> anyhow::Result<&Bytes> {
		self.input
			.get_or_try_init(|| async {
				let value: Value = ctx
					.chain
					.provider
					.raw_request("eth_getTransactionByHash".into(), (self.hash,))
					.await?;
				let input = value.get("input").or_else(|| value.get("data")).cloned();
				let Some(input) = input else {
					return Err(anyhow!(Retry(format!("no transaction {:#x}", self.hash))));
				};
				serde_json::from_value(input).context("decoding transaction input")
			})
			.await
	}
}

/**
 * `HyperBridgeService.handleRequestHandledEvent`: a delivered message on `chain`, and the
 * relayer's stats. With the delivery transaction (EVM), the stats include the whole
 * transaction's gas and 50 points; in a batched delivery every message adds them again.
 */
pub async fn request_handled(
	ctx: &EventContext<'_>,
	relayer: &str,
	chain: &str,
	timestamp: u64,
	transaction: Option<&TxData>,
) -> anyhow::Result<()> {
	let db = ctx.db;
	add_chain_stats(
		db,
		chain,
		&ChainStatsDelta { delivered_messages: numeric(1), ..Default::default() },
	)
	.await?;
	// `findOrCreate` saved the stats row before the receipt was read.
	add_relayer_stats(db, relayer, chain, &RelayerStatsDelta::default()).await?;
	match transaction {
		Some(transaction) => {
			let receipt = transaction.receipt(ctx).await?;
			let gas_used = receipt.gas_used.ok_or_else(|| anyhow!("receipt has no gasUsed"))?;
			let gas_price = receipt
				.effective_gas_price
				.ok_or_else(|| anyhow!("receipt has no effectiveGasPrice"))?;
			let native_price = native_currency_price(ctx.chain, ctx.log.block_number).await?;
			let mut gas_fee = gas_price * gas_used;
			if ctx.chain.config.op_stack {
				gas_fee += receipt.l1_fee.unwrap_or_default();
			}
			let usd_fee = gas_fee * native_price / U256::from(10).pow(U256::from(18));
			add_relayer_stats(
				db,
				relayer,
				chain,
				&RelayerStatsDelta {
					messages_delivered: numeric(1),
					gas_used: numeric(gas_used),
					gas_fee: numeric(gas_fee),
					usd_gas_fee: numeric(usd_fee),
					..Default::default()
				},
			)
			.await?;
			award_points(
				db,
				Award {
					address: relayer,
					chain,
					points: alloy::primitives::U256::from(50),
					earner: ProtocolParticipantType::Relayer,
					activity: PointsActivityType::RewardPointsEarned,
					transaction_hash: &format!("{:#x}", ctx.log.transaction_hash),
					description: "Points awarded for successful message delivered",
					timestamp,
				},
			)
			.await?;
		},
		None => {
			let delta = RelayerStatsDelta { messages_delivered: numeric(1), ..Default::default() };
			add_relayer_stats(db, relayer, chain, &delta).await?;
		},
	}
	touch_relayer_activity(db, relayer, timestamp).await
}

/** Which `Contract.<module>` volume a sweep's transfers count towards. */
pub enum Attribution<'a> {
	/** Transfers to or from the request's sender (dispatch events), named as the event gave it. */
	Sender(&'a str),
	/** Transfers to or from the first module the delivery calldata names. */
	Modules(MessageFunction),
}

/**
 * The transfer sweep: each ERC-20 `Transfer` in the transaction becomes a `transfers` row with
 * id `<tx>-index-<i>` and lowercase addresses, and adds its USD value to `Transfer.<symbol>`
 * and, when attributed, `Contract.<module>`. A transfer already stored by another event of the
 * transaction is skipped. A transfer whose data is no amount (ERC-721) ends the sweep.
 */
pub async fn sweep_transfers(
	ctx: &EventContext<'_>,
	transaction: &TxData,
	timestamp: u64,
	attribution: Attribution<'_>,
) -> anyhow::Result<()> {
	let db = ctx.db;
	let chain = ctx.chain.state_machine_id();
	let modules = match attribution {
		Attribution::Modules(function) =>
			handler_message_modules(transaction.input(ctx).await?, function),
		Attribution::Sender(_) => vec![],
	};
	let receipt = transaction.receipt(ctx).await?;
	for (index, log) in receipt.logs.iter().enumerate() {
		if log.topics.len() < 3 || log.topics[0] != ERC20_TRANSFER_TOPIC {
			continue;
		}
		let value = transfer_amount(&log.data)?;
		let tx_hash = log.transaction_hash.unwrap_or(ctx.log.transaction_hash);
		let transfer_id = format!("{tx_hash:#x}-index-{index}");
		if transfer_exists(db, &transfer_id).await? {
			continue;
		}
		let from = topic_address(&log.topics[1]);
		let to = topic_address(&log.topics[2]);
		let (symbol, decimals) =
			token_metadata(ctx.chain, log.address, ctx.log.block_number).await?;
		let indexer = ctx.indexer;
		let price =
			token_price(&indexer.orderbook, indexer.config.environment, db, &symbol, timestamp)
				.await?;
		// The TypeScript skipped `amountValueInUSD === "0"`, which `toFixed(18)` never is.
		let usd = amount_value_in_usd(value, decimals, &price);
		store_transfer(db, &transfer_id, chain, value, &from, &to).await?;
		update_volume(db, &format!("Transfer.{symbol}"), chain, &usd, timestamp).await?;

		let touches = |module: &str| {
			let module = module.to_lowercase();
			module == from || module == to
		};
		let contract = match attribution {
			Attribution::Sender(sender) => touches(sender).then_some(sender),
			Attribution::Modules(_) => modules.iter().map(String::as_str).find(|m| touches(m)),
		};
		if let Some(contract) = contract {
			update_volume(db, &format!("Contract.{contract}"), chain, &usd, timestamp).await?;
		}
	}
	Ok(())
}

/** `BigInt(log.data)`. Empty data, as an ERC-721 `Transfer` has, is no number. */
fn transfer_amount(data: &[u8]) -> anyhow::Result<U256> {
	if data.is_empty() {
		anyhow::bail!("transfer data 0x is no amount");
	}
	let start = data.iter().position(|b| *b != 0).unwrap_or(data.len());
	U256::try_from_be_slice(&data[start..])
		.ok_or_else(|| anyhow!("transfer amount wider than 256 bits"))
}

/** `extractAddressFromTopic`: the low 20 bytes, lowercase. */
pub fn topic_address(topic: &B256) -> String {
	format!("{:#x}", Address::from_word(*topic))
}

async fn transfer_exists(db: &Transaction<'_>, id: &str) -> anyhow::Result<bool> {
	Ok(db
		.query_opt("SELECT 1 FROM app.transfers WHERE id = $1", &[&id])
		.await?
		.is_some())
}

/** `TransferService.storeTransfer`: overwrites by id. */
pub async fn store_transfer(
	db: &Transaction<'_>,
	id: &str,
	chain: &str,
	amount: U256,
	from: &str,
	to: &str,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.transfers (id, chain, amount, \"from\", \"to\") VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, amount = EXCLUDED.amount,
		   \"from\" = EXCLUDED.\"from\", \"to\" = EXCLUDED.\"to\"",
		&[&id, &chain, &numeric(amount), &from, &to],
	)
	.await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn topics_become_lowercase_addresses() {
		let topic: B256 = "0x000000000000000000000000ae041F7B0CB581876832830baeB6a2Aa2a3C9716"
			.parse()
			.unwrap();
		assert_eq!(topic_address(&topic), "0xae041f7b0cb581876832830baeb6a2aa2a3c9716");
	}

	#[test]
	fn amounts_read_like_bigint() {
		assert!(transfer_amount(&[]).is_err());
		let mut word = [0u8; 32];
		word[31] = 25;
		assert_eq!(transfer_amount(&word).unwrap(), U256::from(25));
		assert_eq!(transfer_amount(&[0u8; 40]).unwrap(), U256::ZERO);
	}

	#[test]
	fn receipts_decode_with_and_without_l1_fees() {
		let receipt: Receipt = serde_json::from_value(serde_json::json!({
			"logs": [{
				"address": "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
				"topics": [format!("{ERC20_TRANSFER_TOPIC:#x}")],
				"data": "0x",
				"transactionHash": format!("{:#x}", B256::repeat_byte(0x22)),
			}],
			"gasUsed": "0x5208",
			"effectiveGasPrice": "0x3b9aca00",
			"l1Fee": null,
		}))
		.unwrap();
		assert_eq!(receipt.gas_used, Some(U256::from(21_000)));
		assert_eq!(receipt.l1_fee, None);
		assert_eq!(receipt.logs.len(), 1);
	}
}
