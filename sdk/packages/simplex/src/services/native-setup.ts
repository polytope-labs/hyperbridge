import { concat, formatEther, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { BATCHED_APPROVE_GAS, DELEGATION_TX_GAS_FLOOR } from "@/services/DelegationService"
import { APPROVE_TX_GAS, resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"

/** Native gas a chain needs before its paymaster can sponsor the wallet's transactions. */
export interface NativeSetupNeed {
	/** Native to send, in whole units: twice the current cost, so a gas-price rise does not leave it short. */
	required: number
	/** The fee token whose one-time Permit2 approval has to be paid in native. */
	feeToken: "USDC" | "USDT"
}

/**
 * What a chain still needs in native gas before the paymaster can take over, or null when it needs none.
 *
 * Every sponsored transaction pays its gas through Permit2, which needs a one-time approval of the fee
 * token. A token with an EIP-2612 permit pays for that approval itself. One without, like USDC and USDT
 * on BNB Chain, needs an ordinary transaction paid in native, folded into the delegation when the wallet
 * has not delegated yet. Null when the wallet already holds enough native, the approval is in place, or
 * it holds no fee token yet.
 */
export async function nativeSetupNeed(
	client: PublicClient,
	configService: FillerConfigService,
	wallet: HexString,
	chain: string,
	nativeBalance: bigint,
): Promise<NativeSetupNeed | null> {
	const paymaster = configService.getSimplexPaymasterAddress(chain)
	if (!paymaster) return null

	// The most setup can cost is a delegation with the approve folded in. Holding that is enough.
	const gasPrice = await client.getGasPrice()
	if (nativeBalance >= (DELEGATION_TX_GAS_FLOOR + BATCHED_APPROVE_GAS) * gasPrice) return null

	const pending = await resolvePendingPermit2Approval(client, wallet, paymaster, chain, configService)
	if (!pending || pending.permitCapable) return null

	const solverAccount = configService.getSolverAccountContractAddress(chain)
	const code = await client.getCode({ address: wallet })
	const delegated = !!solverAccount && code?.toLowerCase() === concat(["0xef0100", solverAccount]).toLowerCase()
	const cost = (delegated ? APPROVE_TX_GAS : DELEGATION_TX_GAS_FLOOR + BATCHED_APPROVE_GAS) * gasPrice
	if (nativeBalance >= cost) return null

	const usdc = configService.getUsdcAsset(chain)
	return {
		required: Number(formatEther(cost * 2n)),
		feeToken: pending.token.toLowerCase() === usdc.toLowerCase() ? "USDC" : "USDT",
	}
}
