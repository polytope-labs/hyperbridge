# 2026-10-16 — Guards on sends, rebalancing, BRIDGE top-ups, bid validity and unsponsored bids

## Sends

`TokenSender.send` refuses four recipients before reading or submitting anything, compared
case-insensitively: the zero address, the token contract being sent, a vault configured on that
chain, and the filler's own wallet. Each has its own message (`Recipient cannot be …`), which
`/api/send` returns as a 400. The own-wallet rule covers native and ERC-20 sends only: sending a
configured vault's shares to the filler redeems them into the wallet, which is how an exact amount
is withdrawn from a vault.

## Rebalancing

`validateRebalancingConfig` (`src/config/filler-toml.ts`) requires `triggerPercentage` to be finite
and strictly between 0 and 1, and every `baseBalances` entry to parse (with `parseFloat`, as
`FillerConfigService.getBaseBalance` reads it) to a finite number above 0. It runs in
`validateConfig` at boot, in `RebalanceController.set`, and on `PUT /api/rebalancing`, which keeps
its own checks for at least one entry and numeric chain-id keys.

`executePortfolioRebalancing` keeps one lane per chain, since every transfer sends from one EOA with
no nonce manager. A transfer waits for the previous transfer on every chain it sends on: its source
always, and its destination too when it goes by CCTP, whose mint (`receiveMessage`) is a second tx
from the same EOA. USDT0 (delivered by LayerZero) and Binance (which withdraws) send nothing from us
on the destination. Transfers that share no chain run in parallel, so several surplus chains
filling one deficit chain by CCTP run one after another, while a USDT0 leg into that chain does not
wait for them. `executedTransfers` keeps plan order, and a failed leg does not hold up the next one
in its lane.

## Bid validity

A measured block time below a quarter of the chain's declared one, or below 0.1s on a chain that
declares none, is discarded with a warning and the declared figure (or 0.1s) is used, so a bad
sample cannot stretch every bid on that chain for the life of the process.

## Setup wizard funding reads

The wizard's balance check (`defaultReadFunding`) goes through
`readChainFundingFromUnverifiedRpcs`, which probes every typed RPC URL with the bounded
`fetchChainId` before anything is read. A URL that answers another chain id fails the check; one
that cannot be reached is left out; if none answers, the check fails with `None of the RPC endpoints
for chain … answered`. The read then goes through a fallback client over the URLs that answered the
right chain. The fee-token and Permit2-support caches are process-wide and shared with the filler
the wizard boots, so a wrong-chain URL would otherwise pin the wrong fee token. The filler's own
clients are already verified at boot and on runtime RPC edits.

## BRIDGE top-ups

- **Reclaimed** needs `gateway.isOrderRefunded(order)` to be true, and only for a record with a
  `commitment` (it is true for an empty escrow, including one never placed). The indexer's
  `REFUNDED` supplies the `FUNDS_CLAIMED` step's tx hash and time only. The same rule now applies
  when a sync finds `REFUNDED` on an `awaiting_bids`/`filling` record. If the indexer reports a
  refund and the chain read fails, the check throws, so the reclaim retries instead of cancelling.
- **Delivered** needs Hyperbridge's `queryRequestReceipt`. The indexer's `DESTINATION` status is
  still stored in `deliveryStatuses` for the timeline.
- **Start** quotes the solver fee with the gateway's `quoteOrderFees` before persisting. The wallet
  must cover `amountIn` in the pay token and the fee in the host fee token (summed when they are the
  same token), net of the paymaster reserve, or `start()` throws `Not enough …` and nothing is
  placed. The SDK still prices the fee it places; the quote is only for this check.

## Cross-chain partial fills

A cross-chain under-fill (including a leg of a multi-leg order) is only eligible when the source
gateway reports the release the SDK requires, checked with the SDK's `assertGatewayRelease`. Only
the destination gateway was checked before, but the source gateway settles the under-fill with
`RedeemEscrowPartial`. A supported release is cached per chain. An older release or a failed read
skips the partial with a log line.

## Wallet paymaster reserve

`PAYMASTER_RESERVE_TOKENS` is now `RECOMMENDED_AMOUNT_USD` (5), up from 2, because a sponsored op on
Ethereum prefunds 2–5 tokens. On a paymaster chain, every paymaster token the wallet holds (USDC and
USDT) now keeps 5 back from fills, sends and BRIDGE top-ups, and the top-up sources list shows 5 as
the reserve. See the updated wallet-reserve decision note.

A chain still counts as funded at 5 of its fee token (`MIN_FEE_TOKEN_BALANCE`): funded means the
paymaster's gas is covered. A wallet holding exactly 5 therefore has nothing spendable for fills,
sends or top-ups, and the wizard's Chains step, the operator Chains page and the `init` help text
now say the 5 are kept for gas and fills spend what is above them.

## Bids with no paymaster

When paymaster selection returns `none`, `prepareBidUserOp` reads the account's EntryPoint deposit
and native balance once the bid's own preVerificationGas has been estimated. It throws, so the bid
is skipped before Hyperbridge's bid fee is paid, unless the two together cover
`(callGasLimit + verificationGasLimit + preVerificationGas) × maxFeePerGas` at the limits the op is
signed with.
A failed read lets the bid through, as the paymaster deposit gate does.
