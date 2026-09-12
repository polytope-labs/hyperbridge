# 2026-09-04 — Link successful sends to their block explorer

The Send funds success row now renders the transaction hash and an external-link icon as one link
that opens the selected network's block explorer in a new tab. The explorer URL is captured with the
completed send so changing the form's network afterward cannot redirect the prior hash to the wrong
chain.

Files: `ui/src/operator/Operations.tsx`, `ui/src/styles/operator.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
