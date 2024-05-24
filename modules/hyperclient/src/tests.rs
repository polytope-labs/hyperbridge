#![cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

use ismp::{host::StateMachine, router::Request};

use crate::{
	indexing::query_request_status_from_indexer,
	testing::{subscribe_to_request_status, test_timeout_request},
	types::MessageStatusWithMetadata,
};

pub fn setup_logging() {
	use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	let _ = tracing_subscriber::fmt().with_env_filter(filter).finish().try_init();
}

#[tokio::test]
#[ignore]
async fn hyperclient_integration_tests() -> Result<(), anyhow::Error> {
	setup_logging();
	test_timeout_request().await?;
	subscribe_to_request_status().await?;
	Ok(())
}

// {
// 	"id": "0x000007717468646f730e52dade89d1aef0325c51b9b9afa28c52b13698289da8",
// 	"chain": "ARBI",
// 	"source": "0x41524249",
// 	"dest": "0x45544845",
// 	"data": "0x68656c6c6f2066726f6d2041524249",
// 	"status": "SOURCE",
// 	"nonce": "959",
// 	"statusMetadata": {
// 	  "nodes": [
// 		{
// 		  "status": "SOURCE"
// 		}
// 	  ]
// 	},
// 	"from": "0xb51d235cf4461d17fea88733fed1865873c8d686",
// 	"to": "0xb51d235cf4461d17fea88733fed1865873c8d686",
// 	"timeoutTimestamp": "1716062948"
//   },
//   {
// 	"id": "0x00005f364898fe1749e1d6fbd15891b85aec57d3e6be559ceccc7f4897b2c10e",
// 	"chain": "BASE",
// 	"source": "0x42415345",
// 	"dest": "0x41524249",
// 	"data": "0x68656c6c6f2066726f6d2042415345",
// 	"status": "DEST",
// 	"nonce": "17359",
// 	"statusMetadata": {
// 	  "nodes": [
// 		{
// 		  "status": "DEST"
// 		},
// 		{
// 		  "status": "SOURCE"
// 		}
// 	  ]
// 	},
// 	"from": "0xe8cb27bad1c5071189b154bec0088209d1ec2582",
// 	"to": "0xe8cb27bad1c5071189b154bec0088209d1ec2582",
// 	"timeoutTimestamp": "1716227076"
//   },

#[tokio::test]
#[ignore]
async fn test_query_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("42415345".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("41524249".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 17359,
		from: hex::decode("e8cb27bad1c5071189b154bec0088209d1ec2582".to_string()).unwrap(),
		to: hex::decode("e8cb27bad1c5071189b154bec0088209d1ec2582".to_string()).unwrap(),
		timeout_timestamp: 1716227076,
		data: hex::decode("68656c6c6f2066726f6d2042415345".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let status = query_request_status_from_indexer(request).await?.unwrap();

	dbg!(&status);
	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));

	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("41524249".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("45544845".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 959,
		from: hex::decode("b51d235cf4461d17fea88733fed1865873c8d686".to_string()).unwrap(),
		to: hex::decode("b51d235cf4461d17fea88733fed1865873c8d686".to_string()).unwrap(),
		timeout_timestamp: 1716062948,
		data: hex::decode("68656c6c6f2066726f6d2041524249".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let status = query_request_status_from_indexer(request).await?.unwrap();

	// This request was not delivered so it should
	assert!(matches!(status, MessageStatusWithMetadata::SourceFinalized { .. }));

	dbg!(status);

	Ok(())
}
