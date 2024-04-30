# Pallet ISMP


The interoperable state machine protocol implementation for substrate-based chains. This pallet provides the ability to

1. Track the finalized state of a remote state machine (blockchain) through the use of consensus proofs which attest to a finalized "state commitment".
2. Execute incoming ISMP-compliant messages from a connected chain, through the use of state proofs which are verified through a known, previously finalized state commitment.
3. Dispatch ISMP requests and responses to a connected chain.


## Overview

The ISMP Pallet itself provides Calls which alow for:

* Creating Consensus Clients
* Updating Consensus Clients
* Executing ISMP Messages

To use it in your runtime, you need to implement the ismp
[`pallet_ismp::Config`](pallet/trait.Config.html). The supported dispatchable functions are documented in the
[`pallet_ismp::Call`](pallet/enum.Call.html) enum.


### Terminology

* **ISMP:** Interoperable State Machine Protocol, is a framework for secure, cross-chain interoperability. Providing both messaging and state reading capabilities.
* **State Commitment:** This refers to a cryptographic commitment of an entire blockchain state, otherwise known as state root.
* **State Machine:** This refers to the blockchain itself, we identify blockchains as state machines.
* **Consensus State:** This is the minimum data required by consensus client to verify consensus proofs which attest to a newly finalized state.
* **Consensus Client:** This is an algorithm that verifies consensus proofs of a particular consensus mechanism.
* **Unbonding Period:** Refers to how long it takes for validators to unstake their funds from the connected chain.
* **Challenge Period:** A configurable value for how long to wait for state commitments to be challenged, before they can be used to verify incoming requests/responses.

### Goals

The ISMP pallet in Substrate is designed to make the following possible:

* Create consensus clients with their respective unbonding, challenge periods and any initial state machine commitments.
* Update consensus client metadata.
* Execute ISMP messages

### Dispatchable Functions

* `handle` - Handles incoming ISMP messages.
* `create_consensus_client` - Handles creation of various properties for a particular consensus client. Can only be called by the `AdminOrigin`.
* `update_consensus_state` - Updates consensus client properties in storage. Can only be called by the `AdminOrigin`.


Please refer to the [`Call`](pallet/enum.Call.html) enum and its associated
variants for documentation on each function.

### Runtime Configuration

The following example shows how to configure `pallet-ismp` in your runtime

```rust
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
    type TimeProvider = Timestamp;
    // Router implementation for routing requests/responses to their respective modules
    type Router = Router;
    // Optional coprocessor for incoming requests/responses
    type Coprocessor = Coprocessor;
    // Supported consensus clients
    type ConsensusClients = (
        // as an example, the parachain consensus client
        ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,
    );
    // Optional merkle mountain range overlay tree, for cheaper outgoing request proofs.
    // You most likely don't need it, just use the `NoOpMmrTree`
    type Mmr = NoOpMmrTree;
    // Weight provider for local modules
    type WeightProvider = ();
}

#[derive(Default)]
struct Router;

impl IsmpRouter for Router {
    fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        let module = match id {
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

impl IsmpModule for YourModule {
    /// Called by the ISMP hanlder, to notify module of a new POST request
    /// the module may choose to respond immediately, or in a later block
    fn on_accept(&self, request: PostRequest) -> Result<(), Error> {
        // do something useful with the request
    }

    /// Called by the ISMP hanlder, to notify module of a response to a previously
    /// sent out request
	fn on_response(&self, response: Response) -> Result<(), Error> {
        // do something useful with the response
    }

    /// Called by the ISMP hanlder, to notify module of requests that were previously
    /// sent but have now timed-out
	fn on_timeout(&self, request: Timeout) -> Result<(), Error> {
        // revert any state changes that were made prior to dispatching the request
    }
}
```
