# ISMP Parachain Runtime API

This exports the runtime API definitions required by client subsystems like the inherents provider.

## Usage

The required methods are already implemented in [`ismp_parachain::Pallet<T>`](https://docs.rs/ismp-parachain/latest/ismp_parachain/pallet/struct.Pallet.html)

```rust,ignore
impl_runtime_apis! {
    impl ismp_parachain_runtime_api::IsmpParachainApi<Block> for Runtime {
        fn para_ids() -> Vec<u32> {
            ismp_parachain::Pallet::<Runtime>::para_ids()
        }

        fn current_relay_chain_state() -> RelayChainState {
            ismp_parachain::Pallet::<Runtime>::current_relay_chain_state()
        }
    }
}
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
