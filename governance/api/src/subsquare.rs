use anyhow::Context;
use reqwest::{header::REFERER, Client, StatusCode};
use serde_json::Value;

/// A client for a SubSquare style API, SubSquare's own or this one.
pub struct SubSquareApi {
	client: Client,
	base: String,
	referer: Option<String>,
}

impl SubSquareApi {
	pub fn new(base: &str) -> Self {
		Self { client: Client::new(), base: base.trim_end_matches('/').to_owned(), referer: None }
	}

	/// SubSquare serves its API under the site and only answers some routes for its own pages.
	pub fn site(site: &str) -> Self {
		let site = site.trim_end_matches('/');
		Self { referer: Some(format!("{site}/")), ..Self::new(&format!("{site}/api")) }
	}

	/// A response, or `None` when the route says there is no such item.
	pub async fn find(&self, path: &str) -> anyhow::Result<Option<Value>> {
		let url = format!("{}/{path}", self.base);
		let request = self.client.get(&url);
		let request = match &self.referer {
			Some(referer) => request.header(REFERER, referer),
			None => request,
		};
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
