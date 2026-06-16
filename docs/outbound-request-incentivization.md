# Outbound Request Delivery Incentivization

How relayers that deliver a hyperbridge-originated request to a destination chain get paid in BRIDGE from the Hyperbridge treasury.

Tracking issue: [polytope-labs/hyperbridge#532](https://github.com/polytope-labs/hyperbridge/issues/532).

## The problem

A regular cross-chain message that flows *through* hyperbridge has a fee attached at origin (the source chain transfers `fee.payer → RELAYER_FEE_ACCOUNT` and records `RequestPayments[commitment]` in pallet-hyperbridge's child trie). When a relayer delivers and the destination receipt lands back on hyperbridge, the existing `accumulate_fees` flow credits that fee to the relayer. That whole pipeline assumes a *user* paid at origin.

But hyperbridge itself originates requests too: host parameter propagation, host-executive updates, intents-coprocessor responses, token-governor messages, the relayer pallet's withdrawal request. Today these all dispatch with `FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() }` (see `modules/pallets/host-executive/src/lib.rs:228`, `modules/pallets/intents-coprocessor/src/lib.rs:486`, `modules/pallets/relayer/src/lib.rs:638`, and `modules/pallets/token-governor/src/impls.rs`). Zero fee, zero payer. So relayers have no economic reason to pick them up, and the only thing that keeps them flowing today is altruism.

## The shape of the solution

The issue creator's preferred shape ([comment 4428807013](https://github.com/polytope-labs/hyperbridge/issues/532#issuecomment-4428807013)): use `pallet-relayer` to pay BRIDGE to whoever proves they delivered a hyperbridge-originated request. The messaging task in the tesseract relayer submits the claim.

Not every pallet on hyperbridge that dispatches a request is in scope. `pallet_ismp::child_trie::RequestCommitments` ends up holding commitments for every successful dispatch via `IsmpDispatcher`, which includes both the system messages we want to incentivize (host-executive, intents-coprocessor, token-governor, the relayer pallet's withdrawal path, future modules like bandwidth) and any other pallet that ends up dispatching from hyperbridge. The reward storage is therefore keyed by `source_module_id` and only modules with a non-zero reward are eligible. The `module_id` is the `from` field on the `PostRequest`, which each pallet sets to its unique module identifier. A module with zero reward is treated as not on the allowlist and rejected before any state proof verification runs.

This is structurally identical to the existing `claim_outbound_consensus_delivery_reward` (see `modules/pallets/relayer/src/outbound_consensus.rs`) on the consensus side. The request claim lives in its own `modules/pallets/relayer/src/outbound_request.rs` module that mirrors it: swap "consensus rotation delivered" for "request delivered," key the reward storage by `module_id`, and have the relayer ship the full `PostRequest` in the claim so the pallet can hash it on chain.

No changes to pallet-hyperbridge or to any of the system-message dispatch sites. The reward is decoupled from the dispatch path and paid out at claim time against a destination state proof.

## Why this shape and not the alternative

The other option was to attach a real fee at dispatch time, paid from a system-owned PalletId account, so that `RequestPayments` gets populated and the existing `accumulate_fees` flow Just Works. That sounds appealing because it reuses more code, but it has two problems.

First, `accumulate_fees` reads a *source* state proof to find the fee. For hyperbridge-originated requests the source *is* hyperbridge itself, so verifying a proof of hyperbridge state on hyperbridge is meaningless. It would need a new code path that reads local child-trie storage directly, which is the same thing we would have to add for the new claim anyway.

Second, the per-byte fee schedule was designed for protocol-fee economics on the source chain. Using it to size relayer rewards conflates two different knobs. A governance-set per-module reward is the right level of control, same as `OutboundConsensusDeliveryReward`.

## The on-chain pieces

### Storage additions on pallet-relayer

```rust
// Per source_module_id reward, paid out from the treasury when a relayer
// proves delivery of a hyperbridge-originated request from that module to
// any destination. 0 (the default) means both "no reward" and "module not
// on the allowlist".
#[pallet::storage]
pub type OutboundRequestDeliveryReward<T: Config> =
    StorageMap<_, Blake2_128Concat, BoundedVec<u8, ModuleIdBound>, BalanceOf<T>, ValueQuery>;

// Idempotency. Presence of `commitment` means some relayer already
// collected the reward for delivering this request.
#[pallet::storage]
pub type OutboundRequestsClaimed<T: Config> =
    StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>;
```

`ModuleIdBound` is `ConstU32<64>`, comfortable above substrate pallet ids (8 bytes), EVM contract addresses (20 bytes), and the 32-byte module identifiers most ISMP modules use.

### Claim payload

```rust
pub struct OutboundRequestDeliveryClaim {
    // The hyperbridge-originated request being claimed against. Hashed on
    // chain to derive the commitment; `source` is verified against
    // `IsmpHost::host_state_machine()`; `from` keys the reward map.
    pub request: PostRequest,

    // State proof of the destination chain at a height where Hyperbridge
    // already has a state commitment. `state_proof.height.id.state_id`
    // must equal `request.dest`.
    pub state_proof: Proof,

    // Sr25519 public key on Hyperbridge that the reward is paid to.
    pub payee: [u8; 32],

    // Signature over `outbound_request_delivery_message(commitment, destination, payee)`.
    // For EVM destinations: `Signature::Evm`, recovered address must equal the
    // address proven in `RequestReceipts[commitment]`. For substrate destinations:
    // any variant, the recovered signer must equal the bytes proven in
    // `RequestReceipts[commitment]`.
    pub signature: Signature,
}
```

The relayer carries the full request through the claim pipeline so the pallet, not the relayer, derives the commitment. This is what makes the allowlist enforceable: the source module is the `from` field on the request, and the relayer never gets to claim under a different module identifier than the one the request was actually dispatched with.

### Extrinsics

Two new calls on pallet-relayer, both following the consensus-claim shape.

```rust
// Unsigned, spam-protected via validate_unsigned (encoded claim becomes the tag).
#[pallet::call_index(5)]
pub fn claim_outbound_request_delivery_reward(
    origin: OriginFor<T>,
    claim: OutboundRequestDeliveryClaim,
) -> DispatchResult {
    ensure_none(origin)?;
    Self::process_outbound_request_delivery_claim(claim)
}

#[pallet::call_index(6)]
pub fn set_outbound_request_delivery_reward(
    origin: OriginFor<T>,
    module_id: BoundedVec<u8, ModuleIdBound>,
    amount: BalanceOf<T>,
) -> DispatchResult {
    T::RelayerOrigin::ensure_origin(origin)?;
    OutboundRequestDeliveryReward::<T>::insert(&module_id, amount);
    Self::deposit_event(Event::OutboundRequestDeliveryRewardUpdated {
        module_id,
        new_reward: amount,
    });
    Ok(())
}
```

### Verification pipeline

`process_outbound_request_delivery_claim` runs these checks in order. Ordering is deliberate: every cheap rejection happens before the state-proof verification, so non-allowlisted claims and replays are dropped without ever touching the trie verifier.

1. **Hash the request.** `commitment = hash_request::<IsmpHost>(&Request::Post(request))`. The relayer never gets to pick the commitment.

2. **Source check.** `request.source` must equal `IsmpHost::host_state_machine()`. Rejects forged claims for requests that didn't originate on this hyperbridge instance.

3. **Local presence check.** `pallet_ismp::child_trie::RequestCommitments::get(commitment).is_some()`. Defence in depth on top of the source check: the dispatcher already enforces source on insert, so anything missing from the trie was never dispatched here.

4. **Idempotency.** Reject if `OutboundRequestsClaimed[commitment]` is set.

5. **Module-id bound.** `BoundedVec::<u8, ModuleIdBound>::try_from(request.from.clone())`. Anything longer than 64 bytes is treated as not on the allowlist.

6. **Allowlist lookup.** `reward = OutboundRequestDeliveryReward::<T>::get(module_id)`. If zero, reject. This is the only place the allowlist is enforced; governance enables a module by setting a non-zero reward.

7. **State-machine match.** `state_proof.height.id.state_id == request.dest`. Defends against a relayer building a proof against a different chain than the request was sent to.

8. **Destination type and receipt key.** Use the `Pallet::request_receipt_key` helper (defined alongside the claim in `outbound_request.rs`):
   - EVM destinations: 32-byte slot hash `derive_unhashed_map_key(commitment, REQUEST_RECEIPTS_SLOT)`, the same key the EVM state machine's `receipts_state_trie_key` produces.
   - Substrate destinations: `pallet_ismp::child_trie::RequestReceipts::<T>::storage_key(commitment)`, identical to the substrate state machine's receipt key.

   A destination that is neither EVM nor substrate is rejected with `OutboundRequestUnsupportedDestination`.

9. **State proof verification.** Resolve the destination client with `ismp::handlers::validate_state_machine(&host, height)`, then `verify_withdrawal_proof(state_machine, &state_proof, vec![key])` against hyperbridge's stored state commitment for the destination. A verification failure maps to `OutboundDestinationStateNotKnown` (no commitment at that height), and a missing or null slot value maps to `OutboundDeliveryNotProven`.

10. **Signature attribution.** Recover the signer from `signature.verify(&outbound_request_delivery_message(commitment, destination, payee), None)` and check it matches the address proven in the receipt slot. For EVM, both are 20-byte addresses; for substrate, the bytes from the receipt must equal `signature.signer()`. Mismatch → `OutboundRequestSignerMismatch`.

11. **Payout.** Transfer `reward` from the treasury PalletId account to `payee`.

12. **Persist and emit.** Insert `OutboundRequestsClaimed[commitment] = ()`. Deposit `OutboundRequestDeliveryRewarded { commitment, state_machine: destination, module_id, relayer: payee, amount: reward }`.

### Signed payload

```rust
pub fn outbound_request_delivery_message(
    commitment: H256,
    dest_chain: StateMachine,
    payee: [u8; 32],
) -> [u8; 32] {
    sp_io::hashing::keccak_256(&(commitment, dest_chain, payee).encode())
}
```

Replay protection is structural: the on-chain `OutboundRequestsClaimed[commitment]` tag, plus the `validate_unsigned` txpool tag derived from `claim.encode()`. No per-relayer nonce, same model as the consensus claim.

### Error additions

Mirror the consensus-claim error set, prefixed `OutboundRequest...`:

- `OutboundRequestAlreadyClaimed`
- `OutboundRequestNotKnown` (the commitment is not in local `RequestCommitments`, so this is not a hyperbridge-originated request we are willing to pay for)
- `OutboundRequestSourceNotHyperbridge`
- `OutboundRequestModuleIdTooLong` (`request.from` is longer than `ModuleIdBound`, so it cannot key the reward map)
- `OutboundRequestNoRewardConfigured`
- `OutboundRequestRewardTransferFailed`
- `OutboundRequestSignerMismatch`
- `OutboundRequestUnsupportedDestination` (the destination is neither EVM nor substrate)
- `OutboundDestinationStateNotKnown` (already exists, reused)
- `OutboundDeliveryNotProven` (already exists, reused)

The EVM-only error from the consensus claim does not apply here. This claim supports both EVM and substrate destinations from day one; the receipt key and the relayer decoding both branch on the destination state machine type, the same split `accumulate_fees` uses.

## Off-chain wiring (tesseract)

The messaging task already delivers hyperbridge-originated requests today (it processes the dispatch root the same way as user-originated requests). What it does not do is fire a claim afterwards. The hook point is the same place that already submits the `accumulate_fees` withdrawal proof for user-funded requests.

```text
tesseract/messaging/messaging/src/outbound.rs
    after delivery to destination
        if request.source == NEXUS:
            wait until destination state commitment lands on hyperbridge
            build OutboundRequestDeliveryClaim
            sign with delivery key
            submit claim_outbound_request_delivery_reward
```

Three practical pieces:

- **Distinguishing hyperbridge-originated requests.** Receipts whose `query.source_chain == coprocessor` are forwarded; everything else is ignored.
- **Carrying the request.** Each `PendingRequestDeliveryClaim` carries the full `PostRequest` because the pallet hashes it on chain to derive the commitment and reads `request.from` to key the allowlist. The tesseract trigger extracts post requests from the outgoing batch before submit, indexes them by commitment, and pairs them back to receipts after the submit returns.
- **Waiting for the state commitment.** Same wait the existing accumulate_fees flow uses.

Backpressure: the claim channel uses `try_send` with a 512-slot buffer. If the channel is saturated the trigger drops the channel push, but the DB row persisted right above the send survives and is replayed on the next trigger. The DB is the source of truth.

## How requests are identified

Hyperbridge-originated requests flow through the exact same channel as every other request. The relayer never "scans for system messages." It does, however, apply two filter stages around the on-chain allowlist so it never spends gas delivering something it cannot get paid for.

The authoritative gate is on chain: `process_outbound_request_delivery_claim` rejects any commitment whose `request.from` is not in `OutboundRequestDeliveryReward`. Everything off chain is a gas-saving optimisation on top of that.

Walking the flow end to end:

1. A pallet on hyperbridge (host executive, intents coprocessor, token governor, the relayer pallet's own withdrawal path) calls `IsmpDispatcher::dispatch_request(post, fee)`. That creates an entry in `pallet_ismp::child_trie::RequestCommitments` keyed by the request commitment hash, and emits an ISMP `Event::PostRequest` with `post.source` set to the hyperbridge nexus state machine.

2. The tesseract outbound task is already subscribed to `proof_accepted_notification` on hyperbridge. When a BEEFY proof advances the dispatch root past the new request, the task wakes up and calls `query_ismp_events(cursor, latest_height)`. The returned vec contains that `Event::PostRequest` alongside everything else hyperbridge emitted in that range.

3. **Pre-delivery filter (off chain, gas saver).** Once per BEEFY notification the outbound loop calls `incentivized_outbound_request_modules()` on hyperbridge to snapshot every `module_id` with a non-zero `OutboundRequestDeliveryReward` into a `BTreeSet<Vec<u8>>`. `submit_for_dest` then runs `retain_incentivized_requests`: any `Event::PostRequest` whose `post.source == coprocessor` is dropped unless `post.from` is in the set. User-originated requests (`source != coprocessor`) are untouched. If the snapshot fetch fails the filter is a no-op for that cycle, so the relayer keeps delivering everything (same risk profile as before the filter existed). Governance updates show up in the next snapshot, so allowlist edits take effect within one BEEFY cycle.

4. `translate_events_to_messages` turns the surviving events into deliverable messages. `dest.submit(batch, coprocessor)` lands the batch on the destination. The returned receipts carry the original `query: Query { source_chain, dest_chain, nonce, commitment }` for each delivered message.

5. **Post-delivery trigger filter (off chain, claim shaping).** `forward_request_delivery_claims` walks `result.receipts`, keeps every `TxReceipt` whose `query.source_chain == coprocessor`, pairs each receipt back to the matching `PostRequest` extracted from the outgoing batch, persists `(commitment, encoded_request, delivery_height)` to the local DB, and pushes the claim onto the request-claim channel via `try_send`. `TxReceipt` only carries requests now (responses were removed from the protocol in #840), so there is nothing to filter out by message type. The pre-delivery filter has already narrowed the population to allowlisted modules, so in steady state every hyperbridge-originated receipt at this point should be claim-eligible.

6. **On-chain enforcement.** When the claim arrives, the pallet hashes the request, confirms `request.source == IsmpHost::host_state_machine()`, confirms `RequestCommitments::get(commitment).is_some()`, then keys the reward lookup by `request.from`. The module must be in the allowlist (non-zero reward set by governance) or the claim is rejected before any state proof verification work runs. This is the only gate that actually matters for correctness; the off-chain stages above just avoid burning destination gas on requests the pallet would reject anyway.

So the relayer never queries a "list of hyperbridge requests" index or treats hyperbridge-originated events as a special class at translation time. Identification is a single equality check on a field already on the wire, narrowed by an on-chain allowlist snapshot for gas, and enforced authoritatively at claim time.

## Test plan

The feature splits cleanly into four test layers; each one catches a different class of bug. Layers 1 and 2 run in CI; layers 3 and 4 are manual.

### Layer 1: pallet unit tests (in CI)

Lives in `modules/pallets/testsuite/src/tests/pallet_ismp_relayer.rs::outbound_request_delivery`. 15 tests cover:

- Pre-verification gates: unknown commitment, already-claimed, source-not-hyperbridge, module-id-too-long, allowlist-off, placeholder-proof smoke test that reaches the state-proof step.
- Substrate end-to-end: builds a real substrate trie with `RequestReceipts[commitment]`, signs the claim message with sr25519, runs the full verification pipeline, asserts the payee balance moves by the configured reward and `OutboundRequestsClaimed[commitment]` is set. Plus negative paths for signer-mismatch and no-reward-configured.
- Helper coverage for `request_receipt_key` (EVM / substrate / unsupported) and `decode_request_receipt_relayer` (EVM RLP address / substrate raw bytes / substrate signature-wrapper / unsupported).

This layer proves the on-chain verification logic is correct end-to-end in isolation. Run with `cargo test -p pallet-ismp-testsuite --lib tests::pallet_ismp_relayer::outbound_request_delivery`.

### Layer 2: tesseract trigger unit test (in CI)

Lives in `tesseract/messaging/messaging/src/outbound.rs::tests`. Two clusters:

`forward_request_delivery_claims` (post-delivery trigger):

- A `Vec<TxReceipt>` mixing hyperbridge-originated and user-originated requests produces exactly the right set of `PendingRequestDeliveryClaim`s on the channel (only the hyperbridge-originated ones).
- `claim_sender: None` is a clean no-op.
- Empty receipts is a clean no-op.
- Defensive case: a HB-sourced receipt with no matching `PostRequest` in the batch is dropped (cannot key the reward without the request).

`retain_incentivized_requests` (pre-delivery filter):

- HB-originated requests whose `from` is not in the on-chain allowlist snapshot are dropped; allowlisted ones survive; user-originated requests pass regardless of module.
- `incentivized = None` (snapshot fetch failed this cycle) is a no-op so the relayer keeps delivering everything.
- Non-`PostRequest` events flow through untouched.

This layer catches "we filtered wrong" or "we sent the wrong commitment" before they hit the network. Runs in-process, no zombienet. Run with `cargo test -p messaging --lib outbound::tests`.

### Layer 3: simnode runtime-metadata test (local, manual)

Lives next to the existing simtests at `parachain/simtests/src/pallet_outbound_request_claim.rs`. Marked `#[ignore]` like every other simtest, so CI doesn't run it; the operator starts simnode locally and runs `cargo test -p simtests`.

**Why simnode and not zombienet:** simnode is a single-node, manual-seal hyperbridge runtime. No second chain to spin up, no relay chain, deterministic block production via `engine_createBlock`. The destination chain doesn't actually exist in the test — its state is synthesized locally and planted on hyperbridge via sudo-driven `System::set_storage`, the same pattern the existing `pallet_ismp.rs::test_txpool_should_reject_duplicate_requests` already uses.

**Prerequisite:** simnode running locally. One command from the repo root:

```
./parachain/simtests/hyperbridge-old-simnode simnode --chain=gargantua-2000 \
  --name=alice --tmp --rpc-port=1944
```

Then `cargo test -p simtests -- --ignored pallet_outbound_request_claim` against `ws://127.0.0.1:1944`.

**Flow:**

1. Sudo-call `HostExecutive::set_host_params(...)` for a fake destination (e.g. `StateMachine::Kusama(3000)`). This is itself a hyperbridge-originated dispatcher, so it produces a real `RequestCommitments[commitment]` entry on hyperbridge with no child-trie sudo hacks.
2. Capture the emitted `Request` event and extract its commitment and `from` (the dispatching module's id).
3. Sudo-call `Relayer::set_outbound_request_delivery_reward(module_id, REWARD)`, where `module_id` is the `from` captured in step 2.
4. Fund the `TreasuryPalletId` account via sudo `Balances::force_set_balance`.
5. Build a substrate destination trie locally with `RequestReceipts[commitment] = payee_pubkey` using `TrieDBMutBuilder` + `Recorder` (same primitives the pallet unit test uses).
6. Sudo-`System::set_storage` to plant the destination's state commitment for that root at `height`, plus the matching update-time entry. Helpers `state_machine_commitment_storage_key` and `state_machine_update_time_storage_key` already exist in `subxt-utils`.
7. Submit `Relayer::claim_outbound_request_delivery_reward` via subxt with the synthesized proof and an sr25519 signature over `outbound_request_delivery_message(commitment, destination, payee)`. **This is the only step where the real subxt encoding contract is exercised**, and it's the exact gap Layer 1 cannot cover.
8. Assert the `OutboundRequestDeliveryRewarded` event fired with the expected commitment, the payee balance moved by exactly `REWARD`, and `OutboundRequestsClaimed[commitment]` is now `Some(())`.

**What this catches that Layer 1 and Layer 2 cannot:**

- `outbound_request_delivery_claim_to_value` (our subxt encoder in `modules/utils/subxt/src/values.rs`) is encoding the claim in a layout the live runtime metadata accepts. Field-order or variant-index mismatches in dynamic extrinsics are the most common shipping bug for this kind of feature and are invisible until a real runtime decodes the bytes.
- The `validate_unsigned` path accepts the claim through the txpool under real conditions, including the `provides` tag dedup.
- Sizing and weight accounting work against the actual runtime, not a mock.

**Estimated effort:** roughly 200 to 250 lines, modeled line-for-line on `parachain/simtests/src/pallet_ismp.rs`. Not implemented yet; queued as a follow-up.

### Layer 4 prerequisite: deploy the runtime upgrade (do this first)

Layer 4 calls `Relayer::set_outbound_request_delivery_reward` and the relayer submits `claim_outbound_request_delivery_reward`. Neither extrinsic exists in the runtime the gargantua testnet is currently running, so the runtime has to be upgraded before any Layer 4 step will work. This branch bumps `spec_version` from 6600 to 6800 (the live testnet is on 6700, so 6800 clears the strictly-increasing check).

Gargantua holds a sudo key, so the upgrade goes through `Sudo` rather than the preimage-and-referendum flow in `docs/runtime-upgrade-guide.md`. With sudo there is no preimage deposit, so a single `Sudo::sudo(System::set_code(wasm))` call is the simplest path. The "too large for `system.setCode`" warning in `docs/runtime-upgrade-guide.md` is about the mainnet *preimage cost* (5 BRIDGE/byte), not a block-size limit: `set_code` runs in the `Operational` dispatch class, which gets the full 5 MB `RuntimeBlockLength` (`parachain/runtimes/gargantua/src/lib.rs:318`), and the WASM is only ~1.81 MB. Use plain `set_code`, not `set_code_without_checks`: `set_code` runs `can_set_code`, which enforces the `spec_name` match and the strictly-increasing `spec_version` (6800 > 6700).

**The artifact.** Built locally with `./scripts/build_release_runtime.sh gargantua-runtime` (the gargantua branch of that script adds `--features=metadata-hash,no-bandwidth`, same as CI):

- WASM: `target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.compressed.wasm`
- size: 1,898,631 bytes
- keccak256 code hash: `0x3e0f7ff38ace7f92042ba393269c3d3d253e909569786434c180bcc912483789` (not needed for `set_code`; keep it to confirm the file you upload is the one that was built)

If you rebuild, recompute the hash the way CI does to re-verify the artifact:

```
python3 -c "from Crypto.Hash import keccak; k=keccak.new(digest_bits=256); k.update(open('target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.compressed.wasm','rb').read()); print('0x'+k.hexdigest())"
```

**Steps** (polkadot.js Apps connected to `wss://gargantua.rpc.polytope.technology`, signed by the sudo account):

1. **Submit the upgrade.** Developer → Sudo → `sudo(call)`, with the inner call set to `system.setCode(code)`. Toggle file upload on the `code` field and select the `gargantua_runtime.compact.compressed.wasm` file from the path above.
2. **Submit and wait one block.** The runtime runs `can_set_code` (spec_name match + spec_version 6800 > 6700), then `System::CodeUpdated` fires and the node swaps the runtime.

**Verify the upgrade landed:**

- Developer → Chain state → `system.lastRuntimeUpgrade()` reads `specVersion: 6800`, `specName: gargantua`.
- The metadata now exposes the new extrinsics. In Developer → Extrinsics, `relayer` should list `setOutboundRequestDeliveryReward(moduleId, amount)` and `claimOutboundRequestDeliveryReward(...)`. If they are missing, the upgrade did not take.
- Developer → Chain state → `relayer.outboundRequestDeliveryReward(<module id>)` should decode without error (it returns 0 until a reward is registered).

Only once `lastRuntimeUpgrade` shows 6800 and the new `relayer` extrinsics are in the metadata should you move on to the Layer 4 steps below.

### Layer 4: manual runbook on gargantua with the ping module

This is the end to end smoke test, and it is the chosen approach for manual verification: run it against the live gargantua testnet rather than a locally spawned stack. The team holds the gargantua testnet sudo key, so the one privileged step (registering the reward) is available, and this avoids standing up a relay chain, collator, and BEEFY prover. The fully local alternative is documented in the next section for the case where testnet sudo is not available or a fully uncontested run is wanted.

It uses `pallet-ismp-demo` (the "ping" pallet, `IsmpDemo` in the gargantua runtime) as the hyperbridge-originated request producer. Its `dispatch_to_evm` extrinsic dispatches a `PostRequest` with `from = PALLET_ID.to_bytes()`, and `PALLET_ID` is `ModuleId::Pallet(PalletId(*b"ismp-ast"))`, so the module id on the wire is the raw 8 bytes `b"ismp-ast"` (`0x69736d702d617374`). That is the value governance registers a reward for.

One caveat for the live testnet: delivery is a race. Other relayers on the testnet also see `b"ismp-ast"` in their allowlist snapshot once the reward is registered, so whichever relayer lands the destination delivery first wins the reward (see the race explanation in the next section). To reliably observe your own relayer claiming, point it at an EVM destination other testnet relayers are not actively serving. The feature still proves out either way, you just may not be the payee.

Runtime facts that shape the steps:

- `pallet_ismp_relayer::Config::RelayerOrigin` on gargantua is `EnsureRoot`, so `set_outbound_request_delivery_reward` has to go through `Sudo::sudo`.
- The reward is paid from `TreasuryPalletId = PalletId(*b"hb/trsry")`. That account must hold enough balance or the claim fails with `OutboundRequestRewardTransferFailed`.
- Gargantua's host state machine is `StateMachine::Kusama(<para_id>)` (para id `4009` on the current testnet).

**Prerequisites:**

- Access to the live gargantua testnet with the sudo key in hand (the team holds it).
- An EVM destination chain reachable by the relayer, with the ISMP host contract deployed. `dispatch_to_evm` targets `StateMachine::Evm(destination)`.
- The consolidated tesseract relayer built from this branch, configured with gargantua as the hyperbridge source and that EVM chain as an outbound destination with a signer. Fees enabled, so the DB-backed claim pipeline runs.

**Steps:**

1. **Fund the treasury.** Confirm the `hb/trsry` account holds at least the reward amount, or top it up via `Sudo::sudo(Balances::force_set_balance(treasury_account, amount))`. The account id is `PalletId(*b"hb/trsry")` run through `into_account_truncating()`, which is `0x6d6f646c68622f7472737279` padded with zeros to 32 bytes (`0x6d6f646c68622f74727372790000000000000000000000000000000000000000`). On the current testnet this account already holds ~9.99M tBRIDGE, so no top-up was needed.
2. **Register the reward.** `Sudo::sudo(Relayer::set_outbound_request_delivery_reward(module_id = 0x69736d702d617374, amount = REWARD))`. The deployed run used `REWARD = 10000000000000` (10 tBRIDGE, the token has 12 decimals). Confirm the `OutboundRequestDeliveryRewardUpdated` event fired and `Relayer::OutboundRequestDeliveryReward(0x69736d702d617374)` now reads that amount.
3. **Let the relayer pick up the allowlist.** The outbound task calls `incentivized_outbound_request_modules` once per BEEFY notification, so within one cycle its snapshot includes `b"ismp-ast"`. Nothing to do here except wait one cycle before step 4.
4. **Dispatch from the ping module.** Call `IsmpDemo::dispatch_to_evm(EvmParams { module, destination, timeout, count: 1 })` from any signed account. `module` is the destination EVM module address, `destination` is the EVM chain id. This dispatches a `PostRequest` with `source = Kusama(4009)`, `from = b"ismp-ast"`, `dest = Evm(destination)`, and writes it into `RequestCommitments`.
5. **Watch delivery.** The outbound task picks up the BEEFY proof, keeps the request through `filter_events` (no `module_filter` configured, so `is_allowed_module` permits it) and `retain_incentivized_requests` (`b"ismp-ast"` is in the snapshot), delivers the batch to the EVM chain, then persists a claim row and pushes a `PendingRequestDeliveryClaim`.
6. **Watch the claim.** The outbound-request-claim task waits for gargantua's consensus client for the EVM chain to verify the delivery height, builds the `RequestReceipts[commitment]` state proof, signs, and submits `claim_outbound_request_delivery_reward`. Look for the log line `submitting outbound request delivery claim to hyperbridge`.
7. **Verify on gargantua.**
   - `OutboundRequestDeliveryRewarded { commitment, module_id, relayer, amount }` fired with `module_id = 0x69736d702d617374`.
   - The relayer's payee account balance moved by exactly `REWARD`.
   - `Relayer::OutboundRequestsClaimed(commitment)` is now `Some(())`.

**Negative checks worth running once:**

- Before step 2 (no reward registered), dispatch a ping request. `retain_incentivized_requests` should drop it off chain, and if a claim is forced through anyway the pallet rejects it with `OutboundRequestNoRewardConfigured`.
- Re-submit a claim for an already-claimed commitment. The pallet rejects it with `OutboundRequestAlreadyClaimed` and the payee balance does not move again.

This layer catches network-condition issues (state-commitment latency, RPC quirks, the live runtime metadata accepting the dynamic extrinsic) that no other layer exercises. Run it once per shipped change.

## Running the manual test fully locally (alternative)

Layer 4 against the live testnet is the chosen path. This section is the documented alternative, for when testnet sudo is not available or a fully uncontested, deterministic run is wanted. It is heavier: a local stack means standing up a relay chain, a collator, a BEEFY prover, and the consensus-state init that Layer 4 gets for free from the live network.

### Why testnet delivery is a race

The reward goes to whichever relayer lands the delivery transaction on the destination chain first. That relayer's address is what the destination's ISMP host writes into `RequestReceipts[commitment]`, and `process_outbound_request_delivery_claim` only pays the address proven in that slot. A relayer that delivers second just no-ops, and its claim fails `OutboundRequestSignerMismatch`. The on-chain reward is registered globally, so every relayer watching gargantua sees the module in its allowlist snapshot and competes. Running locally sidesteps this: your relayer is the only one delivering.

### The full local stack

A real end-to-end run, with the actual consensus path rather than a synthesized one, needs all of:

1. A `rococo-local` relay chain. This is a validator set, not a single node. `scripts/zombienet/local-testnet.toml` brings up four (alice, bob, charlie, dave) on the `polkadot` binary. The relay chain is required because gargantua is a parachain and does not finalize on its own; the BEEFY proofs the outbound task consumes trace back to relay-chain finality.
2. A gargantua collator, the `hyperbridge` binary, as parachain id 2000 (also in the zombienet config).
3. A destination chain (see the next section, this is the hard part).
4. The BEEFY prover. This is a **separate binary** from the consolidated relayer (`tesseract/relayer/src/config.rs` notes "The BEEFY prover runs as a separate" process). It watches the relay chain and submits to `pallet-beefy-consensus-proofs::submit_proof` on gargantua. The pallet processes the proof inline, emits `ProofAccepted`, and writes the proof bytes into the gargantua node's offchain storage.
5. The consolidated relayer (`tesseract/relayer`). It runs the messaging inbound and outbound tasks. Its `OffchainProofSource` reads the proof bytes back out of gargantua's offchain storage via the `offchain_localStorageGet` RPC, bundles them into a `Message::Consensus`, and delivers them to the destination alongside the messages. The relayer consumes proofs; it does not produce them, which is why the BEEFY prover in step 4 is load-bearing.

Prerequisites: `zombienet` installed, the `polkadot` and `polkadot-parachain` binaries (the config points at `../polkadot-sdk/target/release/`), a release build of `hyperbridge`, and the `gargantua-2000` chain spec. The config also includes an asset-hub parachain that this test does not need and can be trimmed.

On the BEEFY prover: the gargantua runtime's `AllowedBeefyProofTypes` accepts both `PROOF_TYPE_NAIVE` and `PROOF_TYPE_SP1`, so a naive proof is accepted on chain with no runtime change. For a local run the naive prover is preferable because the SP1 path needs the SP1 toolchain, proving keys, and multi-minute proving time per proof, none of which the feature under test cares about. Open item: the `BeefyHost` builder in `tesseract/consensus/beefy/src/lib.rs` is wired with `zk_beefy::LocalProver`, so the exact way to select naive output from the prover binary still needs to be confirmed.

### The destination chain is the hard part

`anvil` (Foundry's local node) is the obvious EVM node choice since the `evm/` project is entirely Foundry-based, and the deploy scripts exist (`evm/script/DeployIsmp.s.sol` for the Host and Handler, `evm/script/DeployPing.s.sol` for the destination module). But anvil cannot be a real ISMP destination of gargantua:

- The claim pallet verifies the `RequestReceipts[commitment]` proof against gargantua's **stored state commitment for the destination**, and the relayer's claim task first calls `wait_for_state_machine_update`, which blocks until gargantua has a verified state-machine update for the destination at or past the delivery height.
- anvil has no consensus. It is instant-mine with no finality, no BEEFY, no sync committee. There is nothing for an ISMP consensus client on gargantua to verify, so there is no legitimate path to get anvil's state root onto gargantua as a trusted commitment. `wait_for_state_machine_update` never completes and the claim task hangs.

So there are two ways forward, and they are genuinely different:

- **Substrate destination (recommended for a true local end-to-end).** A second gargantua or nexus style parachain has real GRANDPA and BEEFY consensus, so gargantua can track it through a proper consensus client with nothing faked. The ping pallet's `transfer` extrinsic (call index 0) targets a parachain, and the claim pallet already supports substrate destinations via the child-trie receipt key (Layer 1 exercises exactly that path). Open item: confirm the zombienet config can run a second ISMP parachain and what that wiring looks like.
- **Sudo-plant the EVM state.** On a local gargantua you have Alice and sudo, so you can `System::set_storage` the destination's state root and update-time directly into `pallet-ismp`. The relayer then builds a real proof against the planted root. This is exactly the Layer 3 simnode approach: it is a test harness that fakes the consensus link, not "running everything."

Bottom line: anvil works as an EVM node, but standing it up as a real ISMP destination requires either faking the consensus link or running a chain with real finality. For a true local end-to-end the substrate-to-substrate path is the right call.

### Open items before this is a clean runbook

- Confirm the exact BEEFY prover binary (`tesseract/consensus/relayer/bins/` has `polyhedron.rs` and `relayer.rs`) and how to run it in naive mode.
- Confirm whether the zombienet config can run a second ISMP parachain for the substrate-destination path.
- The `local` deploy target in `evm/script/deploy.sh` and `evm/script/README.md` references the long-dead `goerli` network, so the local/anvil path may have bit-rotted and needs a once-over if the EVM path is pursued.

## Build verification

The branch (merged with `main`, with the review changes applied) compiles cleanly. Verified by reproducing the CI build setup locally:

1. `cd evm && pnpm install && forge build` — generates the solidity ABI artifacts under `evm/out/`, including `evm/out/EcdsaBeefy.sol/EcdsaBeefy.json` that `ismp-solidity-abi` embeds via the `sol!` macro. This step has to run before any cargo build; `evm/out/` is not checked in.
2. `cargo build -p hyperbridge` — exit 0, the parachain binary is produced.
3. `cargo metadata --locked` — exit 0, so `Cargo.lock` is consistent with `Cargo.toml` and the `--locked` CI jobs will not fail on the lockfile.

If a cargo build fails with `failed to canonicalize path ".../evm/out/EcdsaBeefy.sol/EcdsaBeefy.json"` or `unresolved import Beefy`, the forge artifacts are missing — run step 1 first. CI does this automatically (`pnpm install` + `forge build` in `evm/` before every cargo step), so it is a local-environment gotcha, not a code defect.

## Live testnet deployment notes (2026-05-15)

Captured while running the Layer 4 manual flow against the gargantua testnet. Everything here is operational, either corrections that need to land before the branch ships or gotchas a future operator should know about.

### Runtime upgrade through Sudo

The Sudo `set_code` path worked exactly as documented. Block sequence, signed by the gargantua sudo account:

1. `sudo.sudo(System::set_code(wasm))` inserted into the block.
2. `parachainSystem.ValidationFunctionStored` fired the same block.
3. `sudo.Sudid` confirmed the inner call returned `Ok(())`.
4. One block later `parachainSystem.ValidationFunctionApplied` fired and `System::CodeUpdated` followed. From that block on `system.lastRuntimeUpgrade()` reads `specVersion: 6800`.

The new metadata surfaced `relayer.setOutboundRequestDeliveryReward(moduleId, amount)` and `relayer.claimOutboundRequestDeliveryReward(claim)`, and `relayer.outboundRequestDeliveryReward(b"ismp-ast")` decoded as `0` until the reward was set.

### Reward registration call

The hex-encoded sudo call that registered the ping-module reward (`Sudo` pallet index `25`, `Relayer` pallet index `53`, `set_outbound_request_delivery_reward` call index `6`):

```
0x1900350620 69736d702d617374 070010a5d4e80000000000000000000000
```

That set `OutboundRequestDeliveryReward[b"ismp-ast"] = 10_000_000_000_000` (10 tBRIDGE, 12 decimals). `OutboundRequestDeliveryRewardUpdated` fired with `newReward: 10,000,000,000,000`.

Note on shape: the reward is keyed by `module_id` alone (`StorageMap`), matching the design section above. A module's reward applies to its requests to any destination. If per-destination granularity is ever needed, widening to a `(destination, module_id)` double map is a follow-up.

### Missing OutboundRequestClaims prisma migration

The Prisma model `OutboundRequestClaims` is defined in `tesseract/messaging/fees/prisma/schema.prisma:83-96` and `cargo prisma generate` produces the matching Rust types. The migration directory for it was not in the branch, so the table never gets created on relayer startup. The symptom is two warnings every time a hyperbridge-originated request gets delivered:

```
WARN messaging-outbound: failed to persist outbound-request claims; claims will not survive a restart
WARN messaging-outbound-request-claim: list_pending_request_claims failed; trigger-only this cycle
err=P2021 The table 'main.OutboundRequestClaims' does not exist
```

This branch adds `tesseract/messaging/fees/prisma/migrations/20260515090000_outbound_request_claims/migration.sql`. On a fresh relayer DB the startup `migrate_deploy` applies it cleanly. For a relayer DB that pre-existed the fix (the live one we ran against), apply the SQL by hand and insert a matching `_prisma_migrations` row so a future restart-with-rebuild does not try to re-apply:

```
sqlite3 ~/hyperbridge-incentivize/relayer-fees.db <<SQL
CREATE TABLE "OutboundRequestClaims" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "dest" TEXT NOT NULL,
    "commitment" TEXT NOT NULL,
    "encoded_request" BLOB NOT NULL,
    "delivery_height" BIGINT NOT NULL,
    "status" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL,
    "note" TEXT
);
CREATE INDEX "OutboundRequestClaims_status_idx" ON "OutboundRequestClaims"("status");
CREATE UNIQUE INDEX "OutboundRequestClaims_commitment_key" ON "OutboundRequestClaims"("commitment");
SQL
```

Then `INSERT INTO _prisma_migrations (id, checksum, finished_at, migration_name, started_at, applied_steps_count)` with the sha256 of the new `migration.sql` as the checksum, the timestamp directory name as `migration_name`, and `applied_steps_count = 1`.

### PingModule deployment addresses

The integration test `tesseract/consensus/integration-tests/src/ping.rs:29` uses `0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35` as the canonical PingModule. CREATE2 deterministic deploy, present on Ethereum Sepolia, Base Sepolia, Arbitrum Sepolia, Optimism Sepolia, and BSC Testnet. Pharos and `EVM-420420417` ship a different build at `0xBB3dFCcBAE0ae8F00320E46719c342fd69f5516C` (22,782 bytes). For Layer 4 dispatches the canonical address is the right `module` argument to `IsmpDemo::dispatch_to_evm` on the five chains above; the two outliers need per-chain configuration.

### Pharos `debug_traceCall` drops logs

Pharos is currently unusable as a Layer 4 destination because of an RPC bug. `dispatchIncoming` runs and emits `PostRequestHandled` in the receipt, but `debug_traceCall` / `debug_traceTransaction` with `withLog: true` return an empty `logs` field for the same call. The relayer's `check_trace_for_event("PostRequestHandled")` therefore always fails and the message is marked `Skipping Failed tx`. Reproducer: pick a Pharos transaction whose receipt shows N logs, run `debug_traceTransaction(<hash>, {tracer:"callTracer", withLog:true})`, and observe the trace returns 0 logs.

Workaround: pick a destination chain whose `debug_trace*` correctly returns logs. Base Sepolia is verified to work. The Pharos integration owners have been notified.

### The claim task is event driven, not polled

`outbound_request_claim::run` in `tesseract/messaging/messaging/src/outbound_request_claim.rs:85` is a single `while let Some(trigger) = receiver.recv().await` loop. `merge_pending` inside the loop reads DB rows and unions them with the trigger, so DB rows survive a restart and are reconsidered on every tick. But the loop only ticks when `forward_request_delivery_claims` (`tesseract/messaging/messaging/src/outbound.rs:712`) pushes a non-empty batch onto the channel.

The consequence in manual testing: a pending DB row whose only blocker was `wait_for_state_machine_update` stays pending until any new hyperbridge-originated delivery to any destination fires the channel again. In steady-state production this is fine because system messages flow constantly. In Layer 4 it is easy to dispatch once, see delivery succeed, and then watch the row sit `pending` for ten or more minutes while the destination's state commitment catches up but no new delivery happens to flush the queue.

Follow-up worth considering: give the task an independent tick (e.g. one minute) so DB rows progress once the state machine update lands, without depending on inbound deliveries.

### End-to-end observed on Base Sepolia

The first end-to-end delivery succeeded at `tx 0xd24ea5ae94afb262f3c331116fb471bc763cb7656ca09da3f4495a691ce9763d`. The persisted DB rows showed up after the migration was in place:

```
1 | EVM-84532 | 0xbe237750... | delivery_height=41534939 | pending
2 | EVM-84532 | 0xaf3db306... | delivery_height=41534989 | pending
```

Hyperbridge's view of Base Sepolia's finalized height advances ~600 blocks every ~20 minutes through the OP Stack dispute game (visible in the relayer log as `Skipping latest finalized height N` against EVM-84532). Both rows cleared the finalized-height bar about 30 minutes after delivery, but were waiting on the next delivery to act as a channel trigger as described above.

## Open questions

1. **One commitment, one reward, regardless of message size.** The consensus claim is a flat per-destination value. Do we want the request claim flat too, or scaled by request body size? Flat is simpler and matches the creator's wording ("award BRIDGE tokens"). Recommend flat for v1 and revisit if operators report it under or overpaying.
2. **Payee account type.** Sr25519 only (matches consensus claim) or also support Ed25519 / EVM addresses on the payee side? Recommend Sr25519 only for v1.
3. **Claim ordering vs delivery race.** There is a first-wins race on the destination side: whichever relayer lands the delivery transaction first has its address written into `RequestReceipts[commitment]`, and every later delivery of the same commitment no-ops. The race winner is unambiguous because the receipt slot records exactly one address, and `process_outbound_request_delivery_claim` only pays the address proven in that slot, so a relayer that delivered second fails `OutboundRequestSignerMismatch`. Idempotency is automatic and no extra logic is needed, but operators should understand the reward goes to the fastest deliverer, not the first to claim.
4. **Treasury funding model.** The same `T::TreasuryPalletId` already used by the consensus claim. Governance tops it up. No new account.
