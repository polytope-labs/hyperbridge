# 2026-09-07 — Off by default, enabled from the UI only

Enabling remote access makes the embedded SSH server reachable by anyone who scans the relay, so
it is opt-in, and pairing lives in the operator UI rather than the `simplex init` wizard (Seun's
call: the wizard stays focused on the filler config). The UI writes `[simplex.tunnel]` back to
the config file so the choice survives restarts.
