# 2026-08-19 — Not adopted: soft-failing the prefund, and oracle-derived validity bounds (#1071)

Rejected: returning `prefunded = false` instead of reverting in the mode-2 `_prefund`. Upstream advises it to protect bundler reputation, but reading EntryPoint v0.8 shows both outcomes are `revert FailedOp` — AA33 for a paymaster revert, AA34 for a sig-failure — so both revert `handleOps` identically. The change would trade the `Permit2Failed(token, reason)` diagnostic, which carries Permit2's own revert data, for no bundle-level benefit.

Also deferred at the maintainer's direction: bounding `validationData`'s `validUntil` by oracle freshness so bundlers drop soon-to-be-stale ops instead of building bundles that revert. Sound in principle and would have made stale-oracle failures expire cleanly, but it touches every pricing path and was out of scope for this pass.
