# The gateway approval is exactly what the fill pays

`buildApprovalAndFillCalldata` approves the IntentGateway for what the fill pays and nothing else:

- each ERC20 output, summed per token;
- on a cross-chain fill, `fillOptions.relayerFee` (the dispatch fee) added to the destination fee
  token. An output in the fee token and the fee share one approval. A same-chain fill dispatches
  nothing and approves no fee token. Native outputs (`ADDRESS_ZERO`) need no approval.

The fill's gas is not in the approval. The paymaster pays it through its Permit2 pull, and the
gateway pulls only the dispatch fee in the fee token. The cached `totalCostInSourceFeeToken` is in
the source chain's fee-token decimals and is used only by the profitability gates.

`fillOptions.relayerFee` is the only source of the dispatch fee: the approval, the `fillOrder`
calldata and `debitOrder`'s `fee` all read it. `approvalAmounts` builds the per-token amounts, and
both the batch and the call gas allowance use it.

Every approval is set `approve(gateway, 0)`, then `approve(gateway, amount)`. The batch ends with
`approve(gateway, 0)` for each token it approved, after `fillOrder` and after `debitOrder`, so no
allowance to the upgradeable gateway proxy stands once the batch completes. `debitOrder` reads the
allowance that is left, so the resets come after it. It also clears the budget token's allowance
itself, so a batch that settles a budget leaves that token out of the trailing resets.

The bundler estimates a lone `fillOrder` (`estimateGasFillPost` passes `prependCalls: undefined`),
so neither the funding prepends nor the approvals are in its figure. `FUNDING_GAS_PER_CALL` covers
the prepends, and `callGasLimitFor` adds `APPROVAL_GAS_PER_TOKEN` (45,000, for the three `approve`
calls a token costs) for each distinct non-zero ERC20 the batch approves: the non-native output tokens,
plus the fee token on a cross-chain fill unless an output already pays it.

## Allowances left by earlier releases are revoked at boot

A solver that ran the old code can still hold about 1.09e16 units of a destination fee token
approved to the gateway proxy on a chain, and only its next fill in that token there would clear it.
After delegation is set up, and only when the filler has a signer, `revokeStaleGatewayAllowances`
(`src/services/gateway-allowance.ts`, called from `bootFiller` once the `TokenSender` exists) reads
`allowance(solver, gateway)` for the gateway's fee token and the configured USDC and USDT on every
configured chain that is not watch-only. Each non-zero allowance gets one `approve(gateway, 0)` through
`TokenSender.submitCalls`, one call per token: an undelegated wallet sends it as a plain transaction,
a delegated one gets it sponsored.

It is best-effort and runs in the background: each revocation is logged, a failure is a warning, and
neither delays or fails boot. It is idempotent, since a revoked allowance reads zero on the next boot.
It costs one small transaction per affected token and chain, once.
