# 2026-09-17 — Gateway owner lives in the implementation and only pauses placement

Chosen: the owner is stored by `IntentGatewayV2` at an ERC-7201 namespaced slot, and its only
power is pausing `placeOrder`. Relayer rotation, params, dust and upgrades stay with governance, as
the 2026-09-05 decision on `setRelayer` settled: an owner key that could redirect deliveries without
a governance message is what that decision removed. Pausing placement cannot move funds, and it is
the operational lever a storage-reinterpreting upgrade needs to drain in-flight orders first.

Alternative rejected — a sequential variable appended to `IntentsBase`. It works, but it makes the
owner part of the layout the modules share for a value no module reads.

Alternative rejected — a sequential variable declared in `IntentGatewayV2`. Storage of the most
derived contract comes after every base's, so the next field appended to `IntentsBase` would take
its slot and silently move the owner; it also breaks the module layout test's variable count.

Alternative rejected — an immutable owner, as before the module split. Rotating it needs a new
implementation and an upgrade, and governance could not replace a lost key without one.

Two-step transfer with the host allowed to propose keeps a mistyped owner recoverable and lets
governance replace the owner without the owner's key.
