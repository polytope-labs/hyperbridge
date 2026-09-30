# 2026-09-05 — `relayer()` on both contracts, `version()` on the gateway from `Initializable`

Chosen: the manager and the gateway both answer `relayer()`. The gateway's `version()` returns
`_getInitializedVersion()` from OpenZeppelin's `Initializable`: 1 once `initialize` has run, and
raised only by a `reinitializer(n)` migration, so it tracks the storage-level migrations a proxy
has been through rather than a number someone has to remember to bump. Implementations from
before the gate have no `version()` at all and revert. The manager is not upgradeable and gets no
`version()`; a manager without `relayer()` is one from before the admin became the relayer.

The underscore-public convention in `IntentsBase` (`_filled`, `_orders`, ...) would have given
`_relayer()` for free, but a named getter is what the interface should publish, and the gateway
had 26 bytes of headroom: `relayer()` only fits once the auto-generated getter is dropped, and
`version()` needed the `_instances` getter dropped as well. That getter duplicated
`instance(bytes)` and nothing off-chain called it.

Alternative rejected — a hand-maintained version constant, as a semver string or a number. The
string cost 82 bytes and did not fit; both would drift from what is actually deployed.
