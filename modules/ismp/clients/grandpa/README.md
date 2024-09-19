# ISMP GRANDPA Consensus Client

This allows standalone chains communicate with Hyperbridge over ISMP.

The consensus client relies on a pallet which stores a list of parachains and State machine identifiers authorized to use this client.

## Overview

The Pallet allows the [`AdminOrigin`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html#associatedtype.AdminOrigin) configured in [`pallet-ismp`](https://docs.rs/pallet-ismp/latest/pallet_ismp) to dispatch calls for adding and removing parachains or standalone chains from the pallet whitelist.

## Setting up

When using this consensus client the following should be done in order:
-   Create the consensus state using [`create_consensus_client`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/dispatchables/fn.create_consensus_client.html)

-   The supported parachain ids or state machine identifiers should be whitelisted in the pallet by calling `add_parachains` or `add_state_machines` from the [`AdminOrigin`].</br>
    Note, if a parachain id is not found in the whitelist state machine updates for that parachain will be unavailable.</br>
    If a state machine identifier of a solo chain is not found in the whitelist, ismp messages from that chain will be rejected.

### Dispatchable Functions

- `add_parachains` - Adds some parachains to the list of whitelisted parachains.
- `remove_parachains` - Removes some parachains from the whitelisted parachains.
- `add_state_machines` - Adds some standalone chain state machine identifiers to the whitelist.
- `remove_state_machines` - Removes some standalone chain state machine identifiers from the whitelist.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2024 Polytope Labs.
