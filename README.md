## tesseract

Messaging and Consensus Relayer for the ismp protocol

## Integration test guide
The Integration tests test messaging between two parachains.
For these tests, it is expected that both parachains use the same runtime and have `pallet-ismp`, `ismp-parachain` and `ismp-demo` included.

To run the tests follow the guide below:
1. `git clone git@github.com:polytope-labs/tesseract.git`
2. `cargo install subxt-cli`
3. In a separate terminal launch the relaychain and two parachains with offchain indexing enabled on both parachains.   
   The first parachain should be configured with para_id 2000 and websocket port 9988  
   The second should be configured with para_id 2001 and websocket port 9188.
4. Generate relaychain runtime types `subxt codegen --url=ws://localhost:9944 | rustfmt --edition=2018 --emit=stdout > /<absolute path to tesseract repo root>/parachain/src/codegen/relay_chain.rs`.
5. From the `tesseract` repo root run `cargo +nightly  test -p tesseract-integration-tests test_messaging_relay -- --nocapture` 
6. Navigate to the extrinsics tab of the block explorer and send some transactions from the `ismp-demo` pallet.
7. If an automated testing experience is preferred run `cargo +nightly  test -p tesseract-integration-tests test_parachain_parachain_messaging_relay -- --nocapture`
   and watch the block explorer for events.
