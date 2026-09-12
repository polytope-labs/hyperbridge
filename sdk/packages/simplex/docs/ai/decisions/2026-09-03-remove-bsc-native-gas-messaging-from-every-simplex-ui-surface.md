# 2026-09-03 — Remove BSC native-gas messaging from every Simplex UI surface

Chosen: remove the shared chain-note field and its warning renderers from setup and operator chain
screens, and make the setup review use paymaster coverage without a BNB exception. The CLI funding
help remains unchanged because this request targets the Simplex UI.

Alternative rejected: hiding only the onboarding warning would leave the same obsolete message in
operator settings or review, which is inconsistent now that BSC has a paymaster.
