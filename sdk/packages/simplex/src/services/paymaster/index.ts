import { erc20Abi } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildCirclePaymasterData } from "./provider/circle"
import { packPaymasterAndData } from "./types"
import type { PaymasterOptions, PaymasterDataResult } from "./types"

export type { PaymasterOptions, PaymasterDataResult } from "./types"

/**
 * Returns true if the chain has the Circle Paymaster configured.
 * Used by filler.ts to decide whether to skip EntryPoint deposits.
 */
export function hasPaymaster(chain: string, configService: FillerConfigService): boolean {
	return !!configService.getCirclePaymasterAddress(chain)
}

/**
 * Unified paymaster data builder.
 *
 * Selection:
 * 1. Circle Paymaster — when configured AND solver has ≥1 USDC balance
 * 2. None — returns "0x" (caller falls back to EntryPoint deposit)
 */
export async function buildPaymasterAndData(options: PaymasterOptions): Promise<PaymasterDataResult> {
	const { chain, solverAccount, publicClient, signer, configService, paymasterVerificationGasLimit } = options

	const circleAddr = configService.getCirclePaymasterAddress(chain)
	if (!circleAddr) {
		return { paymasterAndData: "0x" as HexString, type: "none" }
	}

	const usdcAddress = configService.getUsdcAsset(chain)
	const usdcDecimals = configService.getUsdcDecimals(chain)

	if (!usdcAddress) {
		return { paymasterAndData: "0x" as HexString, type: "none" }
	}

	const { sufficient } = await getUsdcBalanceStatus(publicClient, solverAccount, usdcAddress, usdcDecimals)
	if (!sufficient) {
		return { paymasterAndData: "0x" as HexString, type: "none" }
	}

	const pm = await buildCirclePaymasterData(
		publicClient,
		signer,
		solverAccount,
		circleAddr,
		chain,
		configService,
		paymasterVerificationGasLimit,
	)
	return {
		paymasterAndData: packPaymasterAndData(pm),
		type: "circle",
		address: circleAddr,
	}
}

// ── Helpers ──────────────────────────────────────────────────────────

/**
 * Reads `account`'s token balance and reports it against the 1-token minimum the paymaster
 * needs to sponsor a UserOp. Returns the raw balance and required amount too, so callers can
 * log a precise deficit rather than a bare boolean.
 */
export async function getUsdcBalanceStatus(
	publicClient: PaymasterOptions["publicClient"],
	account: HexString,
	tokenAddress: HexString,
	tokenDecimals: number,
): Promise<{ balance: bigint; required: bigint; sufficient: boolean }> {
	const balance = (await publicClient.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "balanceOf",
		args: [account],
	})) as bigint

	const required = 10n ** BigInt(tokenDecimals)
	return { balance, required, sufficient: balance >= required }
}
