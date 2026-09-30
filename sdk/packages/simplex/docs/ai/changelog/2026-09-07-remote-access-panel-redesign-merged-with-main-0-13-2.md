# 2026-09-07 — Remote access panel redesign, merged with main (0.13.2)

Merged `origin/main` (UI improvements 0.13.2, paymaster skip reasons) into the branch; conflicts were
both-sides-appended docs, the version line (kept 0.14.0) and `operator.css`, where the merge dropped
one closing brace. The Remote access sheet is now `wide` and rebuilt on the dashboard's own pieces:
`card` sections with eyebrow/heading and a state `badge`, the `chain-enable-switch` toggle, a
two-column `tunnel-facts` definition list with copy buttons, the relay address behind a `details`
disclosure, a device list, and `PillTabs` for paste-vs-generate. Phone-width rules in `responsive.css`
collapse the grid and the QR/key row. Verified in a browser against the live relay: toggling on from the
panel connected and showed the leased public endpoint.

Files: `ui/src/operator/{RemoteAccess,Operations}.tsx`, `ui/src/styles/{operator,responsive}.css`,
`docs/ai/ChangeLog.md`.
