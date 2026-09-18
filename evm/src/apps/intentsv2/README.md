# IntentGatewayV2 modules

`IntentGatewayV2` is an ERC-1967 proxy whose implementation had reached the EIP-170 code size
limit (24,367 of 24,576 bytes). The implementation now delegatecalls its heavy branches to two
separately deployed modules. The proxy, its address and the governance upgrade path are unchanged.
The ABI lost the host-only `setRelayer` and `upgradeToAndCall` (now reached only through
`Execute`). `migrate(address)` advances the gateway release, and `intrinsicModule()` and
`extrinsicModule()` expose the configured modules.

## Layout

```
                     EIP712
                       |
                  IntentsBase                     storage, events, errors, shared helpers
               /       |        \
  IntentGatewayV2  IntrinsicIntents  ExtrinsicIntents
  implementation       |                  |
                 IntrinsicModule    ExtrinsicModule   deployed modules the implementation delegatecalls
```

| Contract | Responsibility |
|---|---|
| `IntentGatewayV2` | External guards, placement, selection, fill/cancel validation, the steps every fill shares, initialization and migration |
| `IntrinsicModule` | Same-chain leg payment and escrow release, and cancellation |
| `ExtrinsicModule` | Cross-chain leg payment and redemption requests, cancellation proofs, escrow settlement and governance |

Both fill paths use the accounting and rate arithmetic in `IntentsBase`. `fillOrder` validates the
fill, claims the order in `_filled` and delegatecalls the matching module's `fillOrder`. The module
pays the legs, settles the released escrow for its route and returns the `FillResult`. The gateway
then finishes the fill the same way for both routes. A completing fill runs the output calldata and
emits `OrderFilled`. Any other fill clears the claim and emits `PartialFill`. Either way, unspent
native value goes back to the solver. `setRelayer` and `upgradeToAndCall` exist only on the
extrinsic module and are reached through the host-authorized `Execute` request.

## Rules

- **One storage layout.** Each module inherits `IntentsBase` exactly as the implementation does,
  declares no storage and never runs an initializer. `IntentGatewayModulesTest` reads the storage
  layouts out of the forge artifacts and asserts the three contracts agree slot for slot, so
  `foundry.toml` sets `extra_output = ["storageLayout"]`. The append-only rule for storage now
  applies to all three at once. `_filled` must stay at slot 2, which the SDK reads for fill status, and
  `_partialFills` at slot 11, which cross-chain cancel proves per leg as `_partialFills[commitment][index]`.
  The one exception is the unused `bool _paused` that sat at slot 13 offset 0: it was removed, so
  `_relayer` moved from offset 1 to offset 0 and `migrate` shifts it there on existing proxies.
- **The owner is the implementation's alone.** `IntentGatewayV2` inherits OpenZeppelin's
  `Ownable2StepUpgradeable`, whose owner and pending owner sit at ERC-7201 namespaced slots outside
  the shared sequential layout, so the modules never see them and the layout tests are unaffected. The owner
  can only `pause` and `unpause` the gateway, through OpenZeppelin's `PausableUpgradeable`, whose flag
  is namespaced too. `placeOrder`, `fillOrder`, and the escrow deliveries
  of `onAccept` and `onGetResponse` revert while paused, checked on the implementation before any
  delegatecall; governance deliveries and `cancelOrder` are not paused. `initialize` and
  `migrate(owner)` set it and transfers are two-step. `_checkOwner` also accepts the host, so
  governance can pause, resume or propose a new owner through `Execute` carrying
  `upgradeToAndCall(currentImplementation, call)`.
- **Module addresses are immutables.** `intrinsicModule()` and `extrinsicModule()` are set in the
  implementation's constructor, which refuses an address without code. Upgrading a module means
  deploying a new implementation with the new address and installing it through governance like
  any other implementation. There is no storage-held module registry and no new governance
  surface.
- **Guards only at the entry.** `nonReentrant`, `onlyHost` and the transient-storage solver
  selection stay on the implementation's functions. The fill and cancel module functions carry
  `onlyDelegated` and nothing else. `onAccept` and `onGetResponse` are forwarded as `msg.data`
  after `onlyHost`, and run unchanged in the module: relayer gate, peer authentication,
  governance kinds and `Execute`.
- **Modules refuse direct calls.** `onlyDelegated` compares `address(this)` against the module's
  own address baked in as an immutable; under delegatecall `address(this)` is the proxy. The
  inherited host callbacks and host-only functions are closed by `onlyHost` instead.
- **`Execute` delegatecalls the module itself.** `ExtrinsicIntents.onAccept`, already running
  under delegatecall, delegatecalls `__self`, the module's own address baked in as an immutable,
  with the rest of the body. The nested hop keeps the proxy's storage and the host as
  `msg.sender`, so `upgradeToAndCall` writes the proxy's ERC-1967 slot and `setRelayer` its
  relayer. Delegatecall nests freely; only the 1024 call depth and the 63/64 gas rule bound it.
  The body may select any module function, including the fills, with the host as `msg.sender`
  and none of the implementation's validation. That is not new power: the same request can
  install any implementation. It cannot select anything on the implementation, so `migrate(owner)`
  runs only as `upgradeToAndCall` init data; an `Execute` body naming it directly fails with
  `FailedCall()`.
- **Reverts bubble byte for byte.** `IntentGatewayV2._delegate` returns the module's return data,
  which only `fillOrder` uses, and re-raises its revert data verbatim. Custom errors raised inside
  a module such as `PartialFillNotAllowed`, `NotExpired` and `UnknownOrder` keep the selectors the
  SDK matches, and string reasons from tokens surface unchanged. `Expired`, `FillExpired`, `Filled` and `WrongChain` are raised by the
  implementation's validation before any delegatecall, as before.
