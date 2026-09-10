# 2026-09-03 — The `_paused` getter was dropped to stay under EIP-170

Chosen: `bool internal _paused`. The variable stays in slot 13 so the layout is unchanged.

The gateway compiled to 10 bytes over the limit with the new storage, setter, event and checks.
Nothing reads `_paused()` anywhere in the repo, so removing its getter was the only free saving.
The revert reuses `Unauthorized()` instead of a dedicated error for the same reason; the second
selector cost 15 bytes.
