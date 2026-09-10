# 2026-09-08 — The key exchange list is ours, not the client's

ssh2 offers diffie-hellman-group16/17/18-sha512 by default and negotiates by the client's
preference order, which makes the algorithm an unauthenticated peer's choice. Measured on this
machine: group14 2.2ms, group16 14.4ms, group18 107.2ms of synchronous server-side DH — on the
same event loop that prices and fills orders, and repeatable via rekey without ever attempting
to log in. Rate-limiting was the alternative, but the failure counter only sees login attempts,
and a per-source connection cap would still leave the first handshake expensive. Restricting the
offer removes the lever instead of policing it: curve25519 and the ECDH groups cover every SSH
app anyone pairs, and group14 stays as a floor at ~2ms.
