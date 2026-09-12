# Operator market drawer

Selecting a row in `OperatorMarkets` opens the shared wide `OperatorSheet` and mounts
`StrategyMarketEditor` for that strategy. The editor derives its visible buy/sell sides from the
persisted strategy plus unsaved enable-side state, then renders market identity, maximum-order risk,
and price controls as one flat, divider-led sequence. `CurveEditor` remains the canonical point
editor; inside an operator drawer its preview is the only dedicated visual surface and sits beside
the point table on desktop without changing the curve data model or API payloads.

The maximum-order action still calls `applyMaxOrderSize` independently. The drawer's final action
bar calls `applyCurves`, while disabled directions first become local draft curves through Add buy
side or Add sell side. No mutation occurs until the matching save/apply action is used.

Every operator drawer is rendered through the local shadcn-style `Sheet`, composed from Radix Dialog
Root, Portal, Overlay, Content, Title, Description, and Close primitives. Radix applies open and closed
state attributes to the overlay and content; `operator.css` uses those states for paired fade and
full-width slide animations, so closing completes before the portal is removed.
