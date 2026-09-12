# 2026-09-03 — Make setup market rows summarize prices instead of order limits

Chosen: after a Buy or Sell curve has a configured point, show its first configured price in the
market row with the shared `token1/token0` unit. Hide the maximum-order summary from the row while
keeping the cap field in the Configure editor, so the list emphasizes the market's pricing behavior
without removing risk controls.

Alternatives rejected: showing every curve breakpoint would make a compact row behave like an editor;
showing a price before both amount and value are present would expose incomplete input while the user
is typing. Reference-only feeds and venue-priced markets have no Buy/Sell curve summary to render.
