import { useState } from "react"
import { api } from "../../api"
import { CurveEditor } from "../../components/CurveEditor"
import { PillTabs } from "../../components/PillTabs"
import { enabledChains, newCrossAssetDraft, patchAt, removeAt, type ChainDraft, type PairDraft } from "../state"
import type { StepProps } from "../Wizard"

const PRICING_TABS = [
	{ value: "curves", label: "Static bid/ask curves" },
	{ value: "uniswapV4", label: "Uniswap V4 positions" },
] as const

export function StepStrategies({ state, setState, defaults }: StepProps) {
	const chains = enabledChains(state)

	// Exotic registry symbols deployed on at least one enabled chain.
	const knownExotics = [
		...new Set(
			chains.flatMap((chain) =>
				(defaults.knownTokens[chain.meta.stateMachineId] ?? [])
					.map((t) => t.symbol)
					.filter((symbol) => !defaults.usdStables.includes(symbol)),
			),
		),
	]

	const patchPair = (index: number, patch: Partial<PairDraft>) =>
		setState((s) => ({ ...s, pairs: patchAt(s.pairs, index, patch) }))

	const sameAssetRows = state.pairs
		.map((pair, index) => ({ pair, index }))
		.filter(({ pair }) => pair.kind === "sameAsset")
	const crossAssetRows = state.pairs
		.map((pair, index) => ({ pair, index }))
		.filter(({ pair }) => pair.kind === "crossAsset")

	return (
		<div>
			<div className="card">
				<h2>Cross-chain transfer markets</h2>
				<p className="hint">
					Fill same-asset transfers between chains (e.g. USDC on Base for USDC on Arbitrum). The ask price is
					the fraction of the input you pay back out — keep it below 1; the gap to 1 is your spread on every
					fill. Order size and prices are in the asset's own units.
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
				<h2>Cross-asset FX markets — stablecoin ↔ exotic token (e.g. cNGN)</h2>
				<p className="hint">
					Each market prices its base token (token1) in units of its quote (token0) with your own curves.
					Assets are picked by symbol — addresses come from the built-in registry; custom tokens get an
					[assets] entry.
				</p>

				{crossAssetRows.length > 0 && (
					<div>
						<p className="hint">Price source for these markets:</p>
						<PillTabs
							options={PRICING_TABS}
							value={state.fxPricing}
							onChange={(fxPricing) => setState((s) => ({ ...s, fxPricing }))}
						/>
					</div>
				)}

				{crossAssetRows.map(({ pair, index }) => (
					<CrossAssetMarket
						key={index}
						pair={pair}
						knownExotics={knownExotics}
						usdStables={defaults.usdStables}
						chains={chains}
						pricing={state.fxPricing}
						customAddresses={state.customAssets[pair.token1] ?? {}}
						onPatch={(patch) => patchPair(index, patch)}
						onRename={(from, to) =>
							setState((s) => {
								const customAssets = { ...s.customAssets }
								if (from && customAssets[from] && from !== to) {
									customAssets[to] = customAssets[from]
									delete customAssets[from]
								}
								return { ...s, customAssets, pairs: patchAt(s.pairs, index, { token1: to }) }
							})
						}
						onCustomAddress={(chain, address) =>
							setState((s) => ({
								...s,
								customAssets: {
									...s.customAssets,
									[pair.token1]: { ...(s.customAssets[pair.token1] ?? {}), [chain]: address },
								},
							}))
						}
						onRemove={() => setState((s) => ({ ...s, pairs: removeAt(s.pairs, index) }))}
					/>
				))}
				<button
					type="button"
					onClick={() => setState((s) => ({ ...s, pairs: [...s.pairs, newCrossAssetDraft()] }))}
				>
					+ Add market
				</button>

				{state.fxPricing === "uniswapV4" && crossAssetRows.length > 0 && (
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

function CrossAssetMarket(props: {
	pair: PairDraft
	knownExotics: string[]
	usdStables: string[]
	chains: ChainDraft[]
	pricing: "curves" | "uniswapV4"
	customAddresses: Record<string, string>
	onPatch: (patch: Partial<PairDraft>) => void
	onRename: (from: string, to: string) => void
	onCustomAddress: (chain: string, address: string) => void
	onRemove: () => void
}) {
	const { pair, knownExotics, usdStables, chains, pricing, customAddresses, onPatch, onRename, onCustomAddress, onRemove } = props
	const isKnown = knownExotics.includes(pair.token1)
	const [custom, setCustom] = useState(!isKnown && pair.token1 !== "CNGN")

	return (
		<div style={{ border: "1px solid var(--border, #ddd)", borderRadius: "8px", padding: "0.8rem", margin: "0.6rem 0" }}>
			<div className="row" style={{ flexWrap: "wrap", alignItems: "flex-end" }}>
				<label className="field" style={{ margin: 0 }}>
					<span>Quote (token0)</span>
					<select value={pair.token0} onChange={(e) => onPatch({ token0: e.target.value })}>
						{usdStables.map((symbol) => (
							<option key={symbol} value={symbol}>
								{symbol}
							</option>
						))}
					</select>
				</label>
				<label className="field" style={{ margin: 0 }}>
					<span>Base (token1)</span>
					<select
						value={custom ? "custom" : pair.token1}
						onChange={(e) => {
							if (e.target.value === "custom") {
								setCustom(true)
								onRename(pair.token1, "")
							} else {
								setCustom(false)
								onRename(pair.token1, e.target.value)
							}
						}}
					>
						{knownExotics.map((symbol) => (
							<option key={symbol} value={symbol}>
								{symbol}
							</option>
						))}
						<option value="custom">custom token…</option>
					</select>
				</label>
				{custom && (
					<label className="field" style={{ margin: 0, maxWidth: "10rem" }}>
						<span>Symbol</span>
						<input
							type="text"
							placeholder="e.g. BRZ"
							value={pair.token1}
							onChange={(e) => onRename(pair.token1, e.target.value.trim().toUpperCase())}
						/>
					</label>
				)}
				<label className="field" style={{ margin: 0, maxWidth: "12rem" }}>
					<span>Max {pair.token0} per order</span>
					<input type="text" value={pair.maxOrderSize} onChange={(e) => onPatch({ maxOrderSize: e.target.value })} />
				</label>
				<button type="button" onClick={onRemove}>
					✕
				</button>
			</div>

			{custom && pair.token1 && (
				<CustomAssetAddresses
					symbol={pair.token1}
					chains={chains}
					addresses={customAddresses}
					onAddress={onCustomAddress}
				/>
			)}

			{pricing === "curves" && (
				<div>
					<p className="hint">
						Prices are {pair.token1 || "base"} per {pair.token0}. Disabling one side is one-sided LP: bid-only
						keeps buying (accumulates {pair.token1 || "the base token"}), ask-only keeps selling (accumulates{" "}
						{pair.token0}). The bid must stay above the ask everywhere.
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
							valueLabel={`${pair.token1 || "Base"} per ${pair.token0}`}
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
							valueLabel={`${pair.token1 || "Base"} per ${pair.token0}`}
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
