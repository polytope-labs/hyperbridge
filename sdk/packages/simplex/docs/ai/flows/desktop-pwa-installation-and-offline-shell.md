# Native desktop and PWA installation

The public docs installation page leads to GitHub releases tagged `simplex-desktop-v<version>` and
names the DMG, NSIS, AppImage, and deb artifact for every supported architecture. The native app
bundles the solver and matching Node runtime, connects its renderer through a private socket, and
writes a first-run config into Electron's per-user data directory. Closing the window leaves the
solver running; the two quit actions distinguish closing Electron from stopping the solver.

`@hyperbridge/simplex-desktop` and `@hyperbridge/simplex` carry the same version. Packaging fixes the
installed name as `Simplex` and bundle identifier as `network.hyperbridge.simplex`. The macOS
`.icns`, Windows `.ico`, Linux PNG set, and development application icon all derive from the current
PWA `mobile-logo.svg` mark. Packaged resources also contain the Electron and Chromium license notices.

The PWA is the browser/remote-access path for a solver that is already running. `ui/index.html` links
`public/manifest.webmanifest`; `ui/src/main.tsx` registers `public/sw.js` in production. The worker
pre-caches only versioned static shell assets and uses navigation fallback so the dashboard interface
can open offline without persisting live `/api` balance or activity responses.

`InstallAppProvider` owns the captured `beforeinstallprompt` event, standalone detection, install
dialog state, and toast feedback. `InstallAppButton` renders in the setup brandbar and permanently in
the operator navigation. Both open the same desktop-only `InstallGuidePanel`: identify the install
icon, confirm Install, then open Simplex from the desktop/app list. Native prompt cancellation closes
nothing and writes no inline state; Sonner displays the retry message.

Every install path writes private keys to `filler-config.toml` in plaintext. Unix creation is atomic
and mode `0600`; `simplex.substratePrivateKey` remains required even when the EVM signer is Turnkey or
MPC Vault. The CLI's TCP dashboard is unauthenticated and loopback is shared by all local users, so a
shared macOS or Linux host uses `--ui-socket` plus SSH forwarding, or the authenticated tunnel. The
native desktop path already uses its private socket and opens no TCP listener.
