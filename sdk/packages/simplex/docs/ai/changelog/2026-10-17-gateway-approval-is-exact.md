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

The bundler estimates only the prepends and a lone `fillOrder`, so the approvals are not in its
figure. `callGasLimitFor` adds `APPROVAL_GAS_PER_TOKEN` (45,000, for the three `approve` calls a
token costs) for each distinct non-zero ERC20 the batch approves: the non-native output tokens,
plus the fee token on a cross-chain fill unless an output already pays it.
