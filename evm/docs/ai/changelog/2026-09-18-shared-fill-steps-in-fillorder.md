# IntentGatewayV2: shared fill steps in `fillOrder`

`IntentGatewayV2.fillOrder` now runs the steps that same-chain and cross-chain fills share. After
validation it claims the order in `_filled` and delegatecalls the route's module. It then finishes
the fill with the `FillResult` the module returns:

- A completing fill runs the order's output calldata and emits `OrderFilled`.
- Any other fill clears the claim and emits `PartialFill`.
- Unspent native value is refunded to the solver.

Each module keeps only its route's work, in `_fillOrder`. `IntrinsicModule.fillOrder` pays the
legs and releases the escrow on this chain. `ExtrinsicModule.fillOrder` pays the legs and posts
`RedeemEscrow` or `RedeemEscrowPartial` to the source chain. It deducts the native dispatch fee from
`nativeRemaining` before returning the result. Both take `(Order, FillOptions, bytes32 commitment)`
and return `FillResult`. They replace `fillSameChain` and `fillCrossChain`.

One ordering change: a completing cross-chain fill now dispatches its `RedeemEscrow` request before
it runs the output calldata. The host's `PostRequestEvent` therefore precedes the output sweep's
`DustCollected`. Same-chain fills keep their order of effects and events.

`placeOrder` now rejects an output token with any of its upper 12 bytes set, as it already did for
inputs. Fills still check both tokens: the destination of a cross-chain order never sees
`placeOrder`, and the output sweep's `_isRepeatedToken` needs every token in one form.

`SolverAccount` drops its `version()`; nothing on chain or off it reads the value. It still refuses
the current and the two historical `fillOrder` selectors on its plain ECDSA path, so a replayed bid
cannot burn a solver's nonce and gas.

The gateway's own ABI, events and errors are unchanged. The modules' bytecode changes, so an
upgrade deploys new modules with the new implementation, as any module change does.
