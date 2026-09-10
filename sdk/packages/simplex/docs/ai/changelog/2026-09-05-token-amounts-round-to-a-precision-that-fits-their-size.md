# 2026-09-05 — Token amounts round to a precision that fits their size

`formatTokenAmount` picked four fraction digits regardless of magnitude, so 19,990.9995 USDC and
27,393,956.4843 CNGN filled the amount cells. It now rounds half-up to 0 places at 10,000 and above,
2 places at 100 and above, and 4 below that (an explicit `maxFraction` still overrides), and the
order row's leg carries the full-precision amount as a tooltip.
Files: `ui/src/lib/format.ts`, `ui/src/operator/Orders.tsx`, `docs/ai/ChangeLog.md`.
