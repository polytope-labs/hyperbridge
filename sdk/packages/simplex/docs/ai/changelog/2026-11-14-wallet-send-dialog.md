# Wallet send is a dialog laid out like the HyperFX app's

Sending from the filler wallet opens `SendDialog` (`ui/src/operator/SendDialog.tsx`), a centred
dialog on desktop and a drawer on a phone. It replaces the "Send funds" side sheet with its Network
and Asset selects.

- The asset row shows the token on its network with the balance, and opens `TokenNetworkPicker`,
  which picks the token and the network together.
- "To address" shows the network's logo once the address is valid, and says whether it is. The
  paste button appears only where the browser exposes the clipboard, which needs HTTPS, localhost or
  the desktop app.
- The amount card has 25%, 50%, 75% and Max. Each fills a share of the available balance, rounded
  down to six decimals so it never asks for more than is held. An amount above the available balance
  turns the card red and disables Send.
- A token that is not listed is still sent by contract address, through the link under the asset row.

Send still opens the `SendConfirmDialog` review, and nothing is submitted until that is confirmed.
`POST /api/send` and its request are unchanged.
