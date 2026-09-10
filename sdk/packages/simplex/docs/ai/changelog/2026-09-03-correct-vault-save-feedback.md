# 2026-09-03 — Correct vault save feedback

Vault saves no longer turn a successful persisted response into an error instructing the operator to
restart the filler. A persisted save now emits a clear success toast, while the vault editor limits
its explanatory copy to the configuration persistence it can accurately promise. Runtime, server,
and vault lifecycle behavior are unchanged.

Files: `ui/src/operator/Operations.tsx` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.
