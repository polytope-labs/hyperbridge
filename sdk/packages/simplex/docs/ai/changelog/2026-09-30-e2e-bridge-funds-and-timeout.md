# 2026-09-30 — The swap harness names an unfunded solver, and always reports

The `partial` scenario reported `NO RESULT` with no fills on main for days, while every other
scenario passed. Two separate faults:

## Solvers 2 and 3 had run out of BRIDGE

Their Hyperbridge accounts held 0.0026 and 0.0011 tBRIDGE, so every bid extrinsic was refused with
`1010: Invalid Transaction: Inability to pay some fees`, and the SDK reported `Transaction failed
after 3 attempts`. Only solver 1 could bid. Every other scenario is fillable by solver 1 alone;
`partial` is deliberately larger than one solver's order, so after solver 1's partial fill nothing
else ever bid.

Preflight now reads each solver's balance on Hyperbridge and refuses to run below `MIN_BRIDGE`
(1 tBRIDGE), naming the solver and its address. Without it, an unfunded solver looks exactly like a
solver that chose not to bid.

## A scenario that nobody bids on now times out rather than vanishing

`executeBest` waits for bids with no deadline, so the loop parked on one `next()` and never got to
check the clock. The runner killed the child at `E2E_SCENARIO_TIMEOUT_MIN + 3`, no result file was
written, and the fills that had already landed were lost with it — `NO RESULT | 0 fills` for a
scenario that in fact filled once.

`nextBefore` races each step against the scenario's deadline, so the run reports `TIMEOUT` with the
fills that landed and the bid rounds it saw.
