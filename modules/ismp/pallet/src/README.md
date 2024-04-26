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
[`assets::Call`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/enum.Call.html) enum.


### Terminology

* **ISMP:** Interoperable State Machine Protocol, which is the protocol responsible for handling request messages and response.
* **State Commitment:** This refers to the state root of a particular state machine.
* **State Machine:** This refers to the Blockchain itself, we identify Blockchains as State Machines since a Blockchain represent states of a particular network.
* **Consensus State:** Refers to the particular state of a consensus.
* **Consensus Client:** Refers to the Blockchain client.
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
