import { useState } from "react"
import { api } from "../../api"
import { CurveEditor, fromPricePoints, type EditorPoint } from "../../components/CurveEditor"
import { PillTabs } from "../../components/PillTabs"
import { unanchoredToken0Symbols } from "../../lib/anchor"
import {
	enabledChains,
	isSameTokenDraft,
	newCrossAssetDraft,
	newReferenceDraft,
	normSymbol,
	patchAt,
	removeAt,
	type ChainDraft,
	type PairDraft,
	type WizardState,
} from "../state"
import type { StepProps } from "../Wizard"

const PRICING_TABS = [
	{ value: "curves", label: "Static bid/ask curves" },
	{ value: "uniswapV4", label: "Uniswap V4 positions" },
] as const

/** A curve editor the user has not touched yet — safe to overwrite with prefills. */
function untouched(points: EditorPoint[]): boolean {
	return points.every((p) => !p.value.trim())
}

/** Sensible starting curves when a market's shape becomes known from its symbols. */
function prefillCurves(draft: PairDraft, usdStables: string[], sameAssetAsk: EditorPoint[]): Partial<PairDraft> {
	if (isSameTokenDraft(draft)) {
		return untouched(draft.ask) ? { ask: sameAssetAsk.map((p) => ({ ...p })) } : {}
	}
	const bothStable = usdStables.includes(normSymbol(draft.token0)) && usdStables.includes(normSymbol(draft.token1))
	if (bothStable) {
		const patch: Partial<PairDraft> = {}
		// Near-par two-sided defaults: buy at a premium, sell at a discount.
		if (untouched(draft.bid)) patch.bid = [{ amount: "0", value: "1.001" }]
		if (untouched(draft.ask)) patch.ask = [{ amount: "0", value: "0.999" }]
		return patch
	}
	return {}
}

