# 2026-09-10 — Log records show their fields without a click

A record's `detail` — the serialized non-envelope fields — was clipped to a single line with
`text-overflow: ellipsis`, and reading it meant clicking the row. The reasoning was that a
serialized error is long and most rows are skimmed. That was the wrong trade twice over.

The fields are the part worth reading. `msg` is frequently generic — "Scanning blocks", "Order
detected", "Vault refreshed" — and the identifiers, amounts and revert reasons that answer a
question all live in `detail`. Hiding them by default made the common case a click.

Worse, it broke search. The search matches `detail` and the page highlights the hit, so a clipped
row could light up as a match with the matching text sitting off the end of the line: a highlighted
row with nothing visibly highlighted on it.

`detail` now wraps by default, clamped to four lines so that one oversized error cannot fill the
pane, and a row whose *detail* matched the current search is expanded automatically — clamping a hit
is the bug above in another form. Clicking still toggles, for the rare record that exceeds four
lines. Measured against the running filler at 1440: 133 of 133 rows fully visible with nothing
clipped, so in normal operation the click is never needed at all.

Files: ui/src/styles/logs.css, ui/src/operator/Logs.tsx.
