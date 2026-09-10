# 2026-09-05 — Drop the volume/profit line under a Filled status

The Filled badge no longer carries "$1 · +$0.01" beneath it; the amount columns already show what
was filled and the maintainer asked for the line to go. Skips and failures keep their reason.
Files: `ui/src/operator/Orders.tsx`, `docs/ai/{ChangeLog,Flow}.md`.
