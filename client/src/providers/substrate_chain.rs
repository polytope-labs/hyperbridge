use subxt::{OnlineClient, PolkadotConfig};

#[derive(Debug, Clone)]
struct SubstrateClient {
    // WS RPC url of a hyperbridge node
    rpc_url: String,
    // An instance of Hyper bridge client using the default config
    client: OnlineClient<PolkadotConfig>,
}
