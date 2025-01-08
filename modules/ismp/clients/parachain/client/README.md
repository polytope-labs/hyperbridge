# ISMP Parachain Client

This allows parachains communicate over ISMP leveraging the relay chain as a consensus oracle.

The consensus client relies on a pallet which stores a list of parachains whom we intend to
track their finalized states. This can be paired with an inherent provider which includes the
proofs for the relevant parachains configured in the pallet at every block.

## Overview

The Pallet allows the [`AdminOrigin`](https://docs.rs/pallet-ismp/latest/pallet_ismp/pallet/trait.Config.html#associatedtype.AdminOrigin) configured in [`pallet-ismp`](https://docs.rs/pallet-ismp/latest/pallet_ismp) to dispatch calls for adding and removing parachains from the pallet whitelist.

## License

This library is licensed under the Apache 2.0 License, Copyright (c) 2025 Polytope Labs.
