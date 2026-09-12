# 2026-09-10 — docs/ai split into one file per entry

`ChangeLog.md`, `Decisions.md` and `Flow.md` are gone. Their entries now live as individual
files under `changelog/`, `decisions/` and `flows/` — 14, 18 and 4 of them in this package.
The prose is unchanged; the only edit is that each entry's headings moved up one level, from
`##` to `#`. That was checked by demoting every new file back a level and comparing it to the
text it came from, byte for byte.

Why. The three files were append-only with newest entries on top, so any two concurrent PRs
wrote the same line and GitHub marked both conflicted. #1251 declared `merge=union` for the two
log files, which does resolve the collision locally — but GitHub ignores merge drivers when it
computes a PR's mergeability, so the PRs stayed blocked however clean the local merge was.
A file that only one PR creates cannot collide at all, which is the same reason changesets
keeps one file per change rather than one shared changelog.

Flows stay one file per flow and are still edited in place, so two PRs revising the same flow
still conflict — correctly, because that means they disagree about how the code runs.

Files: docs/ai/{ChangeLog,Decisions,Flow}.md (removed), docs/ai/README.md (new),
docs/ai/{changelog,decisions,flows}/*.md (new). Repo root: CLAUDE.md, .gitattributes (removed).
