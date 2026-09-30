# 2026-09-30 — The swap harness names an unfunded solver, and always reports

The `partial` scenario reported `NO RESULT` with no fills on main for days, while every other
scenario passed. Two separate faults:

## Solvers 2 and 3 had run out of BRIDGE

Their Hyperbridge accounts held 0.0026 and 0.0011 tBRIDGE, so every bid extrinsic was refused with
`1010: Invalid Transaction: Inability to pay some fees`, and the SDK reported `Transaction failed
after 3 attempts`. Only solver 1 could bid. Every other scenario is fillable by solver 1 alone;
`partial` is deliberately larger than one solver's order, so after solver 1's partial fill nothing
else ever bid.

Preflight now reads each solver's balance on Hyperbridge and tops up anything below `MIN_BRIDGE`
(1 tBRIDGE, `E2E_MIN_BRIDGE`) to `TARGET_BRIDGE` (10, `E2E_TARGET_BRIDGE`), from the account in
`SECRET_PHRASE` — the repository secret the SDK tests already use. The funder keeps 1 tBRIDGE for
its own fees.

Without a `SECRET_PHRASE`, or with a funder that cannot spare the amount, the shortfall is named
and the run refuses to start rather than reporting a scenario nobody bid on. This is the same
two-way arrangement the EVM balances already have: the wallets hold what a run needs between them,
and the run says so plainly when they do not.

## A scenario that nobody bids on now times out rather than vanishing

`executeBest` waits for bids with no deadline, so the loop parked on one `next()` and never got to
check the clock. The runner killed the child at `E2E_SCENARIO_TIMEOUT_MIN + 3`, no result file was
written, and the fills that had already landed were lost with it — `NO RESULT | 0 fills` for a
scenario that in fact filled once.

`nextBefore` races each step against the scenario's deadline, so the run reports `TIMEOUT` with the
fills that landed and the bid rounds it saw.
