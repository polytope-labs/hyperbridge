export class ConfirmationPolicy {
	// Maps chainId -> policy configuration
	private policies: Map<
		number,
		{
			minAmount: bigint
			maxAmount: bigint
			minConfirmations: number
			maxConfirmations: number
		}
	>

	constructor(
		policyConfig: Record<
			string,
			{
				minAmount: string
				maxAmount: string
				minConfirmations: number
				maxConfirmations: number
			}
		>,
	) {
		this.policies = new Map()

		Object.entries(policyConfig).forEach(([chainId, config]) => {
			this.policies.set(Number(chainId), {
				minAmount: BigInt(config.minAmount),
				maxAmount: BigInt(config.maxAmount),
				minConfirmations: config.minConfirmations,
				maxConfirmations: config.maxConfirmations,
			})
		})
	}

	getConfirmationBlocks(chainId: number, amount: bigint): number {
		const chainPolicy = this.policies.get(chainId)
		if (!chainPolicy) return this.getDefaultConfirmations(chainId)

		if (amount <= chainPolicy.minAmount) {
			return chainPolicy.minConfirmations
		}

		if (amount >= chainPolicy.maxAmount) {
			return chainPolicy.maxConfirmations
		}

		const amountRange = chainPolicy.maxAmount - chainPolicy.minAmount
		const confirmationRange = BigInt(chainPolicy.maxConfirmations - chainPolicy.minConfirmations)
		const amountPosition = amount - chainPolicy.minAmount

		const confirmationPosition = (amountPosition * confirmationRange) / amountRange

		return chainPolicy.minConfirmations + Number(confirmationPosition)
	}

	private getDefaultConfirmations(chainId: number): number {
		// Default confirmation blocks based on chain
		const defaults: Record<number, number> = {
			97: 1, // BSC Testnet
			10200: 1, // Gnosis Chiado
		}

		return defaults[chainId] || 1
	}
}
