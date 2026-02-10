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

//! CatFee API client for purchasing TRON energy.
//!
//! This module integrates with the CatFee service (https://catfee.io) to purchase
//! energy resources before submitting transactions, reducing TRX costs.
//!
//! ## Authentication
//! CatFee requires HMAC-SHA256 signatures for all API requests:
//! - Header: CF-ACCESS-KEY (your API key)
//! - Header: CF-ACCESS-SIGN (Base64 encoded HMAC-SHA256 signature)
//! - Header: CF-ACCESS-TIMESTAMP (ISO 8601 timestamp, e.g., 2023-08-26T12:34:56.789Z)
//!
//! Signature format: `Base64(HMAC-SHA256(secret, timestamp + method + requestPath))`
//! where requestPath includes query parameters (e.g.,
//! `/v1/order?quantity=65000&receiver=ADDR&duration=1h`)

use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::time::{Duration, SystemTime};

type HmacSha256 = Hmac<Sha256>;

/// Base URL for the CatFee API (production)
const CATFEE_API_BASE: &str = "https://api.catfee.io";

/// Configuration for the CatFee API client
#[derive(Debug, Clone)]
pub struct CatFeeConfig {
	/// API key for authentication (required)
	pub api_key: String,
	/// API secret for HMAC signature generation (required)
	pub api_secret: String,
	/// Base URL for the CatFee API
	pub api_base: String,
	/// HTTP request timeout
	pub timeout: Duration,
}

impl Default for CatFeeConfig {
	fn default() -> Self {
		Self {
			api_key: String::new(),
			api_secret: String::new(),
			api_base: CATFEE_API_BASE.to_string(),
			timeout: Duration::from_secs(30),
		}
	}
}

/// CatFee API client
#[derive(Clone, Debug)]
pub struct CatFeeClient {
	client: reqwest::Client,
	config: CatFeeConfig,
}

impl CatFeeClient {
	/// Create a new CatFee client
	pub fn new(config: CatFeeConfig) -> anyhow::Result<Self> {
		if config.api_key.is_empty() {
			return Err(anyhow!("CatFee API key is required"));
		}
		if config.api_secret.is_empty() {
			return Err(anyhow!("CatFee API secret is required"));
		}

		let client = reqwest::Client::builder().timeout(config.timeout).build()?;

		Ok(Self { client, config })
	}

	/// Generate HMAC signature for API request
	///
	/// Signature format: Base64(HMAC-SHA256(secret, timestamp + method + requestPath))
	/// where requestPath includes query parameters
	fn generate_signature(&self, timestamp: &str, method: &str, request_path: &str) -> String {
		let message = format!("{}{}{}", timestamp, method, request_path);

		let mut mac = HmacSha256::new_from_slice(self.config.api_secret.as_bytes())
			.expect("HMAC can take key of any size");
		mac.update(message.as_bytes());

		BASE64.encode(mac.finalize().into_bytes())
	}

	/// Get current timestamp in ISO 8601 format
	/// Format: 2023-08-26T12:34:56.789Z
	fn get_timestamp() -> String {
		let now = SystemTime::now();
		let datetime: chrono::DateTime<chrono::Utc> = now.into();
		datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
	}

