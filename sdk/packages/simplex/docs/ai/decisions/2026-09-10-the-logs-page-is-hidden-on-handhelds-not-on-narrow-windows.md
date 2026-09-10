# 2026-09-10 — The Logs page is hidden on handhelds, not on narrow windows

Decided: the gate is `(max-width: 600px), (pointer: coarse) and (max-height: 600px)` — the layout's
own mobile breakpoint, plus a touch primary input on a short viewport.

Width alone was wrong in both directions. A phone in landscape is 700-950px wide and read as a
desktop, so the page the rule exists to withhold mounted and opened a stream on exactly that
hardware; a desktop browser dragged narrow read as a phone. The second clause catches the rotated
phone by pairing touch input with a short viewport, which a tablet in landscape (768px+ tall) and a
laptop both fail.

The layout query is left alone and still shared with the drawer/dialog switch, because "how should
this be laid out" and "is this worth showing at all" are different questions that happen to agree at
600px.

Why the page is desktop-only at all: a log line is a wide, dense, monospace record, and reading logs
means scanning and comparing them — the thing a 390px column is worst at. It is also the only page
holding an open stream and thousands of rows, which is real battery and memory on a device that came
to check a balance or unpause filling. The guard is in the component (`tab === "logs" && !handheld`),
not in CSS, so the page never mounts and no stream is opened even for the frame before the redirect.
