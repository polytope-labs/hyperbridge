# 2026-09-03 — Coalesce overlapping vault saves to the latest draft

Chosen: when a vault save is already active, remember a subsequent Save click and send the latest
editor rows after the current request and config refresh finish. Keep the queue local to vault saves,
and require the API's `persisted` flag before presenting success. This preserves edits made during
slow vault hydration without changing the semantics of unrelated shared actions.

Alternatives rejected: the shared `useAction` guard previously dropped the second click without any
feedback, leaving the newer visible draft unsaved. Allowing parallel vault reconfiguration requests
would race runtime hydration and whole-file persistence. Disabling the editor for the full request
would prevent useful drafting during slow on-chain validation and still require the operator to
notice when saving becomes available again.
