import { encodePacked, maxUint256, getContract, erc20Abi, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { EIP2612_ABI } from "@/config/abis/EIP2612"

// ── Constants ────────────────────────────────────────────────────────

/**
 * Recommended gas limits from Circle's documentation.
 * These are safe upper bounds — actual usage is typically lower.
 * Unused gas is not charged.
 */
export const PAYMASTER_VERIFICATION_GAS_LIMIT = 200_000n
export const PAYMASTER_POST_OP_GAS_LIMIT = 100_000n

// ── Types ────────────────────────────────────────────────────────────

export interface PaymasterResult {
	/** Circle Paymaster contract address */
	paymaster: HexString
	/** ABI-packed paymaster data (mode + token + permitAmount + permitSig) */
	paymasterData: HexString
	/** Gas limit for paymaster verification phase */
	paymasterVerificationGasLimit: bigint
	/** Gas limit for paymaster postOp phase */
	paymasterPostOpGasLimit: bigint
}

export interface CirclePaymasterConfig {
	/** USDC contract address on the destination chain (from config service) */
	usdcAddress: HexString
	/** Circle Paymaster contract address on the destination chain (from config service) */
	paymasterAddress: HexString
	/** Chain ID of the destination chain */
	chainId: number
	/** USDC decimal count on this chain (from config service) */
	usdcDecimals: number
	/** Max USDC the paymaster may pull (human units $10). Computed from decimals if omitted. */
	permitAmount?: bigint
}

// ── Core integration ─────────────────────────────────────────────────

/**
 * Computes the default permit amount ($5 worth of USDC) for the given decimals.
 * This is a max-spend cap per UserOp, not the actual charge.
 * The paymaster charges actual gas cost and refunds the rest.
 */
function defaultPermitAmount(usdcDecimals: number): bigint {
	return 5n * 10n ** BigInt(usdcDecimals)
}

/**
 * Builds the paymaster fields for a PackedUserOperation using Circle Paymaster v0.8.
 *
 * Flow:
 * 1. Signs an EIP-2612 permit granting the Circle Paymaster an allowance
 *    to pull up to `permitAmount` USDC from the solver's smart account.
 * 2. Encodes the paymaster data as:
 *    `abi.encodePacked(uint8(0), address(usdc), uint256(permitAmount), bytes(permitSig))`
 * 3. Returns the paymaster address, encoded data, and gas limits.
 *
 * The permit uses `deadline = maxUint256` because the paymaster contract
 * cannot access `block.timestamp` due to ERC-4337 opcode restrictions.
 *
 * Use `packPaymasterAndData(result)` to produce the final `paymasterAndData`
 * bytes for the PackedUserOperation.
 *
 * @param client - Public client for reading USDC contract state (nonces, name, version).
 * @param signer - The solver's signer, capable of EIP-712 signTypedData.
 * @param solverAccount - The solver's smart account address (the "owner" in permit terms).
 * @param config - Chain-specific addresses, decimals, chain ID, and optional permit amount.
 */
export async function buildCirclePaymasterData(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	config: CirclePaymasterConfig,
): Promise<PaymasterResult> {
	const {
		usdcAddress,
		paymasterAddress,
		chainId,
		usdcDecimals,
		permitAmount = defaultPermitAmount(usdcDecimals),
	} = config

	// The paymaster contract skips the permit section when paymasterData is
	// shorter than PAYMASTER_PERMIT_SIGNATURE_OFFSET and goes straight to transferFrom.
	const existingAllowance = (await client.readContract({
		address: usdcAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	if (existingAllowance >= permitAmount) {
		// No permit data needed — paymaster will use existing allowance
		const paymasterData = encodePacked(["uint8"], [0]) as HexString
		return {
			paymaster: paymasterAddress,
			paymasterData,
			paymasterVerificationGasLimit: PAYMASTER_VERIFICATION_GAS_LIMIT,
			paymasterPostOpGasLimit: PAYMASTER_POST_OP_GAS_LIMIT,
		}
	}

	const permitSignature = await signUsdcPermit(
		client,
		signer,
		solverAccount,
		paymasterAddress,
		usdcAddress,
		permitAmount,
		chainId,
	)

	// Encode paymasterData: mode(0) + token + permitAmount + permitSignature
	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "bytes"],
		[0, usdcAddress, permitAmount, permitSignature],
	) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: PAYMASTER_VERIFICATION_GAS_LIMIT,
		paymasterPostOpGasLimit: PAYMASTER_POST_OP_GAS_LIMIT,
	}
}

// ── EIP-2612 Permit signing ──────────────────────────────────────────

async function signUsdcPermit(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	owner: HexString,
	spender: HexString,
	usdcAddress: HexString,
	value: bigint,
	chainId: number,
): Promise<HexString> {
	const token = getContract({
		client,
		address: usdcAddress,
		abi: EIP2612_ABI,
	})

	const [name, version, nonce] = await Promise.all([
		token.read.name(),
		token.read.version(),
		token.read.nonces([owner]),
	])

	const typedData = {
		types: {
			EIP712Domain: [
				{ name: "name", type: "string" },
				{ name: "version", type: "string" },
				{ name: "chainId", type: "uint256" },
				{ name: "verifyingContract", type: "address" },
			],
			Permit: [
				{ name: "owner", type: "address" },
				{ name: "spender", type: "address" },
				{ name: "value", type: "uint256" },
				{ name: "nonce", type: "uint256" },
				{ name: "deadline", type: "uint256" },
			],
		},
		primaryType: "Permit" as const,
		domain: {
			name,
			version,
			chainId,
			verifyingContract: usdcAddress,
		},
		message: {
			owner,
			spender,
			value,
			nonce,
			// maxUint256: paymaster can't read block.timestamp (ERC-4337 opcode restriction)
			deadline: maxUint256,
		},
	}

	return signer.signTypedData(typedData, chainId)
}

// ── Helper: pack paymaster fields into paymasterAndData ──────────────

/**
 * For EntryPoint v0.8, the `paymasterAndData` field in PackedUserOperation
 * is encoded as:
 *   paymaster (20 bytes) || paymasterVerificationGasLimit (uint128, 16 bytes)
 *   || paymasterPostOpGasLimit (uint128, 16 bytes) || paymasterData (variable)
 *
 * Use this helper to produce the final packed bytes from a PaymasterResult,
 * then pass the result as `paymasterAndData` in `SubmitBidOptions`.
 */
export function packPaymasterAndData(pm: PaymasterResult): HexString {
	return encodePacked(
		["address", "uint128", "uint128", "bytes"],
		[pm.paymaster, pm.paymasterVerificationGasLimit, pm.paymasterPostOpGasLimit, pm.paymasterData],
	) as HexString
}
