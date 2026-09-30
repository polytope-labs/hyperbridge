# Page routes

`useTabRoute` (`ui/src/lib/route.ts`) maps the sidebar pages to `/`, `/orders`, `/wallet`, `/logs`
and `/operations`. The active tab is initialised from `location.pathname`, navigation calls
`history.pushState` and `popstate` updates it, so reloads and back/forward keep the page. The
server needs nothing: `serveStatic` falls back to `index.html` for any non-file path, and the
service worker fetches navigations from the network first. Routes are single-segment because
`index.html` references its assets as `./assets/…`.

`/logs` is the one route that is not reachable everywhere. `Operator` reads `useIsHandheld()`
(`ui/src/lib/hooks.ts`), drops any `desktopOnly` tab from the nav, and runs an effect that calls
`setTab("overview", { replace: true })` when a handheld lands on it — replace rather than push, so
Back does not bounce off a page the app just redirected away from. The render is guarded separately
(`tab === "logs" && !handheld`), which is what keeps the log stream from being opened for the frame
before that effect runs, and what closes it when a desktop window is dragged below the breakpoint.
