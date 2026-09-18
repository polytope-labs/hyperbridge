# 2026-08-27 — The implementation address identifies the FillOptions shape

Chosen: `getFillOptionsVersion` reads the ERC-1967 implementation slot and checks the address
against `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS`, a set of implementations deployed before
`validUntil` existed. Anything else is v2.

EIP-1967 standardises three slots — implementation, admin, beacon — all holding addresses. There
is no version field in the spec to read, and OZ `Initializable`'s `uint64` only moves under
`reinitializer(N)`, which this contract does not use. The implementation address is the only value
the proxy actually updates on upgrade, so it is what identifies the deployed code.

The list is of **legacy** implementations, not current ones, so the default is v2. That direction
is the whole point: a newly shipped implementation needs no edit here, and once every deployment
is upgraded the set is vestigial and still correct.

A single entry covers every chain that runs it. The protocol contracts are CREATE2-deployed, so
`0x976B268b06f545c4A2BF44866Aa2465bd8B3C67d` is the pre-`validUntil` implementation on those
chains — confirmed with the maintainers rather than inferred, since the CREATE2 claim in the tree
is about the proxies and does not by itself say anything about implementations.

`CHAINS_WITHOUT_VALID_UNTIL` covers the rest. The testnets have not been redeployed and their
implementation addresses are not tracked here, so the address check alone would read them as
current and every fill would revert on a selector that does not exist. It is checked before the
slot read, both because the address is uninformative there and because it saves a round trip.
Delete a chain from that set as its gateway is redeployed; once it is empty the address check
covers everything on its own. Listing known-good implementations instead
would be the version constant this replaced wearing a different hat — a value someone must
remember to update on every upgrade, where forgetting breaks every fill on that chain.

Only v2 answers are cached, keyed by proxy address. A deployment can move from legacy to current
but never back, so a v2 result is true forever; caching a v1 result would pin the old encoding
across the very upgrade that changes it, since the proxy address does not move and nothing would
invalidate it. A still-legacy gateway therefore costs one storage read per fill, an upgraded one
costs none.

Also considered and dropped: scanning the implementation's runtime code for the v2 `fillOrder`
selector. It needs no address list and self-updates, and the selector does survive `via-ir` and
the optimizer — but it is a heuristic (a 4-byte sequence can appear in non-dispatcher data), and
both failure directions break every fill on the chain, since the two shapes cannot decode each
other. An address match is exact.

Earlier still, and rejected: a `fillOptionsVersion()` getter on the contract. A hand-maintained
integer is a second source of truth that answers what a deployment claims rather than what it can
decode.
