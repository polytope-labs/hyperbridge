import { encodePacked, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"

// ── Shared paymaster result type ────────────────────────────────────

export interface PaymasterResult {
	paymaster: HexString
	paymasterData: HexString
	paymasterVerificationGasLimit: bigint
	paymasterPostOpGasLimit: bigint
}

// ── Unified orchestration types ─────────────────────────────────────

export interface PaymasterOptions {
	chain: string
	solverAccount: HexString
	publicClient: PublicClient
	walletClient: WalletClient
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> }
	configService: FillerConfigService
	/**
	 * Override for the Circle paymaster verification gas limit (default 200k).
	 * Ignored when the Simplex paymaster is selected — its limits are mode-specific
	 * ({@link VERIFICATION_GAS_LIMIT_PERMIT} / {@link VERIFICATION_GAS_LIMIT_APPROVE}).
	 */
	paymasterVerificationGasLimit?: bigint
	/**
	 * Skips EIP-2612 permit detection and uses approve mode for the Simplex paymaster.
	 * Delegation UserOps rely on fixed, measured gas limits; executing a permit during
	 * paymaster validation adds tens of thousands of verification gas and would
	 * invalidate them.
	 */
	forceApproveMode?: boolean
}

export interface PaymasterDataResult {
	/** Packed paymasterAndData bytes, or "0x" when no paymaster is available. */
	paymasterAndData: HexString
	/** Which paymaster was selected. */
	type: "circle" | "simplex" | "none"
	/** Paymaster contract address (undefined when type is "none"). */
	address?: HexString
	/** Token the paymaster will charge (undefined when type is "none"). */
	token?: HexString
	/** Why no paymaster was selected (set only when type is "none"), for caller logging. */
	reason?: string
}

// ── Authorization amount constants ──────────────────────────────────

/** Dollar amount to authorize (permit). Safe upper bound — unused gas is refunded. */
export const RECOMMENDED_AMOUNT_USD = 5n
/** When existing allowance drops below this, re-authorize. */
export const THRESHOLD_USD = 2n

// ── Gas limit constants ─────────────────────────────────────────────

/** Verification gas limit for Circle Paymaster (recommended by Circle docs). */
export const VERIFICATION_GAS_LIMIT_CIRCLE = 200_000n
/** Simplex paymaster verification gas when executing an EIP-2612 permit during validation. */
export const VERIFICATION_GAS_LIMIT_PERMIT = 250_000n
/** Simplex paymaster verification gas when relying on an existing approval. */
export const VERIFICATION_GAS_LIMIT_APPROVE = 150_000n
/** Post-operation gas limit. */
export const POST_OP_GAS_LIMIT = 100_000n

// ── Shared helpers ──────────────────────────────────────────────────

/**
 * For EntryPoint v0.8, the `paymasterAndData` field in PackedUserOperation
 * is encoded as:
 *   paymaster (20 bytes) || paymasterVerificationGasLimit (uint128, 16 bytes)
 *   || paymasterPostOpGasLimit (uint128, 16 bytes) || paymasterData (variable)
 */
export function packPaymasterAndData(pm: PaymasterResult): HexString {
	return encodePacked(
		["address", "uint128", "uint128", "bytes"],
		[pm.paymaster, pm.paymasterVerificationGasLimit, pm.paymasterPostOpGasLimit, pm.paymasterData],
	) as HexString
}
