# 2026-09-03 — Token balance cards emphasize usable liquidity without hiding ownership

Chosen: reuse the existing `TokenIcon` asset mapping in each dashboard balance card and give total,
wallet, vault, and available-to-fill values distinct visual roles. Token-specific accents aid
scanning, while partial and unavailable cards switch to the warning treatment. Native gas belongs in
the network header because it funds the chain rather than representing fill inventory.

Alternative rejected: using token tickers alone makes a multi-asset dashboard harder to scan;
presenting wallet, vault, and available values with equal prominence repeats the ambiguity that led
operators to question whether vault funds were included.
