# zk-beefy

This circuit is responsible for proving the verification of super-majority votes on a BEEFY consensus message. The signature scheme is secp256k1. For authority membership, we simply reveal the full tree instead of sparse merkle proof since you can't pass dynamically sized data to circuits.

Some things yet to be implemented

 - [x] Solidity Verifier
 - [x] Rust library for proof generation
