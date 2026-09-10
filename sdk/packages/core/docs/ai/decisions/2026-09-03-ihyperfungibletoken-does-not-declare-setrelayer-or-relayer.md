# 2026-09-03 — `IHyperFungibleToken` does not declare `setRelayer` or `relayer`

`supportsInterface` returns true for `type(IHyperFungibleToken).interfaceId`. Extending the
interface changes that id, so existing deployments would stop matching it and new ones would report
an id integrators have not seen. The relayer functions are reachable through the contract type.
