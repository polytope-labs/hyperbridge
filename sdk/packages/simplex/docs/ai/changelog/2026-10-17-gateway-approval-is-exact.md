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
`TokenSender.submitCalls`, one call per token. `submitCalls` tries a sponsored op whenever a paymaster
and bundler are configured, and sends natively only when the bundler refuses it; a single call goes
straight to the token, so that fallback works from a wallet that is not delegated yet.

The sweep also covers the gateways a chain has since left. The over-approval has been in the code
since 2026-03-11, and moving to a new gateway does not clear what was granted to the old one. So for
each chain it reads the same tokens against every address in `RETIRED_GATEWAY_ADDRESSES` and revokes
a non-zero allowance the same way, with the same nonce key. An address equal to the chain's current
gateway is skipped, and a chain with no retired address does no extra reads. The list is per chain
ID, taken from the history of the SDK's `chain.ts`:

- Mainnets 1, 10, 56, 100, 137, 8453 and 42161: `0x2d61…`, `0x09a7…` and `0x16F9…`, before the move
  to `0xAe04…` (#996).
- Polkadot Hub mainnet (420420419): `0x16F9…`.
- BSC Chapel (97) and Polygon Amoy (80002): `0xFbF5…` and `0xE13f…`, before `0x6CF4…`.

A later gateway move has to add the address it leaves behind to that list.

The revoke covers the gateway fee token and the configured USDC and USDT, where the large leftover
allowance was. Other output tokens a pre-fix clamped fill paid may hold a small leftover allowance,
and they are not swept.

The sponsored revoke runs on its own EntryPoint nonce key (`submitCalls`' optional
`{ nonceKey }`; every other caller keeps key 0). Boot also starts `bridgeTopUp.restore()` and, about
30s later, the first vault sweep on key 0, and two ops on one key read the same `getNonce`, so the
bundler would reject or replace one of them. The key is a fixed hash of
`simplex.gateway-allowance-revoke`, shifted to fit the key's 192 bits. A bid's key is a hash of its
commitment, session key and calldata, so the two do not meet. One chain's revokes (its tokens, and
its current and retired gateways) go one after another on that key. That keeps them apart only
while each op confirms: if one is left unconfirmed, the next op on the key collides with it and
fails, and the next boot retries it.

It is best-effort and runs in the background: each revocation is logged (with the `gateway` and
whether it is `retired`), a failure is a warning, and neither delays nor fails boot. It is idempotent,
since a revoked allowance reads zero on the next boot. It costs one small transaction per affected
token, gateway and chain, once.
