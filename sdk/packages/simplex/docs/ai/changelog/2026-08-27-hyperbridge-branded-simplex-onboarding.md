# 2026-08-27 — Hyperbridge-branded Simplex onboarding

Restyled the setup wizard around the shared Hyperbridge visual language: Aeonik and Aeonik Mono,
the Hyperbridge mark and spectrum accent, the `#131417` canvas, blue-black surface layers, muted
`#929daa` copy, white primary actions, and rounded controls. The wizard now presents a persistent
desktop journey rail, a compact horizontally scrollable mobile rail, a step-level progress header,
and a local-credentials reassurance without changing any setup state or API behavior. The app shell
also anchors the brand gradient to the top edge and centers all states in a max-width container.

Files: `ui/src/App.tsx`, `ui/src/styles.css`, `ui/src/wizard/Wizard.tsx`,
`ui/src/assets/hyperbridge-logo.svg`, `ui/src/assets/fonts/*.woff2`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
