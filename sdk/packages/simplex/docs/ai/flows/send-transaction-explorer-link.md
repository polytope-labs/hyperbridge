# Send transaction explorer link

When a reviewed send is confirmed, `SendDialog` resolves the reviewed chain through the canonical
initialization catalog before it posts to `/api/send`. A confirmed response stores that explorer URL
alongside its transaction hash, and the success screen's "View transaction" link opens the chain's
`/tx/<hash>` page in a new tab. A send that is still pending shows the truncated hash in the form's
warning, linked the same way when it has a transaction hash. The captured URL stays paired with that
transaction whatever is selected in the form afterwards.
