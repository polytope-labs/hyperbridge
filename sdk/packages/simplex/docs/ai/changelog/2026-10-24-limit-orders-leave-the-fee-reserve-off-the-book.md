# Limit orders leave a $10 fee reserve off the book

A cross-chain fill pays the escrow-release dispatch fee in the fill chain's fee token, out of the
wallet that pays the fill. The orderbook advertises `min(quoted, balance)`, so an order paying out
the fee token and sized to everything held got swaps matched against the whole balance. The filler
then skipped each fill with "fill leaves too little of the fee token to dispatch the escrow release".

## Behaviour

- `LimitOrderService.create` refuses an order that pays out the fill chain's fee token
  (`ContractInteractionService.getFeeTokenWithDecimals(fillChain)`) unless the wallet plus the
  configured vaults hold `LIMIT_ORDER_FEE_RESERVE_TOKENS` (10, in `src/config/defaults.ts`) more
  than the order pays out. Fee tokens are dollar stablecoins, so this is $10. The error gives the
  most the order can pay out. Other payout tokens are checked as before.
- A fee token that cannot be read holds nothing back and logs a warning. The fill-time dispatch-fee
  check still skips a fill that would leave too little, so the cost is a skipped fill.
- `AssetBalanceRow.limitOrderFeeReserve` (10) is set on the fee token's row when
  `BalanceProvider` is given a `feeToken` lookup, which boot passes. `available` is unchanged,
  since a fill can spend that amount.
- The create form's Max button and the "held" figure beside the amount subtract
  `limitOrderFeeReserve` on each fill chain, so Max gives an amount the solver accepts.

The reserve is checked when an order is created. The orderbook keeps backing a posting with the
live balance, so a balance drawn down after creation by something other than this order's fills,
such as a withdrawal or another order paying out the same token, can again leave the book
advertising all of it.
