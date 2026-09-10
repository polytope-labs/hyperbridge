# Page routes

`useTabRoute` (`ui/src/lib/route.ts`) maps the sidebar pages to `/`, `/orders`, `/wallet` and
`/operations`. The active tab is initialised from `location.pathname`, navigation calls
`history.pushState` and `popstate` updates it, so reloads and back/forward keep the page. The
server needs nothing: `serveStatic` falls back to `index.html` for any non-file path, and the
service worker fetches navigations from the network first. Routes are single-segment because
`index.html` references its assets as `./assets/…`.
