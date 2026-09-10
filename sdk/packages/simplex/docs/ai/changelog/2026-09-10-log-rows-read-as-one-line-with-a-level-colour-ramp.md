# 2026-09-10 — Log rows read as one line, with a colour ramp on the level

Two changes to how a record renders.

**The level column is a severity ramp.** Only `warn` and `error` had a colour; `trace`, `debug` and
`info` all shared one grey, so the column carried no information until something went wrong. Each
level now has its own: trace a dim slate that recedes below the body text, debug a muted violet,
info green, and warn and error keeping the palette's alarm colours. Violet rather than blue for
debug because the field keys are already blue and debug is the level that dominates a page — it has
to stay quiet and stay distinguishable from the fields beside it. Only warn and error still tint the
row itself: colour in the level column is for telling levels apart, the row treatment is for raising
an alarm. Trace was first set at `#6b7684`, which measured 4.12 against the stream's ground and
failed the 4.5 AA floor for small text; it is `#79838f` at 4.9.

**The message and its fields are one line.** They were stacked, message above fields. They now flow
together and wrap together, comma-separated — `Scanning blocks  chainId: 8453, fromBlock: 51128842,
toBlock: 51128843, gap: 1` — which is how a console prints them and roughly doubles the records
visible in the pane. The four-line clamp moved from the fields to the body, since there is no longer
a separate block to clamp.

The gap between message and fields is a real character in the DOM, not the CSS margin it started as.
A margin looks right and copies wrong: selecting a row yielded `Vault refreshedchain: "EVM-8453"`,
and these lines get pasted into tickets.

The fields moved from a `<button>` to a `<span>` with `role="button"` to make any of this work. A
button is an atomic inline box — it occupies one rectangle and cannot break across lines — so a long
record's fields could not share the message's line and the whole box dropped below it. Short records
looked right and long ones did not, which is what the one-line change appeared to fail at. A span
breaks like the inline text it is; the keyboard toggle moved to an `onKeyDown` handler, since a span
does not activate on Enter by itself.

Files: ui/src/styles/logs.css, ui/src/operator/Logs.tsx.
