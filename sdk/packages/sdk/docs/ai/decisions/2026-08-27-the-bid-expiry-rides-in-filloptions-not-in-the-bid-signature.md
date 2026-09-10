# 2026-08-27 — The bid expiry rides in `FillOptions`, not in the bid signature

Chosen: `validUntil` is a field on `FillOptions`, checked by `fillOrder` at execution.

The alternative was to put the expiry in the ERC-4337 signature blob and return it from
`SolverAccount.validateUserOp` as a `validUntil` validation range — the mechanism 4337 provides for exactly this.
That was built first and then abandoned, for reasons worth recording:

- **It needs a new signed field.** `userOpHash` does not cover `op.signature`, so an expiry carried there is
  rewritable by whoever replays the bid. Making it tamper-proof meant changing what the solver signs (an EIP-712
  `BidValidity` digest) and widening the selection signature 162 → 168 bytes.
- **That is a `SolverAccount` redeploy.** The account is reached by EIP-7702 delegation, so a new version means a new
  address and every solver re-delegating — and, since the old account keeps accepting the old format forever, it
  retires nothing already signed.
- **`FillOptions` needs none of that.** The options are part of `callData`, which `userOpHash` *does* cover. The
  expiry is authenticated for free, with no signature format change, no account redeploy, and no migration.
- **It covers more.** The signature-side check only bounds solver-selection bids. A check in `fillOrder` bounds every
  path into it.

The cost is that this fires at execution rather than validation: an expired bid is included, the nonce is consumed
and the account pays that op's gas, where a validation-time range would have had the bundler drop it for free. That
is a bounded, one-off cost per bid — and consuming the nonce permanently retires the bid, which the validation-time
version does not do. Fund loss, the thing that matters, is prevented either way.

Denominated in blocks rather than a timestamp so it reads against the same clock as `order.deadline` (`_blockNumber()`,
the L2 block number where those differ), and so the two cannot disagree about what "expired" means.

`0` means unbounded. That is the right default for a solver filling directly — it is only exposed to its own
staleness — and it keeps every existing caller working. The protection is opt-in by the party that needs it.
