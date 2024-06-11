#![cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

use ismp::{
	host::StateMachine,
	router::{PostResponse, Request},
};

use crate::{
	indexing::{query_request_status_from_indexer, query_response_status_from_indexer},
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

// "source": "0x42415345",
// "nonce": "6055",
// "from": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// "to": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// "timeoutTimestamp": "1716240884",
// "dest": "0x4f505449",
// "data": "0x68656c6c6f2066726f6d2042415345",

#[tokio::test]
#[ignore]
async fn test_query_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("42415345".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("4f505449".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 6055,
		from: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		to: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		timeout_timestamp: 1716240884,
		data: hex::decode("68656c6c6f2066726f6d2042415345".to_string()).unwrap(),
	};

	let request = Request::Post(post);

	let status = query_request_status_from_indexer(request, None).await?.unwrap();

	dbg!(&status);
	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));

	Ok(())
}

// "request": {
// 	"source": "0x425343",
// 	"nonce": "3516",
// 	"from": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// 	"to": "0x9cc29770f3d643f4094ee591f3d2e3c98c349761",
// 	"timeoutTimestamp": "1716240473",
// 	"dest": "0x4f505449",
// 	"data": "0x68656c6c6f2066726f6d20425343"
//   },
//   "id": "0x0039f125db9eb51dd1e25d6dafab8e68e4bc3367145ab943e8350a9e755d3574",
//   "status": "DEST",
//   "chain": "OPTI",
//   "responseTimeoutTimestamp": "3432417653",
//   "responseMessage": "0x48656c6c6f2066726f6d204f505449",
// }

#[tokio::test]
#[ignore]
async fn test_query_response_status_from_indexer() -> Result<(), anyhow::Error> {
	let post = ismp::router::Post {
		source: StateMachine::from_str(
			&String::from_utf8(hex::decode("425343".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		dest: StateMachine::from_str(
			&String::from_utf8(hex::decode("4f505449".to_string()).unwrap()).unwrap(),
		)
		.unwrap(),
		nonce: 3516,
		from: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		to: hex::decode("9cc29770f3d643f4094ee591f3d2e3c98c349761".to_string()).unwrap(),
		timeout_timestamp: 1716240473,
		data: hex::decode("68656c6c6f2066726f6d20425343".to_string()).unwrap(),
	};

	let response = PostResponse {
		post,
		response: hex::decode("48656c6c6f2066726f6d204f505449".to_string()).unwrap(),
		timeout_timestamp: 3432417653,
	};

	let status = query_response_status_from_indexer(ismp::router::Response::Post(response), None)
		.await?
		.unwrap();

	dbg!(&status);
	assert!(matches!(status, MessageStatusWithMetadata::DestinationDelivered { .. }));
	Ok(())
}
