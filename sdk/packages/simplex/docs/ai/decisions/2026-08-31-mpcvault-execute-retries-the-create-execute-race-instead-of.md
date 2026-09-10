# 2026-08-31 — MPCVault execute retries the create/execute race instead of pausing before execute

Chosen: `executeSigningRequest` retries INVALID_ARGUMENT and NOT_FOUND up to twice (300ms, then
600ms) before failing. MPCVault's backend intermittently does not recognise a signing-request uuid
its own `createSigningRequest` just returned: live fills failed with
`3 INVALID_ARGUMENT: Invalid uuid` and no callback-server contact, and since MPCVault only contacts
the callback co-signer during execute, the rejection precedes any approval step — the request
exists but the execute handler cannot see it yet.

Alternatives. An unconditional sleep between create and execute taxes every signature to paper over
a fault that shows up in a small minority of calls. Retrying every gRPC error is unsafe: after an
ambiguous failure (DEADLINE_EXCEEDED, UNAVAILABLE mid-call) the ceremony may already be running,
whereas INVALID_ARGUMENT/NOT_FOUND mean the server did nothing, so re-sending is idempotent. The
retry lives inside `executeSigningRequest` rather than in callers because the uuid is what is being
retried — recreating the request instead would orphan one pending signing request per attempt in
the vault.
