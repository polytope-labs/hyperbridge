const ALL_CHAINS_DELEGATION_FAILURE =
	/^EIP-7702 delegation failed on all chains:\s*(.+?)\.\s*Shutting down for restart\.?$/i

function joinLabels(labels: string[]): string {
	if (labels.length <= 1) return labels[0] ?? "the enabled network"
	if (labels.length === 2) return `${labels[0]} and ${labels[1]}`
	return `${labels.slice(0, -1).join(", ")}, and ${labels.at(-1)}`
}

/** Convert internal boot failures into instructions an operator can act on. */
export function formatSetupStartError(message: string, chainLabels: ReadonlyMap<number, string>): string {
	const delegationFailure = message.match(ALL_CHAINS_DELEGATION_FAILURE)
	if (!delegationFailure) return message

	const labels = Array.from(delegationFailure[1].matchAll(/\bEVM-(\d+)\b/g), ([, chainId]) => {
		const numericId = Number.parseInt(chainId, 10)
		return chainLabels.get(numericId) ?? `EVM chain ${numericId}`
	})
	const networks = joinLabels(Array.from(new Set(labels)))

	return `Simplex could not activate the filler wallet on ${networks}. Make sure it holds at least 1 USDC or USDT on ${networks}, confirm the RPC and bundler endpoints are reachable, then retry. If you use Docker, restart Simplex with the latest image.`
}
