# 2026-09-09 — Deny framing of the operator UI (clickjacking)

The UI server sent no `X-Frame-Options` and no `frame-ancestors` CSP, so any page could put the operator
UI in an invisible iframe — including a page in a phone's browser while the remote-access tunnel's local
forward is up on `127.0.0.1:8686`. The existing defences do not cover this and were never meant to: a
framing page can neither read nor script the cross-origin document, so the `X-Simplex-UI` preflight
requirement and the Host-header rebinding check both still hold, but an overlay can make the operator
click the real UI's own buttons, and those clicks are same-origin and carry the header. One click each
reaches `/api/pause`, `/api/reset-halt`, `/api/vault/sweep` and `/api/vault/redeem`. `/api/stop` sits
behind a `window.confirm`, which browsers suppress in cross-origin frames, and `/api/send` needs typed
input.

`handle()` now sets `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY` on
every response, before any route can return, so rejections carry them too. Setting them with `setHeader`
rather than at each `writeHead` means the four existing header-writing sites (JSON, SSE, static, the
UI-not-built page) merge them in and a future route cannot forget them. Added a test asserting both
headers on HTML, on JSON, and on a 403.

Files: src/services/server/UiServer.ts, src/tests/ui-server.test.ts.
