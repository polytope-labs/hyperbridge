# 2026-09-03 — Treat a persisted vault save as UI success

Chosen: after a vault update persists successfully, emit a success toast and do not turn the server's
`restartNeeded` advisory into a vault-editor error. Emit the toast after the queued-save loop drains so
one operator action receives one confirmation for its final saved draft. Keep the API response shape
intact and leave all runtime and server behavior unchanged.

Alternative rejected: presenting `restartNeeded` as a failed save incorrectly tells the operator to
restart the filler even though the vault edit was accepted and saved.
