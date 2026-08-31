import { encodePacked, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet/types"

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
	signer: Pick<Signer, "signTypedData">
	configService: FillerConfigService
	/**
	 * Override for the Circle paymaster verification gas limit (default 200k).
	 * Only honored when the paymaster allowance is already in place — a permit
	 * executed during validation needs the full default. Ignored when the Simplex
	 * paymaster is selected — its limits are mode-specific
	 * ({@link VERIFICATION_GAS_LIMIT_PERMIT} / {@link VERIFICATION_GAS_LIMIT_APPROVE}).
	 */
	paymasterVerificationGasLimit?: bigint
	/**
	 * Skips EIP-2612 permit detection for the Simplex paymaster (PERMIT2 and APPROVE
	 * modes stay available). Delegation UserOps rely on fixed, measured gas limits;
	 * executing a permit during paymaster validation adds tens of thousands of
	 * verification gas and would invalidate them.
	 */
	skipPermit?: boolean
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
/**
 * Simplex paymaster verification gas when prefunding through Permit2. Measured at
 * ~135k on Ethereum and BSC forks (EOA and delegated senders).
 */
export const VERIFICATION_GAS_LIMIT_PERMIT2 = 200_000n
/**
 * Permit2 signatures use unordered nonces, so an unspent one (a losing bid) stays
 * valid until its deadline; keep that window short but well past bid-to-execution
 * latency and clock skew.
 */
export const PERMIT2_DEADLINE_SECONDS = 3600n
/** Post-operation gas limit for the Circle Paymaster (its own contract, its own postOp). */
export const POST_OP_GAS_LIMIT_CIRCLE = 100_000n
/**
 * Post-operation gas limit for the Simplex paymaster. The contract accepts the band
 * [MIN_POST_OP_GAS_LIMIT 30k, MAX_POST_OP_GAS_LIMIT 100k] — the ceiling stays at 100k so
 * an in-place proxy upgrade never rejects clients still sending the old limit. The
 * EntryPoint penalises the unused part of this limit without billing the user for it, and
 * waives the penalty entirely while the limit stays within 40k of actual usage — so 40k is
 * the largest penalty-free value, whatever postOp costs. Measured refunds are ~8-12k gas,
 * and both USDC and USDT execute postOp at 30k.
 */
export const POST_OP_GAS_LIMIT_SIMPLEX = 40_000n

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
