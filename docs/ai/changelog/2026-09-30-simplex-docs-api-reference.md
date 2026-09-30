# 2026-09-30 — Simplex docs gain an API reference

The Simplex operator guide (`docs/content/developers/evm/simplex/`) now introduces the dashboard
before limit orders, and documents the solver's whole HTTP API in its own section.

- The sidebar order is overview, installation, configuration, dashboard, limit orders, treasury,
  confirmations, troubleshooting, then the API reference.
- `dashboard.mdx` opens with what the dashboard is for and a tour of its five pages (Overview,
  History, Wallet, Logs, Operations). The access and security material follows under
  "Keeping it private", with its anchors unchanged.
- `api/` is the new "API reference" section, one page per area: `overview` (base URL, the
  `X-Simplex-UI` and `Host` rules, setup and operator modes, status codes, SSE streams, and an
  index of every route), `limit-orders`, `status`, `activity`, `wallet`, `configuration`, `logs`,
  `notifications`, `remote-access` and `setup`. It covers every route `UiServer` serves.
  The `/api/desktop/*` routes are served by Simplex Desktop's own protocol handler, not the
  solver, and are not documented here.
- `limit-orders-api.mdx` moved to `api/limit-orders.mdx`. The old URL redirects through
  `docs/public/developers/evm/simplex/limit-orders-api/index.html`, with matching entries in
  `vercel.json` and `_redirects`. The limit orders guide links to it near the top and in
  "Scripting limit orders", and the API page links back to the guide.
- Corrections carried into the moved page: `expiresAt` is an ISO 8601 timestamp, the order object
  includes `orderNonce`, `bookExpiresAt` and `bookPrice`, and cancelling an unknown ID answers
  `400`, not `404`. The SDK reference's "Over HTTP" section says the same and links to the page.
