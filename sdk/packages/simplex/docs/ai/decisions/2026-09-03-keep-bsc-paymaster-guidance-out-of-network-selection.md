# 2026-09-03 — Keep BSC paymaster guidance out of network selection

Chosen: remove the BSC and BSC Chapel `note` values from the shared initialization catalog. The web
onboarding chain step renders those values as a native-gas warning, and that stale paymaster caveat is
not needed while selecting networks. Keep the separate review and CLI funding guidance because this
change is presentation-only.

Alternative rejected: changing paymaster selection or native-gas funding logic would broaden a small
onboarding copy fix into runtime behavior.
