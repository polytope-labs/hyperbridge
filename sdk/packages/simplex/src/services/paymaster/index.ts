import { erc20Abi } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildCirclePaymasterData } from "./provider/circle"
import { buildPaymasterData as buildSimplexPaymasterData } from "./provider/simplex"
import { packPaymasterAndData } from "./types"
import type { PaymasterOptions, PaymasterDataResult } from "./types"

export type { PaymasterOptions, PaymasterDataResult } from "./types"

/**
 * Returns true if the chain has any paymaster (Circle or Simplex) configured.
 * Used by filler.ts to decide whether to skip EntryPoint deposits.
 */
export function hasPaymaster(chain: string, configService: FillerConfigService): boolean {
	return !!(configService.getCirclePaymasterAddress(chain) || configService.getSimplexPaymasterAddress(chain))
}

/**
 * Unified paymaster data builder.
 *
 * Selection priority:
 * 1. Circle Paymaster — when configured AND solver has ≥1 USDC balance
 * 2. Simplex Paymaster — when configured (supports USDC/USDT, permit or approve)
 * 3. None — returns "0x" (caller falls back to EntryPoint deposit)
 */
export async function buildPaymasterAndData(options: PaymasterOptions): Promise<PaymasterDataResult> {
	const { chain, solverAccount, publicClient, walletClient, signer, configService, forceApproveMode } = options

	const circleAddr = configService.getCirclePaymasterAddress(chain)
	const simplexAddr = configService.getSimplexPaymasterAddress(chain)

	// 1. Try Circle Paymaster (USDC only)
	if (circleAddr) {
		const usdcAddress = configService.getUsdcAsset(chain)
		const usdcDecimals = configService.getUsdcDecimals(chain)

		if (usdcAddress && (await hasSufficientBalance(publicClient, solverAccount, usdcAddress, usdcDecimals))) {
			const pm = await buildCirclePaymasterData(
				publicClient,
				signer,
				solverAccount,
				circleAddr,
				chain,
				configService,
			)
			return {
				paymasterAndData: packPaymasterAndData(pm),
				type: "circle",
				address: circleAddr,
			}
		}
	}

	// 2. Try Simplex Paymaster (USDC/USDT, permit or approve)
	if (simplexAddr) {
		const pm = await buildSimplexPaymasterData(
			publicClient,
			walletClient,
			signer,
			solverAccount,
			simplexAddr,
			chain,
			configService,
			forceApproveMode,
		)
		return {
			paymasterAndData: packPaymasterAndData(pm),
			type: "simplex",
			address: simplexAddr,
		}
	}

	// 3. No paymaster available
	return {
		paymasterAndData: "0x" as HexString,
		type: "none",
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