	/// Create an order to purchase energy
	///
	/// API: POST /v1/order?count={energy_amount}&target_address={receiver}&period={hours}
	///
	/// # Arguments
	/// * `energy_amount` - Amount of energy to purchase (in energy units)
	/// * `receiver_address` - TRON address that will receive the energy (base58)
	/// * `period` - Period in hours (1 or 24). Note: API returns duration in minutes.
	///
	/// # Returns
	/// Order response with order ID and status. Duration field will be in minutes (e.g., 60 for 1
	/// hour).
	pub async fn create_order(
		&self,
		energy_amount: u64,
		receiver_address: &str,
		period: u32,
	) -> anyhow::Result<CreateOrderResponse> {
		// Validate period
		if period != 1 && period != 24 {
			return Err(anyhow!("Invalid period: must be 1 or 24 hours"));
		}

		let request_path = format!(
			"/v1/order?quantity={}&receiver={}&duration={}h",
			energy_amount, receiver_address, period
		);
		let timestamp = Self::get_timestamp();
		let signature = self.generate_signature(&timestamp, "POST", &request_path);

		let url = format!("{}{}", self.config.api_base, request_path);
		log::trace!(
			"Creating order: energy={}, receiver={}, period={}h",
			energy_amount,
			receiver_address,
			period
		);

		let response = self
			.client
			.post(&url)
			.header("Content-Type", "application/json")
			.header("CF-ACCESS-KEY", &self.config.api_key)
			.header("CF-ACCESS-SIGN", &signature)
			.header("CF-ACCESS-TIMESTAMP", &timestamp)
			.send()
			.await
			.context("Failed to send create order request")?;

		let status = response.status();
		let text = response.text().await?;

		log::trace!("Create order response: status={}", status);

		if !status.is_success() {
			log::error!("Create order failed with HTTP {}: {}", status, text);
			return Err(anyhow!("CatFee API returned HTTP {}: {}", status, text));
		}

		let api_response: ApiResponse<CreateOrderResponse> =
			serde_json::from_str(&text).context("Failed to parse create order response")?;

		if api_response.code != 0 {
			return Err(anyhow!("CatFee API error code {}: {}", api_response.code, text));
		}

		api_response
			.data
			.ok_or_else(|| anyhow!("CatFee API returned success but no order data"))
	}

	/// Query detailed order information
	///
	/// API: GET /v1/order/{order_id}
	///
	/// # Arguments
	/// * `order_id` - The order ID to query
	///
	/// # Returns
	/// Detailed order status and information
	pub async fn get_order_detail(&self, order_id: &str) -> anyhow::Result<OrderDetailResponse> {
		let request_path = format!("/v1/order/{}", order_id);
		let timestamp = Self::get_timestamp();
		let signature = self.generate_signature(&timestamp, "GET", &request_path);

		let url = format!("{}{}", self.config.api_base, request_path);
		log::trace!("Querying order detail: {}", order_id);

		let response = self
			.client
			.get(&url)
			.header("Content-Type", "application/json")
			.header("CF-ACCESS-KEY", &self.config.api_key)
			.header("CF-ACCESS-SIGN", &signature)
			.header("CF-ACCESS-TIMESTAMP", &timestamp)
			.send()
			.await
			.context("Failed to send order detail request")?;

		let status = response.status();
		let text = response.text().await?;

		if !status.is_success() {
			log::error!("Order detail query failed with HTTP {}: {}", status, text);
			return Err(anyhow!("CatFee API returned HTTP {}: {}", status, text));
		}

		let api_response: ApiResponse<OrderDetailResponse> =
			serde_json::from_str(&text).context("Failed to parse order detail response")?;

		if api_response.code != 0 {
			return Err(anyhow!("CatFee API error code {}: {}", api_response.code, text));
		}

		api_response
			.data
			.ok_or_else(|| anyhow!("CatFee API returned success but no order detail"))
	}

	/// Wait for an order to be confirmed and completed
	///
	/// This polls the order detail endpoint until the order reaches delegation confirmed status.
	///
	/// # Arguments
	/// * `order_id` - The order ID to wait for
	/// * `max_wait` - Maximum time to wait for completion
	///
	/// # Returns
	/// Final order details
	pub async fn wait_for_order_completion(
		&self,
		order_id: &str,
		max_wait: Duration,
	) -> anyhow::Result<OrderDetailResponse> {
		let poll_interval = Duration::from_secs(3);
		let start = tokio::time::Instant::now();

		loop {
			let detail = self.get_order_detail(order_id).await?;

			// Check order status
			// Status: DELEGATE_SUCCESS means submitted to blockchain
			// ConfirmStatus: DELEGATION_CONFIRMED means successfully delivered
			if detail.confirm_status == ConfirmStatus::DelegationConfirmed {
				log::info!(
					"Order {} completed successfully (status={:?}, confirm={:?})",
					order_id,
					detail.status,
					detail.confirm_status
				);
				return Ok(detail);
			}

			let elapsed = start.elapsed();
			if elapsed >= max_wait {
				return Err(anyhow!(
					"Timeout waiting for CatFee order {} to complete after {}s. Status: {:?}, Confirm: {:?}",
					order_id,
					elapsed.as_secs(),
					detail.status,
					detail.confirm_status
				));
			}

			log::trace!(
				"Order {} status: {:?}, confirm: {:?}, waiting... ({}s elapsed)",
				order_id,
				detail.status,
				detail.confirm_status,
				elapsed.as_secs()
			);

			tokio::time::sleep(poll_interval).await;
		}
	}