- **Events and errors live in `IntentsBase`.** Module code emits from the proxy's address, so
  indexers see nothing new, and every ABI still contains the declarations. The OpenZeppelin
  errors `ERC1967Utils` and `Address` can raise during an upgrade now surface from the module and
  are no longer listed in the gateway's ABI.
- **Module functions are payable where their entry point is.** Delegatecall preserves
  `msg.value`, so the fill and cancel functions are `payable`.

Two things that look like problems but are not: OpenZeppelin's EIP-712 recomputes the domain
separator when `address(this)` differs from the deploying address, which is already the case for
every call behind the proxy; and transient storage is shared under delegatecall, so the solver
selection `fillOrder` reads is visible to module code.

## Cost

One delegatecall per fill or cancel, including a cold access to the module address and the
calldata re-encoding. Measured with `forge snapshot` before and after, on the fork test suites:

| Path | Extra gas |
|---|---|
| same-chain fill | 4.8k to 6.0k |
| cross-chain fill | 5.9k to 6.2k |
| same-chain cancel | 4.3k to 5.6k |
| cross-chain cancel (either side) | 5.7k to 6.2k |
| `onAccept` redeem or refund, `onGetResponse` | 2.6k to 2.9k |
| governance (`UpdateParams`, `SweepDust`, `NewDeployment`) | 3.3k to 3.4k |
| `Execute` rotation | 3.2k |

Fills and cancels pay a cold access to the module address plus the re-encoding of the order into
the module call; the callbacks forward `msg.data` as is and pay only the cold access and the hop.

Fills also return their `FillResult` to the gateway, measured separately with the same method.
That adds about 2.1k gas to a single-leg same-chain fill and about 2.6k to a single-leg
cross-chain fill, and more for each extra leg.

## Solver quotes

`fillOrder` takes one quote per order leg: `FillOptions.inputs[i]` is the most input the solver
will take and `FillOptions.outputs[i]` the most output it will pay. Their ratio is the solver's
rate, which may not be below the order's. Quoting zero on both sides skips the leg; the arrays must
match the order's leg count.

`IntentsBase._priceLeg` settles a leg in three steps. The take priced at the order's rate, capped
to what the leg still needs, is the credit. The credit unlocks escrow as a difference of cumulative
floors, so the completing fill drains the leg exactly. The released escrow priced at the solver's
rate, rounded up and never below the credit, is the payment; the excess over the credit is surplus,
split by `surplusShareBps` or kept whole by the protocol on output-call orders. Events and
cross-chain proofs carry the credit, not the surplus. Rounding can release less than the full take,
so unused ERC-20 budget stays with the solver and unused native value is refunded.

The `inputs` field gives `fillOrder` the selector `0x68ddf058`. Bids signed against an earlier
selector need new calldata and signatures; `SolverAccount` still refuses those selectors on its
plain ECDSA path. The gateway reports release 3, which lands the module split, the owner and solver
quotes on a proxy together.

## Deploying and upgrading

`script/DeployIntentGatewayImpl.s.sol` (implementation only) and `script/DeployIntentGateway.s.sol`
(implementation, proxy where none exists, solver account) share `script/IntentGatewayScript.sol`,
which:

1. deploys `IntrinsicModule` and `ExtrinsicModule` via CREATE2 with the script's salt
   (`keccak256(VERSION)`), reusing a module already at its address on a re-run;
2. deploys the implementation via CREATE2 with `(intrinsic, extrinsic)`. Module addresses are
   identical across chains, so the implementation address is too;
3. records `INTENT_GATEWAY_V2_INTRINSIC_MODULE`, `INTENT_GATEWAY_V2_EXTRINSIC_MODULE` and
   `INTENT_GATEWAY_V2_IMPL` in the config and prints the `data` for the `execute_on_gateway`
   governance call that installs it.

Run it per chain with `script/deploy.sh`; `--mode full` verifies every contract the run created,
modules included, and `--mode verify` re-verifies from the broadcast artifacts.

```bash
./script/deploy.sh --mode full --network mainnet DeployIntentGatewayImpl ethereum,base,arbitrum
```

The upgrade itself is a Hyperbridge governance call, `execute_on_gateway(data)` on the
intents-coprocessor pallet. The pallet prepends the `Execute` discriminator (`0x05`) itself, so
`data` is bare `upgradeToAndCall(newImplementation, initData)` calldata, exactly what the script
prints. `initData` is `migrate(owner)` for a proxy at version 2 and empty for one already at 3. That
`migrate` is required: it moves `_relayer` to slot 13 offset 0 and sets the owner. Installing this
implementation on a version-2 proxy with empty `initData` leaves the relayer gate reading a wrong
address, and it would refuse every delivery, governance included.

Upgrade only once every outstanding order is filled or cancelled and escrow, fees and pending
messages are drained on every chain, and keep placement stopped until matching gateways and modules
are installed everywhere. A partially filled order carried across the upgrade would settle its
remaining legs under cumulative accounting, leaving the per-slice rounding dust of its earlier fills
in escrow with no cancellation path once it completes.

A relayer rotation is a separate `execute_on_gateway` carrying
`setRelayer(next)`; it cannot ride in `initData`, which runs against the new implementation, where
`setRelayer` does not exist. Whether the upgrade changes the implementation's own code, a module,
or both, the procedure is the same: new modules if needed, new implementation, one
`execute_on_gateway` per chain.

## Adding a module

Only if a fixed third branch appears that does not fit the two. Make the abstract contract holding
the logic concrete, copy the constructor and `onlyDelegated` modifier from an existing module
(`__self` comes from `IntentsBase`), add the immutable and constructor check to the implementation,
route to it from the entry point, extend `IntentGatewayScript` and the storage layout test. This is a fixed split, not a diamond: no selector table, no loupe.
