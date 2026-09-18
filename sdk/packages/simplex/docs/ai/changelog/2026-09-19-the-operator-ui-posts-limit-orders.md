# 2026-09-19 — The operator UI posts limit orders

The dashboard lost its curve editors when prices moved onto limit orders, and nothing replaced them:
the API had `/api/limit-orders` from the start, but the only way to reach it was curl. An operator
could see the markets simplex watches and not what it was offering on any of them.

A **Limit orders** page sits between Overview and Orders, at `/limit-orders`.

## What it shows

One row per order, live orders first and closed ones under their own heading. Each row reads as the
offer it is: `1,500,000 CNGN left at 1,500 CNGN per USDC`, the token taken in, the chain it fills on,
and how long is left on the clock.

Amounts are whole tokens throughout, never the orderbook's 1e18. The operator states `1000` and reads
`1,000` back; `fromScaled` is the only place the scale is handled.

The badge answers "is this actually working", which the status column alone does not:

- `open` with a commitment is **On the book**.
- `open` with no commitment is **Posting**, because the row is live before the orderbook has answered.
- `open` with a `lastError` stays **On the book** but turns amber and carries the refusal, since the
  row stays open and the next cycle tries again.
- `resizing` is **Resizing**, a fill being settled.
- `cancelled`, `expired`, `filled` and `rejected` say so plainly.

## What it does

**Posting an order** asks for the two amounts, in whole tokens, and shows the rate they imply before
anything is sent — the operator chooses amounts, not a rate. It also asks which chain the order fills
on and which sources it accepts, defaulting to every chain the filler watches, since an order
accepting none is one the orderbook refuses outright.

**Opening an order** shows what is left of it, what live bids are holding against it, and the fills
that took the difference, which is what makes a shrunken `remaining` explicable rather than a number
that quietly moved. It can be cancelled from there while it is still live.

The list reloads every ten seconds and after every mutation. A posting lands, expires or is filled
without the operator doing anything, so the page cannot be a snapshot of the moment it opened.
