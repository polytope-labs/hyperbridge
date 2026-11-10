# BSC consensus client: validator-set rotation hardening

Hardens the on-chain BSC (Parlia) consensus client around authority-set rotation. Three
independent issues are fixed; all live in the on-chain verifier/client, not the relayer.

## 1. Rotation point now tracks the BOHR `TurnLength`

`parse_extra` previously skipped the BOHR turn byte without reading it, and the verifier
staged `next_validators.rotation_block = epoch_boundary + validators.len()/2` — the
pre-BOHR rule. Since the BOHR fork the real chain rotates at
`minerHistoryCheckLen() = (len/2 + 1) * TurnLength - 1` into the epoch (e.g. 21 validators,
`TurnLength = 8` → +87, not +10). The stale offset opened the promotion window ~80 blocks
early, so for that window the client promoted the incoming set and then rejected every
still-valid outgoing-set-signed update, stalling `finalized_height` for ~80 blocks each
epoch.

- `ExtraData` now carries `turn_length` (read from the epoch header's turn byte, default 1
  pre-BOHR / for non-epoch headers, clamped to ≥ 1).
- New `primitives::rotation_block(epoch_boundary, validator_count, turn_length)` computes the
  authoritative point; both verifier staging branches use it.
- The off-chain `bsc_prover::get_rotation_block` is intentionally unchanged: it is only a
  relayer scan lower bound (the enactment loop walks forward and locally re-verifies each
  candidate), so a pre-BOHR value is still safe. Its doc comment now points at the on-chain
  helper as authoritative.

## 2. Promotion bound to the signed finalized height

The promotion gate reads `attested_header.number`, which is **not** covered by the BLS vote
signature (the vote signs `vote_data` only). A submitter could set `attested_header.number`
independently of the finalized `source_header`, choosing which validator set verifies an
update — e.g. keeping a compromised *outgoing* set in charge while finalizing a forged
`source_header` past the rotation block.

Fix: `verify_bsc_header` now binds the attested header to the signed vote. A Parlia vote
attestation always justifies the attesting block's direct parent, so the carrying header
sits at `vote_data.target_number + 1`. `target_number` is signed; `attested_header.number`
must equal `target_number + 1` (`Error::AttestedHeaderNotChildOfTarget`). The promotion gate
keeps using `attested_header.number`, which is now pinned to signed data, so it can no longer
be decoupled from the finalized source. This keeps the gate aligned with the relayer's
enactment loop (which selects the enactment update by local verification at the attested
height), avoiding any liveness regression.

## 3. Staged set must activate in `current_epoch + 1`

Binding the epoch ancestry to `source_header`'s own epoch boundary still allowed a caller to
stage the *current* epoch's own set as `next_validators`: a source in epoch E carrying a
full-epoch ancestry back to E's boundary extracts E's set with a `rotation_block` already in
the past. That set can never promote (its rotation epoch is E, which finality has left), and
once finality crosses into E+1 the client is stranded until governance resets it — a
permissionless freeze reachable at any epoch boundary.

Fix: after extracting a staged set, the client requires its activation epoch
(`compute_epoch(rotation_block)`) to equal `current_epoch + 1`
(`Error::UnexpectedNextValidatorsEpoch`). The honest flow — stage epoch E+1's set while in
epoch E — is unaffected; staging the current (or any non-next) epoch's set is rejected.

## Files

- `modules/consensus/bsc/verifier/src/primitives.rs` — `turn_length` on `ExtraData`, turn-byte
  read, `rotation_block` helper.
- `modules/consensus/bsc/verifier/src/lib.rs` — turn-length-aware staging, attested-header
  binding, tests.
- `modules/consensus/bsc/verifier/src/error.rs` — `AttestedHeaderNotChildOfTarget`,
  `UnexpectedNextValidatorsEpoch`.
- `modules/ismp/clients/bsc/src/lib.rs` — staged-set epoch guard; promotion comment.
- `modules/consensus/bsc/prover/src/lib.rs` — doc comment only.

## Validation

`cargo test -p bsc-verifier --lib` (9 passed), `cargo test -p bsc-prover --lib
get_rotation_block` (4 passed). `bsc-verifier`, `ismp-bsc`, `bsc-prover`, `tesseract-bsc`
build clean.
