# 2026-09-05 — Sidebar pages have URLs that survive a reload

Each dashboard page has a path (`/`, `/orders`, `/wallet`, `/operations`). `ui/src/lib/route.ts`
holds the map and a `useTabRoute` hook that reads the path on load, pushes a history entry on
navigation and follows back/forward; `Operator` uses it in place of its tab state. No server change:
`serveStatic` already returns index.html for any path that is not a file. Paths stay single-segment
because index.html loads assets relatively.
Files: `ui/src/lib/route.ts`, `ui/src/operator/Operator.tsx`, `docs/ai/{ChangeLog,Flow}.md`.
