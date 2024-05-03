# Pallet ISMP

A module for decompressing and executing runtime calls related to pallet ismp and pallet ismp relayer.


## Overview

This Pallet provides functionality which includes:

* Decompressing encoded calls
* Executing encoded calls.

To use it in your runtime, you need to implement the call decompressor config
[`call_decompressor::Config`](https://docs.rs/pallet-call-decompressor/latest/pallet_call_decompressor/pallet/trait.Config.html).

The supported dispatchable functions are documented in the
[`call_decompressor::Call`](https://docs.rs/pallet-call-decompressor/latest/pallet_call_decompressor/pallet/enum.Call.html) enum.


### Terminology

* **compressed:** This is the compressed encoded call represented in bytes.
* **encoded_call_size:** This refers to the size of the original(uncompressed) encoded runtime call.

### Goals

This pallet is designed so as to allow for large execution of runtime calls, the following is possible using this pallet:

* Decompress encoded runtime calls.
* Decode the runtime call.
* Execute the runtime calls.

## Interface

### Dispatchable Functions

* `decompress_call` - This decompresses the compressed encoded runtime call and also executes them.

This pallet only supports these 2 runtime call executions:

* pallet ismp handle messages, `pallet_ismp::Call::handle`
* pallet ismp relayer accumulate fees, `pallet_ismp_relayer::Call::accumulate_fees`

Any other runtime call executions that compressed and sent to it will result in `ErrorExecutingCall` error

Please refer to the [`Call`](https://docs.rs/pallet-call-decompressor/latest/pallet_call_decompressor/enum.Call.html) enum and its associated
variants for documentation on each function.

### Runtime Usage

```rust
impl pallet_call_decompressor::Config for Runtime {
    type MaxCallSize = ConstU32<3>;
}
```

* `MaxCallSize` -  The MaxCallSize represents the maximum original(uncompressed) encoded call size in Megabyte that the pallet allows in the runtime. 