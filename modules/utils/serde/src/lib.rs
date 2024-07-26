// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Utilities for serde serialization and deserialization

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;
use alloc::{format, vec::Vec};
use anyhow::anyhow;

const HEX_ENCODING_PREFIX: &str = "0x";

/// Vec from Hex string
pub fn try_bytes_from_hex_str(s: &str) -> Result<Vec<u8>, anyhow::Error> {
	let target = s.replace(HEX_ENCODING_PREFIX, "");
	let data = hex::decode(target).map_err(|e| anyhow!("{e:?}"))?;
	Ok(data)
}

/// Hex serializer and Deserializer for Vec<u8>
pub mod as_hex {
	use super::*;
	use alloc::string::String;
	use serde::de::Deserialize;

	/// Serialize Vec<u8> into hex string
	pub fn serialize<S, T: AsRef<[u8]>>(data: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let encoding = hex::encode(data.as_ref());
		let output = format!("{HEX_ENCODING_PREFIX}{encoding}");
		serializer.collect_str(&output)
	}

	/// Deserialize hex string into Vec<u8>
	pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
	where
		D: serde::Deserializer<'de>,
		T: TryFrom<Vec<u8>>,
	{
		let s = <String>::deserialize(deserializer)?;

		let data = try_bytes_from_hex_str(&s).map_err(serde::de::Error::custom)?;

		let inner = T::try_from(data)
			.map_err(|_| serde::de::Error::custom("type failed to parse bytes from hex data"))?;
		Ok(inner)
	}
}

/// Hex serializer and Deserializer for utf8 bytes
pub mod as_utf8_string {
	use super::*;
	use alloc::string::String;
	use serde::de::Deserialize;

	/// Serialize [u8;4] into a utf8 string
	pub fn serialize<S, T: AsRef<[u8]>>(data: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let output =
			String::from_utf8(data.as_ref().to_vec()).map_err(serde::ser::Error::custom)?;
		serializer.collect_str(&output)
	}

	/// Deserialize a string into utf8 bytes
	pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
	where
		D: serde::Deserializer<'de>,
		T: From<[u8; 4]>,
	{
		let s = <String>::deserialize(deserializer)?;

		let mut bytes = [0u8; 4];
		bytes.copy_from_slice(s.as_bytes());
		Ok(bytes.into())
	}
}

/// Hex serializer and Deserializer for Vec<Vec<u8>>
pub mod seq_of_hex {
	use super::*;
	use core::{fmt, marker::PhantomData};
	use serde::{de::Deserializer, ser::SerializeSeq};

	/// Serialize Vec<Vec<u8>> into an array of hex string
	pub fn serialize<S, T>(data: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
		T: AsRef<[Vec<u8>]>,
	{
		let mut seq = serializer.serialize_seq(None)?;
		for elem in data.as_ref().iter() {
			let encoding = hex::encode(elem);
			let output = format!("{HEX_ENCODING_PREFIX}{encoding}");
			seq.serialize_element(&output)?;
		}
		seq.end()
	}

	struct Visitor(PhantomData<Vec<Vec<u8>>>);

	impl<'de> serde::de::Visitor<'de> for Visitor {
		type Value = Vec<Vec<u8>>;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("sequence of string")
		}

		fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
		where
			S: serde::de::SeqAccess<'de>,
		{
			let mut coll = Vec::with_capacity(access.size_hint().unwrap_or(0));

			while let Some(elem) = access.next_element()? {
				let recovered_elem =
					try_bytes_from_hex_str(elem).map_err(serde::de::Error::custom)?;
				coll.push(recovered_elem);
			}
			Ok(coll)
		}
	}

	/// Deserialize for an array of hex strings into Vec<Vec<u8>>
	pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let data = deserializer.deserialize_seq(Visitor(PhantomData))?;
		Ok(data)
	}
}

/// String serializer and deserializer
pub mod as_string {
	use alloc::{format, string::String};
	use core::{fmt, str::FromStr};
	use serde::de::Deserialize;

	/// Serialize into a string
	pub fn serialize<S, T: fmt::Display>(data: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let output = format!("{data}");
		serializer.collect_str(&output)
	}

	/// Deserialize from string
	pub fn deserialize<'de, D, T: FromStr>(deserializer: D) -> Result<T, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s: String = <String>::deserialize(deserializer)?;
		let inner: T = s
			.parse()
			.map_err(|_| serde::de::Error::custom("failure to parse string data"))?;
		Ok(inner)
	}
}

