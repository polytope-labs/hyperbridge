# 2026-09-08 — Every generated ed25519 pair is parsed before it is stored

ssh2's `generateKeyPairSync("ed25519")` returns a pair its own `parseKey` rejects about once in
256 — 28 of 5,000 in a direct measurement, which is the rate you get from a dropped leading zero
byte. The alternative was to treat it as a rare transient and let the caller retry, but the
operator and host keys are written to disk on first boot and re-read on every start: a bad one is
not transient, it is remote access permanently broken with "Malformed OpenSSH private key" until
someone deletes the file by hand. Generating our own keys with `node:crypto` and encoding the
OpenSSH format ourselves would remove the dependency on ssh2's generator entirely, but that is a
lot of format code to own for a bug a round-trip check catches. `generateKeyPair` therefore
generates, parses both halves, and retries up to 8 times; 8 consecutive failures is (1/256)^8.
