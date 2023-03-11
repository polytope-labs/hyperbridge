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
