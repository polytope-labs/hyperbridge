use anyhow::anyhow;
use reqwest::{StatusCode, Url};
use reqwest_chain::Chainer;
use reqwest_middleware::Error;

/// Middleware for switching between providers on failures
pub struct SwitchProviderMiddleware {
    /// Providers for the url
    pub providers: Vec<String>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct LocalState {
    pub active_url_index: usize,
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
        let mut next_state = || {
            let mut next_index = _state.active_url_index + 1;
            if next_index >= self.providers.len() {
                next_index = 0;
            }
            _state.active_url_index = next_index;
            let next_provider = self.providers[next_index].clone();
            let url_ref = request.url_mut();
            let full_url = url_ref.as_str();
            // We split the url at /eth since all our queries are in the /eth namespace
            let host = full_url.split_inclusive("/eth").collect::<Vec<_>>();
            let path = host.get(1).ok_or_else(|| anyhow!("Invalid path in url"))?;
            let new_url = format!("{next_provider}/eth{}", path);
            *url_ref = Url::parse(&new_url).map_err(|e| anyhow!("{e:?}"))?;
            log::trace!(target:"sync-committee-prover", "Retrying request with new proiver {next_provider:?}");
            Ok::<_, anyhow::Error>(())
        };
        match result {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    return Ok(Some(response));
                };
                let _ = next_state()?;
            },
            Err(e) => {
                log::trace!(target:"sync-committee-prover", "Encountered error submitting request, switching provider {e:?}");
                let _ = next_state()?;
            },
        }

        Ok(None)
    }

    fn max_chain_length(&self) -> u32 {
        u32::MAX
    }
}
