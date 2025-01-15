# Pallet ISMP Runtime API

This exports the runtime API definitions required by client subsystems like the RPC.

## Usage

The required methods are already implemented in [`pallet_ismp::Pallet<T>`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/struct.Pallet.html)

```rust,ignore

sp_api::impl_runtime_apis! {
    impl pallet_ismp_runtime_api::IsmpRuntimeApi<Block, <Block as BlockT>::Hash> for Runtime {
        fn host_state_machine() -> StateMachine {
            <Runtime as pallet_ismp::Config>::HostStateMachine::get()
        }

        fn challenge_period(consensus_state_id: [u8; 4]) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::challenge_period(consensus_state_id)
        }

        /// Fetch all ISMP events and their extrinsic metadata, should only be called from runtime-api.
        fn block_events() -> Vec<ismp::events::Event> {
            pallet_ismp::Pallet::<Runtime>::block_events()
        }

        /// Fetch all ISMP events and their extrinsic metadata
        fn block_events_with_metadata() -> Vec<(ismp::events::Event, Option<u32>)> {
            pallet_ismp::Pallet::<Runtime>::block_events_with_metadata()
        }

        /// Return the scale encoded consensus state
        fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
            pallet_ismp::Pallet::<Runtime>::consensus_states(id)
        }

        /// Return the timestamp this client was last updated in seconds
        fn consensus_update_time(id: ConsensusClientId) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::consensus_update_time(id)
        }

        /// Return the latest height of the state machine
        fn latest_state_machine_height(id: StateMachineId) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::latest_state_machine_height(id)
        }


        /// Get actual requests
        fn get_requests(commitments: Vec<H256>) -> Vec<Request> {
            pallet_ismp::Pallet::<Runtime>::requests(commitments)
        }

        /// Get actual requests
        fn get_responses(commitments: Vec<H256>) -> Vec<Response> {
            pallet_ismp::Pallet::<Runtime>::responses(commitments)
        }
    }
}

```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
