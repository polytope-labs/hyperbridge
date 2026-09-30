# Setup wizard presentation and navigation

`ui/src/App.tsx` owns the page shell for every UI state. It places the animated brand line at the
viewport's top edge and centers the active view inside `.app-container`; setup mode renders
`Wizard`, while loading, error, and operator states use the same shell. In the operator state the
shell carries `app-shell-operator`, which removes the page padding and width cap so the dashboard
fills the viewport edge to edge; the wizard keeps the padded, card-style presentation.

`ui/src/wizard/Wizard.tsx` owns the setup draft, active step, per-step requirements, and forward/back
navigation. It maps the active step to its editor and derives completed/active/upcoming rail states,
display numbering, and percentage progress. At desktop widths the rail and active step form a
two-column grid; below 900px the rail moves above the form and scrolls horizontally. Continue stays
disabled while the current step has unresolved requirements, and the footer lists every blocker so
the operator can see what must be fixed without guessing.

The shared desktop and browser wizard is mainnet-only. `GET /api/setup/defaults` returns the mainnet
catalog and Nexus WebSocket default, and setup preview/save rejects a selected testnet chain. The
CLI initializer keeps its separate testnet bucket for advanced use.
