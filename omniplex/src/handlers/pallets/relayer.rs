//! `pallet-relayer`: fees accumulated on Hyperbridge, and their withdrawal.

use std::str::FromStr;

use alloy::primitives::U256;
use async_trait::async_trait;
use codec::Encode;
use ismp::host::StateMachine;
use sp_crypto_hashing::blake2_128;
use tokio_postgres::Transaction;

use super::{encode_address, is_hyperbridge, timestamp_ms};
use crate::{
	handlers::abi::pallets::Relayer,
	registry::{EventContext, EventHandler},
	services::relayers::{add_relayer_stats, touch_relayer_activity, RelayerStatsDelta},
	store::{numeric, timestamp_to_date},
};

/** `encodeAddress`'s default prefix, which SubQuery used for relayer-pallet bytes. */
const RELAYER_PREFIX: u8 = 42;

/** `twox128("HostExecutive") ++ twox128("FeeTokenDecimals")`. */
const FEE_TOKEN_DECIMALS_PREFIX: &str =
	"cae7464375580d6a6683e78dcd64d91bc7e5c41dd1fd7d306a7ebba39c500265";

/** `handleAccumulateFeesEvent`. */
pub struct AccumulateFees;

#[async_trait]
impl EventHandler for AccumulateFees {
	type Event = Relayer::AccumulateFees;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		// Named as `Withdraw` names the relayer. SubQuery's `encodeAddress` threw on 20-byte EVM
		// addresses, so it dropped their fees.
		let Some(relayer) = relayer_address(&event.relayer) else {
			tracing::warn!(
				relayer = %event.relayer,
				"AccumulateFees: relayer is not an EVM address or SS58 key; dropped"
			);
			return Ok(());
		};
		// SubQuery read the decimals at the chain tip; this reads them at the event's block.
		let decimals = match fee_token_decimals_key(&event.stateMachine) {
			Some(key) => {
				let value = ctx.chain.storage(&key, ctx.log.block_number).await?;
				value.first().copied().unwrap_or(18)
			},
			None => 18,
		};
		let Some(fees) = normalize_fees(event.amount, decimals) else {
			tracing::warn!(
				decimals,
				"AccumulateFees: fee decimals above 18; dropped as SubQuery did"
			);
			return Ok(());
		};
		record_accumulated_fees(ctx.db, &relayer, &event.stateMachine, fees, timestamp_ms(ctx.log))
			.await
	}
}

/**
 * `HostExecutive.FeeTokenDecimals(state_machine)`. `None` where SubQuery's key encoder threw
 * (Tendermint and Relay ids, or an unparseable one), which meant 18 decimals.
 */
pub fn fee_token_decimals_key(state_machine: &str) -> Option<Vec<u8>> {
	if state_machine.starts_with("TNDRMINT-") || state_machine.starts_with("RELAY-") {
		return None;
	}
	let encoded = StateMachine::from_str(state_machine).ok()?.encode();
	let mut key = hex::decode(FEE_TOKEN_DECIMALS_PREFIX).expect("constant is hex");
	key.extend_from_slice(&blake2_128(&encoded));
	key.extend_from_slice(&encoded);
	Some(key)
}

/** `amount / 10^(18 - decimals)`, floored. `None` above 18 decimals, where BigInt threw. */
pub fn normalize_fees(amount: U256, decimals: u8) -> Option<U256> {
	let shift = 18u8.checked_sub(decimals)?;
	Some(amount / U256::from(10).pow(U256::from(shift)))
}

/** Credits fees to the relayer's stats and its `accumulated_fees` row for the chain. */
pub async fn record_accumulated_fees(
	db: &Transaction<'_>,
	relayer: &str,
	chain: &str,
	fees: U256,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let fees = numeric(fees);
	let delta = RelayerStatsDelta { fees_earned: fees.clone(), ..Default::default() };
	add_relayer_stats(db, relayer, chain, &delta).await?;
	touch_relayer_activity(db, relayer, timestamp_ms).await?;
	db.execute(
		"INSERT INTO app.accumulated_fees AS t (id, relayer, chain_id, lifetime_fees, last_updated_at)
		 VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (id) DO UPDATE SET
		   lifetime_fees = t.lifetime_fees + EXCLUDED.lifetime_fees,
		   last_updated_at = EXCLUDED.last_updated_at",
		&[
			&format!("{relayer}-{chain}"),
			&relayer,
			&chain,
			&fees,
			&timestamp_to_date(timestamp_ms),
		],
	)
	.await?;
	Ok(())
}

/** `handleRelayerWithdrawEvent`. */
pub struct Withdraw;

#[async_trait]
impl EventHandler for Withdraw {
	type Event = Relayer::Withdraw;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let Some(relayer) = relayer_address(&event.relayer) else {
			tracing::warn!(
				relayer = %event.relayer,
				"Withdraw: relayer is not an EVM address or SS58 key; dropped as SubQuery did"
			);
			return Ok(());
		};
		// Raw, unlike AccumulateFees: no decimal normalization.
		let delta = RelayerStatsDelta { withdrawn: numeric(event.amount), ..Default::default() };
		add_relayer_stats(ctx.db, &relayer, &event.stateMachine, &delta).await
	}
}

/** A pallet relayer: 20 bytes as lowercase `0x` hex, anything else as SS58 with prefix 42. */
pub fn relayer_address(bytes: &[u8]) -> Option<String> {
	if bytes.len() == 20 {
		return Some(format!("0x{}", hex::encode(bytes)));
	}
	encode_address(bytes, RELAYER_PREFIX)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fee_decimals_key_matches_the_storage_layout() {
		let key = fee_token_decimals_key("EVM-1").unwrap();
		let encoded = StateMachine::Evm(1).encode();
		assert_eq!(hex::encode(&key[..32]), FEE_TOKEN_DECIMALS_PREFIX);
		assert_eq!(&key[32..48], &blake2_128(&encoded));
		assert_eq!(&key[48..], &encoded[..]);
		assert_eq!(hex::encode(&key[48..]), "0001000000");
		assert_eq!(fee_token_decimals_key("TNDRMINT-osmo"), None);
		assert_eq!(fee_token_decimals_key("RELAY-pase-1000"), None);
		assert_eq!(&fee_token_decimals_key("SUBSTRATE-cere").unwrap()[48..], b"\x03cere");
	}

	#[test]
	fn fees_are_floored_to_the_fee_token_decimals() {
		let amount = U256::from(1_234_567_890_123_456_789u128);
		assert_eq!(normalize_fees(amount, 18), Some(amount));
		assert_eq!(normalize_fees(amount, 6), Some(U256::from(1_234_567u64)));
		assert_eq!(normalize_fees(amount, 0), Some(U256::from(1u64)));
		assert_eq!(normalize_fees(amount, 19), None);
	}

	#[test]
	fn relayers_keep_evm_addresses_as_hex() {
		let evm = hex::decode("BC50B907000000000000000000000000000000AA").unwrap();
		assert_eq!(relayer_address(&evm).unwrap(), "0xbc50b907000000000000000000000000000000aa");
		let alice = hex::decode("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")
			.unwrap();
		assert_eq!(
			relayer_address(&alice).unwrap(),
			"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
		);
		assert_eq!(relayer_address(&[0u8; 21]), None);
	}
}
