import { encodePacked, erc20Abi, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { RECOMMENDED_AMOUNT_USD, THRESHOLD_USD, VERIFICATION_GAS_LIMIT_CIRCLE, POST_OP_GAS_LIMIT, type PaymasterResult } from "../types"
import { signEip2612Permit } from "../permit"

/**
 * Builds the paymaster fields for a PackedUserOperation using Circle Paymaster v0.8.
 *
 * Flow:
 * 1. Signs an EIP-2612 permit granting the Circle Paymaster an allowance
 *    to pull up to the recommended amount of USDC from the solver's smart account.
 * 2. Encodes the paymaster data as:
 *    `abi.encodePacked(uint8(0), address(usdc), uint256(permitAmount), bytes(permitSig))`
 * 3. Returns the paymaster address, encoded data, and gas limits.
 *
 * The permit uses `deadline = maxUint256` because the paymaster contract
 * cannot access `block.timestamp` due to ERC-4337 opcode restrictions.
 */
export async function buildCirclePaymasterData(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
	/**
	 * Override for the paymaster verification gas limit. Defaults to the
	 * Circle-recommended {@link VERIFICATION_GAS_LIMIT_CIRCLE}. A caller can pass a
	 * tighter value for a known, cheap op (e.g. a re-delegation) so the bundler's
	 * verification-gas-limit efficiency policy accepts it — see the delegation path.
	 */
	paymasterVerificationGasLimit: bigint = VERIFICATION_GAS_LIMIT_CIRCLE,
): Promise<PaymasterResult> {
	const usdcAddress = configService.getUsdcAsset(chain)
	const usdcDecimals = configService.getUsdcDecimals(chain)
	const chainId = configService.getChainId(chain)

	const threshold = THRESHOLD_USD * 10n ** BigInt(usdcDecimals)
	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(usdcDecimals)

	// The paymaster contract skips the permit section when paymasterData is
	// shorter than PAYMASTER_PERMIT_SIGNATURE_OFFSET and goes straight to transferFrom.
	const existingAllowance = (await client.readContract({
		address: usdcAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	if (existingAllowance >= threshold) {
		// No permit data needed — paymaster will use existing allowance
		const paymasterData = encodePacked(["uint8"], [0]) as HexString
		return {
			paymaster: paymasterAddress,
			paymasterData,
			paymasterVerificationGasLimit,
			paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
		}
	}

	const permitSignature = await signEip2612Permit(
		client,
		signer,
		solverAccount,
		paymasterAddress,
		usdcAddress,
		recommended,
		chainId,
	)

	// Encode paymasterData: mode(0) + token + permitAmount + permitSignature
	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "bytes"],
		[0, usdcAddress, recommended, permitSignature],
	) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_CIRCLE,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
	}
}
