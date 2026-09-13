# 2026-09-13 — Preserve enrichment rules during cleanup

Keep the cleanup local to receipt selection, token conversion, and migration fixtures. The next operation is still selected before checking its sender; placement still requires the correct execution boundary. Transfer grouping preserves the existing log ordering, exact count requirement, native-output nulls, and zero-valued partial-fill slots. Generated order types describe the same order tuple in both historical and current fill calldata.

Rejected: merging full-fill and partial-fill persistence into a generic handler, unifying optional placement lookup with the existing fill error policy, or changing the host cache. Those paths have different side effects and failure behavior, and none needs to change to remove the duplication found in this review. Lifecycle writes, replay policy, commitment validation, ABI compatibility, and historical block reads remain as implemented before this cleanup.
