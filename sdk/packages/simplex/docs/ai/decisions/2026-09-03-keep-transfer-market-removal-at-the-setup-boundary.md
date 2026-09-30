# 2026-09-03 — Keep transfer-market removal at the setup boundary

Chosen: remove the dedicated same-token transfer experience from setup and new-market creation,
including its prefab rows, state model, curve branch, defaults payload, and CSS, while preserving
runtime and operator read/edit compatibility for markets already present in an existing config.
New setup markets are cross-asset only and reject identical symbols with a clear UI error.

Alternative rejected: deleting same-token support from the runtime engine and config validator would
make existing operators unable to start or safely migrate configurations that still contain those
markets, which is a broader behavioral change than removing the irrelevant setup section.
