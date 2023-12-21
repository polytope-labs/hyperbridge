use ethers::prelude::{Provider, Ws};
use std::sync::Arc;

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct PolygonPosProver {
    /// Execution Rpc client
    pub client: Arc<Provider<Ws>>,
}

impl PolygonPosProver {
    pub fn new(client: Provider<Ws>) -> Self {
        Self { client: Arc::new(client) }
    }
}
