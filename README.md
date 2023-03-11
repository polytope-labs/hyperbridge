# <h1 align="center"> Sync-committee-rs ⚙️ </h1>

<p align="center">
    <strong>Ethereum Beacon Chain Light Client SDK in Rust</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<br/>

The sync-committee-rs is the implementation of the Ethereum beacon light client verifier in Rust. This follows the specifications
initially defined by Seun Lanlege [here](https://polytopelabs.notion.site/ICS-15-ethereum-beacon-chain-light-client-specification-for-IBC-9c28567b02424585b4deceeb21b9beaf)


This library consists of
- ✅ The primitives.
- ✅ The prover.
- ✅ The verifier


## The primitives
Consists of the types and structs as defined and described in the spec mentioned earlier. It also consists of the utility functions
to be used in the verifier and prover.

## The prover
Consists of the various proof generations for the ethereum beacon chain structs/types such as:

- Execution payload
- Finalized header
- Block roots
- Sync committee update

The prover also defines the function for fetching various ethereum types from the beacon chain node using the `SyncCommitteeProver` which can be used to generate proofs.
The various function it defines are:

- fetch_finalized_checkpoint: Fetches the finalized checkpoint for the `head` via this endpoint `eth/v1/beacon/states/{state_id}/finality_checkpoints`
- fetch_header: Fetches the header via the endpoint `/eth/v1/beacon/headers/{block_id}`
- fetch_block: Fetches the Beacon block via the endpoint `/eth/v2/beacon/blocks/{block_id}`
- fetch_sync_committee: Fetches the sync_committee via the endpoint `/eth/v1/beacon/states/{state_id}/sync_committees`
- fetch_validator: Fetches the node validator for a particular state via the endpoint `/eth/v1/beacon/states/{state_id}/validators/{validator_index}`
- fetch_beacon_state: Fetches the Beacon state via the endpoint `/eth/v2/debug/beacon/states/{state_id}`
- fetch_processed_sync_committee: Constructs the actual `SyncCommittee` after aggregating the validator `public_keys`

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
- It verifies the ancestry proofs of a sync committee attestation update. It iterates over the ancestor blocks in the update and for each block, checks if its ancestry proof is either a BlockRoots proof or a HistoricalRoots proof. It then calculates the merkle roots and checks if the merkle branches are valid using the is_valid_merkle_branch function. 
If any of the merkle branches are invalid, the function returns an error.
- It verifies the associated execution header of the finalized beacon header.
- If all the above checks pass, the function returns a new LightClientState object with the updated trusted_state.


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
