# 2026-09-07 — Review a transfer in a dialog instead of a browser confirm

Send funds asked for confirmation through `window.confirm` with a one-line string, which the button
("Review transfer") had already promised more than. It now opens a summary: the token with its logo
and the amount, the network with its logo, the full recipient, the wallet balance, what is available,
and the split the sender will actually make — the wallet covers what it can and the vault covers the
rest, which is what `TokenSender` does. The vault line is a real check, not a restatement: a
withdrawal draws on one vault rather than several, so when the shortfall exceeds the largest single
vault the dialog says the transfer will fail before it is submitted. Everything is read from the
balance snapshot the dashboard already polls, so the review costs no extra call.

Files: `ui/src/components/SendConfirmDialog.tsx`, `ui/src/operator/WalletTools.tsx`,
`ui/src/styles/{operator,responsive}.css`, `docs/ai/ChangeLog.md`.
