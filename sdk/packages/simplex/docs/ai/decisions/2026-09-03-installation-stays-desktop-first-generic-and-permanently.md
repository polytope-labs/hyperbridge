# 2026-09-03 — Installation stays desktop-first, generic, and permanently discoverable

Updated after the native application shipped: Simplex Desktop is the recommended local installation
for macOS, Windows, and Linux. Its released name is `Simplex`, its permanent bundle identifier is
`network.hyperbridge.simplex`, and its version tracks `@hyperbridge/simplex` exactly. The installer
uses the current PWA app icon on every platform. Docker and the npm binary remain supported
advanced paths for servers and managed deployments.

The PWA remains intentionally available. It is the browser-installable dashboard for an existing
solver, especially on phones and through the authenticated tunnel; it does not package the solver
and is not the primary local installation. Keep the persistent install action and three-step guide
in the web UI. The PWA caches only its application shell, so live balances and activity still come
from the solver API after reconnection.

Rejected: independent desktop versioning would make operator bug reports ambiguous; changing the
bundle identifier after release would orphan existing installs; treating the PWA as a second local
distribution would hide that it has no solver runtime; caching API responses could display stale
operational or financial state as current.
