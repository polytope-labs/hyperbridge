# 2026-09-05 — Orders another solver filled read "Outbid", not "Lost"

The amber "Lost · filled by 0x…" status read as a fault. It is now a neutral "Outbid" badge (same
tone as Detected) with the winner's short address beneath in mono, full address on hover. Chosen
from three mocked options; the maintainer declined a link on the winner.
Files: `ui/src/operator/Orders.tsx`, `ui/src/styles/operator.css`, `docs/ai/{ChangeLog,Flow}.md`.
