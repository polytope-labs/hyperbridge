# 2026-09-07 — Fix the double rule under the operator lists

`.operator-section` draws a bottom rule and each of the tool, balance and market lists ends its
section, so a last row drawing its own border left two lines a section-padding apart — visible on
the Wallet page under Vault treasury, and on the Overview under both the balances and the markets.
Pre-existing; the last row in those lists no longer draws a border. Same defect as the remote-access
device list fixed earlier.

Files: `ui/src/styles/operator.css`, `docs/ai/ChangeLog.md`.
