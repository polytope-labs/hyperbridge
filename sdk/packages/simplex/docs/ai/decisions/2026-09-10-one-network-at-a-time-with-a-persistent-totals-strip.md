# 2026-09-10 — One network at a time, with a persistent totals strip

Decided: the Overview's balances section shows a single network's token cards, chosen with a
switcher in the section heading, and keeps every token's cross-network total in a strip above them.

Why not keep stacking every network: it is the thing being fixed. Nine networks produced about
fourteen cards down one scroll, and the answer to "where is my USDC" was buried in the middle of it.

Why not collapse each network to a summary row that expands: it trades one scroll for a lot of
clicking, and the balances a filler operator wants are per token, not per network.

Why the totals strip is not optional: narrowing to one network silently deletes the fleet-wide
figure the stacked view gave away for free. The strip is what makes the narrowing safe, so it is
persistent rather than a second tab or a hover.

Why tokens are not sorted by size: they are different assets. 151,744 cNGN is not "more" than
98,144 USDC, and a strip that ranks them implies an exchange rate the app does not have. Config
order it is, with a `+2 more EURC, DAI` note when there are more tokens than cells, so a token is
never dropped without saying so.

Why a token with one unread contributor reads `Unavailable`: `availableStablecoinLiquidity` already
refuses to estimate — one null `available` and the whole figure is null. A strip that quietly summed
the legs it could read would understate liquidity in exactly the situation where the operator is
least able to notice. Both now share `availableStablecoins()`.

Why there is no per-network health signal in the switcher: a first cut gave each row a status dot
and rolled "2 with unread balances" into the trigger, derived from `issues[]` and `asset.status`.
It was machinery for a state the section already reports — the "Some balances are unavailable"
notice covers the whole snapshot, and a network whose read failed shows `Unavailable` in the
stables column on its own row. `ChainBalanceRow` has no health field, and the right amount of
health to infer from its absence turned out to be none.

Why the section-level "Some balances are unavailable" notice stays: with one network on screen it
is the only surface that can report a failure on a network that is not.

Why `AppSelect` rather than a hand-rolled popover: the switcher needs keyboard behaviour, focus
management and a focus ring that match the rest of the app, and Radix Select already has them.
The cost is five optional props (`description`, `trailing`, `caption`, `header`,
`contentClassName`); each is a generic select feature and none changes an existing caller's DOM.

Why the default network is simply the first configured one: it is stable across refreshes and needs
no state. Letting a network with a failed read win the default meant deriving that condition and
then pinning it against later refreshes, so it did not move under the operator mid-read — two
mechanisms for a default nobody had asked to be clever.

Why the column count is a CSS custom property rather than an inline `grid-template-columns`: the
count is data (one cell per token held, so two tokens do not leave two dead cells), but the mobile
breakpoints have to override the layout, and they cannot override an inline style.
