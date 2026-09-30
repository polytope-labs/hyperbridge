# 2026-08-31 — MPCVault RPC failures name their call, and execute retries the create/execute race

`MpcVaultService` rethrows gRPC failures naming the RPC, the signing-request uuid and MPCVault's
x-request-id (previously the bare grpc-js error propagated, so a production
`3 INVALID_ARGUMENT: Invalid uuid` could not be attributed to create vs execute from the order
log). The app-level error guard now also trips on code-only errors — MPCVault sends `message: ""`
with only a code set. `executeSigningRequest` retries INVALID_ARGUMENT/NOT_FOUND twice with short
backoff because MPCVault intermittently rejects a uuid its own createSigningRequest just returned,
before the callback co-signer is contacted. Created uuids are logged at debug for dashboard
correlation.
Files: src/services/wallet/mpcvault.ts.
