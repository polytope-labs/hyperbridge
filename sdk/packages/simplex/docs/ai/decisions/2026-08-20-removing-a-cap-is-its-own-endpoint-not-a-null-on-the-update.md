# 2026-08-20 — Removing a cap is its own endpoint, not a null on the update

Chosen: `DELETE /api/strategies/:index/max-order-size`, with `AdminStrategy.clearMaxOrderSize` and
`Simplex.clearMaxOrderSize(index)` behind it. `PUT /api/strategies/:index` still requires a positive
value.

The obvious alternative — accept `maxOrderSize: null` (or `""`) on the PUT — is one fewer endpoint
and was rejected on blast radius. `DELETE /api/strategies/:index` already removes the _market_. A
scheme where a cap removal and a market removal differ by a path suffix is bad enough; one where a
cap removal is a field value that a serialization bug, a form default, or a stray `undefined` can
produce is worse, because the failure is silent and the fix is a re-add. An explicit verb on an
explicit resource cannot be reached by accident.

The handler is idempotent: clearing an already-uncapped market returns 200 with the same state
rather than 404 or 409. The UI decides what to send from the field's contents alone and does not
track which state the market is in, so a strict version would make the client carry knowledge it has
no reason to have.

Alternatives rejected:

- _`PATCH` with `maxOrderSize: null`._ See above.
- _A `capped: boolean` toggle beside the value._ Two controls for one setting, and it forces a
  decision about what the value field holds while the toggle is off — remembered, cleared, or
  ignored. A blank field is the same information with nothing to keep in sync.
- _Reuse `PUT /api/strategies/:index` with an empty body._ Ambiguous with a no-op update, and the
  handler already rejects unknown/absent fields to catch exactly that class of mistake.

In the UI the field itself is the control: blank means uncapped, the button relabels to "Remove
cap", and it enables on any divergence from the persisted value rather than on a non-empty value —
the old guard made blanking unsubmittable, which is precisely the state that now needs submitting.
