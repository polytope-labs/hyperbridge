# 2026-09-05 — Watch-only orders are recorded as skipped at intake

`IntentFiller`'s intake queue checks the destination's watch-only flag before `evaluateOrder` and
returned after a debug log, so a watch-only order reached the activity feed as "detected" with no
reason; `evaluateOrder`'s own watch-only check, which records `orderSkipped` with reason
"watch-only", never ran for it. The intake check now logs at info and emits the same skip. Found
while explaining why a paused-then-resumed watch-only instance never bid on a Base order.
Files: `src/core/filler.ts`, `docs/ai/{ChangeLog,Flow}.md`.
