# ISMP Parachain Inherent

This exports the inherent provider which includes ISMP parachain consensus updates as block
inherents.

## Usage

To use this, you'll need to include the inherent into your collator parameters like so:

```rust,ignore
fn start_consensus(
    client: Arc<FullClient>,
    backend: Arc<FullBackend>,
    block_import: ParachainBlockImport,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<sc_transaction_pool::FullPool<opaque::Block, FullClient>>,
    sync_oracle: Arc<SyncingService<opaque::Block>>,
    keystore: KeystorePtr,
    relay_chain_slot_duration: Duration,
    para_id: ParaId,
    collator_key: CollatorPair,
    overseer_handle: OverseerHandle,
    announce_block: Arc<dyn Fn(opaque::Hash, Option<Vec<u8>>) + Send + Sync>,
) {
    // .. omitted calls

    let (client_clone, relay_chain_interface_clone) =
        (client.clone(), relay_chain_interface.clone());
    let params = lookahead::Params {
        create_inherent_data_providers: move |parent, ()| {
            let client = client_clone.clone();
            let relay_chain_interface = relay_chain_interface_clone.clone();
            async move {
                let inherent = ismp_parachain_inherent::ConsensusInherentProvider::create(
                    parent,
                    client,
                    relay_chain_interface,
                ).await?;

                Ok(inherent)
            }
        },
        ..Default::default()
        // omitted fields
    };

    // ..omitted calls
}
```


## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
