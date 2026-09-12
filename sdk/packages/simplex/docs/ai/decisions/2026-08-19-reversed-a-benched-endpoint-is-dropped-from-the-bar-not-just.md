# 2026-08-19 — REVERSED: a benched endpoint is dropped from the bar, not just the traffic

Chosen (maintainer decision, reversing the entry below): a rate-limited endpoint is excluded from the query set AND the quorum bar — each call's threshold is `quorumThreshold(endpoints actually queried)`. With every endpoint benched, all are queried again.

What it replaces: the first design kept the threshold over the full set and queried benched endpoints whenever the quorum was impossible without them, preserving the trust model exactly at the cost of availability — at n ≤ 3 a sustained 429 meant every call failed for the whole window.

Why the reversal is right: a throttled endpoint answers nothing either way, so keeping it in the denominator can only fail calls the remaining endpoints agree on — and a scanner that cannot form a quorum misses orders, which is a concrete revenue loss. The threat the fixed bar defended against (an attacker inducing 429s on public endpoints to lower the agreement bar) degrades 4-of-5 to 3-of-4 among endpoints the operator still chose — a marginal weakening against a speculative adversary, paid for with certain blindness under ordinary provider throttling. The voters are always exclusively the operator's own endpoints; the bar simply matches who was asked.

`threshold` (the public field) still reports the full-set bar — the scanner logs it — and the per-call bar appears in every QuorumError message.
