# 2026-09-04 — Bind send explorer links to completed transaction state

Chosen: resolve the canonical block explorer when a send starts and store that URL with the successful
transaction result. Render the truncated hash and external-link icon inside one native anchor that
opens in a new tab. Networks without configured explorer metadata retain a non-interactive hash.

Alternative rejected: resolving the URL from the live network selector during render could send an
already-completed transaction hash to the wrong chain after the operator changes the form selection.
