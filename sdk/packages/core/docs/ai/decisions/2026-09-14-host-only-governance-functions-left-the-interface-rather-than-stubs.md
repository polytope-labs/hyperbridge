# 2026-09-14 — Host-only governance functions left the interface rather than being kept as stubs

Chosen: `setRelayer` and `upgradeToAndCall` are gone from `IIntentGatewayV2`. They now live on
the `ExtrinsicModule`, which `Execute` delegatecalls directly, and the implementation behind the
proxy has neither selector. This supersedes the 2026-09-05 decision that `Execute` delegatecalls
the implementation: the branch in `ExtrinsicIntents.onAccept` now delegatecalls the module's own
address (`__self`), so the host-only functions run where their code is.

Keeping them declared would advertise functions the proxy cannot answer, the same drift this
interface was resynced to remove in August. Nothing outside governance ever called them: both are
`onlyHost`, and the pallet forwards the calldata governance supplies verbatim, so only the
selectors matter, and they are unchanged on the module.

Alternative rejected — thin forwarding stubs on the implementation. About fifteen lines that
exist only so the interface can keep two names, and they would put the upgrade path behind an
extra hop through the implementation instead of running it where the code is.

Consequence accepted — an upgrade's init data runs against the new implementation, where
`setRelayer` does not exist, so a relayer rotation can no longer ride in an upgrade. Upgrade and
rotation are two `execute_on_gateway` calls.
