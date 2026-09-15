import { useRef, useState } from "react"
import { api } from "../../api"
import type { AdminStrategyDto } from "../../types"

interface StrategyStatus {
	busy: boolean
	error?: string
}

/**
 * Actions available on a live market.
 *
 * Prices are not among them: what the filler pays comes from the limit orders
 * the operator posts, so a market itself is only ever opened or closed.
 */
export function useStrategyEditor(options: {
	strategy: AdminStrategyDto
	onApplied: () => Promise<void> | void
	hasVaults: boolean
}) {
	const { strategy, onApplied, hasVaults } = options
	const [status, setStatus] = useState<StrategyStatus>({ busy: false })
	const mutationRef = useRef(false)

	const token0 = strategy.token0 || "first asset"
	const token1 = strategy.token1 || "second asset"
	const title = strategy.exotic ?? `${token0}/${token1}`

	const removeMarket = async () => {
		const vaultNote = hasVaults
			? " Funds already swept into vaults stay there and remain redeemable from the operations tab."
			: ""
		const confirmed = window.confirm(
			`Remove market ${title}? The filler stops bidding on it immediately and it is deleted from the config.${vaultNote}`,
		)
		if (!confirmed || mutationRef.current) return
		mutationRef.current = true
		setStatus({ busy: true })
		try {
			await api.del(`/api/strategies/${strategy.index}`)
			await onApplied()
			setStatus({ busy: false })
		} catch (cause) {
			setStatus({ busy: false, error: cause instanceof Error ? cause.message : String(cause) })
		} finally {
			mutationRef.current = false
		}
	}

	return { status, token0, token1, title, removeMarket }
}
