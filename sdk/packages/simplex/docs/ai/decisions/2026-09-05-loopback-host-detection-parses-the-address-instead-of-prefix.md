# 2026-09-05 — Loopback Host detection parses the address instead of prefix-matching the string

Chosen: `isLoopbackHost` returns true only for `localhost`, `::1`, `::ffff:127.0.0.1`, or a string
that `node:net` `isIP` confirms is an IPv4 literal whose first octet is `127`. The DNS-rebinding
guard (`hostHeaderAllowed`) and the wizard bind-gate both hang off this one function, so it is the
security boundary for an unauthenticated API that can move funds.

Alternatives rejected:
- Keeping `startsWith("127.")` and adding a separate DNS-name blocklist — the prefix test is the
  vulnerability (a leading-digit label like `127.0.0.1.evil.com` is a legal hostname); anything that
  still trusts the raw string is one creative label away from another bypass.
- A hand-rolled `/^127(\.\d{1,3}){3}$/` regex — narrower than the real loopback block, and it would
  reject valid forms like `127.1`; delegating to `isIP` uses the platform's own parser and stays
  correct for every IPv4 spelling.
- An allowlist of exact strings (`127.0.0.1`, `localhost`) — would break operators who legitimately
  bind another `127.0.0.0/8` address, and still needs `isIP` to reject look-alikes safely.

Also folded in: the non-loopback branch of `hostHeaderAllowed` used `hostname.includes(":")` as its
IPv6 test, which accepted a non-numeric port that survived the `:\d+$` strip (`evil.com:abc`).
Replacing it with `isIP(hostname) !== 0` closes that and unifies the two branches on one parser.