	/// Complete purchase flow: create order and wait for confirmation
	///
	/// This is a convenience method that combines create_order and
	/// wait_for_order_completion into a single operation.
	///
	/// # Arguments
	/// * `energy_amount` - Amount of energy to purchase
	/// * `receiver_address` - TRON address that will receive the energy (base58)
	/// * `period` - Period in hours (1 or 24). Note: API returns duration in minutes.
	/// * `max_wait` - Maximum time to wait for order completion
	///
	/// # Returns
	/// Final order details after successful completion. Duration field will be in minutes.
	pub async fn purchase_energy(
		&self,
		energy_amount: u64,
		receiver_address: &str,
		period: u32,
		max_wait: Duration,
	) -> anyhow::Result<OrderDetailResponse> {
		// Step 1: Create the order
		let order = self.create_order(energy_amount, receiver_address, period).await?;

		log::info!(
			"Order created: id={}, status={:?}, confirm={:?}",
			order.id,
			order.status,
			order.confirm_status
		);

		// Step 2: Wait for order to be completed
		log::info!("Waiting for order {} to complete...", order.id);
		let final_detail = self.wait_for_order_completion(&order.id, max_wait).await?;

		Ok(final_detail)
	}
}

/// Generic API response wrapper
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
struct ApiResponse<T> {
	/// Response code: 0 = success, non-zero = error
	code: i32,
	/// Response data (present on success)
	#[serde(default)]
	data: Option<T>,
}

/// Response from create order endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderResponse {
	/// Order ID
	pub id: String,
	/// Resource type (ENERGY or BANDWIDTH)
	pub resource_type: ResourceType,
	/// Billing type
	pub billing_type: BillingType,
	/// Source type
	pub source_type: SourceType,
	/// Payment timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub pay_timestamp: Option<i64>,
	/// Receiver address
	pub receiver: String,
	/// Delegation transaction hash
	#[serde(default)]
	pub delegate_hash: Option<String>,
	/// Delegation timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub delegate_timestamp: Option<i64>,
	/// Reclaim transaction hash
	#[serde(default)]
	pub reclaim_hash: Option<String>,
	/// Reclaim timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub reclaim_timestamp: Option<i64>,
	/// Payment amount in SUN
	pub pay_amount_sun: i64,
	/// Activation amount in SUN
	#[serde(default)]
	pub activate_amount_sun: Option<i64>,
	/// Amount of resource ordered
	pub quantity: i32,
	/// Staked amount in SUN (optional, may not be present in initial response)
	#[serde(default)]
	pub staked_sun: Option<i64>,
	/// Duration in minutes
	pub duration: i32,
	/// Expiration timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub expired_timestamp: Option<i64>,
	/// Order status
	pub status: OrderStatus,
	/// Activation status
	pub activate_status: ActivateStatus,
	/// Confirmation status
	pub confirm_status: ConfirmStatus,
	/// Balance in SUN
	#[serde(default)]
	pub balance: Option<i64>,
}

/// Resource type enum
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceType {
	Energy,
	Bandwidth,
}

/// Order status enum
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
	/// Payment was successful
	PaymentSuccess,
	/// Delegation was successful
	DelegateSuccess,
	/// Insufficient balance
	InsufficientBalance,
	/// Quantity too high
	QuantityTooHigh,
	/// Quantity too low
	QuantityTooLow,
	/// Address not activated
	AddressNotActivated,
	/// Invalid address
	InvalidAddress,
	/// Unknown status
	#[serde(other)]
	Unknown,
}

