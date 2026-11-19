//! The block explorer's writes against the real `explorer` schema. Set
//! `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use alloy::primitives::{Address, Bytes, FixedBytes, B256};
use codec::Encode;
use common::{database, text};
use omniplex::{
	handlers::{
		abi::pallets::{Balances, Explorer},
		explorer::{
			touched_accounts, write_account, write_block, write_event, write_extrinsic,
			write_transfer,
		},
	},
	registry::LogMeta,
	substrate::encode_address,
};

const HEIGHT: u64 = 12_597_592;

fn log(transaction_index: u64, log_index: u64) -> LogMeta {
	LogMeta {
		address: Address::ZERO,
		block_number: HEIGHT,
		block_hash: B256::repeat_byte(0xbb),
		block_timestamp: 1_793_711_196,
		transaction_hash: B256::repeat_byte(0xee),
		transaction_index,
		log_index,
	}
}

fn block() -> Explorer::Block {
	Explorer::Block {
		specVersion: 8_800,
		parentHash: B256::repeat_byte(1),
		stateRoot: B256::repeat_byte(2),
		extrinsicsRoot: B256::repeat_byte(3),
		extrinsicsCount: 2,
		eventsCount: 3,
	}
}

fn transfer_call(signer: [u8; 32]) -> Explorer::Extrinsic {
	Explorer::Extrinsic {
		index: 1,
		signed: true,
		success: true,
		signer: Bytes::copy_from_slice(&signer),
		section: "balances".into(),
		method: "transferKeepAlive".into(),
		args: r#"[{"name":"value","type":"Compact<Balance>","value":"5000000000000"}]"#.into(),
		eventsCount: 2,
	}
}

/** `AccountInfo` with a nonce, the four counters and free, reserved and frozen balances. */
fn account_info(nonce: u32, free: u128, reserved: u128) -> Vec<u8> {
	(nonce, 0u32, 1u32, 0u32, free, reserved, 0u128, 0u128).encode()
}

#[tokio::test]
async fn a_block_its_extrinsics_events_and_transfers_are_stored_as_the_site_reads_them() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let (alice, bob) = ([7u8; 32], [9u8; 32]);

	write_block(&db, &log(2, 3), &block()).await.unwrap();
	write_extrinsic(&db, &log(1, 4), &transfer_call(alice)).await.unwrap();
	let event = Explorer::Event {
		index: 1,
		isExtrinsic: true,
		extrinsicIndex: 1,
		section: "balances".into(),
		method: "Transfer".into(),
		args: "null".into(),
	};
	write_event(&db, &log(1, 6), &event).await.unwrap();
	let transfer = Balances::Transfer {
		from: FixedBytes(alice),
		to: FixedBytes(bob),
		amount: 5_000_000_000_000,
	};
	write_transfer(&db, &log(1, 1), &transfer).await.unwrap();

	let block = text(
		&db,
		"SELECT hash || ' ' || time || ' ' || spec_version || ' ' || extrinsics_count || ' ' ||
		   events_count FROM explorer.blocks",
	)
	.await;
	assert_eq!(
		block.as_deref(),
		Some(
			"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb 1793711196000 8800 2 3"
		)
	);

	let alice_address = encode_address(&alice, 0).unwrap();
	let extrinsic = text(
		&db,
		"SELECT signer || ' ' || section || '.' || method || ' ' || (args->0->>'value') || ' ' ||
		   signed || ' ' || success FROM explorer.extrinsics WHERE height = 12597592 AND index = 1",
	)
	.await;
	assert_eq!(
		extrinsic,
		Some(format!("{alice_address} balances.transferKeepAlive 5000000000000 true true"))
	);

	let event = text(
		&db,
		"SELECT extrinsic_index || ' ' || section || '.' || method || ' ' || (args IS NULL)
		 FROM explorer.events",
	)
	.await;
	assert_eq!(event.as_deref(), Some("1 balances.Transfer true"));

	let transfer = text(
		&db,
		"SELECT \"from\" || ' ' || \"to\" || ' ' || amount || ' ' || extrinsic_index
		 FROM explorer.transfers",
	)
	.await;
	let bob_address = encode_address(&bob, 0).unwrap();
	assert_eq!(transfer, Some(format!("{alice_address} {bob_address} 5000000000000 1")));

	let names = text(
		&db,
		"SELECT string_agg(spec_version || ' ' || kind || ' ' || section || '.' || method, ','
		   ORDER BY kind) FROM explorer.call_names",
	)
	.await;
	assert_eq!(
		names.as_deref(),
		Some("8800 call balances.transferKeepAlive,8800 event balances.Transfer")
	);

	let touched = touched_accounts(&db, HEIGHT, HEIGHT).await.unwrap();
	assert_eq!(touched.into_iter().collect::<Vec<_>>(), {
		let mut both = vec![alice_address.clone(), bob_address];
		both.sort();
		both
	});
	assert!(touched_accounts(&db, HEIGHT + 1, HEIGHT + 5).await.unwrap().is_empty());
}

/** An account keeps its latest balances, and goes once the chain has reaped it. */
#[tokio::test]
async fn accounts_follow_the_chain_and_disappear_when_reaped() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let address = encode_address(&[7u8; 32], 0).unwrap();

	write_account(&db, &address, &account_info(1, 100, 20), HEIGHT).await.unwrap();
	write_account(&db, &address, &account_info(2, 300, 50), HEIGHT + 1)
		.await
		.unwrap();
	let account = text(
		&db,
		"SELECT nonce || ' ' || free || ' ' || reserved || ' ' || total || ' ' || height
		 FROM explorer.accounts",
	)
	.await;
	assert_eq!(account.as_deref(), Some("2 300 50 350 12597593"));

	write_account(&db, &address, &[], HEIGHT + 2).await.unwrap();
	assert_eq!(text(&db, "SELECT address FROM explorer.accounts").await, None);
}
