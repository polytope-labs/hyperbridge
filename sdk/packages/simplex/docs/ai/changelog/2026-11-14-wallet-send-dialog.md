# Wallet send is a dialog laid out like the HyperFX app's

Sending from the filler wallet opens `SendDialog` (`ui/src/operator/SendDialog.tsx`), a centred
dialog on desktop and a full-width drawer on a phone. It replaces the "Send funds" side sheet with
its Network and Asset selects.

- The asset row shows the token on its network with the balance. It opens a list of the tokens the
  wallet holds, on every network, largest balance first (`SendAssetSelect`). A token is left out
  when its balance is zero, is dust that shows as zero, or has not been read. The list is laid over
  the fields inside the dialog, so the dialog keeps its height while a token is chosen.
- "To address" shows the network's logo once the address is valid, and says whether it is. The
  paste button appears only where the browser exposes the clipboard, which needs HTTPS, localhost or
  the desktop app.
- The amount groups thousands as it is typed (`AmountInput`), and the plain number is what is sent.
  The card has 25%, 50%, 75% and Max. Each fills a share of the available balance, rounded down to
  six decimals so it never asks for more than is held. An amount above the available balance turns
  the card red and disables Send.
- A token that is not listed is still sent by contract address, through the link under the asset
  row. It goes out on the network of the asset that is selected.
- Send shows the review (`SendReview`) in place of the form, in the same dialog. Nothing is
  submitted until the review is confirmed, and Back returns to the form as it was.
- A confirmed send replaces the form with a success screen: a large tick, the amount and network,
  a link to the transaction and Done. A send that is still pending returns to the form with its
  warning.

`POST /api/send` and its request are unchanged.

`TokenNetworkPicker`, which the BRIDGE top-up still uses, had no background when it opened as a
phone drawer. It now has the surface its desktop dialog has.
