# Pallet ISMP

The interoperable state machine protocol implementation for substrate-based chains. This pallet provides the ability to

1. Track the finalized state of a remote state machine (blockchain) through the use of consensus proofs which attest to a finalized "state commitment".
2. Execute incoming ISMP-compliant messages from a connected chain, through the use of state proofs which are verified through a known, previously finalized state commitment.
3. Dispatch ISMP requests and responses to a connected chain.

## Overview

The ISMP Pallet provides calls which allow for:

- Creating consensus clients with their respective unbonding, challenge periods and any initial state machine commitments.
- Updating consensus clients metadata
- Executing ISMP-compliant Messages
- Funding in-flight messages (Request or Response)

To use it in your runtime, you need to implement the ismp
[`pallet_ismp::Config`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html). The supported dispatchable functions are documented in the
[`pallet_ismp::Call`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/enum.Call.html) enum.

### Terminology

- **ISMP:** Interoperable State Machine Protocol, is a framework for secure, cross-chain interoperability. Providing both messaging and state reading capabilities.
- **State Commitment:** This refers to a cryptographic commitment of an entire blockchain state, otherwise known as state root.
- **State Machine:** This refers to the blockchain itself, we identify blockchains as state machines.
- **Consensus State:** This is the minimum data required by consensus client to verify consensus proofs which attest to a newly finalized state.
- **Consensus Client:** This is an algorithm that verifies consensus proofs of a particular consensus mechanism.
- **Unbonding Period:** Refers to how long it takes for validators to unstake their funds from the connected chain.
- **Challenge Period:** A configurable value for how long to wait for state commitments to be challenged, before they can be used to verify incoming requests/responses.

### Dispatchable Functions

- `handle` - Handles incoming ISMP messages.
- `handle_unsigned` Unsigned variant for handling incoming messages, enabled by `feature = ["unsigned"]`
- `create_consensus_client` - Handles creation of various properties for a particular consensus client. Can only be called by the `AdminOrigin`.
- `update_consensus_state` - Updates consensus client properties in storage. Can only be called by the `AdminOrigin`.
- `fund_message` - In cases where the initially provided relayer fees have now become insufficient, due to a transaction fee spike on the destination chain. Allows a user to add more funds to the request to be used for delivery and execution. Should never be called on a completed request.

Please refer to the [`Call`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/enum.Call.html) enum and its associated
variants for documentation on each function.

### Runtime Configuration

The following example shows how to configure `pallet-ismp` in your runtime

```rust,ignore
use frame_support::parameter_types;
use frame_system::EnsureRoot;
use ismp::Error;
use ismp::host::StateMachine;
use ismp::module::IsmpModule;
use ismp::router::{IsmpRouter, Post, Response, Timeout};

parameter_types! {
    // The hyperbridge parachain on Polkadot
    pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
    // The host state machine of this pallet
    pub const HostStateMachine: StateMachine = StateMachine::Polkadot(1000); // your paraId here
}

impl pallet_ismp::Config for Runtime {
    // configure the runtime event
    type RuntimeEvent = RuntimeEvent;
    // Permissioned origin who can create or update consensus clients
    type AdminOrigin = EnsureRoot<AccountId>;
    // The state machine identifier for this state machine
    type HostStateMachine = HostStateMachine;
    // The pallet_timestamp pallet
    type TimestampProvider = Timestamp;
    // The currency implementation that is offered to relayers
    type Currency = Balances;
    // The balance type for the currency implementation
    type Balance = Balance;
    // Router implementation for routing requests/responses to their respective modules
    type Router = Router;
    // Optional coprocessor for incoming requests/responses
    type Coprocessor = Coprocessor;
    // Supported consensus clients
    type ConsensusClients = (
        // as an example, the parachain consensus client
        ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,
    );
    // Offchain database implementation. Outgoing requests and responses are
    // inserted in this database, while their commitments are stored onchain.
    type OffchainDB = ();
    // The fee handler implementation
    type FeeHandler = WeightFeeHandler<()>;
}

#[derive(Default)]
struct Router;
impl IsmpRouter for Router {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        let module = match id.as_slice() {
            YOUR_MODULE_ID => Box::new(YourModule::default()),
            _ => Err(Error::ModuleNotFound(id))?
        };
        Ok(module)
    }
}

/// Some custom module capable of processing some incoming/request or response.
/// This could also be a pallet itself.
#[derive(Default)]
struct YourModule;

pub const YOUR_MODULE_ID: &'static [u8] = &[12, 24, 36, 48];

impl IsmpModule for YourModule {
    /// Called by the ISMP hanlder, to notify module of a new POST request
    /// the module may choose to respond immediately, or in a later block
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        // do something useful with the request
        Ok(())
    }

    /// Called by the ISMP hanlder, to notify module of a response to a previously
    /// sent out request
    fn on_response(&self, response: Response) -> Result<(), Error> {
        // do something useful with the response
        Ok(())
    }

    /// Called by the ISMP hanlder, to notify module of requests that were previously
    /// sent but have now timed-out
	fn on_timeout(&self, request: Timeout) -> Result<(), Error> {
        // revert any state changes that were made prior to dispatching the request
        Ok(())
    }
}
```

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
