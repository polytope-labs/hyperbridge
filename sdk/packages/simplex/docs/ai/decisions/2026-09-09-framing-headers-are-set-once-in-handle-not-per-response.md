# 2026-09-09 — Framing headers are set once in `handle()`, not per response

Decided: set `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY` with
`setHeader` at the top of `UiServer.handle()`, before the Host and CSRF checks run.

Why there rather than at each `writeHead`: the server writes headers from four places (`sendJson`, the
SSE stream, `serveStatic`, the UI-not-built fallback), and Node merges `setHeader` values into a later
`writeHead`, so one assignment covers all four and every route added later. Setting them ahead of the
Host and `X-Simplex-UI` checks also means the 403s carry them — a rejected response is still a document
a page can frame.

Why both headers: `frame-ancestors` is the specified control and what current browsers honour;
`X-Frame-Options: DENY` costs one line and still covers older WebViews that ignore CSP, which is a
plausible way for an operator to open this UI from a phone.

Rejected: a full CSP (`default-src`, `script-src`, …). Stronger, but the SPA is a Vite bundle with
inline styles and an SSE connection, so a usable policy has to be derived from the build output and
checked against the running UI. That is worth doing on its own terms rather than as a side effect of
closing the framing hole, which is the one gap the existing defences left open.
