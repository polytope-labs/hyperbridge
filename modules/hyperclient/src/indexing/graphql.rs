#![allow(non_camel_case_types)]
#![allow(unused_imports)]
use crate::indexing::BigInt;
pub struct RequestQuery;
pub mod request_query {
	#![allow(dead_code)]
	use std::result::Result;
	pub const OPERATION_NAME: &str = "RequestQuery";
	pub const QUERY : & str = "query RequestQuery($id: String!) {\n  request(id: $id) {\n    id\n    chain\n    data\n    dest\n    fee\n    from\n    nonce\n    source\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n    timeoutTimestamp\n    to\n  }\n}\n\nquery ResponseQuery($id: String!) {\n  response(id: $id) {\n    chain\n    id\n    response_message\n    responseTimeoutTimestamp\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n  }\n}\n\nquery StateMachineUpdatesQuery($stateMachineId: String!, $chain: SupportedChain!, $height: BigInt!) {\n  stateMachineUpdateEvents(\n    filter: {and: {stateMachineId: {equalTo: $stateMachineId }, chain: {equalTo: $chain}, height: {greaterThanOrEqualTo: $height}}}\n  ) {\n    nodes {\n      blockHash\n      blockNumber\n      chain\n      height\n      id\n      stateMachineId\n      transactionHash\n    }\n  }\n}\n" ;
	use super::*;
	use serde::{Deserialize, Serialize};
	#[allow(dead_code)]
	type Boolean = bool;
	#[allow(dead_code)]
	type Float = f64;
	#[allow(dead_code)]
	type Int = i64;
	#[allow(dead_code)]
	type ID = String;
	type BigInt = super::BigInt;
	#[derive(Clone, Debug, Eq, PartialEq)]
	pub enum SupportedChain {
		ETHEREUM_SEPOLIA,
		BASE_SEPOLIA,
		OPTIMISM_SEPOLIA,
		ARBITRUM_SEPOLIA,
		BSC_CHAPEL,
		HYPERBRIDGE_GARGANTUA,
		Other(String),
	}
	impl ::serde::Serialize for SupportedChain {
		fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
			ser.serialize_str(match *self {
				SupportedChain::ETHEREUM_SEPOLIA => "ETHEREUM_SEPOLIA",
				SupportedChain::BASE_SEPOLIA => "BASE_SEPOLIA",
				SupportedChain::OPTIMISM_SEPOLIA => "OPTIMISM_SEPOLIA",
				SupportedChain::ARBITRUM_SEPOLIA => "ARBITRUM_SEPOLIA",
				SupportedChain::BSC_CHAPEL => "BSC_CHAPEL",
				SupportedChain::HYPERBRIDGE_GARGANTUA => "HYPERBRIDGE_GARGANTUA",
				SupportedChain::Other(ref s) => &s,
			})
		}
	}
	impl<'de> ::serde::Deserialize<'de> for SupportedChain {
		fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			let s: String = ::serde::Deserialize::deserialize(deserializer)?;
			match s.as_str() {
				"ETHEREUM_SEPOLIA" => Ok(SupportedChain::ETHEREUM_SEPOLIA),
				"BASE_SEPOLIA" => Ok(SupportedChain::BASE_SEPOLIA),
				"OPTIMISM_SEPOLIA" => Ok(SupportedChain::OPTIMISM_SEPOLIA),
				"ARBITRUM_SEPOLIA" => Ok(SupportedChain::ARBITRUM_SEPOLIA),
				"BSC_CHAPEL" => Ok(SupportedChain::BSC_CHAPEL),
				"HYPERBRIDGE_GARGANTUA" => Ok(SupportedChain::HYPERBRIDGE_GARGANTUA),
				_ => Ok(SupportedChain::Other(s)),
			}
		}
	}
	#[derive(Clone, Debug, Eq, PartialEq)]
	pub enum RequestStatus {
		SOURCE,
		MESSAGE_RELAYED,
		DEST,
		TIMED_OUT,
		Other(String),
	}
	impl ::serde::Serialize for RequestStatus {
		fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
			ser.serialize_str(match *self {
				RequestStatus::SOURCE => "SOURCE",
				RequestStatus::MESSAGE_RELAYED => "MESSAGE_RELAYED",
				RequestStatus::DEST => "DEST",
				RequestStatus::TIMED_OUT => "TIMED_OUT",
				RequestStatus::Other(ref s) => &s,
			})
		}
	}
	impl<'de> ::serde::Deserialize<'de> for RequestStatus {
		fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			let s: String = ::serde::Deserialize::deserialize(deserializer)?;
			match s.as_str() {
				"SOURCE" => Ok(RequestStatus::SOURCE),
				"MESSAGE_RELAYED" => Ok(RequestStatus::MESSAGE_RELAYED),
				"DEST" => Ok(RequestStatus::DEST),
				"TIMED_OUT" => Ok(RequestStatus::TIMED_OUT),
				_ => Ok(RequestStatus::Other(s)),
			}
		}
	}
	#[derive(Serialize)]
	pub struct Variables {
		pub id: String,
	}
	impl Variables {}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct ResponseData {
		pub request: Option<RequestQueryRequest>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct RequestQueryRequest {
		pub id: ID,
		pub chain: SupportedChain,
		pub data: Option<String>,
		pub dest: Option<String>,
		pub fee: Option<BigInt>,
		pub from: Option<String>,
		pub nonce: Option<BigInt>,
		pub source: Option<String>,
		pub status: RequestStatus,
		#[serde(rename = "statusMetadata")]
		pub status_metadata: Vec<Option<RequestQueryRequestStatusMetadata>>,
		#[serde(rename = "timeoutTimestamp")]
		pub timeout_timestamp: Option<BigInt>,
		pub to: Option<String>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct RequestQueryRequestStatusMetadata {
		pub id: ID,
		pub status: RequestStatus,
		pub chain: SupportedChain,
		pub timestamp: BigInt,
		#[serde(rename = "blockNumber")]
		pub block_number: String,
		#[serde(rename = "transactionHash")]
		pub transaction_hash: String,
		#[serde(rename = "blockHash")]
		pub block_hash: String,
	}
}
impl graphql_client::GraphQLQuery for RequestQuery {
	type Variables = request_query::Variables;
	type ResponseData = request_query::ResponseData;
	fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
		graphql_client::QueryBody {
			variables,
			query: request_query::QUERY,
			operation_name: request_query::OPERATION_NAME,
		}
	}
}
pub struct ResponseQuery;
pub mod response_query {
	#![allow(dead_code)]
	use std::result::Result;
	pub const OPERATION_NAME: &str = "ResponseQuery";
	pub const QUERY : & str = "query RequestQuery($id: String!) {\n  request(id: $id) {\n    id\n    chain\n    data\n    dest\n    fee\n    from\n    nonce\n    source\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n    timeoutTimestamp\n    to\n  }\n}\n\nquery ResponseQuery($id: String!) {\n  response(id: $id) {\n    chain\n    id\n    response_message\n    responseTimeoutTimestamp\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n  }\n}\n\nquery StateMachineUpdatesQuery($stateMachineId: String!, $chain: SupportedChain!, $height: BigInt!) {\n  stateMachineUpdateEvents(\n    filter: {and: {stateMachineId: {equalTo: $stateMachineId }, chain: {equalTo: $chain}, height: {greaterThanOrEqualTo: $height}}}\n  ) {\n    nodes {\n      blockHash\n      blockNumber\n      chain\n      height\n      id\n      stateMachineId\n      transactionHash\n    }\n  }\n}\n" ;
	use super::*;
	use serde::{Deserialize, Serialize};
	#[allow(dead_code)]
	type Boolean = bool;
	#[allow(dead_code)]
	type Float = f64;
	#[allow(dead_code)]
	type Int = i64;
	#[allow(dead_code)]
	type ID = String;
	type BigInt = super::BigInt;
	#[derive(Clone, Debug, Eq, PartialEq)]
	pub enum SupportedChain {
		ETHEREUM_SEPOLIA,
		BASE_SEPOLIA,
		OPTIMISM_SEPOLIA,
		ARBITRUM_SEPOLIA,
		BSC_CHAPEL,
		HYPERBRIDGE_GARGANTUA,
		Other(String),
	}
	impl ::serde::Serialize for SupportedChain {
		fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
			ser.serialize_str(match *self {
				SupportedChain::ETHEREUM_SEPOLIA => "ETHEREUM_SEPOLIA",
				SupportedChain::BASE_SEPOLIA => "BASE_SEPOLIA",
				SupportedChain::OPTIMISM_SEPOLIA => "OPTIMISM_SEPOLIA",
				SupportedChain::ARBITRUM_SEPOLIA => "ARBITRUM_SEPOLIA",
				SupportedChain::BSC_CHAPEL => "BSC_CHAPEL",
				SupportedChain::HYPERBRIDGE_GARGANTUA => "HYPERBRIDGE_GARGANTUA",
				SupportedChain::Other(ref s) => &s,
			})
		}
	}
	impl<'de> ::serde::Deserialize<'de> for SupportedChain {
		fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			let s: String = ::serde::Deserialize::deserialize(deserializer)?;
			match s.as_str() {
				"ETHEREUM_SEPOLIA" => Ok(SupportedChain::ETHEREUM_SEPOLIA),
				"BASE_SEPOLIA" => Ok(SupportedChain::BASE_SEPOLIA),
				"OPTIMISM_SEPOLIA" => Ok(SupportedChain::OPTIMISM_SEPOLIA),
				"ARBITRUM_SEPOLIA" => Ok(SupportedChain::ARBITRUM_SEPOLIA),
				"BSC_CHAPEL" => Ok(SupportedChain::BSC_CHAPEL),
				"HYPERBRIDGE_GARGANTUA" => Ok(SupportedChain::HYPERBRIDGE_GARGANTUA),
				_ => Ok(SupportedChain::Other(s)),
			}
		}
	}
	#[derive(Clone, Debug, Eq, PartialEq)]
	pub enum ResponseStatus {
		SOURCE,
		MESSAGE_RELAYED,
		DEST,
		TIMED_OUT,
		Other(String),
	}
	impl ::serde::Serialize for ResponseStatus {
		fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
			ser.serialize_str(match *self {
				ResponseStatus::SOURCE => "SOURCE",
				ResponseStatus::MESSAGE_RELAYED => "MESSAGE_RELAYED",
				ResponseStatus::DEST => "DEST",
				ResponseStatus::TIMED_OUT => "TIMED_OUT",
				ResponseStatus::Other(ref s) => &s,
			})
		}
	}
	impl<'de> ::serde::Deserialize<'de> for ResponseStatus {
		fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			let s: String = ::serde::Deserialize::deserialize(deserializer)?;
			match s.as_str() {
				"SOURCE" => Ok(ResponseStatus::SOURCE),
				"MESSAGE_RELAYED" => Ok(ResponseStatus::MESSAGE_RELAYED),
				"DEST" => Ok(ResponseStatus::DEST),
				"TIMED_OUT" => Ok(ResponseStatus::TIMED_OUT),
				_ => Ok(ResponseStatus::Other(s)),
			}
		}
	}
	#[derive(Serialize)]
	pub struct Variables {
		pub id: String,
	}
	impl Variables {}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct ResponseData {
		pub response: Option<ResponseQueryResponse>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct ResponseQueryResponse {
		pub chain: SupportedChain,
		pub id: ID,
		pub response_message: Option<String>,
		#[serde(rename = "responseTimeoutTimestamp")]
		pub response_timeout_timestamp: Option<BigInt>,
		pub status: ResponseStatus,
		#[serde(rename = "statusMetadata")]
		pub status_metadata: Vec<Option<ResponseQueryResponseStatusMetadata>>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct ResponseQueryResponseStatusMetadata {
		pub id: ID,
		pub status: ResponseStatus,
		pub chain: SupportedChain,
		pub timestamp: BigInt,
		#[serde(rename = "blockNumber")]
		pub block_number: String,
		#[serde(rename = "transactionHash")]
		pub transaction_hash: String,
		#[serde(rename = "blockHash")]
		pub block_hash: String,
	}
}
impl graphql_client::GraphQLQuery for ResponseQuery {
	type Variables = response_query::Variables;
	type ResponseData = response_query::ResponseData;
	fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
		graphql_client::QueryBody {
			variables,
			query: response_query::QUERY,
			operation_name: response_query::OPERATION_NAME,
		}
	}
}
pub struct StateMachineUpdatesQuery;
pub mod state_machine_updates_query {
	#![allow(dead_code)]
	use std::result::Result;
	pub const OPERATION_NAME: &str = "StateMachineUpdatesQuery";
	pub const QUERY : & str = "query RequestQuery($id: String!) {\n  request(id: $id) {\n    id\n    chain\n    data\n    dest\n    fee\n    from\n    nonce\n    source\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n    timeoutTimestamp\n    to\n  }\n}\n\nquery ResponseQuery($id: String!) {\n  response(id: $id) {\n    chain\n    id\n    response_message\n    responseTimeoutTimestamp\n    status\n    statusMetadata {\n            id\n            status\n            chain\n            timestamp\n            blockNumber\n            transactionHash\n            blockHash\n    }\n  }\n}\n\nquery StateMachineUpdatesQuery($stateMachineId: String!, $chain: SupportedChain!, $height: BigInt!) {\n  stateMachineUpdateEvents(\n    filter: {and: {stateMachineId: {equalTo: $stateMachineId }, chain: {equalTo: $chain}, height: {greaterThanOrEqualTo: $height}}}\n  ) {\n    nodes {\n      blockHash\n      blockNumber\n      chain\n      height\n      id\n      stateMachineId\n      transactionHash\n    }\n  }\n}\n" ;
	use super::*;
	use serde::{Deserialize, Serialize};
	#[allow(dead_code)]
	type Boolean = bool;
	#[allow(dead_code)]
	type Float = f64;
	#[allow(dead_code)]
	type Int = i64;
	#[allow(dead_code)]
	type ID = String;
	type BigInt = super::BigInt;
	#[derive(Clone, Debug, Eq, PartialEq)]
	pub enum SupportedChain {
		ETHEREUM_SEPOLIA,
		BASE_SEPOLIA,
		OPTIMISM_SEPOLIA,
		ARBITRUM_SEPOLIA,
		BSC_CHAPEL,
		HYPERBRIDGE_GARGANTUA,
		Other(String),
	}
	impl ::serde::Serialize for SupportedChain {
		fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
			ser.serialize_str(match *self {
				SupportedChain::ETHEREUM_SEPOLIA => "ETHEREUM_SEPOLIA",
				SupportedChain::BASE_SEPOLIA => "BASE_SEPOLIA",
				SupportedChain::OPTIMISM_SEPOLIA => "OPTIMISM_SEPOLIA",
				SupportedChain::ARBITRUM_SEPOLIA => "ARBITRUM_SEPOLIA",
				SupportedChain::BSC_CHAPEL => "BSC_CHAPEL",
				SupportedChain::HYPERBRIDGE_GARGANTUA => "HYPERBRIDGE_GARGANTUA",
				SupportedChain::Other(ref s) => &s,
			})
		}
	}
	impl<'de> ::serde::Deserialize<'de> for SupportedChain {
		fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			let s: String = ::serde::Deserialize::deserialize(deserializer)?;
			match s.as_str() {
				"ETHEREUM_SEPOLIA" => Ok(SupportedChain::ETHEREUM_SEPOLIA),
				"BASE_SEPOLIA" => Ok(SupportedChain::BASE_SEPOLIA),
				"OPTIMISM_SEPOLIA" => Ok(SupportedChain::OPTIMISM_SEPOLIA),
				"ARBITRUM_SEPOLIA" => Ok(SupportedChain::ARBITRUM_SEPOLIA),
				"BSC_CHAPEL" => Ok(SupportedChain::BSC_CHAPEL),
				"HYPERBRIDGE_GARGANTUA" => Ok(SupportedChain::HYPERBRIDGE_GARGANTUA),
				_ => Ok(SupportedChain::Other(s)),
			}
		}
	}
	#[derive(Serialize)]
	pub struct Variables {
		#[serde(rename = "stateMachineId")]
		pub state_machine_id: String,
		pub chain: SupportedChain,
		pub height: BigInt,
	}
	impl Variables {}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct ResponseData {
		#[serde(rename = "stateMachineUpdateEvents")]
		pub state_machine_update_events: Option<StateMachineUpdatesQueryStateMachineUpdateEvents>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct StateMachineUpdatesQueryStateMachineUpdateEvents {
		pub nodes: Option<Vec<Option<StateMachineUpdatesQueryStateMachineUpdateEventsNodes>>>,
	}
	#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
	pub struct StateMachineUpdatesQueryStateMachineUpdateEventsNodes {
		#[serde(rename = "blockHash")]
		pub block_hash: String,
		#[serde(rename = "blockNumber")]
		pub block_number: BigInt,
		pub chain: SupportedChain,
		pub height: BigInt,
		pub id: ID,
		#[serde(rename = "stateMachineId")]
		pub state_machine_id: String,
		#[serde(rename = "transactionHash")]
		pub transaction_hash: String,
	}
}
impl graphql_client::GraphQLQuery for StateMachineUpdatesQuery {
	type Variables = state_machine_updates_query::Variables;
	type ResponseData = state_machine_updates_query::ResponseData;
	fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
		graphql_client::QueryBody {
			variables,
			query: state_machine_updates_query::QUERY,
			operation_name: state_machine_updates_query::OPERATION_NAME,
		}
	}
}
