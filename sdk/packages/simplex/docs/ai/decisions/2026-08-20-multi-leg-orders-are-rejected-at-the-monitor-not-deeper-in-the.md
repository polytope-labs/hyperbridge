# 2026-08-20 — Multi-leg orders are rejected at the monitor, not deeper in the fill path

Chosen: `EventMonitor` filters on leg count, so an order with more than one input or output asset
never reaches `IntentFiller`.

The monitor is the one place every real order passes through exactly once, and it already owns the
other two intake filters (filler-address matching on fills, de-duplication). Rejecting there means
the constraint holds for every strategy without each of them re-checking, and the cost of a
rejected order stays at one debug log.

Alternatives rejected:

- _Reject in `FXFiller.evaluateOrder`._ It already refuses an order whose input and output counts
  disagree, but a balanced multi-leg order would still be priced, quoted and possibly bid. The
  work is wasted, and a second strategy would need its own copy of the rule.
- _Filter in the scanner._ The scanner is shared between fillers; a filler that wants multi-leg
  orders could not opt back in. Per-filler intake policy belongs to the per-filler event bus.
- _Strip extra legs and fill the first one._ Silently changes what the user asked for. An order is
  filled whole or not at all.
