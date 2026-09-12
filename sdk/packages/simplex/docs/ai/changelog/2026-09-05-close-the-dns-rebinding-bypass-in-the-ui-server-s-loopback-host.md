# 2026-09-05 — Close the DNS-rebinding bypass in the UI server's loopback Host check

`isLoopbackHost` decided loopback with `host.startsWith("127.")`, a string-prefix test on a
hostname. A leading-digit DNS label is legal, so `127.0.0.1.evil.com` (and `127.evil.com`,
`127.0.0.1.nip.io`, the bare `127.`) passed it. That predicate is the whole of the UI server's
auth — `hostHeaderAllowed` delegates to it on the `boundLoopback` path, and there is no
Origin/CORS check — so a page an operator visits could rebind DNS to `127.0.0.1`, become
same-origin, and reach `POST /api/send` (drains the solver wallet + vault positions) and the
unmasked keys in `GET /api/chains`. Now the host must parse as an IPv4 literal via `node:net`
`isIP` before its `127.` octet is trusted; `localhost`, `::1`, and the IPv4-mapped `::ffff:127.0.0.1`
stay allowed. The same change fixes the `--ui 127.x.evil.com` bind-gate bypass (same function) and
replaces the non-loopback branch's `hostname.includes(":")` IPv6 test with `isIP`, which also
rejects a stray non-numeric port like `evil.com:abc`. Extended the rebinding test with the bypass
vectors; it fails on the old prefix code and passes now (60/60).
Files: src/services/server/http-util.ts, src/tests/ui-server.test.ts.
