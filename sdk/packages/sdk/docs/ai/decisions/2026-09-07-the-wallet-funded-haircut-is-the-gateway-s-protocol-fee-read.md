# 2026-09-07 — The wallet-funded haircut is the gateway's protocol fee, read live, not a constant

Chosen: every bid that declares no Uniswap V4 positions is haircut by the `protocolFeeBps` the IntentGateway on
the phantom order's chain reports from `params()`, read once per aggregation run. Supersedes the entry below,
made earlier the same day, which had removed the flat 5bps.

Why: the published rate is what a taker is told they can trade against, and the gateway deducts its protocol
fee from every order's input before the solver's quote applies. A flat 5bps happened to equal that fee today;
tying it to the contract makes the published rate track the fee takers actually pay, and moves with governance
instead of with a constant someone has to remember. The pool tier stays a constant because it stands in for a
Uniswap pool fee, which is not the gateway's to set.

Alternatives considered:

- **Keep a constant and document that it mirrors the fee.** Rejected: the two drift silently the first time the
  fee changes, and nothing in the pipeline would notice.
- **Read the fee from a fixed chain (Base) regardless of the order's chain.** Rejected: the fee that applies to
  fills of this rate is the one collected by the gateway on the chain the order lives on, and the aggregation
  already holds that gateway's address and RPC. Reading elsewhere would price one chain's rate with another
  chain's fee if they ever differed. Today every deployment charges the same 5bps, so the outcome is identical.
- **Default to zero (or the old 5bps) when the read fails.** Rejected: a gateway address with no code is a
  misconfiguration, not a zero fee, and a window priced without the haircut on the back of it publishes a
  rate nobody realizes. Failing the run keeps the previous window's rate standing, visibly old, instead.
- **Memoize the fee across runs alongside balances.** Rejected: it is one static call per window and it is
  governance state; a stale value here outlives a stale balance by hours.



Chosen: drop `PHANTOM_QUOTE_HAIRCUT_BPS` (5bps) and `applyPhantomQuoteHaircut`. A bid that declares Uniswap V4
positions still pays `UNISWAP_QUOTE_HAIRCUT_BPS` (10bps); every other bid is published at the amount it named.
This reverses the 2026-08-27 decision below.

Why: the 10bps pool haircut nets out a real cost — the pool fee a pool-priced quote has not yet paid. The 5bps
wallet haircut netted out nothing: a wallet-funded solver has already paid its cost of goods and names the amount
it will actually clear, so the shade only moved the published rate 5bps off the executable one and made the
snapshot disagree with what fills at.

Alternatives considered:

- **Keep a smaller wallet haircut (1–2bps).** Rejected: any nonzero value re-raises the question of what cost it
  represents, and there is none; the margin between quote and fill belongs to the solver's own pricing.
- **Fold the two into one constant applied to every bid.** Rejected: it would either charge wallet bids a pool
  fee they never pay or under-charge pool bids, and the two tiers exist precisely because the costs differ.
