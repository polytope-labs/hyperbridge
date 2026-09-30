# 2026-09-09 — Fix SSH tunnel auth bypass: strict signature verification + key-algorithm check

The embedded SSH server's public-key auth accepted a forged signature. ssh2's `key.verify()` returns
`true` on success but an `Error` object (not `false`) on a critical failure such as an unsupported digest
for the key; the handler used `if (!key.verify(...))`, and `!errorObject` is `false`, so the rejection was
skipped and the client was authenticated. The Error path is reachable without any private key via
key-algorithm confusion: an attacker who knows a paired device's public key (public by design) offers that
ed25519 key blob tagged as an RSA signature algorithm — it parses as ed25519 so its fingerprint matches an
authorized device, while ssh2 drives `verify()` with a SHA-2 digest ed25519 cannot compute, which throws.
A tunnelled session has full operator privileges (`/api/send`, `/api/vault/*`), so this was a fund-drain
path over the public relay port. Reproduced end-to-end with a real ssh2 client holding no private key.

Two-part fix in the `authentication` handler: (1) require the parsed key type to equal the declared
signature algorithm (`key.type === ctx.key.algo`; ssh2 already normalises rsa-sha2-256/512 to `ssh-rsa`),
rejecting the confusion up front and never rejecting an honest client; (2) require a strict boolean
`key.verify(...) !== true`, which closes the Error-return bypass for any key type. Added a regression test
that drives the exact forged-key exploit and asserts rejection — it fails against the old code and passes
against the fix. Also gave `FakeRelay`'s forwarded pipe `error` listeners so a rejected handshake's EPIPE
no longer surfaces as an unhandled error attributed to a later test.

Files: src/services/tunnel/EmbeddedSshServer.ts, src/tests/tunnel.test.ts.
