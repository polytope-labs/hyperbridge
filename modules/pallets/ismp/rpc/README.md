# Pallet ISMP RPC

Exports a jsonrpsee handler implementation which provides the neccessary RPC interface for handling ISMP RPC requests.

## Usage

```rust,ignore
/// Full client dependencies
pub struct FullDeps<C, P, B> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// Backend used by the node.
    pub backend: Arc<B>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
    where
        C: ProvideRuntimeApi<Block>,
        C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
        C: Send + Sync + 'static,
        C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
        C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
        C::Api: BlockBuilder<Block>,
        // pallet_ismp_runtime_api bound
        C::Api: pallet_ismp_runtime_api::IsmpRuntimeApi<Block, H256>,
        P: TransactionPool + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut module = RpcModule::new(());
    let FullDeps { client, pool, deny_unsafe, backend } = deps;

    module.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    // IsmpRpcHander goes here
    module.merge(IsmpRpcHandler::new(client, backend)?.into_rpc())?;


    Ok(module)
}
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
