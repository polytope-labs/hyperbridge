import { useState } from "react"
import { isRegistrySymbol } from "@/config/asset-registry"
import { bookCrossedAt } from "@/config/interpolated-curve"
import { api } from "../../api"
import { CurveEditor } from "../../components/CurveEditor"
import { toPricePoints } from "../../components/curveModel"
import { TokenSelect } from "../../components/TokenSelect"
import { isSameTokenDraft, normSymbol, type ChainDraft, type PairDraft } from "../state"

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
			<div className="field market-symbol-field">
				<span className="field-label field-label-required-mark">
					{label}
					<span className="field-required-mark" aria-hidden="true">
						*
					</span>
				</span>
				<TokenSelect
					label={label}
					value={value}
					symbols={symbols}
					custom={custom}
					onSelect={onSelect}
					onCustom={onCustom}
				/>
			</div>
			{custom && (
				<label className="field market-symbol-field market-custom-symbol-field">
					<span className="field-label">
						Symbol <span className="field-required">Required</span>
					</span>
					<input
						type="text"
						required
						placeholder="e.g. BRZ"
						value={value}
						onChange={(e) => onCustomSymbol(normSymbol(e.target.value))}
					/>
				</label>
			)}
		</>
	)
}

export function MarketRow(props: {
	pair: PairDraft
	symbols: string[]
	usdStables: string[]
	chains: ChainDraft[]
	pricing: "curves" | "uniswapV4"
	duplicate: boolean
	customAssets: Record<string, Record<string, string>>
	onPatch: (patch: Partial<PairDraft>) => void
	onSymbolChange: (patch: Partial<PairDraft>) => void
	onRenameAsset: (from: string, to: string) => void
	onCustomAddress: (symbol: string, chain: string, address: string) => void
}) {
	const {
		pair,
		symbols,
		usdStables,
		chains,
		pricing,
		duplicate,
		customAssets,
		onPatch,
		onSymbolChange,
		onRenameAsset,
		onCustomAddress,
	} = props
	// Picker mode lives on the draft (not component state) so it survives row deletion/reordering.
	const custom0 = pair.custom0 ?? false
	const custom1 = pair.custom1 ?? false
	const sameToken = isSameTokenDraft(pair)
	const venueNeedsStable = pricing === "uniswapV4" && !sameToken && !usdStables.includes(normSymbol(pair.token0))
	const shadowed = [pair.token0, pair.token1].filter(
		(symbol, i) => (i === 0 ? custom0 : custom1) && symbol && isRegistrySymbol(symbol),
	)
	const crossedAt =
		!sameToken && pricing === "curves" && pair.bidEnabled && pair.askEnabled
			? (bookCrossedAt(toPricePoints(pair.bid), toPricePoints(pair.ask))?.amount ?? null)
			: null

	if (pair.referenceOnly) {
		return (
			<section className="market-editor market-reference-editor">
				<div className="market-editor-heading">
					<div>
						<span className="markets-kicker">Reference feed</span>
						<h3>
							Set the {pair.token0} ↔ {pair.token1} reference price
						</h3>
					</div>
				</div>
				<div className="market-reference-grid">
					<label className="field">
						<span className="field-label">
							Reference price ({pair.token1} per {pair.token0}){" "}
							<span className="field-required">Required</span>
						</span>
						<input
							type="text"
							required
							value={pair.ask[0]?.value ?? ""}
							onChange={(e) => onPatch({ ask: [{ amount: "0", value: e.target.value }] })}
						/>
					</label>
				</div>
				{duplicate && (
					<p className="error">
						This market is already declared (a pair and its reverse are the same market) — remove this feed
						or the duplicate row.
					</p>
				)}
				<p className="market-editor-note">
					Anchors {pair.token1} in USD for confirmation sizing without opening this market.
				</p>
			</section>
		)
	}

	return (
		<section className="market-editor">
			<div className="market-editor-heading">
				<div>
					<span className="markets-kicker">Token pair</span>
					<h3>Choose the two assets</h3>
					<p className="hint">Simplex can buy or sell either side of this pair.</p>
				</div>
			</div>
			<div className="market-asset-grid">
				<SymbolPicker
					label="First asset"
					value={pair.token0}
					symbols={symbols}
					custom={custom0}
					onSelect={(symbol) => {
						onRenameAsset(pair.token0, symbol)
						onSymbolChange({ token0: symbol, custom0: false })
					}}
					onCustom={() => onSymbolChange({ token0: "", custom0: true })}
					onCustomSymbol={(symbol) => {
						onRenameAsset(pair.token0, symbol)
						onSymbolChange({ token0: symbol })
					}}
				/>
				<SymbolPicker
					label="Second asset"
					value={pair.token1}
					symbols={symbols}
					custom={custom1}
					onSelect={(symbol) => {
						onRenameAsset(pair.token1, symbol)
						onSymbolChange({ token1: symbol, custom1: false })
					}}
					onCustom={() => onSymbolChange({ token1: "", custom1: true })}
					onCustomSymbol={(symbol) => {
						onRenameAsset(pair.token1, symbol)
						onSymbolChange({ token1: symbol })
					}}
				/>
				<label className="field market-limit-field">
					<span>
						Maximum order in {pair.token0 || "the first asset"} <em>Optional</em>
					</span>
					<input
						type="text"
						value={pair.maxOrderSize}
						placeholder="uncapped"
						onChange={(e) => onPatch({ maxOrderSize: e.target.value })}
					/>
				</label>
			</div>

			{duplicate && (
				<p className="error">
					This market is already declared (a pair and its reverse are the same market — one orientation only).
				</p>
			)}
			{venueNeedsStable && (
				<p className="error">
					Uniswap pricing needs the first asset to be a USD stablecoin. Choose USDC, USDT, or DAI first, or
					switch to manual prices.
				</p>
			)}
			{shadowed.length > 0 && (
				<p className="error">
					{shadowed.join(", ")} ships with the registry — pick it from the dropdown instead of entering it as
					a custom token (a custom address here would silently repoint the real asset).
				</p>
			)}
			{crossedAt !== null && (
				<p className="hint">
					⚠ The book is crossed at order size {crossedAt} (bid at or below ask) — both sides still fill at
					their own curve, but a full round trip at these prices loses money. Leave it only if deliberate.
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
				<div className="market-curves market-single-curve">
					<div className="market-pricing-heading">
						<div>
							<span className="markets-kicker">Transfer price</span>
							<h3>How much {pair.token0} should be delivered?</h3>
						</div>
						<p className="market-editor-note">Keep the return below 1. The difference is your spread.</p>
					</div>
					<CurveEditor
						points={pair.ask}
						onChange={(points) => onPatch({ ask: points })}
						amountLabel={`Customer sends (${pair.token0})`}
						valueLabel={`${pair.token0} returned per 1 ${pair.token0}`}
					/>
					{pair.ask.some((p) => p.value.trim() && Number(p.value) >= 1) && (
						<p className="error">Prices at or above 1 give back at least what was received — no spread.</p>
					)}
				</div>
			)}

			{!sameToken && pricing === "curves" && (
				<div className="market-curves">
					<div className="market-pricing-heading">
						<div>
							<span className="markets-kicker">Price settings</span>
							<h3>Set a price for each direction</h3>
						</div>
						<p className="market-editor-note">
							Rates are shown as {pair.token1 || "second asset"} per {pair.token0 || "first asset"}.
						</p>
					</div>
					<div className="market-curve-grid">
						<section className="market-curve">
							<label className="market-curve-toggle">
								<input
									type="checkbox"
									checked={pair.bidEnabled}
									onChange={(e) => onPatch({ bidEnabled: e.target.checked })}
								/>
								<span>
									<strong>Simplex buys {pair.token1 || "the second asset"}</strong>
									<small>
										Customers send {pair.token1 || "the second asset"} and receive{" "}
										{pair.token0 || "the first asset"}
									</small>
								</span>
							</label>
							{pair.bidEnabled && (
								<CurveEditor
									points={pair.bid}
									onChange={(points) => onPatch({ bid: points })}
									amountLabel={`Order size (${pair.token0})`}
									valueLabel={`${pair.token1 || "Second asset"} per ${pair.token0 || "first asset"}`}
								/>
							)}
						</section>
						<section className="market-curve">
							<label className="market-curve-toggle">
								<input
									type="checkbox"
									checked={pair.askEnabled}
									onChange={(e) => onPatch({ askEnabled: e.target.checked })}
								/>
								<span>
									<strong>Simplex sells {pair.token1 || "the second asset"}</strong>
									<small>
										Customers send {pair.token0 || "the first asset"} and receive{" "}
										{pair.token1 || "the second asset"}
									</small>
								</span>
							</label>
							{pair.askEnabled && (
								<CurveEditor
									points={pair.ask}
									onChange={(points) => onPatch({ ask: points })}
									amountLabel={`Order size (${pair.token0})`}
									valueLabel={`${pair.token1 || "Second asset"} per ${pair.token0 || "first asset"}`}
								/>
							)}
						</section>
					</div>
					{!pair.bidEnabled && !pair.askEnabled && <p className="error">Enable at least one direction.</p>}
				</div>
			)}
		</section>
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
		<div className="custom-asset-addresses">
			<p className="hint">{symbol} contract address per chain it exists on (at least one required):</p>
			{chains.map((chain) => (
				<div className="field custom-asset-chain" key={chain.meta.chainId}>
					<span>{chain.meta.label}</span>
					<div className="custom-asset-chain-control">
						<input
							type="text"
							aria-label={`${symbol} contract address on ${chain.meta.label}`}
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
				</div>
			))}
		</div>
	)
}
