import { useRef, useState } from "react"
import { bookCrossedAt } from "@/config/interpolated-curve"
import { api } from "../../api"
import { toPricePoints, type EditorPoint } from "../../components/curveModel"
import { CUSTOM_TOKEN } from "./marketModel"

interface CreateMarketDraft {
	token0: string
	token1: string
	customSymbol: string
	customAddresses: Record<string, string>
	verified: Record<string, string>
	maxOrderSize: string
	bidEnabled: boolean
	askEnabled: boolean
	bid: EditorPoint[]
	ask: EditorPoint[]
}

export function useCreateMarket(options: { symbols: string[]; onAdded: () => Promise<void> | void }) {
	const { symbols, onAdded } = options
	const [draft, setDraft] = useState<CreateMarketDraft>(() => ({
		token0: "USDC",
		token1: symbols.find((symbol) => symbol !== "USDC") ?? CUSTOM_TOKEN,
		customSymbol: "",
		customAddresses: {},
		verified: {},
		maxOrderSize: "",
		bidEnabled: true,
		askEnabled: true,
		bid: [{ amount: "1", value: "" }],
		ask: [{ amount: "1", value: "" }],
	}))
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()
	const busyRef = useRef(false)
	const patch = (changes: Partial<CreateMarketDraft>) => setDraft((current) => ({ ...current, ...changes }))

	const customSide = draft.token0 === CUSTOM_TOKEN || draft.token1 === CUSTOM_TOKEN
	const resolved0 = draft.token0 === CUSTOM_TOKEN ? draft.customSymbol.trim().toUpperCase() : draft.token0
	const resolved1 = draft.token1 === CUSTOM_TOKEN ? draft.customSymbol.trim().toUpperCase() : draft.token1
	const crossedAt =
		draft.bidEnabled && draft.askEnabled
			? (bookCrossedAt(toPricePoints(draft.bid), toPricePoints(draft.ask))?.amount ?? null)
			: null

	const verifyToken = async (chainKey: string) => {
		const address = draft.customAddresses[chainKey]?.trim()
		if (!address) return
		try {
			const result = await api.post<{ ok: boolean; symbol?: string; decimals?: number; error?: string }>(
				"/api/setup/validate-token",
				{ chain: chainKey, address },
			)
			patch({
				verified: {
					...draft.verified,
					[chainKey]: result.ok ? `✓ ${result.symbol} (${result.decimals} decimals)` : `✗ ${result.error}`,
				},
			})
		} catch (cause) {
			patch({
				verified: {
					...draft.verified,
					[chainKey]: `✗ ${cause instanceof Error ? cause.message : String(cause)}`,
				},
			})
		}
	}

	const submit = async () => {
		if (busyRef.current) return
		setError(undefined)
		if (resolved0 && resolved0 === resolved1) {
			setError("Choose two different assets")
			return
		}
		if (draft.token0 === CUSTOM_TOKEN && draft.token1 === CUSTOM_TOKEN) {
			setError("Add one custom token at a time")
			return
		}
		if (customSide && !draft.customSymbol.trim()) {
			setError("The custom token needs a symbol")
			return
		}
		const addresses = Object.fromEntries(
			Object.entries(draft.customAddresses)
				.map(([chain, address]) => [chain, address.trim()])
				.filter(([, address]) => address),
		)
		if (customSide && Object.keys(addresses).length === 0) {
			setError("The custom token needs an address on at least one chain")
			return
		}

		busyRef.current = true
		setBusy(true)
		try {
			const result = await api.post<{ applied: boolean; restartNeeded: boolean }>("/api/strategies", {
				token0: resolved0,
				token1: resolved1,
				maxOrderSize: draft.maxOrderSize,
				...(draft.bidEnabled ? { bidPriceCurve: toPricePoints(draft.bid) } : {}),
				...(draft.askEnabled ? { askPriceCurve: toPricePoints(draft.ask) } : {}),
				...(customSide ? { assets: { [draft.customSymbol.trim().toUpperCase()]: addresses } } : {}),
			})
			if (result.restartNeeded) setError("Saved to config — restart the filler to open the market")
			else await onAdded()
		} catch (cause) {
			setError(cause instanceof Error ? cause.message : String(cause))
		} finally {
			busyRef.current = false
			setBusy(false)
		}
	}

	return { draft, patch, busy, error, customSide, resolved0, resolved1, crossedAt, verifyToken, submit }
}
