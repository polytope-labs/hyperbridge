# Wallet send is a dialog laid out like the HyperFX app's

Sending from the solver wallet opens `SendDialog` (`ui/src/operator/SendDialog.tsx`), a centred
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
  Text that is not a number is refused. Balances in the dialog use the same notation whatever the
  browser's locale.
- A comma typed after digits is the decimal mark, so `1,5` is 1.5 and `1,000` typed one key at a
  time is 1. In text that arrives whole, by paste, drop or a keyboard's clipboard, a lone comma is
  the decimal mark too, with one exception.
- Text that arrives whole as one to three digits, a comma and exactly three digits, such as
  `1,500`, is not entered (`isAmbiguousAmount`). It means fifteen hundred to one reader and one and
  a half to another, and either guess is wrong by a factor of a thousand. The dialog itself writes
  fifteen hundred as `1,500`. A line under the field gives both readings and asks for the amount
  to be typed.
- Commas are separators only where they can be nothing else: beside a point, as in `1,000.50`, or
  several of them in groups of three in text that arrives whole, as in `1,000,000`. A pasted
  `0,500` is 0.5, since grouping never starts with a 0 group.
- Text an input event names (`data`, or a drop's transfer) is read from where the event put it.
  Matched against the old amount, text put over a selected `1,000` would lose the comma the two
  share. A composing keyboard names its whole composition on every key, so it is read as typing.
  A fill that names no text is still matched against the old amount.
- A paste is read from the clipboard text and the selection it replaces (`applyAmountPaste`). Read
  as a plain edit, a paste of `1,5` over a selected `1,000` would lose its comma, because the two
  share the text `1,`.
- The amount is reviewed and sent without trailing zeros (`trimAmount`). An amount typed as `1,000`
  is reviewed as `1`, which cannot pass for a thousand the way `1.000` can.
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
- The amount is cleared after every send, whatever its outcome, so the next click cannot send it
  again. When the solver gives no answer, or a proxy answers for it with 502, 503 or 504, the send
  may have gone through. The form then says so, in place of the error, and asks the operator to
  wait a few minutes and check the balance. The wallet history is no check: a send is written to
  it only once the solver has its outcome.
- Closing the dialog clears the form and the error of a failed send. A send in flight keeps the
  dialog open, so its outcome is not carried into the next open. The balance refresh that follows
  a send is not waited for, so it cannot hold the dialog or the next send.

`POST /api/send` and its request are unchanged.

`TokenNetworkPicker`, which the BRIDGE top-up still uses, had no background when it opened as a
phone drawer. It now has the surface its desktop dialog has.
