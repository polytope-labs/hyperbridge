# BSC consensus client: validator-set rotation hardening

Hardens the on-chain BSC (Parlia) consensus client and the relayer that feeds it around
authority-set rotation. All three fixes concern how and when the client promotes a staged
validator set.

## 1. Rotation point tracks the Bohr `TurnLength`

`parse_extra` previously skipped the Bohr turn byte without reading it, and the client staged
`rotation_block = epoch_boundary + validators.len() / 2` — the pre-Bohr rule. Since the Bohr
fork the chain switches the validator set at
`minerHistoryCheckLen() = (len / 2 + 1) * TurnLength - 1` blocks into the epoch (21 validators,
`TurnLength = 8` → +87, not +10).

- `ExtraData` now carries the `turn_length` byte, and `primitives::rotation_block(epoch_boundary,
  validator_count, turn_length)` computes the Bohr-aware switch point used when staging.
- The relayer's enactment scan starts at the stored `rotation_block` instead of re-deriving the
  pre-Bohr offset, and no longer blocks the task on a not-yet-produced enactment block — it
  falls through to a steady-state update signed by the current set, so `finalized_height` keeps
  advancing through the window before `rotation_block`.

## 2. Promotion gated on the signed source height

Promotion is gated on `source_header.number`, not the attested header's number. The attested
header's `number` is not covered by the BLS vote signature, so gating on it let a compromised
*outgoing* set keep itself in charge past its rotation: it could sign a vote with a
`target_number` below `rotation_block` while finalizing a forged source past it, and choose the
unsigned attested height to dodge promotion.

Parlia verifies a vote for target `T` against the snapshot at `T - 1`, and the snapshot switches
sets when `rotation_block` is applied. With the adjacency check (`source` is `target`'s parent,
`target` is `source`'s direct child) the finalized source is `T - 1`, so the incoming set signs a
finalized source exactly when `source_header.number >= rotation_block`. The client now promotes
when `source_header.number >= rotation_block` and `source_epoch == rotation_epoch`.
`source_header.number` is bound by the signed `source_hash`, so a compromised outgoing set can
never finalize a source at or beyond its rotation block. This also removes the two-block early
promotion that gating on the attested height caused.

The earlier `attested_header.number == target_number + 1` binding is removed: since Fermi
(BEP-590, mainnet 2026-01-14) a Parlia vote may justify any of the attesting header's last three
ancestors, so the attesting header no longer always sits at `target_number + 1` (e.g. mainnet
block 132941639 votes for target 132941637), and the source-height gate needs no such binding.

## 3. Staged set must activate in `current_epoch + 1`

Binding the epoch ancestry to `source_header`'s own epoch boundary still allowed staging the
*current* epoch's own set as `next_validators` (a source in epoch E carrying a full-epoch ancestry
back to E's boundary). Such a set can never promote and strands the client once finality crosses
into E+1. After extracting a staged set the client requires its activation epoch to equal
`current_epoch + 1`; the honest flow (stage epoch E+1's set while in epoch E) is unaffected.
