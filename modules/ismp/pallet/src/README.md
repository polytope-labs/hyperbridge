# Pallet ISMP

A module for the ISMP which is used to create/update consensus clients and also handle messages.


## Overview

The ISMP Pallet provides functionality which includes:

* Creating Consensus Clients
* Updating Consensus Clients
* Handle Messages
* Validating Messages

To use it in your runtime, you need to implement the ismp
[`ismp::Config`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html).

The supported dispatchable functions are documented in the
[`ismp::Call`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/enum.Call.html) enum.


### Terminology

* **ISMP:** Interoperable State Machine Protocol, which is the protocol responsible for handling request messages and response.
* **State Commitment:** This refers to a cryptographic commitment of an entire Blockchain state, otherwise known as state root.
* **State Machine:** This refers to the Blockchain itself, we identify Blockchains as State Machines since a Blockchain represent states of a particular network.
* **Consensus State:** This is the minimum data required by Consensus Client to verify consensus messages.
* **Consensus Client:** This verifies consensus proofs of a particular state machine.
* **Unbonding Period:** Refers to the period at which unbonding occurs.
* **Challenge Period:** Refers to the period at which challenge occurs.

### Goals

The ISMP pallet in Substrate is designed to make the following possible:

* Create Consensus Clients with their respective unbonding, challenge periods with their state machine commitments .
* Update Consensus Clients.
* Handles Messages.
* Validates ISMP messages using an unsigned origin.

## Interface

### Dispatchable Functions

* `handle` - Handles ISMP messages.
* `create_consensus_client` - Handles creation of various properties for a particular consensus client.
* `update_consensus_state` - Updates consensus client properties in storage.
* `validate_messages` - An unsigned call to validate ISMP messages.

Please refer to the [`Call`](https://docs.rs/pallet-ismp/latest/pallet_ismp/enum.Call.html) enum and its associated
variants for documentation on each function.

### Runtime Usage

```rust
impl pallet_ismp::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = EnsureRoot<AccountId>;
    type HostStateMachine = HostStateMachine;
    type TimeProvider = Timestamp;
    type Router = Router;
    type Coprocessor = Coprocessor;
    type ConsensusClients = (
        ismp_bsc::BscClient<Host<Runtime>>,
        ismp_sync_committee::SyncCommitteeConsensusClient<Host<Runtime>, Mainnet>,
    );

    type Mmr = Mmr;
    type WeightProvider = ();
}
```


* `RuntimeEvent` -  The runtime event
* `AdminOrigin` -  The origin allowed to execute the extrinsic in the pallet.
* `HostStateMachine` -  The state machine(Blockchain) that is hosting this pallet.
* `TimeProvider` -  The timestamp used for this pallet.
* `Router` -  The implementation required for receiving request or response based on the Module defined.
* `Coprocessor` -  This represents the state machine proxy allowed for verifying requests.
* `ConsensusClients` -  The consensus clients supported for verification.
* `Mmr` -  Mmr used, this is not compulsory.
* `WeightProvider` -  The weights, this is not compulsory.

