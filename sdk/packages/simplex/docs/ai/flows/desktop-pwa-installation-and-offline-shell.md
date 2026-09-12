# Desktop PWA installation and offline shell

`ui/index.html` links `public/manifest.webmanifest`; `ui/src/main.tsx` registers `public/sw.js` in
production. The worker pre-caches only versioned static shell assets and uses navigation fallback so
the setup/dashboard interface can open offline without persisting live `/api` balance or activity
responses.

`InstallAppProvider` owns the captured `beforeinstallprompt` event, standalone detection, install
dialog state, and toast feedback. `InstallAppButton` renders in the setup brandbar and permanently in
the operator navigation. Both open the same desktop-only `InstallGuidePanel`: identify the install
icon, confirm Install, then open Simplex from the desktop/app list. Native prompt cancellation closes
nothing and writes no inline state; Sonner displays the retry message.
