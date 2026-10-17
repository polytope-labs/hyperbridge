use anyhow::Context;
use reqwest::{
	header::{CONTENT_TYPE, REFERER},
	Client, RequestBuilder, StatusCode,
};
use serde_json::Value;

/// A client for a forum API, the current forum's or this one.
pub struct ForumClient {
	client: Client,
	base: String,
	referer: Option<String>,
}

impl ForumClient {
	pub fn new(base: &str) -> Self {
		Self { client: Client::new(), base: base.trim_end_matches('/').to_owned(), referer: None }
	}

	/// The current forum serves its API under the site and only answers some routes for its own
	/// pages.
	pub fn site(site: &str) -> Self {
		let site = site.trim_end_matches('/');
		Self { referer: Some(format!("{site}/")), ..Self::new(&format!("{site}/api")) }
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

	/// A file the site serves, with its content type.
	pub async fn file(&self, path: &str) -> anyhow::Result<(String, Vec<u8>)> {
		let (url, request) = self.request(path);
		let response = request
			.send()
			.await?
			.error_for_status()
			.with_context(|| format!("fetching {url}"))?;
		let content_type = response
			.headers()
			.get(CONTENT_TYPE)
			.and_then(|value| value.to_str().ok())
			.unwrap_or("application/octet-stream")
			.to_owned();
		Ok((content_type, response.bytes().await?.to_vec()))
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
