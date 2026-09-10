# 2026-08-31 — Simplex UI cleanup and operator-market module

Reviewed the onboarding and operator UI against the agreed design system and extracted live market
administration from the dashboard shell into a dedicated module. Runtime market mutations now reject
duplicate submissions synchronously, token selection uses the shared image-rich control, mobile
operator navigation remains fixed in view, and the wizard lists every unresolved requirement instead
of hiding additional blockers. The live chain editor now shares the onboarding logos, collapsibles,
plain-language endpoint labels, and toast feedback. Added a clean UI check command and corrected the
setup flow notes to match the current navigation and validation ownership.

Follow-up cleanup moved application boot state into a discriminated-state hook, extracted the render
error boundary, replaced native dashboard drawers and wizard dialogs with Radix-backed shadcn-style
Sheet/Dialog primitives, split the monolithic stylesheet into ordered domain files, and separated
market and chain business logic from their view modules.

Files: `ui/src/app/`, `ui/src/operator/`, `ui/src/wizard/{steps,strategies}/`,
`ui/src/components/{ui/,OperatorSheet,ScreenErrorBoundary}.tsx`, `ui/src/lib/hooks.ts`,
`ui/src/styles/`, `package.json`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
