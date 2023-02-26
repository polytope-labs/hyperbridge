# ethereum-beacon-light-client
Implementation of the Ethereum beacon chain light client in Rust

# Running the prover tests
**NOTE**
1. To run these tests make sure the latest fork version on your devnet is the BELLATRIX_FORK_VERSION as defined in the mainnet config  
2. Modify `sync_committee_primitives::types::GENESIS_ROOT_VALIDATORS` defined under the testing  
   feature flag to match the one that is present in the devnet you are running the tests with
3. Make sure the SLOTS_PER_EPOCH is set to 32 in your devnet.  