# 2026-08-13 — Gateway daily volume reuses `DailyVolumeUSD` instead of a new indexed entity (#1085)

Chosen: record gateway-level filled volume with base ID `IntentGatewayV3.FILLED` through the existing `VolumeService.updateVolume`, landing in the same `DailyVolumeUSD` / `CumulativeVolumeUSD` entities as the per-filler and per-user records.

Alternative considered: a new `DailyIntentGatewayVolumeUSD` entity keyed `chain-volumeType-date`, written inside `IntentGatewayV3Service.recordOrderVolume` next to `CumulativeIntentGatewayVolumeUSD`. That would give typed, indexed `chain` and `date` columns (the `DailyVolumeUSD` ID is an opaque string with no filterable columns), following the pattern `BandwidthAppDailyConsumption` uses.

Why the reuse won: the issue explicitly asked for the entry to appear in the existing `dailyVolumeUSDs` query alongside the FILLER and USER rows, it needs no schema change or codegen, and it exactly parallels the existing filler-independent `IntentGatewayV3.USER` record. If consumers later need to filter or sort daily gateway volume server-side, revisit the dedicated-entity alternative.
