# Simplex: BRIDGE top-up

The operator overview's **BRIDGE available** tile has a **Top up** button. It buys BRIDGE with
the filler wallet's USDC or USDT and delivers it to the filler's Hyperbridge (nexus) account.
That account holds the BRIDGE that pays for bids.

## Flow

**Pay with** opens a two-pane token picker (`TokenNetworkPicker`). Networks are on the left
and the filler wallet's USDC and USDT balances are on the right, and both panes can be searched.

It offers only the networks the filler fills on: those it runs, whose account is delegated
(EIP-7702) to the current SolverAccount. On those networks the placement batch goes through the
account with gas paid in stablecoins. A network that is not delegated would need a one-time
delegation, which costs native gas where the token has no EIP-2612 permit (BSC). Balances load
behind a spinner and do not block the form. The picker defaults to the largest balance, and
the $5 and $10 presets fill in the amount.

One IntentGatewayV2 order is placed from the chosen chain (`BridgeTopUp`, `src/services/BridgeTopUp.ts`):

- **Input:** the stablecoin amount.
- **Destination:** `EVM-56`. BRIDGE books exist only on BSC.
- **Output:** BRIDGE at the orderbook's pessimistic quote, minus 1%, rounded down to a whole
  nexus unit (1e6 wei). Nexus holds BRIDGE at 12 decimals and truncates.
- **Beneficiary:** the BSC gateway's CallDispatcher, read from `params().dispatcher` on-chain.
- **Referrer:** `placeOrder`'s graffiti is `SIMPLEX_REFERRER`, "Simplex" as UTF-8 right-padded to
  `bytes32` (`stringToHex("Simplex", { size: 32 })`), as the HyperFX app tags its orders "HyperFX".
  It is emitted in `OrderPlaced` and indexed as the order's `referrer`.
- **`output.call`:** one `Call` to BRIDGE `send({dest: "POLKADOT-3367", to: <sr25519 pubkey>,
  amount: output, timeout: 0, relayerFee: 0, data: 0x})`.

When the order is filled, the gateway runs the call. The HFT burns the dispatcher's BRIDGE, and
the nexus `pallet-hyper-fungible-token` credits the native balance of the 32-byte recipient.

Placement is one batch: approve the input token, approve the fee token (merged when they are
the same token), then call `placeOrder`.
- **Sending.** The batch goes through `TokenSender.submitCalls`, which uses a sponsored UserOp
  and falls back to a native transaction.
- **Calldata.** The SDK `IntentGateway.executeBest` supplies the calldata, the fee and the
  session key.
- **Receipt.** Simplex reads the receipt itself, through the filler's multi-endpoint client, and
  rebuilds the order from `OrderPlaced` (`deriveCanonicalPlacedOrder`, now exported). The SDK's
  own read uses one endpoint. A node that has not reached the new block fails that read and
  loses track of an order that did land.
- **Tracking.** `IntentGateway.resumeBest` follows bids and the fill, using the filler's
  coprocessor connection.

## Delivery to Hyperbridge

The HFT `send` dispatches with no relayer fee. Relayers have delivered it about a minute after
the fill, and have also not delivered it at all. Once the order fills, the top-up is
`delivering`, and its progress is read rather than assumed:
- **The fill:** the executor's event, or the indexer's `iOrderV3` `FILLED` status (transaction
  and filler).
- **The request:** the indexer's request from that fill transaction into `POLKADOT-3367`. The
  fill's `Sent` log, read through a fallback over every BSC endpoint, is the fallback.
- **Its statuses:** `IsmpClient.queryRequestWithStatus` gives `SOURCE`, `SOURCE_FINALIZED` and
  `DESTINATION`, stored as `deliveryStatuses` in ms. The indexer reports some timestamps in
  seconds and others in ms.

If no relayer has delivered within 90 s of `SOURCE_FINALIZED`, Simplex relays the request
itself with `IsmpClient.aggregateTransactionWithCommitment`. A request into Hyperbridge is an
unsigned extrinsic and costs nothing. The top-up becomes `delivered` on `DESTINATION` or a
request receipt on Hyperbridge. Every network read has a 30 s timeout, and delivery retries
until it succeeds.

**Sync.** `GET /api/bridge-top-up?sync=1` brings a pending top-up up to date from the indexer
and the chains, because another party may fill, deliver or refund it. The progress dialog polls
this every 5 s while it is open on a pending top-up. Sync is single-flight, and it shares one
delivery read with the background driver.

## Pending top-up, resume and reclaim

Only one top-up can be open at a time. It is kept in the runtime state store as
`RuntimeState.bridgeTopUp`, with the order and its session key, so a restart resumes it.

- **Stages.** `placing`, `awaiting_bids`, `filling`, `delivering`, `unfilled` and
  `reclaiming` count as pending. `delivered`, `reclaimed` and `failed` are final. `filled` is
  the final stage of records made before delivery was tracked.
- **Stage times.** The record keeps `stageTimes` (when each stage began), the executor's
  `events` (bids with solver and outputs, the selection, the fill), `filler`, `fillTxHash`,
  `deliveryCommitment` and `deliveryStatuses` for the timeline.
- **Resume on boot.** An open order is resumed with `IntentGateway.resumeBest`, and a delivery
  is re-run. A reclaim is re-run too, and the SDK continues from the cancellation progress it
  saved. A top-up with a placement transaction but no recorded order is recovered from that
  transaction's receipt. One with no placement transaction is `failed`, because nothing was
  escrowed.
