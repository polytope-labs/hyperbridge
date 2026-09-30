# 2026-09-07 — `SetRelayer` request kind plus `migrate`, not an `Execute` kind

Chosen: rotation is `RequestKind.SetRelayer` (one ABI word), and the live proxies are armed by
`migrate(relayer)` carried as the `init_data` of the existing `upgrade_paymaster` extrinsic, so no
runtime release is needed for the upgrade itself. The gateway's `Execute` (raw delegatecall of
governance calldata) was not copied: the paymaster's whole admin surface is already enum
dispatch, and one more kind is smaller and easier to audit than an arbitrary-calldata door.
