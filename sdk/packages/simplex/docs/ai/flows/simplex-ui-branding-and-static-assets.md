# Simplex UI branding and static assets

`ui/src/operator/Operator.tsx` and `ui/src/wizard/Wizard.tsx` import the shared
`ui/src/assets/hyperfx-logo.webp` asset for their brandbars. The brandbar styles provide a compact
white surface around the transparent dark-lettered wordmark, with a narrower width at mobile sizes.
`ui/index.html` references `./favicon.ico` (the Hyperbridge mark, a copy of `docs/public/favicon.ico`);
Vite copies `ui/public/favicon.ico` into `dist/ui`, `ui/public/sw.js` precaches it under a versioned
cache name that must be bumped whenever a precached static asset changes, and
`src/services/server/static.ts` serves both the favicon and bundled WebP assets with image MIME types.
