# 2026-09-07 — 0.15.0, not 0.14.0: main took that version

Merged `origin/main`, which released 0.14.0 for the relayer-gated paymaster work while this branch
was also sitting on 0.14.0 — both sides wrote the same string, so git merged it without a conflict
and the collision was only visible by comparing against main. Remote access ships as 0.15.0. The
only real conflicts were `docs/ai/ChangeLog.md` and `docs/ai/Decisions.md`, where both sides had
prepended entries; both sets are kept.

Files: `package.json`, `docs/ai/{ChangeLog,Decisions}.md`.