export function StepStrategies({ state, setState, defaults }: StepProps) {
	const chains = enabledChains(state)

	// Registry symbols deployed on at least one enabled chain, USD stables first.
	const availableSymbols = [
		...new Set(chains.flatMap((chain) => (defaults.knownTokens[chain.meta.stateMachineId] ?? []).map((t) => t.symbol))),
	].sort((a, b) => Number(defaults.usdStables.includes(b)) - Number(defaults.usdStables.includes(a)))

	const patchPair = (index: number, patch: Partial<PairDraft>) =>
		setState((s) => ({ ...s, pairs: patchAt(s.pairs, index, patch) }))

	const sameAssetRows = state.pairs
		.map((pair, index) => ({ pair, index }))
		.filter(({ pair }) => pair.kind === "sameAsset")
	const marketRows = state.pairs
		.map((pair, index) => ({ pair, index }))
		.filter(({ pair }) => pair.kind === "crossAsset")

	const enabled = state.pairs.filter((p) => p.enabled)
	// Reverse orientations are the same market seen from the other side.
	const duplicateKeys = new Set<string>()
	{
		const seen = new Set<string>()
		for (const pair of enabled) {
			const key = `${normSymbol(pair.token0)}/${normSymbol(pair.token1)}`
			const reverse = `${normSymbol(pair.token1)}/${normSymbol(pair.token0)}`
			if (seen.has(key) || seen.has(reverse)) duplicateKeys.add(key)
			seen.add(key)
		}
	}
	const unanchored = unanchoredToken0Symbols(
		enabled
			.filter((p) => !(p.kind === "sameAsset" || isSameTokenDraft(p)))
			.map((p) => ({
				token0: p.token0,
				token1: p.token1,
				hasCurve:
					state.fxPricing === "curves" &&
					((p.bidEnabled && p.bid.some((pt) => pt.value.trim())) ||
						((p.askEnabled || Boolean(p.referenceOnly)) && p.ask.some((pt) => pt.value.trim()))),
			})),
		defaults.usdStables,
	)

	return (
		<div>
			<div className="card">
				<h2>Cross-chain transfer markets</h2>
				<p className="hint">
					Fill same-asset transfers between chains (e.g. USDC on Base for USDC on Arbitrum). The ask price is
					the fraction of the input you pay back out — keep it below 1; the gap to 1 is your spread on every
					fill. Same-chain same-asset swaps are pay-more-get-less, so the filler never fills them: these
					markets are cross-chain only.
				</p>
				{sameAssetRows.map(({ pair, index }) => (
					<div key={pair.token0} style={{ marginBottom: "0.8rem" }}>
						<label className="row">
							<input
								type="checkbox"
								checked={pair.enabled}
								onChange={(e) => patchPair(index, { enabled: e.target.checked })}
							/>
							<strong>
								{pair.token0} → {pair.token1}
							</strong>
						</label>
						{pair.enabled && (
							<div style={{ marginLeft: "1.4rem" }}>
								<label className="field" style={{ maxWidth: "16rem" }}>
									<span>Max {pair.token0} per order (larger orders partially fill)</span>
									<input
										type="text"
										value={pair.maxOrderSize}
										onChange={(e) => patchPair(index, { maxOrderSize: e.target.value })}
									/>
								</label>
								<CurveEditor
									points={pair.ask}
									onChange={(points) => patchPair(index, { ask: points })}
									amountLabel={`Order size (${pair.token0})`}
									valueLabel="Price (below 1)"
								/>
								{pair.ask.some((p) => p.value.trim() && Number(p.value) >= 1) && (
									<p className="error">Prices at or above 1 give back at least what was received — no spread.</p>
								)}
							</div>
						)}
					</div>
				))}
			</div>

			<div className="card">
				<h2>More markets — any asset pair</h2>
				<p className="hint">
					Each market prices its base token (token1) in units of its quote (token0) with your own curves:
					stablecoin to exotic (USDC/CNGN), stable to stable (USDC/USDT — fills same-chain and cross-chain
					swaps in both directions), or exotic to exotic (ZARP/CNGN). Assets are picked by symbol — addresses
					come from the built-in registry; custom tokens get an [assets] entry.
				</p>

				{marketRows.length > 0 && (
					<div>
						<p className="hint">Price source for these markets:</p>
						<PillTabs
							options={PRICING_TABS}
							value={state.fxPricing}
							onChange={(fxPricing) => setState((s) => ({ ...s, fxPricing }))}
						/>
					</div>
				)}

				{marketRows.map(({ pair, index }) => (
					<MarketRow
						key={index}
						pair={pair}
						symbols={availableSymbols}
						usdStables={defaults.usdStables}
						chains={chains}
						pricing={state.fxPricing}
						duplicate={duplicateKeys.has(`${normSymbol(pair.token0)}/${normSymbol(pair.token1)}`)}
						customAssets={state.customAssets}
						onPatch={(patch) => {
							const next = { ...pair, ...patch }
							patchPair(index, {
								...patch,
								...prefillCurves(next, defaults.usdStables, fromPricePoints(defaults.sameAssetAskCurve)),
							})
						}}
						onRenameAsset={(from, to) =>
							setState((s) => {
								const customAssets = { ...s.customAssets }
								if (from && customAssets[from] && from !== to) {
									customAssets[to] = customAssets[from]
									delete customAssets[from]
								}
								return { ...s, customAssets }
							})
						}
						onCustomAddress={(symbol, chain, address) =>
							setState((s) => ({
								...s,
								customAssets: {
									...s.customAssets,
									[symbol]: { ...(s.customAssets[symbol] ?? {}), [chain]: address },
								},
							}))
						}
						onRemove={() => setState((s) => ({ ...s, pairs: removeAt(s.pairs, index) }))}
					/>
				))}
				<button type="button" onClick={() => setState((s) => ({ ...s, pairs: [...s.pairs, newCrossAssetDraft()] }))}>
					+ Add market
				</button>

				{unanchored.length > 0 && (
					<div style={{ marginTop: "0.8rem" }}>
						<p className="error">
							No USD anchor for {unanchored.join(", ")}: confirmation depth is sized in USD and your curves
							are the only price feed, so every quote asset must connect to a dollar through some curve-priced
							pair. Add a reference-only price feed — it anchors the asset without opening a market.
						</p>
						{unanchored.map((symbol) => (
							<button
								key={symbol}
								type="button"
								onClick={() => setState((s) => ({ ...s, pairs: [...s.pairs, newReferenceDraft(symbol)] }))}
							>
								+ Add USDC/{symbol} reference price feed
							</button>
						))}
					</div>
				)}

				{state.fxPricing === "uniswapV4" && marketRows.length > 0 && (
					<div style={{ marginTop: "0.8rem" }}>
						<p className="hint">
							The pool acts as the price oracle and doubles as fill liquidity (withdrawn atomically when the
							wallet is short). Add at least one position; the optional price guard rejects fills when the
							live pool quote drifts from a reference price — set both guard fields or neither.
						</p>
						{state.fxPositions.map((position, index) => (
							// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
							<div className="row" key={index} style={{ marginBottom: "0.5rem" }}>
								<select
									value={position.chain}
									onChange={(e) =>
										setState((s) => ({
											...s,
											fxPositions: patchAt(s.fxPositions, index, { chain: e.target.value }),
										}))
									}
								>
									{chains.map((c) => (
										<option key={c.meta.stateMachineId} value={c.meta.stateMachineId}>
											{c.meta.label}
										</option>
									))}
								</select>
								<input
									type="text"
									placeholder="position token id"
									style={{ maxWidth: "10rem" }}
									value={position.tokenId}
									onChange={(e) =>
										setState((s) => ({
											...s,
											fxPositions: patchAt(s.fxPositions, index, { tokenId: e.target.value }),
										}))
									}
								/>
								<input
									type="text"
									placeholder="reference price (opt)"
									style={{ maxWidth: "10rem" }}
									value={position.referencePrice}
									onChange={(e) =>
										setState((s) => ({
											...s,
											fxPositions: patchAt(s.fxPositions, index, { referencePrice: e.target.value }),
										}))
									}
								/>
								<input
									type="text"
									placeholder="max deviation bps (opt)"
									style={{ maxWidth: "10rem" }}
									value={position.maxDeviationBps}
									onChange={(e) =>
										setState((s) => ({
											...s,
											fxPositions: patchAt(s.fxPositions, index, { maxDeviationBps: e.target.value }),
										}))
									}
								/>
								<button
									type="button"
									onClick={() => setState((s) => ({ ...s, fxPositions: removeAt(s.fxPositions, index) }))}
								>
									✕
								</button>
							</div>
						))}
						<div className="row">
							<button
								type="button"
								onClick={() =>
									setState((s) => ({
										...s,
										fxPositions: [
											...s.fxPositions,
											{
												chain: chains[0]?.meta.stateMachineId ?? "",
												tokenId: "",
												referencePrice: "",
												maxDeviationBps: "",
											},
										],
									}))
								}
							>
								+ Add position
							</button>
							<label className="field" style={{ maxWidth: "16rem", margin: 0 }}>
								<span>Direction</span>
								<select
									value={state.fxSide}
									onChange={(e) => setState((s) => ({ ...s, fxSide: e.target.value as "" | "ask" | "bid" }))}
								>
									<option value="">both directions</option>
									<option value="ask">ask only — sell the base token, accumulate the quote</option>
									<option value="bid">bid only — buy the base token, accumulate it</option>
								</select>
							</label>
							<label className="field" style={{ maxWidth: "10rem", margin: 0 }}>
								<span>Spread (bps, optional)</span>
								<input
									type="text"
									value={state.fxSpreadBps}
									onChange={(e) => setState((s) => ({ ...s, fxSpreadBps: e.target.value }))}
								/>
							</label>
						</div>
					</div>
				)}
			</div>
		</div>
	)
}

