# 2026-09-05 — Use the Hyperbridge favicon in the Simplex web UI

Replaced the HyperFX favicon with a copy of the docs site's Hyperbridge favicon so the browser tab
shows the Hyperbridge mark. Bumped the service worker precache name (`simplex-shell-v4` →
`simplex-shell-v5`) so existing installs refetch `favicon.ico` instead of serving the old icon.
Files: `ui/public/favicon.ico`, `ui/public/sw.js`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
