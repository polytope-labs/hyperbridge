# Simplex desktop owns its renderer

The Simplex desktop custom protocol serves HTML, scripts, styles, icons, and client-side routes from the UI bundled with the installed application. Only `/api/*` and `/health` requests are proxied to the solver's private socket.

This keeps the desktop shell and renderer on the same build when Electron attaches to an already-running solver. A stale local solver with missing or older UI assets can no longer replace the installed desktop UI; API version skew remains blocked by the renderer's existing desktop-versus-solver version check.
