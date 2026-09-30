# 2026-09-03 — Keep delegation failure fatal, but make setup recovery actionable

Chosen: preserve the filler's failed-start state when EIP-7702 delegation fails on every enabled
chain, while translating the internal chain identifiers and restart wording into network labels and
checks the operator can perform: stablecoin funding, RPC/bundler availability, retry, and image
version. The configuration remains saved and the primary action becomes Retry startup.

Alternative rejected: entering the dashboard after every delegation attempt fails would present an
apparently healthy solver that cannot bid. Showing only the internal shutdown message was also
rejected because the browser setup process remains available for recovery and needs actionable copy.
