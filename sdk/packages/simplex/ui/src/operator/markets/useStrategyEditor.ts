import { useRef, useState } from "react"
import { bookCrossedAt } from "@/config/interpolated-curve"
import { api } from "../../api"
import { fromPricePoints, toPricePoints, type EditorPoint } from "../../components/curveModel"
import type { AdminStrategyDto } from "../../types"

interface StrategyEditorDraft {
	bid: EditorPoint[]
	ask: EditorPoint[]
	maxOrderSize: string
	enableBid: boolean
	enableAsk: boolean
}

interface StrategyEditorStatus {
	busy: boolean
	message?: string
	error?: string
}

export function useStrategyEditor(options: {
	strategy: AdminStrategyDto
	onApplied: () => Promise<void> | void
	hasVaults: boolean
}) {
	const { strategy, onApplied, hasVaults } = options
	const [draft, setDraft] = useState<StrategyEditorDraft>(() => ({
		bid: fromPricePoints(strategy.bid),
		ask: fromPricePoints(strategy.ask),
		maxOrderSize: strategy.maxOrderSize ?? "",
		enableBid: false,
		enableAsk: false,
	}))
	const [status, setStatus] = useState<StrategyEditorStatus>({ busy: false })
	const mutationRef = useRef(false)
	const patch = (changes: Partial<StrategyEditorDraft>) => setDraft((current) => ({ ...current, ...changes }))

	const token0 = strategy.token0 || "first asset"
	const token1 = strategy.token1 || "second asset"
	const crossedAt =
		!strategy.sameToken &&
		!strategy.referenceOnly &&
		(strategy.bid || draft.enableBid) &&
		(strategy.ask || draft.enableAsk)
			? (bookCrossedAt(toPricePoints(draft.bid), toPricePoints(draft.ask))?.amount ?? null)
			: null
	const title = strategy.referenceOnly
		? `${token0} ↔ ${token1} reference price`
		: strategy.sameToken
			? `${strategy.exotic ?? token0} cross-chain transfers`
			: `${token0} ↔ ${token1}`
	const capValue = draft.maxOrderSize.trim()
	const capCleared = capValue === ""
	const capChanged = capValue !== (strategy.maxOrderSize ?? "")

	const executeRequest = async (operation: () => Promise<string | undefined>) => {
		if (mutationRef.current) return
		mutationRef.current = true
		setStatus({ busy: true })
		try {
			const message = await operation()
			setStatus({ busy: false, message })
		} catch (cause) {
			setStatus({ busy: false, error: cause instanceof Error ? cause.message : String(cause) })
		} finally {
			mutationRef.current = false
		}
	}

	const applyCurves = () =>
		executeRequest(async () => {
			const result = await api.put<{ persisted: boolean }>(`/api/strategies/${strategy.index}/curves`, {
				...(strategy.bid || draft.enableBid ? { bidPriceCurve: toPricePoints(draft.bid) } : {}),
				...(strategy.ask || draft.enableAsk ? { askPriceCurve: toPricePoints(draft.ask) } : {}),
			})
			patch({ enableBid: false, enableAsk: false })
			await onApplied()
			return result.persisted
				? "Applied & saved to config"
				: "Applied in memory — config file could not be written"
		})

	const applyMaxOrderSize = () =>
		executeRequest(async () => {
			const result = capCleared
				? await api.del<{ persisted: boolean; restartNeeded: boolean }>(
						`/api/strategies/${strategy.index}/max-order-size`,
					)
				: await api.put<{ persisted: boolean; restartNeeded: boolean }>(`/api/strategies/${strategy.index}`, {
						maxOrderSize: capValue,
					})
			await onApplied()
			if (result.restartNeeded) {
				return `Saved to config — restart the filler to apply${capCleared ? " the removal" : " the new cap"}`
			}
			const action = capCleared ? "Cap removed — market is uncapped" : "Cap applied"
			return result.persisted
				? `${action} & saved to config`
				: `${action} in memory — config file could not be written`
		})

	const removeMarket = () => {
		const vaultNote = hasVaults
			? " Funds already swept into vaults stay there and remain redeemable from the operations tab."
			: ""
		const confirmed = window.confirm(
			`Remove market ${strategy.exotic ?? `${token0}/${token1}`}? The filler stops bidding on it immediately and it is deleted from the config.${vaultNote}`,
		)
		if (!confirmed) return
		return executeRequest(async () => {
			await api.del(`/api/strategies/${strategy.index}`)
			await onApplied()
			return undefined
		})
	}

	return {
		draft,
		patch,
		status,
		token0,
		token1,
		title,
		crossedAt,
		capCleared,
		capChanged,
		applyCurves,
		applyMaxOrderSize,
		removeMarket,
	}
}
