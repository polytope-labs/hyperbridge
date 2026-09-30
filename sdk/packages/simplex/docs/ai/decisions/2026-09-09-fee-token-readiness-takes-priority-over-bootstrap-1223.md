# 2026-09-09 — Fee-token readiness takes priority over bootstrap (#1223)

Chosen: scan configured fee tokens in USDC-then-USDT order and select the first with both
at least one whole token of balance and Permit2 allowance covering the existing $5
recommendation. Both thresholds use that token's decimals. Stop when a ready token is
found; otherwise retain the first balance-qualified token as the bootstrap fallback.
Use the same read-only selector for sponsorship and delegation approval resolution.

Alternatives considered: keep selecting the first funded token and approve it before
checking another token; or prefer an EIP-2612-capable bootstrap token over one already
usable through Permit2.

Why: an unapproved USDC balance must not hide funded, approved USDT. Approving USDC in
that state either unnecessarily spends native gas or prevents sponsorship when native is
absent. A permit-funded bootstrap is also unnecessary when another token is ready.
Sharing selection with delegation setup prevents the two paths from disagreeing about
whether an approval is needed. Approval alone is insufficient: empty or sub-minimum
balances remain ineligible even with unlimited allowance.

This is a readiness preference, not a new bootstrap policy. USDC still wins when both
tokens are ready. When neither is ready, the existing first-funded-token fallback and
its permit/native approval rules remain, including the native-funded zero-first reset
for stale non-zero allowances. The existing one-token balance minimum and $5 allowance
recommendation are unchanged; they are selection thresholds, not a guarantee that every
operation can be sponsored.
