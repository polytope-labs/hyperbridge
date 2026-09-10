# 2026-09-03 — Give the shared operator sheet complete motion

Kept the shared shadcn-style Radix sheet used by every operator drawer and replaced its mount-only
effect with state-aware motion. The panel now enters from fully off canvas, exits before Radix removes
the portal, and coordinates both directions with an overlay fade; reduced-motion users receive the
same state change without animation.

Files: `ui/src/styles/operator.css` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.
