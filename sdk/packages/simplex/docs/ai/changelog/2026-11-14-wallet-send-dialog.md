# Wallet send is a dialog laid out like the HyperFX app's

Sending from the filler wallet opens `SendDialog` (`ui/src/operator/SendDialog.tsx`), a centred
dialog on desktop and a full-width drawer on a phone. It replaces the "Send funds" side sheet with
its Network and Asset selects.

- The asset row shows the token on its network with the balance. It opens a list of the tokens the
  wallet holds, on every network, largest balance first (`SendAssetSelect`). A token is left out
  when its balance is zero, is dust that shows as zero, or is not known. The balance snapshot covers
  the native token, USDC, USDT and the other tokens the solver is set up to trade, so a token
  outside those is never listed. The list is laid over the fields inside the dialog, so the dialog
  keeps its height while a token is chosen.
- The balance is what one send can move: the wallet's balance plus what the largest vault can
  return. It is not the dashboard's "available", which leaves out the wallet floor a vault asks
  for. `POST /api/send` does not keep that floor.
- "To address" shows the network's logo once the address is valid, and says whether it is. The
  paste button appears only where the browser exposes the clipboard, which needs HTTPS, localhost or
  the desktop app.
- The amount groups thousands as it is typed (`AmountInput`), and the plain number is what is sent.
  A comma is read as the decimal mark unless it can only be a separator, as in `1,000` or
  `1,000.50`, so `1,5` is 1.5. Text that is not a number is refused. Balances in the dialog use the
  same notation whatever the browser's locale.
- The card has 25%, 50%, 75% and Max. Each fills a share of the balance, rounded down to six
  decimals and never above the balance. An amount above the balance turns the card red and disables
  Send.
- A token that is not listed is sent by contract address, through the link under the asset row.
  The asset row then lists the networks, and the contract has to be a valid address.
- Send shows the review (`SendReview`) in place of the form, in the same dialog. A token sent by
  contract address is shown with its contract. The confirm button comes on half a second after the
  review appears, so a second tap meant for Send cannot confirm it. Confirming submits what the
  review showed, and Back returns to the form as it was.
- A confirmed send replaces the form with a success screen: a large tick, the amount and network,
  a link to the transaction and Done. A send that is still pending returns to the form with its
  warning.
- Closing the dialog clears the form.

`POST /api/send` and its request are unchanged.

`TokenNetworkPicker`, which the BRIDGE top-up still uses, had no background when it opened as a
phone drawer. It now has the surface its desktop dialog has.
