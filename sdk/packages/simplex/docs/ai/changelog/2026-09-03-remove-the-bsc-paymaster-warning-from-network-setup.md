# 2026-09-03 — Remove the BSC paymaster warning from network setup

Removed the stale BSC and BSC Chapel paymaster caveats from the onboarding chain catalog so selecting
those networks no longer displays the native-gas warning in the network setup step. Runtime and review
funding behavior remain unchanged.

Files: `src/cli/init/chains.ts` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.
