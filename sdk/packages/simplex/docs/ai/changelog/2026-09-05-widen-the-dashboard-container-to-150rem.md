# 2026-09-05 — Widen the dashboard container to 150rem

Two nested caps limited the UI: `.app-container` at 80rem (1280px) for everything, and inside it
`.operator-container` at 96rem for the dashboard, which is the one that bound on wide screens and
left the order history about 900px of usable width after the 16rem sidebar and column padding.
Both are now 150rem (2400px); the setup wizard shares the outer container and widens with it.
Files: `ui/src/styles/{foundations,operator}.css`, `docs/ai/ChangeLog.md`.