/// Confirmation status enum
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfirmStatus {
	/// Not yet confirmed
	Unconfirmed,
	/// Delegation confirmed on-chain
	DelegationConfirmed,
	/// Delegation confirmation failed
	DelegationConfirmedFail,
	/// Reclaim confirmed
	ReclaimConfirmed,
	/// Reclaim confirmation failed
	ReclaimConfirmedFail,
}

/// Activation status enum
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivateStatus {
	Deactivate,
	Activate,
	Activated,
	AlreadyActivated,
}

/// Billing type enum
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BillingType {
	Transfer,
	Dapp,
	Balance,
	Api,
	Bot,
}

/// Source type enum (same values as BillingType)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceType {
	Transfer,
	Dapp,
	Balance,
	Api,
	Bot,
}

/// Response from order detail endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct OrderDetailResponse {
	/// Order ID
	pub id: String,
	/// Resource type (ENERGY or BANDWIDTH)
	pub resource_type: ResourceType,
	/// Billing type
	pub billing_type: BillingType,
	/// Source type
	pub source_type: SourceType,
	/// Payment timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub pay_timestamp: Option<i64>,
	/// Delegation timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub delegate_timestamp: Option<i64>,
	/// Reclaim timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub reclaim_timestamp: Option<i64>,
	/// Expiration timestamp (Unix timestamp in milliseconds)
	#[serde(default)]
	pub expired_timestamp: Option<i64>,
	/// Receiver address
	pub receiver: String,
	/// Delegation transaction hash
	#[serde(default)]
	pub delegate_hash: Option<String>,
	/// Reclaim transaction hash
	#[serde(default)]
	pub reclaim_hash: Option<String>,
	/// Payment amount in SUN
	pub pay_amount_sun: i64,
	/// Activation amount in SUN
	#[serde(default)]
	pub activate_amount_sun: Option<i64>,
	/// Amount of resource ordered
	pub quantity: i32,
	/// Staked amount in SUN (optional, may not be present in all responses)
	#[serde(default)]
	pub staked_sun: Option<i64>,
	/// Duration in minutes
	pub duration: i32,
	/// Order status
	pub status: OrderStatus,
	/// Activation status
	pub activate_status: ActivateStatus,
	/// Confirmation status
	pub confirm_status: ConfirmStatus,
	/// Balance in SUN
	#[serde(default)]
	pub balance: Option<i64>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_catfee_config_default() {
		let config = CatFeeConfig::default();
		assert_eq!(config.api_key, "");
		assert_eq!(config.api_secret, "");
		assert_eq!(config.api_base, CATFEE_API_BASE);
		assert_eq!(config.timeout, Duration::from_secs(30));
	}

	#[test]
	fn test_catfee_client_requires_api_key() {
		let config = CatFeeConfig {
			api_key: String::new(),
			api_secret: "secret".to_string(),
			..Default::default()
		};
		let result = CatFeeClient::new(config);
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("API key"));
	}

	#[test]
	fn test_catfee_client_requires_api_secret() {
		let config = CatFeeConfig {
			api_key: "key".to_string(),
			api_secret: String::new(),
			..Default::default()
		};
		let result = CatFeeClient::new(config);
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("secret"));
	}

	#[test]
	fn test_catfee_client_requires_both_credentials() {
		let config = CatFeeConfig {
			api_key: "key".to_string(),
			api_secret: String::new(),
			..Default::default()
		};
		assert!(CatFeeClient::new(config).is_err());

		let config = CatFeeConfig {
			api_key: String::new(),
			api_secret: "secret".to_string(),
			..Default::default()
		};
		assert!(CatFeeClient::new(config).is_err());
	}

	#[test]
	fn test_hmac_signature_generation() {
		let config = CatFeeConfig {
			api_key: "test_key".to_string(),
			api_secret: "test_secret".to_string(),
			..Default::default()
		};
		let client = CatFeeClient::new(config).unwrap();

		let timestamp = "2023-08-26T12:34:56.789Z";
		let sig = client.generate_signature(
			timestamp,
			"POST",
			"/v1/order?quantity=65000&receiver=TRON_ADDRESS&duration=1h",
		);
		// Should produce a base64 string
		assert!(!sig.is_empty());
		assert!(BASE64.decode(&sig).is_ok());
	}

	/// Configuration for CatFee integration test
	#[derive(Debug, serde::Deserialize)]
	struct CatFeeTestConfig {
		/// Your CatFee API key
		catfee_api_key: String,
		/// Your CatFee API secret
		catfee_api_secret: String,
		/// Custom API base URL (defaults to production)
		#[serde(default = "default_api_base")]
		catfee_api_base: String,
		/// TRON address to receive energy (base58 format)
		catfee_receiver_address: String,
		/// Amount of energy to purchase (e.g., 65000)
		catfee_energy_amount: u64,
		/// Rental period in hours - either 1 or 24
		catfee_period: u32,
	}

	fn default_api_base() -> String {
		CATFEE_API_BASE.to_string()
	}

	/// Integration test for purchasing energy via CatFee API.
	///
	/// This test requires the following environment variables:
	/// - CATFEE_API_KEY: Your CatFee API key
	/// - CATFEE_API_SECRET: Your CatFee API secret
	/// - CATFEE_API_BASE: (Optional) Custom API base URL (defaults to production)
	/// - CATFEE_RECEIVER_ADDRESS: TRON address to receive energy (base58 format)
	/// - CATFEE_ENERGY_AMOUNT: Amount of energy to purchase (e.g., "65000")
	/// - CATFEE_PERIOD: Rental period in hours - either "1" or "24"
	///
	/// Run with: cargo test test_purchase_energy -- --ignored --nocapture
	#[tokio::test]
	#[ignore] // Ignore by default since it requires API credentials and makes real API calls
	async fn test_purchase_energy() {
		// Initialize logger to see CatFee API logs
		let _ = env_logger::builder().is_test(true).try_init();

		// Load and parse environment variables using envy
		let test_config: CatFeeTestConfig = envy::from_env().expect(
			"Failed to load environment variables. Make sure all required CATFEE_* variables are set.",
		);

		// Validate period
		assert!(
			test_config.catfee_period == 1 || test_config.catfee_period == 24,
			"CATFEE_PERIOD must be either 1 or 24"
		);

		// Create CatFee client
		let config = CatFeeConfig {
			api_key: test_config.catfee_api_key,
			api_secret: test_config.catfee_api_secret,
			api_base: test_config.catfee_api_base,
			timeout: Duration::from_secs(30),
		};

		let client = CatFeeClient::new(config).expect("Failed to create CatFee client");

		// Purchase energy with a 2-minute timeout for order completion
		let max_wait = Duration::from_secs(120);
		let result = client
			.purchase_energy(
				test_config.catfee_energy_amount,
				&test_config.catfee_receiver_address,
				test_config.catfee_period,
				max_wait,
			)
			.await;

		match result {
			Ok(order_detail) => {
				println!("✅ Energy purchase successful!");
				println!("Order ID: {}", order_detail.id);
				println!("Receiver: {}", order_detail.receiver);
				println!("Energy Amount: {}", order_detail.quantity);
				println!("Duration: {} minutes", order_detail.duration);
				println!("Status: {:?}", order_detail.status);
				println!("Confirm Status: {:?}", order_detail.confirm_status);
				println!("Paid Amount (SUN): {}", order_detail.pay_amount_sun);
				if let Some(staked_sun) = order_detail.staked_sun {
					println!("Staked Amount (SUN): {}", staked_sun);
				}

				if let Some(delegate_hash) = &order_detail.delegate_hash {
					println!("Delegation TX: {}", delegate_hash);
				}

				// Verify order completed successfully
				assert_eq!(order_detail.confirm_status, ConfirmStatus::DelegationConfirmed);
				assert_eq!(order_detail.quantity, test_config.catfee_energy_amount as i32);
				assert_eq!(order_detail.receiver, test_config.catfee_receiver_address);
				// Duration is in minutes, so period in hours * 60
				assert_eq!(order_detail.duration, (test_config.catfee_period * 60) as i32);
			},
			Err(e) => {
				panic!("❌ Energy purchase failed: {}", e);
			},
		}
	}
}
