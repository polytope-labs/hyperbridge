# <h1 align="center"> sync-committee-rs ⚙️ </h1>

<p align="center">
    <strong>Ethereum Beacon Chain Light Client SDK in Rust</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<br/>

The sync-committee-rs is the implementation of the Ethereum beacon light client prover & verifier in Rust. This is based on the research done here: https://research.polytope.technology/ethereum-light-client


This library consists of
- ✅ The primitives.
- ✅ The prover.
- ✅ The verifier


## primitives
Consists of the types and structs as defined and described in the spec mentioned earlier. It also consists of the utility functions
to be used in the verifier and prover.

## prover
Consists of the various proof generations for the ethereum beacon chain structs/types such as:

- Execution payload
- Finalized header
- Block roots
- Sync committee update

## verifier
This exports a single function for verifying ethereum's sync committee attestation.


# Major Depedencies
The major dependencies for this SDK/Library are:

- [ssz-rs](https://github.com/ralexstokes/ssz-rs)
- [ethereum-consensus](https://github.com/ralexstokes/ethereum-consensus)


# Running the prover tests
**NOTE**
1. To run these tests make sure the latest fork version on your devnet is the BELLATRIX_FORK_VERSION as defined in the mainnet config
2. Modify `sync_committee_primitives::types::GENESIS_ROOT_VALIDATORS` defined under the testing
   feature flag to match the one that is present in the devnet you are running the tests with
3. Make sure the SLOTS_PER_EPOCH is set to 32 in your devnet.


## License
This library is licensed under the [Apache 2.0 License](./LICENSE), Copyright (c) 2025 Polytope Labs.