- **Recovery.** An order that expires unfilled is `unfilled`. It recovers as the Hyperbridge
  app's swap timeline does, but the filler wallet signs everything, so nothing waits on a
  wallet prompt.
  - **Paid on BSC (same chain).** Order Placed, then a red Claim Funds. **Claim funds** cancels
    on BSC, which refunds at once and shows Funds Claimed.
  - **Paid on another chain, before recovery starts.** The order's steps, ending in a red Order
    Expired, then **Start recovery**.
  - **Paid on another chain, once recovering (`reclaiming`).** `IntentGateway.cancelOrder` from
    the source drives fe's six steps, recorded as `recoverySteps`:
    1. Awaiting BNB Chain Finality
    2. Cancel Order (sent from the filler wallet)
    3. Awaiting {source} Finality
    4. Hyperbridge Verifying
    5. Hyperbridge Finalizing
    6. Waiting for Relayer, then Claim Funds

    They carry fe's ETAs: BSC 3 min, the source chain's finality (Base 1.5 h), 3 min,
    Hyperbridge 5 min.
  - **Claiming the refund.** A relayer normally delivers it. If none has within 3 minutes of
    Hyperbridge finalizing, the filler sends Hyperbridge's finalized response (`claimCalldata`)
    to the source host's `hostParams().handler` itself.
  - **Done.** The refund is confirmed by `isOrderRefunded` or the indexer's REFUNDED status, and
    the top-up is `reclaimed`.
  - **Restarts.** A restart resumes from the SDK's persisted cancellation, and a cancel already
    sent is handed back rather than sent twice.
- **Background work.** The order, its delivery and its recovery run in the solver daemon, which
  is detached from the window. Minimising or closing the window does not stop them. The
  BRIDGE's HFT send has no timeout, so it never needs recovering; its delivery retries until it
  lands.
- **SDK storage location.** `IntentGateway.create` now takes `{ storagePath }`. Simplex points
  it at `<data dir>/.hyperbridge-cache`. The SDK default is the working directory, which the
  desktop daemon (started from `/`) cannot write.

## Progress dialog

While a top-up is pending the button reads **Top up pending** and opens the progress dialog.
Starting a top-up hands off from the form to that dialog.

The progress dialog (`BridgeTopUpProgressDialog`) is the Hyperbridge app's bridge transaction
dialog:
- `@hyperbridge/ui` `Dialog` with no close button and a **Minimize** pill;
- a `TokenTransferPair` (stablecoin on its network → BRIDGE on Hyperbridge) and a headline;
- a `TimelineRoot` with the swap's steps, then the bridge's finality steps for EVM to
  Hyperbridge (`topUpTimelineModel.ts`):
  1. **Order Placed:** the paying network's transaction.
     - **Offers Received:** fe's expandable row. It shows a count badge and "Received offer
       from …". Expanded, it lists each offer once: "solver offered N BRIDGE", plus
       "Selected ↗" and "Landed in Ns".
  2. **Order Fulfilled:** the BNB Chain fill, which also dispatches the BRIDGE.
     - **Transaction Finalized:** BNB Chain final on Hyperbridge (`SOURCE_FINALIZED`).
  3. **Transaction Complete:** the BRIDGE delivered to Hyperbridge, shown with the BRIDGE token
     on Hyperbridge. This is the request's `DESTINATION`. For a request to Hyperbridge itself,
     the SDK's status stream ends at `DESTINATION` and has no separate delivered or finalized
     status.

  A running step counts down to its expected end and disappears at zero. A finished step shows
  how long it took. The expected lengths are the Hyperbridge app's: offers 30 s, fill 5 s, BNB
  Chain finality 3 min, completion 3.5 min. An unfilled order shows Order Expired and Claim
  Funds, and a **Reclaim** button.

Simplex has no Tailwind of its own, so `ui/src/styles/hyperbridge-ui.css` adds only what the
package needs:
- Tailwind's theme and utilities, plus `tw-animate-css`, generated from `@hyperbridge/ui` and
  the two timeline files;
- the package's brand tokens and dialog surface rules.

It leaves out Tailwind's preflight and the package's shadcn variables, which would restyle
Simplex. Inside `.hb-ui` it applies a scoped preflight. It also reverts Simplex's unlayered
element rules with `all: revert-layer`, because they would otherwise beat the layered utilities.
`@hyperbridge/ui` is now 0.0.50.

## Constraints

- **`timeout` must stay 0.** A timed-out HFT send refunds to its sender. Here the sender is the
  CallDispatcher, which has no access control, so anyone could take the refund.
- **No partial fills.** Orders with an output call must fill completely, and all surplus goes to
  the protocol.
- **CallDispatcher address.** The SDK `Calldispatcher` entries now match each gateway's live
  `params().dispatcher`: `0xE2C7…4bEd` on mainnets and `0x2B33…C181` on Polygon Amoy. The
  top-up still reads the address from the gateway rather than from config.

## API

- `GET /api/bridge-top-up/sources` returns the wallet's USDC/USDT balance on each network the
  filler fills on and is delegated on. A balance is `null` when that network could not be read.
- `GET /api/bridge-top-up/quote?chain&token&amount` returns `{ bridgeOut, rate, fillable,
  maxFillableIn, … }`.
- `GET /api/bridge-top-up` returns `{ current }`, the latest top-up without its order or
  session key, or `null`.
- `POST /api/bridge-top-up {chain, token: "USDC"|"USDT", amount}` starts a top-up. It is
  refused while another top-up is pending.
- `POST /api/bridge-top-up/reclaim` cancels an `unfilled` order and refunds its input.
