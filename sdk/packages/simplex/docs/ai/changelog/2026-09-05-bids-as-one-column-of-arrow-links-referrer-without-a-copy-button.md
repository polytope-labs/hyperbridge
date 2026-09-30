# 2026-09-05 — Bids as one column of arrow links; referrer without a copy button

The "Bid placed" and "Retracted" columns became a single "Bids" column holding two icon links for
the latest bid: an up arrow to the bid extrinsic and a down arrow to the retraction extrinsic on
Statescan (green and red respectively), each with time and short hash in the tooltip; a missing one
renders as a grey arrow, a failed bid as "Failed" with its error on hover. The referrer cell is plain text (full tag on hover)
instead of a copy control.
Files: `ui/src/operator/Orders.tsx`, `ui/src/styles/operator.css`, `docs/ai/{ChangeLog,Flow}.md`.