/// Serializing a sequence of any generic types into a sequence of strings
pub mod seq_of_str {
	use super::*;
	use core::{fmt, marker::PhantomData, str::FromStr};
	use serde::{
		de::{Deserializer, Error},
		ser::SerializeSeq,
	};

	/// Serialize generic type into a sequence of strings
	pub fn serialize<S, T, U>(data: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
		T: AsRef<[U]>,
		U: fmt::Display,
	{
		let mut seq = serializer.serialize_seq(None)?;
		for elem in data.as_ref().iter() {
			let rendered_elem = format!("{elem}");
			seq.serialize_element(&rendered_elem)?;
		}
		seq.end()
	}

	struct Visitor<T>(PhantomData<Vec<T>>);

	impl<'de, T: FromStr> serde::de::Visitor<'de> for Visitor<T> {
		type Value = Vec<T>;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("sequence of string")
		}

		fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
		where
			S: serde::de::SeqAccess<'de>,
		{
			let mut coll = Vec::with_capacity(access.size_hint().unwrap_or(0));

			while let Some(elem) = access.next_element()? {
				let recovered_elem = T::from_str(elem).map_err(|_| {
					Error::custom("failure to parse element of sequence from string")
				})?;
				coll.push(recovered_elem);
			}
			Ok(coll)
		}
	}

	/// Deserialize generic type from a sequence of strings
	pub fn deserialize<'de, D, T, U>(deserializer: D) -> Result<T, D::Error>
	where
		D: Deserializer<'de>,
		T: TryFrom<Vec<U>>,
		U: FromStr,
	{
		let data = deserializer.deserialize_seq(Visitor(PhantomData))?;
		T::try_from(data).map_err(|_| serde::de::Error::custom("failure to parse collection"))
	}
}

#[cfg(test)]
mod test {
	use primitive_types::{H256, H512};

	use ismp::router::{GetRequest, PostRequest, PostResponse};

	#[test]
	fn serialize_and_deserialize_post_request() {
		let post = PostRequest {
			source: ismp::host::StateMachine::Polkadot(100),
			dest: ismp::host::StateMachine::Polkadot(2000),
			nonce: 300,
			from: H256::random().0.to_vec(),
			to: H256::random().0.to_vec(),
			timeout_timestamp: 0,
			body: H512::random().0.to_vec(),
		};

		let serialized = serde_json::to_string(&post).unwrap();

		println!("{serialized:?}\n");

		let deserialized: PostRequest = serde_json::from_str(&serialized).unwrap();

		assert_eq!(post, deserialized);
	}

	#[test]
	fn serialize_and_deserialize_post_response() {
		let post = PostRequest {
			source: ismp::host::StateMachine::Polkadot(100),
			dest: ismp::host::StateMachine::Polkadot(2000),
			nonce: 300,
			from: H256::random().0.to_vec(),
			to: H256::random().0.to_vec(),
			timeout_timestamp: 0,
			body: H512::random().0.to_vec(),
		};

		let response =
			PostResponse { post, response: H512::random().0.to_vec(), timeout_timestamp: 30000 };

		let serialized = serde_json::to_string(&response).unwrap();

		println!("{serialized:?}\n");

		let deserialized: PostResponse = serde_json::from_str(&serialized).unwrap();

		assert_eq!(response, deserialized);
	}

	#[test]
	fn serialize_and_deserialize_get_request() {
		let get = GetRequest {
			source: ismp::host::StateMachine::Polkadot(100),
			dest: ismp::host::StateMachine::Polkadot(2000),
			nonce: 300,
			from: H256::random().0.to_vec(),
			keys: vec![
				H256::random().0.to_vec(),
				H256::random().0.to_vec(),
				H256::random().0.to_vec(),
			],
			timeout_timestamp: 40000,
			height: 289900,
		};

		let serialized = serde_json::to_string(&get).unwrap();

		println!("{serialized:?}\n");

		let deserialized: GetRequest = serde_json::from_str(&serialized).unwrap();

		assert_eq!(get, deserialized);
	}

	#[test]
	fn serialize_state_machine_id() {
		use ismp::{
			consensus::StateMachineId,
			host::{ethereum, StateMachine},
		};
		let state_machine_updated = StateMachineId {
			state_id: StateMachine::Ethereum(ethereum::EXECUTION_LAYER),
			consensus_state_id: *b"ETH0",
		};
		let serialized = serde_json::to_string(&state_machine_updated).unwrap();

		println!("{serialized:?}\n");

		let deserialized: StateMachineId = serde_json::from_str(&serialized).unwrap();

		assert_eq!(state_machine_updated, deserialized);
	}
}
