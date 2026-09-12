# 2026-09-03 — `onPostRequestTimeout` is gated on the token, unlike the gateway

Timeouts mint a refund to the original sender, so a forged timeout proof mints. The intent gateway
does not gate its timeout callbacks because `HyperApp`'s defaults revert and it dispatches with no
timeout.
