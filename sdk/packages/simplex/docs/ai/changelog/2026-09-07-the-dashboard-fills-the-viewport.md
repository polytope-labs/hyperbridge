# 2026-09-07 — The dashboard fills the viewport

The operator view sat in a rounded, bordered card inside a padded page, capped at 150rem, so on a
wide monitor it floated with dark margins on every side. `App` now marks the shell with
`app-shell-operator` when the dashboard is showing; that strips the page padding and the width cap,
and `.operator-shell` becomes the viewport itself (100dvh, no border, radius or shadow; the tinted
background and blur stay). The layout height drops the 5rem the padding used to take
(`100dvh - 4.5rem` brandbar). The setup wizard keeps its card. The mobile override that removed the
card's border is gone since it is the default now. The side sheet (`.sheet-content`, shared by the
Environment drawer and the market sheets) widens from 32rem to 40rem so a Hyperbridge SS58 account
fits on one line beside its copy button; the wide variant stays at 58rem and mobile stays full width.
Files: `ui/src/App.tsx`, `ui/src/styles/{foundations,operator,responsive}.css`, `docs/ai/ChangeLog.md`.
