#![cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

use ismp::{host::StateMachine, router::Request};

use crate::{
	indexing::query_request_status_from_indexer,
	testing::{subscribe_to_request_status, test_timeout_request}, types::MessageStatusWithMetadata,
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
// 	"data": "0x68656c6c6f2066726f6d2042415345",
// 	"dest": "0x45544845",
// 	"fee": "30000000000000000000",
// 	"from": "0x3554a2260aa37788dc8c2932a908fda98a10dd88",
// 	"id": "0x00005d387e19a35025286dc3cb40a427f6f6d030dd957c929318d662c2200d74",
// 	"nonce": "8193",
// 	"source": "0x42415345",
// 	"status": "DEST",
// 	"statusMetadata": {
// 	  "nodes": [
// 		{
// 		  "status": "DEST",
// 		  "transactionHash": "0xf03c8c0c4c2d4fcff4b700693231c3851dbee22e774b60b0b78e5c48169c0031",
// 		  "timestamp": "1714591860",
// 		  "chain": "ETHEREUM_SEPOLIA"
// 		},
// 		{
// 		  "status": "SOURCE",
// 		  "transactionHash": "0xa3c7364ec1f8f3bcb8d89e3d43d12c96181cac1dd28bfa0d74c1409c1d6e9462",
// 		  "timestamp": "1714588734",
// 		  "chain": "BASE_SEPOLIA"
// 		}
// 	  ]
// 	},
// 	"chain": "BASE_SEPOLIA",
// 	"timeoutTimestamp": "1714624734",
// 	"to": "0x3554a2260aa37788dc8c2932a908fda98a10dd88"
//   },

// "data": "0x68656c6c6f2066726f6d2045544845",
//           "dest": "0x41524249",
//           "fee": "30000000000000000000",
//           "from": "0x3554a2260aa37788dc8c2932a908fda98a10dd88",
//           "id": "0x000002158c136917e5435c8bf34f8d7165004dda920673c4a969058ba446895f",
//           "nonce": "277",
//           "source": "0x45544845",
//           "status": "SOURCE",
//           "statusMetadata": {
//             "nodes": [
//               {
//                 "status": "SOURCE",
//                 "transactionHash":
// "0x91a36e6ea9d176f1fea0e80a25a54830eab0d0e70a7c3b6a8bea05323729b788",
// "timestamp": "1714584756",                 "chain": "ETHEREUM_SEPOLIA"
//               }
//             ]
//           },
//           "chain": "ETHEREUM_SEPOLIA",
//           "timeoutTimestamp": "1714620756",
//           "to": "0x3554a2260aa37788dc8c2932a908fda98a10dd88"
//         },

#[tokio::test]
#[ignore]
async fn test_query_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("42415345".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("45544845".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 8193,
		from: hex::decode("3554a2260aa37788dc8c2932a908fda98a10dd88".to_string()).unwrap(),
		to: hex::decode("3554a2260aa37788dc8c2932a908fda98a10dd88".to_string()).unwrap(),
		timeout_timestamp: 1714624734,
		data: hex::decode("68656c6c6f2066726f6d2042415345".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let status = query_request_status_from_indexer(request).await?.unwrap();

	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));

	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("45544845".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("41524249".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 277,
		from: hex::decode("3554a2260aa37788dc8c2932a908fda98a10dd88".to_string()).unwrap(),
		to: hex::decode("3554a2260aa37788dc8c2932a908fda98a10dd88".to_string()).unwrap(),
		timeout_timestamp: 1714620756,
		data: hex::decode("68656c6c6f2066726f6d2045544845".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let status = query_request_status_from_indexer(request).await?.unwrap();

	// This request was not delivered so it should
	assert!(matches!(status, MessageStatusWithMetadata::SourceFinalized { .. }));

	Ok(())
}
