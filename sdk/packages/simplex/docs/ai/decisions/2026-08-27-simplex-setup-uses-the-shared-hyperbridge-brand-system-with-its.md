# 2026-08-27 — Simplex setup uses the shared Hyperbridge brand system with its own operator-focused composition

Chosen: reuse the exact Hyperbridge foundations already present in `hyperbridge-ui` and
`hyperbridge-fe`—Aeonik/Aeonik Mono, the `#131417` canvas, layered blue-black surfaces, muted
`#929daa` text, white actions, and the cyan/pink/yellow spectrum—but compose them as a quiet solver
configuration workspace. Desktop keeps the setup journey in a left rail while the active form gets
a stable, centered reading column; below 900px the rail becomes horizontally scrollable above the
form. The gradient stays a thin top-edge signature rather than filling cards or controls. The wizard
also owns explicit per-step requirements and shows the complete unresolved checklist beside its
disabled Continue action.

Alternatives rejected: copying a consumer bridge screen verbatim would obscure the longer,
configuration-heavy workflow; keeping the original full-width horizontal stepper made seven steps
compete with the form for space and produced weak hierarchy; applying the spectrum to primary
buttons would diverge from the sibling apps' higher-contrast white action treatment. The step
editors keep their config semantics, while the wizard shell now owns navigation and requirement
gating so disabled progress always has an actionable explanation.
