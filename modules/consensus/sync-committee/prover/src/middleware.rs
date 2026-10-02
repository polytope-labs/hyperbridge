use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use reqwest::{StatusCode, Url};
use reqwest_chain::Chainer;
use reqwest_middleware::Error;

/// Middleware for switching between providers on failures
pub struct SwitchProviderMiddleware {
	/// Providers for the url
	pub providers: Vec<String>,
}

#[derive(Default, Debug, Clone)]
pub struct LocalState {
	pub active_url_index: usize,
	pub prev_stat: HashMap<usize, Option<StatusCode>>,
	/// How many times this request has already been retried.
	pub retries: u32,
}

impl SwitchProviderMiddleware {
	pub fn _new(providers: Vec<String>) -> Self {
		Self { providers }
	}
}

#[async_trait::async_trait]
impl Chainer for SwitchProviderMiddleware {
	type State = LocalState;

	async fn chain(
		&self,
		result: Result<reqwest::Response, Error>,
		_state: &mut Self::State,
		request: &mut reqwest::Request,
	) -> Result<Option<reqwest::Response>, Error> {
		let mut next_state = |status: Option<StatusCode>| {
			// Only beacon requests can be moved between providers. `fetch_execution_header` goes to
			// the execution rpc, which is a separate endpoint that is not in this list, so there is
			// nothing to switch it to. Retry it where it is instead of rewriting its url.
			let is_beacon_request = self
				.providers
				.iter()
				.any(|provider| request.url().as_str().starts_with(provider.trim_end_matches('/')));
			if !is_beacon_request {
				log::trace!(
					target: "sync-committee-prover",
					"Retrying {} against the same endpoint, status {status:?}",
					request.url(),
				);
				return Ok(());
			}

			let active_index = _state.active_url_index;
			_state.prev_stat.insert(active_index, status);
			let mut next_index = _state.active_url_index + 1;
			if next_index >= self.providers.len() {
				// If resource is not available on all providers we terminate the chain
				if _state.prev_stat.iter().all(|(_, stat)| stat == &Some(StatusCode::NOT_FOUND)) {
					Err(anyhow!("All providers returned {:?}", StatusCode::NOT_FOUND))?
				}
				next_index = 0;
			}
			_state.active_url_index = next_index;
			let next_provider = self.providers[next_index].clone();
			let url_ref = request.url_mut();
			let full_url = url_ref.as_str();
			// We split the url at /eth since all our queries are in the /eth namespace
			let host = full_url.split_inclusive("/eth").collect::<Vec<_>>();
			let path = host.get(1).ok_or_else(|| anyhow!("Invalid path in url"))?;
			let new_url = format!("{next_provider}/eth{path}");
			*url_ref = Url::parse(&new_url).map_err(|e| anyhow!("{e:?}"))?;
			log::trace!(target:"sync-committee-prover", "Retrying request with new provider {next_provider:?}");
			Ok::<_, anyhow::Error>(())
		};
		match result {
			Ok(response) => {
				if response.status() == StatusCode::OK {
					return Ok(Some(response));
				};

				let _ = next_state(Some(response.status()))?;
			},
			Err(e) => {
				log::trace!(target:"sync-committee-prover", "Possibly encountered an os error submitting request, switching provider {e:?}");
				let _ = next_state(None)?;
			},
		}
		// Retry once straight away before backing off. Endpoints that sit behind a proxy tend to
		// fail the first request after an idle period and serve the one right behind it, so a
		// delay on every attempt turns a blip into an exhausted chain.
		_state.retries += 1;
		if _state.retries > 1 {
			tokio::time::sleep(Duration::from_secs(15)).await;
		}
		Ok(None)
	}

	fn max_chain_length(&self) -> u32 {
		// At least three retries for each provider
		(self.providers.len() * 3) as u32
	}
}
