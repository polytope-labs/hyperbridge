# Send transaction explorer link

Before posting to `/api/send`, `SendCard` resolves the selected chain through the canonical
initialization catalog. A successful response stores that explorer URL alongside its transaction hash,
and the success row exposes the truncated hash plus external-link icon as one anchor that opens the
chain's `/tx/<hash>` page in a new tab. The captured URL remains paired with the completed transaction
even if the operator subsequently changes the form's network selection.
