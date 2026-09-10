# 2026-09-03 — Rebrand the Simplex UI header as HyperFX

Replaced the Hyperbridge mark beside the Simplex product name in both the setup wizard and live
operator dashboard with the supplied white HyperFX wordmark. The transparent wordmark now sits
directly on the dark UI without a backing surface. Added the HyperFX website favicon to the UI build
and taught the local static server to serve bundled WebP assets with the correct MIME type.

Files: `ui/src/{operator/Operator,wizard/Wizard}.tsx`, `ui/src/styles/{operator,foundations,responsive}.css`,
`ui/src/{assets/hyperfx-logo.webp,vite-env.d.ts}`, `ui/index.html`, `ui/public/favicon.ico`,
`src/services/server/static.ts`.
