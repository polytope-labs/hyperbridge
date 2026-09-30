# 2026-09-09 — Make the SSH tunnel's publickey guard fail closed

The embedded SSH server treated a publickey request as an unsigned probe whenever *either*
`ctx.signature` or `ctx.blob` was missing (`||`), and answered it with `ctx.accept()`. That is not
neutral: ssh2's `PKAuthContext.accept()` sends PK_OK only when there is no signature, and
authenticates outright when there is one. So a request carrying a signature but no blob would have
been authenticated having never reached `key.verify()`.

Not exploitable against the pinned `ssh2@1.17.0`: its USERAUTH_REQUEST parser builds `methodData`
with `signature` and `blob` together or not at all, and a signed request whose signature fails to
parse is a fatal protocol error that never raises an `authentication` event. The guard was therefore
resting entirely on an undocumented internal invariant of a dependency — the same kind of assumption
that produced the bypass fixed earlier the same day, where the trusted invariant was the return type
of `verify()`. The probe branch now requires *both* halves absent, and a half-populated request is
refused as malformed.

Adds a regression test that drives the handler with each shape (signature only, blob only, neither)
and asserts the first two are rejected while an honest probe still gets its PK_OK; it fails against
the old `||` guard with `accepted: true`.

Files: src/services/tunnel/EmbeddedSshServer.ts, src/tests/tunnel.test.ts, package.json (0.16.1).
