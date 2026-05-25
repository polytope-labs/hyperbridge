# Pallet Messaging Incentives

Mints a reputation asset to the relayer that delivered each ISMP message, scaled
by the byte-size of the delivered payload. It plugs into `pallet-ismp` as a
[`FeeHandler`] and only fires on successful message execution.

The pallet is deliberately minimal — no per-session accounting, no treasury
draws, no protocol-fee bookkeeping. Hyperbridge's other incentive surfaces
(BEEFY proof rewards, prepaid bandwidth, EVM fee tokens) live in their own
pallets.

---

## Overview

1. `pallet-ismp` finishes executing a batch of messages and invokes
   [`FeeHandler::on_executed`].
2. This pallet reads the current `MintPerByte` rate. If it is zero, nothing
   happens (the pallet stays installed but inactive).
3. For each `Request`/`Response` message it:
   - Computes `bytes = max(body_size, 32)` — the same floor `pallet-bandwidth`
     uses, so trivial payloads can't game the mint by costing zero.
   - Multiplies `bytes × MintPerByte` to get the mint amount.
   - Recovers the relayer's account from the sr25519 signature on the message's
     `signer` field.
   - Mints that amount of `ReputationAsset` to the relayer.
4. Consensus messages and any non-Request/Response variants are skipped.
5. `pays_fee = Pays::No` on the dispatch info — the relayer is never charged
   here.

---

## Storage

| Item | Type | Description |
|------|------|-------------|
| `MintPerByte` | `BalanceOf<T>` | Reputation units minted per delivered byte. Zero disables minting. |

---

## Extrinsics

| Call | Origin | Effect |
|------|--------|--------|
| `set_mint_per_byte(amount)` | `AdminOrigin` | Update the per-byte mint rate. Pass `0` to disable. |

---

## Events

| Event | Description |
|-------|-------------|
| `MintRateUpdated { amount }` | Governance set a new per-byte mint rate. |
| `ReputationMinted { relayer, bytes, amount }` | Reputation successfully minted to `relayer` for delivering `bytes` of payload. |

If `ReputationAsset::mint_into` fails (unusual — typically a frozen asset),
the failure is logged at `WARN` against the `messaging-incentives` target and
the rest of the batch continues.

---

## Traits implemented

- [`pallet_ismp::fee_handler::FeeHandler`] — entry point from `pallet-ismp`'s
  `FeeHandler` tuple.
- [`IncentivesManager`] — re-exported here for the `pallet-collator-manager`
  `Config` bound. The canonical impl is a no-op `reset_incentives()` because
  this version doesn't accumulate per-session state.

---

## Runtime integration

```rust
impl pallet_messaging_incentives::Config for Runtime {
    type ReputationAsset = ReputationAsset; // see runtime for the fungible item
    type AdminOrigin = EnsureRoot<AccountId>;
}
```

And wire it into `pallet_ismp`'s `FeeHandler`:

```rust
impl pallet_ismp::Config for Runtime {
    // ...
    type FeeHandler = (
        // other handlers
        pallet_messaging_incentives::Pallet<Runtime>,
    );
}
```