function SymbolPicker(props: {
	label: string
	value: string
	symbols: string[]
	custom: boolean
	onSelect: (symbol: string) => void
	onCustom: () => void
	onCustomSymbol: (symbol: string) => void
}) {
	const { label, value, symbols, custom, onSelect, onCustom, onCustomSymbol } = props
	return (
		<>
			<label className="field" style={{ margin: 0 }}>
				<span>{label}</span>
				<select
					value={custom ? "custom" : value}
					onChange={(e) => (e.target.value === "custom" ? onCustom() : onSelect(e.target.value))}
				>
					{symbols.map((symbol) => (
						<option key={symbol} value={symbol}>
							{symbol}
						</option>
					))}
					<option value="custom">custom token…</option>
				</select>
			</label>
			{custom && (
				<label className="field" style={{ margin: 0, maxWidth: "8rem" }}>
					<span>Symbol</span>
					<input
						type="text"
						placeholder="e.g. BRZ"
						value={value}
						onChange={(e) => onCustomSymbol(normSymbol(e.target.value))}
					/>
				</label>
			)}
		</>
	)
}

function MarketRow(props: {
	pair: PairDraft
	symbols: string[]
	usdStables: string[]
	chains: ChainDraft[]
	pricing: "curves" | "uniswapV4"
	duplicate: boolean
	customAssets: Record<string, Record<string, string>>
	onPatch: (patch: Partial<PairDraft>) => void
	onRenameAsset: (from: string, to: string) => void
	onCustomAddress: (symbol: string, chain: string, address: string) => void
	onRemove: () => void
}) {
	const { pair, symbols, usdStables, chains, pricing, duplicate, customAssets, onPatch, onRenameAsset, onCustomAddress, onRemove } = props
	const [custom0, setCustom0] = useState(() => Boolean(pair.token0) && !symbols.includes(pair.token0))
	const [custom1, setCustom1] = useState(
		() => Boolean(pair.token1) && !symbols.includes(pair.token1) && pair.token1 !== "CNGN",
	)
	const sameToken = isSameTokenDraft(pair)
	const venueNeedsStable = pricing === "uniswapV4" && !sameToken && !usdStables.includes(normSymbol(pair.token0))

	if (pair.referenceOnly) {
		return (
			<div style={{ border: "1px dashed var(--border, #bbb)", borderRadius: "8px", padding: "0.8rem", margin: "0.6rem 0" }}>
				<div className="row" style={{ flexWrap: "wrap", alignItems: "flex-end" }}>
					<span className="badge">price feed only</span>
					<strong>
						{pair.token0}/{pair.token1}
					</strong>
					<label className="field" style={{ margin: 0, maxWidth: "14rem" }}>
						<span>
							Reference price ({pair.token1} per {pair.token0})
						</span>
						<input
							type="text"
							value={pair.ask[0]?.value ?? ""}
							onChange={(e) => onPatch({ ask: [{ amount: "0", value: e.target.value }] })}
						/>
					</label>
					<button type="button" onClick={onRemove}>
						✕
					</button>
				</div>
				<p className="hint" style={{ margin: "0.3rem 0 0" }}>
					Anchors {pair.token1} in USD for confirmation sizing without opening a {pair.token0}/{pair.token1}{" "}
					market. Keep the price roughly current — it never prices fills.
				</p>
			</div>
		)
	}

	return (
		<div style={{ border: "1px solid var(--border, #ddd)", borderRadius: "8px", padding: "0.8rem", margin: "0.6rem 0" }}>
			<div className="row" style={{ flexWrap: "wrap", alignItems: "flex-end" }}>
				<SymbolPicker
					label="Quote (token0)"
					value={pair.token0}
					symbols={symbols}
					custom={custom0}
					onSelect={(symbol) => {
						setCustom0(false)
						onRenameAsset(pair.token0, symbol)
						onPatch({ token0: symbol })
					}}
					onCustom={() => {
						setCustom0(true)
						onPatch({ token0: "" })
					}}
					onCustomSymbol={(symbol) => {
						onRenameAsset(pair.token0, symbol)
						onPatch({ token0: symbol })
					}}
				/>
				<SymbolPicker
					label="Base (token1)"
					value={pair.token1}
					symbols={symbols}
					custom={custom1}
					onSelect={(symbol) => {
						setCustom1(false)
						onRenameAsset(pair.token1, symbol)
						onPatch({ token1: symbol })
					}}
					onCustom={() => {
						setCustom1(true)
						onPatch({ token1: "" })
					}}
					onCustomSymbol={(symbol) => {
						onRenameAsset(pair.token1, symbol)
						onPatch({ token1: symbol })
					}}
				/>
				<label className="field" style={{ margin: 0, maxWidth: "12rem" }}>
					<span>Max {pair.token0 || "token0"} per order</span>
					<input type="text" value={pair.maxOrderSize} onChange={(e) => onPatch({ maxOrderSize: e.target.value })} />
				</label>
				<button type="button" onClick={onRemove}>
					✕
				</button>
			</div>

			{duplicate && (
				<p className="error">
					This market is already declared (a pair and its reverse are the same market — one orientation only).
				</p>
			)}
			{venueNeedsStable && (
				<p className="error">
					Pool pricing only quotes markets with a USD-stable quote asset — pick USDC/USDT/DAI as token0 or
					switch this market to static curves.
				</p>
			)}

			{[
				...(custom0 && pair.token0 ? [pair.token0] : []),
				...(custom1 && pair.token1 && pair.token1 !== pair.token0 ? [pair.token1] : []),
			].map((symbol) => (
				<CustomAssetAddresses
					key={symbol}
					symbol={symbol}
					chains={chains}
					addresses={customAssets[symbol] ?? {}}
					onAddress={(chain, address) => onCustomAddress(symbol, chain, address)}
				/>
			))}

			{sameToken && (
				<div>
					<p className="hint">
						Same asset on both sides: a cross-chain transfer market. Ask-only — the price is the fraction of
						the input paid back out; keep every point strictly below 1.
					</p>
					<CurveEditor
						points={pair.ask}
						onChange={(points) => onPatch({ ask: points })}
						amountLabel={`Order size (${pair.token0})`}
						valueLabel="Price (below 1)"
					/>
					{pair.ask.some((p) => p.value.trim() && Number(p.value) >= 1) && (
						<p className="error">Prices at or above 1 give back at least what was received — no spread.</p>
					)}
				</div>
			)}

			{!sameToken && pricing === "curves" && (
				<div>
					<p className="hint">
						Prices are {pair.token1 || "base"} per {pair.token0 || "quote"}. Disabling one side is one-sided
						LP: bid-only keeps buying (accumulates {pair.token1 || "the base token"}), ask-only keeps selling
						(accumulates {pair.token0 || "the quote"}). The bid must stay above the ask everywhere.
					</p>
					<label className="row">
						<input
							type="checkbox"
							checked={pair.bidEnabled}
							onChange={(e) => onPatch({ bidEnabled: e.target.checked })}
						/>
						Bid curve — price when buying {pair.token1 || "the base token"} from users
					</label>
					{pair.bidEnabled && (
						<CurveEditor
							points={pair.bid}
							onChange={(points) => onPatch({ bid: points })}
							amountLabel={`Order size (${pair.token0})`}
							valueLabel={`${pair.token1 || "Base"} per ${pair.token0 || "quote"}`}
						/>
					)}
					<label className="row" style={{ marginTop: "0.5rem" }}>
						<input
							type="checkbox"
							checked={pair.askEnabled}
							onChange={(e) => onPatch({ askEnabled: e.target.checked })}
						/>
						Ask curve — price when selling {pair.token1 || "the base token"} to users
					</label>
					{pair.askEnabled && (
						<CurveEditor
							points={pair.ask}
							onChange={(points) => onPatch({ ask: points })}
							amountLabel={`Order size (${pair.token0})`}
							valueLabel={`${pair.token1 || "Base"} per ${pair.token0 || "quote"}`}
						/>
					)}
					{!pair.bidEnabled && !pair.askEnabled && <p className="error">Enable at least one direction.</p>}
				</div>
			)}
		</div>
	)
}

