# 2026-09-21 — Testnet IntentGateway on BSC Chapel and Polygon Amoy

`config-testnet.json` sets `intentGatewayV3` to `0x6CF42FA9BecbC5b6a26884964956b113530f7cFA`, the
`IntentGateway` in the SDK's testnet chain config, on BSC Chapel (EVM-97) and Polygon Amoy (EVM-80002).

- On Chapel it replaces `0xFbF50B2b32768127603cC9eF4b871574b881b8eD`, whose events are no longer indexed.
- On Amoy the gateway was not configured before, so its order and fill handlers are new there.
