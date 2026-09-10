# 2026-08-19 — SUPERSEDED (see the reversal above): rate-limit suspension never shrinks the quorum, and yields when the quorum needs the benched endpoint

Chosen: a rate-limited endpoint is suspended for 5 minutes, but (a) the threshold stays `quorumThreshold(full set)` — suspension changes who is asked, never what is required — and (b) when the unsuspended endpoints alone cannot reach that threshold, suspended endpoints are queried anyway.

Alternatives considered: recomputing the threshold over the active set (a 5-endpoint operator would drop from 4-of-5 to 3-of-4 agreement — an attacker who can induce 429s on public endpoints, by hammering them independently, could lower the agreement bar without controlling any endpoint); hard suspension (honouring the bench even when it makes quorum impossible — for the common 2–3 endpoint sets, where the threshold is all of them, one 429 would turn into a guaranteed 5-minute total outage where today's behavior at least retries and fails per-call).

Why this shape: the class's trust model is that the operator provisioned n-way BFT; no availability optimisation may weaken it. The two rules keep both properties exactly: agreement requirements identical to the pre-suspension client in every case, and traffic to a throttled provider reduced precisely when the quorum can afford it (n ≥ 4). The 5-minute window is a constant, not config — no operator knob until someone actually needs one.

Also chosen: suspension is recorded in `settleUntilQuorum`'s rejection handler unconditionally, including stragglers settling after the call already decided early — a rate limit learned late still spares the endpoint on the next call.
