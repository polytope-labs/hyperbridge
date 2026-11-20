use std::time::Duration;

use anyhow::{bail, Context};
use reqwest::{header::REFERER, Client, RequestBuilder, StatusCode};
use serde_json::Value;

/// A client for a forum API, the current forum's or this one.
pub struct ForumClient {
	client: Client,
	base: String,
	referer: Option<String>,
}

impl ForumClient {
	/// A client whose requests give up rather than hang on a slow server.
	pub fn new(base: &str) -> anyhow::Result<Self> {
		let client = Client::builder()
			.connect_timeout(Duration::from_secs(10))
			.timeout(Duration::from_secs(60))
			.build()
			.context("building the HTTP client")?;
		Ok(Self { client, base: base.trim_end_matches('/').to_owned(), referer: None })
	}

	/// The current forum serves its API under the site and only answers some routes for its own
	/// pages.
	pub fn site(site: &str) -> anyhow::Result<Self> {
		let site = site.trim_end_matches('/');
		Ok(Self { referer: Some(format!("{site}/")), ..Self::new(&format!("{site}/api"))? })
	}

	fn request(&self, path: &str) -> (String, RequestBuilder) {
		let url = format!("{}/{path}", self.base);
		let request = self.client.get(&url);
		match &self.referer {
			Some(referer) => (url, request.header(REFERER, referer)),
			None => (url, request),
		}
	}

	/// A response, or `None` when the route says there is no such item.
	pub async fn find(&self, path: &str) -> anyhow::Result<Option<Value>> {
		let (url, request) = self.request(path);
		let response = request.send().await?;
		if response.status() == StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let value = response
			.error_for_status()?
			.json()
			.await
			.with_context(|| format!("reading {url}"))?;
		Ok(Some(value))
	}

	/// A file the site serves, which is refused once it grows past `max_size` bytes.
	pub async fn file(&self, path: &str, max_size: usize) -> anyhow::Result<Vec<u8>> {
		let (url, request) = self.request(path);
		let mut response = request
			.send()
			.await?
			.error_for_status()
			.with_context(|| format!("fetching {url}"))?;
		let too_large = || format!("{url} is larger than {max_size} bytes");
		if response.content_length().is_some_and(|length| length > max_size as u64) {
			bail!(too_large());
		}
		let mut data = Vec::new();
		while let Some(chunk) = response.chunk().await.with_context(|| format!("reading {url}"))? {
			if data.len() + chunk.len() > max_size {
				bail!(too_large());
			}
			data.extend_from_slice(&chunk);
		}
		Ok(data)
	}

	pub async fn get(&self, path: &str) -> anyhow::Result<Value> {
		self.find(path).await?.with_context(|| format!("{path} not found"))
	}

	/// Every item of a paged list.
	pub async fn all(&self, path: &str) -> anyhow::Result<Vec<Value>> {
		let separator = if path.contains('?') { '&' } else { '?' };
		let mut items = Vec::new();
		for page in 1.. {
			let response = self.get(&format!("{path}{separator}page={page}&page_size=100")).await?;
			let batch = response["items"].as_array().cloned().unwrap_or_default();
			let total = response["total"].as_u64().unwrap_or_default() as usize;
			let done = batch.is_empty() || items.len() + batch.len() >= total;
			items.extend(batch);
			if done {
				break;
			}
		}
		Ok(items)
	}
}
