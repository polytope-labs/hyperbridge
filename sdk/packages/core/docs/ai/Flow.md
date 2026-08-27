# Flow

AI-maintained map of how code paths in `sdk/packages/core` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## What `IIntentGatewayV2` is, and what actually reads it

`contracts/apps/IntentGatewayV2.sol` is a declaration-only file: structs, errors, events, and
external function signatures. There is no implementation of `IIntentGatewayV2` in this package —
the gateway lives at `evm/src/apps/IntentGatewayV2.sol` and inherits its events and errors from
`evm/src/apps/intentsv2/IntentsBase.sol`.

Two consumers, and they use it very differently:

1. **`evm/src/apps/intentsv2/SolverAccount.sol`** imports it and reads exactly two things:
   `IIntentGatewayV2.select.selector` and `IIntentGatewayV2.fillOrder.selector`. It resolves via the
   `@hyperbridge/core/` remapping in `evm/remappings.txt`, which points at
   `node_modules/@hyperbridge/core/contracts/` — a workspace symlink, so an edit here is picked up
   by `forge build` in `evm/` with no publish step. Only the two function signatures matter to it.
2. **Integrators**, who read the file as the gateway's published ABI surface. Everything else in it
   — the events especially — exists for them alone.

That split is the whole reason the events drift: nothing in the repo compiles against them, so a
wrong signature is silent locally and only wrong for whoever depends on the package. The
declaration lists in this file and in `IntentsBase.sol` are kept identical; diffing them is the
only check that exists.