function CustomAssetAddresses(props: {
	symbol: string
	chains: ChainDraft[]
	addresses: Record<string, string>
	onAddress: (chain: string, address: string) => void
}) {
	const { symbol, chains, addresses, onAddress } = props
	const [status, setStatus] = useState<Record<string, { ok?: string; err?: string }>>({})

	const verify = async (chain: ChainDraft) => {
		const address = (addresses[chain.meta.stateMachineId] ?? "").trim()
		if (!address || !chain.rpcUrls[0]?.trim()) return
		try {
			const res = await api.post<{ ok: boolean; symbol?: string; decimals?: number; error?: string }>(
				"/api/setup/validate-token",
				{ rpcUrl: chain.rpcUrls[0].trim(), address },
			)
			setStatus((s) => ({
				...s,
				[chain.meta.stateMachineId]: res.ok
					? { ok: `${res.symbol} (${res.decimals} decimals)` }
					: { err: res.error },
			}))
		} catch (err) {
			setStatus((s) => ({
				...s,
				[chain.meta.stateMachineId]: { err: err instanceof Error ? err.message : String(err) },
			}))
		}
	}

	return (
		<div style={{ margin: "0.5rem 0" }}>
			<p className="hint">{symbol} contract address per chain it exists on (at least one required):</p>
			{chains.map((chain) => (
				<label className="field" key={chain.meta.chainId}>
					<span>{chain.meta.label}</span>
					<div className="row">
						<input
							type="text"
							style={{ flex: 1 }}
							placeholder="0x… (leave empty if the token isn't on this chain)"
							value={addresses[chain.meta.stateMachineId] ?? ""}
							onChange={(e) => onAddress(chain.meta.stateMachineId, e.target.value)}
						/>
						<button
							type="button"
							disabled={!(addresses[chain.meta.stateMachineId] ?? "").trim()}
							onClick={() => verify(chain)}
						>
							Verify
						</button>
						{status[chain.meta.stateMachineId]?.ok && (
							<span className="badge ok">{status[chain.meta.stateMachineId].ok}</span>
						)}
						{status[chain.meta.stateMachineId]?.err && (
							<span className="badge err">{status[chain.meta.stateMachineId].err}</span>
						)}
					</div>
				</label>
			))}
		</div>
	)
}
