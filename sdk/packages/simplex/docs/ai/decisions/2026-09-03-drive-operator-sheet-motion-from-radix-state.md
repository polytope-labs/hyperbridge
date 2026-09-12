# 2026-09-03 — Drive operator sheet motion from Radix state

Chosen: keep `components/ui/Sheet.tsx` as the shared shadcn-style sheet composition over Radix Dialog,
and bind its panel and overlay animations to Radix's `data-state="open"` and `data-state="closed"`.
This lets Radix retain the portal until the close animation finishes, gives every operator drawer the
same full-width slide and overlay fade, and preserves the native focus trap, Escape handling, outside
click behavior, and accessibility semantics. Reduced-motion preferences disable both animations.

Alternative rejected: the previous unconditional mount animation could only animate opening and
moved the panel just two rem instead of from off canvas, so closing felt like an abrupt unmount.
Replacing Radix with a second drawer dependency would duplicate an existing shadcn-compatible
primitive without improving the right-side sheet interaction.
