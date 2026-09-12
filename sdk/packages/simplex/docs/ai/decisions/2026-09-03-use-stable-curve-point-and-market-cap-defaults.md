# 2026-09-03 — Use stable curve-point and market-cap defaults

Chosen: use `1` as the amount for newly inserted curve rows and `50000` as the default cap for new
markets in both the web and CLI setup paths. Existing configured caps remain unchanged, and optional
caps continue to be represented by an empty field.

Alternative rejected: changing existing config values would silently alter an operator's risk limit;
the request concerns defaults for newly created markets and rows.
