# 2026-09-09 — SSH tunnel auth: strict verify + declared-algorithm must match the key

Decided: in the embedded SSH server, reject a login unless `key.verify(...) === true` (strict) AND the
parsed key type equals the client-declared signature algorithm (`ctx.key.algo`). This fixes an auth bypass
(see ChangeLog 2026-09-09).

Why the strict `!== true` and not just fixing the confusion vector: ssh2 documents `verify()` as returning
`false` OR an `Error` on failure, and its own examples use `!== true`. Treating only the confusion case
would leave any other Error-returning failure (future ssh2 versions, other digest edge cases) as a bypass.
The strict check is the load-bearing fix; it alone closes the demonstrated exploit.

Why also the key-type/algorithm match, given the strict check already covers it: defence in depth and a
clear rejection reason, and it stops the confused request before `verify()` is even attempted. It is safe
for every legitimate key type — ssh2 normalises rsa-sha2-256 and rsa-sha2-512 to `ssh-rsa`, so an honest
RSA client's `ctx.key.algo` (`ssh-rsa`) still equals its parsed `key.type` (`ssh-rsa`), and an ed25519
client matches trivially.

Rejected: pinning device keys to ed25519 only. It would also close the specific exploit, but the pairing
flow (`addDevice`) accepts any OpenSSH public key an operator pastes, so pinning would lock out a device
legitimately paired with an RSA or ECDSA key — a functional regression for a weaker reason than the two
checks above, which are correct for all key types.
