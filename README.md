# <h1 align="center"> Sync-committee-rs ⚙️ </h1>

<p align="center">
    <strong>Ethereum Beacon Chain Light Client SDK in Rust</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<br/>

The sync-committee-rs is the implementation of the Ethereum beacon light client verifier in Rust. This follows the specifications
initially defined [here](https://polytopelabs.notion.site/ICS-15-ethereum-beacon-chain-light-client-specification-for-IBC-9c28567b02424585b4deceeb21b9beaf)


This library consists of
- ✅ The primitives.
- ✅ The prover.
- ✅ The verifier


## The primitives
Consists of the types and structs as defined and described in the spec mentioned earlier. It also consists of the utility functions
to be used in the verifier and prover, which also defined in the [spec](https://polytopelabs.notion.site/ICS-15-ethereum-beacon-chain-light-client-specification-for-IBC-9c28567b02424585b4deceeb21b9beaf)

## The prover
Consists of the various proof generations for the ethereum beacon chain structs/types such as:

- Execution payload
- Finalized header
- Block roots
- Sync committee update

## The verifier
This consist of the major function for verifying sync committee attestation. It also defines the different error that can occur while verifying.

This contains the `verify_sync_committee_attestation` function. The purpose of this function is to verify that a sync committee attestation, represented by the update argument, 
is valid with respect to the current trusted state, represented by the trusted_state argument.
If the attestation is valid, the function returns the updated trusted state; otherwise, it returns an error.

Detailed explanation of the `verify_sync_committee_attestation` goes as follows:

- It checks whether the update contains the correct number of finality and sync committee branches. If not, it returns an error.
- It verifies whether the number of participants in the sync committee aggregate signature is greater than or equal to two-thirds of the total number of participants. If not, it returns an error.
- It verifies whether the update skips a sync committee period or not. If it does, it returns an error.
- It checks whether the update is relevant by checking whether it attests to a later slot than the trusted_state or contains the next sync committee. If not, it returns an error.
- It verifies the sync committee aggregate signature by checking that it is valid for the given sync committee participants and domain. If not, it returns an error.
- It verifies the finality_branch of the update by checking whether it confirms the finalized_header that matches the finalized checkpoint root saved in the trusted_state. If not, it returns an error.
- It optionally verifies ancestry proofs if they are present.
- It verifies the associated execution payload of the finalized beacon header.
- If all the checks pass, the function returns a new LightClientState.


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
This library is licensed under the [Apache 2.0 License](./LICENSE), Copyright (c) 2023 Polytope Labs.
