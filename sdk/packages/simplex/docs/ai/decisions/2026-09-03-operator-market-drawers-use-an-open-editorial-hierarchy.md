# 2026-09-03 — Operator market drawers use an open editorial hierarchy

Chosen: keep the market drawer wide, but organize it vertically by operator task: market identity,
order risk, pricing directions, then commit or delete actions. Those regions remain visually open and
use typography, whitespace, and single hairline separators—the same language as onboarding—instead
of nested cards, pills, or dashed callout boxes. Each direction uses the full width; its quiet chart
canvas and editable points sit side by side on desktop and stack only when the drawer is narrow.
Shared sheet padding provides consistent separation after every drawer header, while market-specific
styling remains scoped under `operator-market-editor`.

Alternatives rejected: placing explanatory copy, input, and save action in one three-column row
created uneven baselines and made the help text compete with the control; keeping two direction
columns left a one-sided market stranded in half an otherwise empty drawer; wrapping every task in a
rounded surface produced a repetitive, box-heavy hierarchy; solving spacing only in the market
component would leave other operator drawers flush against the shared header.
