# 2026-09-03 — `setRelayer` accepts the host, not `address(this)`

Chosen: `msg.sender == _owner || msg.sender == host()`.

The first draft allowed `address(this)`, expecting `upgradeToAndCall` to call the proxy. It does
not: it delegatecalls the migration calldata, so `msg.sender` is still the host that invoked
`onAccept`. The fork test against the live mainnet proxy failed with `Unauthorized` and exposed it.
Accepting the host adds no trust: the host already gates every callback, and it never calls the
gateway with any selector other than the `IApp` callbacks, so the branch is reachable only from
governance migration calldata that has already passed the hyperbridge-source check and the relayer
gate.
