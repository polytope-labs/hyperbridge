# 2026-08-31 — Review fixes: zero-code guard, abandoned-request cleanup, logger and credentials injection, unit tests

Four fixes from review of the MPCVault retry change. `apiErrorText` no longer treats enum value 0
as an error — both code fields define 0 as UNSPECIFIED, so the previous `!== undefined` check would
have failed every signature against a server that sends an explicit zero; the guard is also now
framed as defensive, since the observed `{"code":16,"message":""}` was a gRPC status envelope, not
the in-band `Error` message. A signing request whose execute fails terminally is best-effort
rejected in the vault (`rejectSigningRequest`) instead of staying pending forever — a pending
request nobody will execute is a standing authorization to sign a stale payload. `MpcVaultClientConfig`
and `MpcVaultSignerConfig` accept an injected `logger` (the default process-wide context is
invisible to embedded fillers that pass `SimplexOptions.logger`) and `credentials` (the TLS
hardcoding left no test seam). New unit tests run `MpcVaultService` against an in-process gRPC
server: retry-then-sign, retries-exhausted with reject and error naming, code-only errors,
UNSPECIFIED-zero, and create-failure naming. Create deliberately gets no retry — see Decisions.md.
Files: src/services/wallet/mpcvault.ts, src/services/wallet/types.ts,
src/services/wallet/accounts/mpc.ts, src/tests/wallet/mpcvault.test.ts.
