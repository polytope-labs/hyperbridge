use anyhow::anyhow;
use reqwest::{header::HOST, StatusCode};
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
        let next_state = || {
            let next_index = _state.active_url_index + 1;
            if next_index >= self.providers.len() {
                return Err(anyhow!("Providers have been exhausted"));
            }
            Ok(next_index)
        };
        match result {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    return Ok(Some(response));
                };
                let next_index = next_state()?;
                _state.active_url_index = next_index;
                let url = self.providers[next_index].clone();
                request.headers_mut().insert(HOST, url.parse().map_err(|e| anyhow!("{e:?}"))?);
            },
            Err(e) => {
                log::trace!("Encountered error submitting http, switching provider request {e:?}");
                let next_index = next_state()?;
                _state.active_url_index = next_index;
                let url = self.providers[next_index].clone();

                request.headers_mut().insert(HOST, url.parse().map_err(|e| anyhow!("{e:?}"))?);
            },
        }

        Ok(None)
    }
}
