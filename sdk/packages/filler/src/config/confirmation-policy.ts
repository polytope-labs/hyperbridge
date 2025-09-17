export class ConfirmationPolicy {
	// Maps chainId -> policy configuration
	private policies: Map<
		number,
		{
			minAmount: number
			maxAmount: number
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
				minAmount: parseFloat(config.minAmount),
				maxAmount: parseFloat(config.maxAmount),
				minConfirmations: config.minConfirmations,
				maxConfirmations: config.maxConfirmations,
			})
		})
	}

	getConfirmationBlocks(chainId: number, amountUsd: number): number {
		const chainPolicy = this.policies.get(chainId)
		if (!chainPolicy) throw new Error(`No confirmation policy found for chainId ${chainId}`)

		if (amountUsd <= chainPolicy.minAmount) {
			return chainPolicy.minConfirmations
		}

		if (amountUsd >= chainPolicy.maxAmount) {
			return chainPolicy.maxConfirmations
		}

		const amountRange = chainPolicy.maxAmount - chainPolicy.minAmount
		const confirmationRange = chainPolicy.maxConfirmations - chainPolicy.minConfirmations
		const amountPosition = amountUsd - chainPolicy.minAmount

		const confirmationPosition = (amountPosition * confirmationRange) / amountRange

		return chainPolicy.minConfirmations + Math.round(confirmationPosition)
	}
}
