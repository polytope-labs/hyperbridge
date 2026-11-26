//! Treasury spends and their payout.

use serde_json::Value;

use super::{amount, int, text, Context};
use crate::store::numeric;

pub(super) const METHODS: &[&str] =
	&["AssetSpendApproved", "Paid", "PaymentFailed", "SpendProcessed", "AssetSpendVoided"];

pub(super) async fn handle(ev: &Context<'_>) -> anyhow::Result<()> {
	let Some(index) = int(ev.arg("index")) else { return Ok(()) };
	let p = &ev.position;
	if ev.method == "AssetSpendApproved" {
		ev.db
			.execute(
				"INSERT INTO governance.governance_treasury_spends (id, index, asset_kind, amount,
					beneficiary, valid_from, expire_at, state, approved_block, approved_at,
					updated_block)
				 VALUES ($1, $2, $3, $4, $5, $6, $7, 'Approved', $8, $9, $8)
				 ON CONFLICT (id) DO UPDATE SET asset_kind = EXCLUDED.asset_kind,
					amount = EXCLUDED.amount, beneficiary = EXCLUDED.beneficiary,
					valid_from = EXCLUDED.valid_from, expire_at = EXCLUDED.expire_at,
					state = EXCLUDED.state, payment_id = NULL,
					approved_block = EXCLUDED.approved_block,
					approved_at = EXCLUDED.approved_at, updated_block = EXCLUDED.updated_block",
				&[
					&index.to_string(),
					&index,
					&text(ev.arg("assetKind")).unwrap_or_default(),
					&numeric(amount(ev.arg("amount")).unwrap_or_default()),
					&text(ev.arg("beneficiary")).unwrap_or_default(),
					&int(ev.arg("validFrom")).unwrap_or_default(),
					&int(ev.arg("expireAt")).unwrap_or_default(),
					&p.block_number,
					&p.timestamp,
				],
			)
			.await?;
		return Ok(());
	}

	let state = match ev.method.as_str() {
		"Paid" => "Paid",
		"PaymentFailed" => "Failed",
		"SpendProcessed" => "Processed",
		_ => "Voided",
	};
	let payment_id = match ev.arg("paymentId") {
		Value::Null => None,
		Value::String(id) => Some(id.clone()),
		id => Some(id.to_string()),
	};
	ev.db
		.execute(
			"UPDATE governance.governance_treasury_spends SET state = $2,
				payment_id = coalesce($3, payment_id), updated_block = $4
			 WHERE id = $1",
			&[&index.to_string(), &state, &payment_id, &p.block_number],
		)
		.await?;
	Ok(())
}
