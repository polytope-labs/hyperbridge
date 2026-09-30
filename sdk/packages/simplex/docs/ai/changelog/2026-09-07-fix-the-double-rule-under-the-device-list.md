# 2026-09-07 — Fix the double rule under the device list

`.sheet-content .card` gives every panel section its own bottom rule, and the last `.tunnel-device`
row drew one too, so two lines sat 25px apart between the device list and the pairing section. The
last row no longer draws its border.

Files: `ui/src/styles/operator.css`, `docs/ai/ChangeLog.md`.
