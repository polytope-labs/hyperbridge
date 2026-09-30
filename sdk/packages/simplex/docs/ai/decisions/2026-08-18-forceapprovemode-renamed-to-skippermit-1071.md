# 2026-08-18 — `forceApproveMode` renamed to `skipPermit` (#1071)

Chosen: the delegation flow's flag now only skips EIP-2612 permit detection; PERMIT2 and APPROVE stay available. The flag exists because delegation ops pass fixed, measured account-side gas limits; the Simplex builder sets its own paymaster verification limit per mode (`VERIFICATION_GAS_LIMIT_PERMIT2` = 200k, measured at ~135k on Ethereum and BSC forks), so PERMIT2 does not disturb them. Keeping the old name would have forced BSC delegations to keep a paymaster allowance, i.e. two funded approvals per token instead of one.
