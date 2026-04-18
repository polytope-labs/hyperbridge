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
	const { chain, solverAccount, publicClient, signer, configService } = options

	const circleAddr = configService.getCirclePaymasterAddress(chain)
	if (!circleAddr) {
		return { paymasterAndData: "0x" as HexString, type: "none" }
	}

	const usdcAddress = configService.getUsdcAsset(chain)
	const usdcDecimals = configService.getUsdcDecimals(chain)

	if (!usdcAddress || !(await hasSufficientBalance(publicClient, solverAccount, usdcAddress, usdcDecimals))) {
		return { paymasterAndData: "0x" as HexString, type: "none" }
	}

	const pm = await buildCirclePaymasterData(publicClient, signer, solverAccount, circleAddr, chain, configService)
	return {
		paymasterAndData: packPaymasterAndData(pm),
		type: "circle",
		address: circleAddr,
	}
}

// ── Helpers ──────────────────────────────────────────────────────────

async function hasSufficientBalance(
	publicClient: PaymasterOptions["publicClient"],
	account: HexString,
	tokenAddress: HexString,
	tokenDecimals: number,
): Promise<boolean> {
	const balance = (await publicClient.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "balanceOf",
		args: [account],
	})) as bigint

	const minBalance = 10n ** BigInt(tokenDecimals)
	return balance >= minBalance
}
