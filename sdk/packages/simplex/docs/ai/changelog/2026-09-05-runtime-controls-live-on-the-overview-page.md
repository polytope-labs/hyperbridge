# 2026-09-05 — Runtime controls live on the Overview page

The "Runtime controls" header button and its sheet are gone; the Overview renders the state line
("Filling is active" / "New fills are paused") with the Pause/Resume and Stop buttons beside it,
directly under the metrics strip. `OperatorOverview` takes a `runtime` prop from `Operator`.
Files: `ui/src/operator/{Operator,OperatorOverview}.tsx`, `ui/src/styles/{operator,responsive}.css`,
`docs/ai/{ChangeLog,Flow}.md`.
