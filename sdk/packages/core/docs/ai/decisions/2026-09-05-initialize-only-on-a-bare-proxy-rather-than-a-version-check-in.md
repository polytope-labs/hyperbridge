# 2026-09-05 — `initialize` only on a bare proxy, rather than a version check in the upgrade path

Chosen: `initialize` is refused unless the proxy is at version 0. `migrate` is the only entry to
`VERSION` for a proxy that already has one, and it is host-only.

The hole both options close: `initialize` has no caller restriction, because a bare proxy is
initialized atomically in its constructor. An `UpgradeContract` that installs this implementation
on a version-1 proxy without `migrate` calldata would leave the proxy below `VERSION` with
`initialize` callable by anyone, who could then set the params and the relayer.

Alternative rejected — require the version to have risen after `upgradeToAndCall`. It forces
every upgrade to carry a migration, so a same-implementation upgrade carrying only `setRelayer`,
the rotation path, would be refused; scoping the check to implementation changes fixes that but
adds a branch and an implementation-slot read to every upgrade. And it cannot protect the first
upgrade off the live pre-gate implementation, whose `onAccept` has no such check. The `initialize`
guard covers that case too, since it is the new implementation's code that runs `initialize`.
