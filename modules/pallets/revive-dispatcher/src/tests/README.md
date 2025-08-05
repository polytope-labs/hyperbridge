# Test Runtime for pallet-revive-ismp-dispatcher

This directory contains a test runtime setup for the `pallet-revive-ismp-dispatcher` with all the required pallets configured.

## Overview

The test runtime includes the following pallets:

1. **frame_system** - Core system functionality
2. **pallet_balances** - Native currency management
3. **pallet_assets** - Multi-asset functionality
4. **pallet_timestamp** - Block timestamp provider
5. **pallet_transaction_payment** - Transaction fee handling
6. **pallet_revive** - PolkaVM smart contract execution
7. **pallet_ismp** - Interoperable State Machine Protocol
8. **pallet_hyperbridge** - Cross-chain messaging hub

## Structure

- `mock.rs` - Contains the mock runtime configuration with all pallets properly configured
- `mod.rs` - Contains the actual test cases

## Key Components

### Mock Runtime Configuration

The mock runtime (`Test`) is configured with:
- AccountId: `AccountId32`
- Balance: `u128`
- Block number: `u64`
- Hash: `H256`

### Mock Implementations

- **MockDispatcher**: A mock ISMP dispatcher that returns random commitments for testing
- **MockExt**: A mock implementation of the `pallet_revive::precompiles::Ext` trait for testing precompile calls

### Test Coverage

The tests cover the following functionality:

1. **Basic Queries**:
   - Getting the host state machine
   - Getting the hyperbridge coprocessor
   - Getting the current nonce
   - Getting the fee token address
   - Getting per-byte fees for different destinations

2. **Message Dispatching**:
   - Dispatching POST requests
   - Dispatching GET requests
   - Dispatching responses

3. **Fee Management**:
   - Funding requests
   - Funding responses

4. **Error Handling**:
   - Invalid state machine names
   - Invalid parameters

## Running Tests

To run the tests, use:

```bash
cd modules/pallets/revive-dispatcher
cargo test --features std
```

Note: The workspace requires a nightly Rust compiler due to some dependencies using edition2024 features.

## Configuration Notes

### ISMP Configuration
- Host State Machine: `Kusama(2000)`
- Coprocessor: `Ethereum(ExecutionLayer)`

### Revive Configuration
- Uses default schedule
- Max code length: 123 KB
- Call stack depth: 5

### Fee Token
- Default fee token address: `0x4242424242424242424242424242424242424242`

## Adding New Tests

To add new tests:

1. Add test functions in `mod.rs`
2. Use `new_test_ext()` to create a test environment
3. Create a `MockExt` instance with the desired account
4. Call the precompile methods through `ReviveDispatcher::call()`
5. Assert the expected results

Example:
```rust
#[test]
fn test_new_functionality() {
    new_test_ext().execute_with(|| {
        let dispatcher_address = [0u8; 20];
        let mut ext = MockExt { account_id: AccountId32::new([1u8; 32]) };
        
        // Create your call
        let call = IDispatcher::yourCall { /* parameters */ };
        let input = IDispatcher::IDispatcherCalls::your_variant(call);
        
        // Execute
        let result = ReviveDispatcher::<Test, MockDispatcher, FeeToken>::call(
            &dispatcher_address,
            &input,
            &mut ext,
        );
        
        // Assert
        assert_ok!(result);
    });
}
```
