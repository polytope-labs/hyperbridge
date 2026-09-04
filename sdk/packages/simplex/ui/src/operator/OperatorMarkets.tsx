import { useState } from "react"
import { ChevronRightIcon } from "../components/InterfaceIcons"
import { OperatorSheet } from "../components/OperatorSheet"
import { TokenPairIcons } from "../components/TokenIcon"
import type { AdminStrategyDto, ConfigDto } from "../types"
import { CreateMarketForm } from "./markets/CreateMarketForm"
import { marketDescription, marketSymbols } from "./markets/marketModel"
import { StrategyMarketEditor } from "./markets/StrategyMarketEditor"

interface OperatorMarketsProps {
	strategies: AdminStrategyDto[]
	config: ConfigDto | undefined
	chains: number[]
	chainLabels?: Record<string, string>
	onChanged: () => Promise<void> | void
}

function firstConfiguredPrice(points: AdminStrategyDto["bid"]): string | undefined {
	return points?.find((point) => {
		const amount = Number(point.amount)
		const price = Number(point.price)
		return (
			point.amount.trim() &&
			point.price.trim() &&
			Number.isFinite(amount) &&
			amount >= 0 &&
			Number.isFinite(price) &&
			price > 0
		)
	})?.price.trim()
}

function MarketPrices({ strategy }: { strategy: AdminStrategyDto }) {
	if (strategy.referenceOnly) return null

	const buyPrice = firstConfiguredPrice(strategy.bid)
	const sellPrice = firstConfiguredPrice(strategy.ask)
	if (!buyPrice && !sellPrice) return null

	const unit = `${strategy.token1 || "token1"}/${strategy.token0 || "token0"}`
	return (
		<span className="operator-market-prices" aria-label={`Current prices in ${unit}`}>
			{buyPrice ? (
				<span className="operator-market-price">
					<small>Buy</small>
					<strong>
						{buyPrice} <em>{unit}</em>
					</strong>
				</span>
			) : null}
			{sellPrice ? (
				<span className="operator-market-price">
					<small>Sell</small>
					<strong>
						{sellPrice} <em>{unit}</em>
					</strong>
				</span>
			) : null}
		</span>
	)
}

export function OperatorMarkets(props: OperatorMarketsProps) {
	const { strategies, config, chains, chainLabels, onChanged } = props
	const [showAddMarket, setShowAddMarket] = useState(false)
	const [selectedStrategy, setSelectedStrategy] = useState<number>()
	const selectedMarket = strategies.find((strategy) => strategy.index === selectedStrategy)

	return (
		<>
			<section className="operator-section">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Pricing</span>
						<h2>Markets</h2>
					</div>
					<button type="button" className="operator-text-button" onClick={() => setShowAddMarket(true)}>
						+ Create market
					</button>
				</div>
				<div className="operator-market-list">
					{strategies.map((strategy) => (
						<button
							type="button"
							className="operator-market-row"
							key={strategy.index}
							onClick={() => setSelectedStrategy(strategy.index)}
						>
							<TokenPairIcons tokenA={strategy.token0} tokenB={strategy.token1} />
							<span className="operator-market-copy">
								<strong>
									{strategy.token0} ↔ {strategy.token1}
								</strong>
								<small>{marketDescription(strategy)}</small>
							</span>
							<span className="operator-market-mode">
								{strategy.pricingMode === "venue" ? "Uniswap V4" : "Manual curve"}
							</span>
							<MarketPrices strategy={strategy} />
							<span className="operator-market-cap">
								{strategy.maxOrderSize
									? `Max ${strategy.maxOrderSize} ${strategy.token0}`
									: "No order cap"}
							</span>
							<ChevronRightIcon aria-hidden="true" />
						</button>
					))}
					{strategies.length === 0 ? <p className="operator-empty">No markets configured.</p> : null}
				</div>
			</section>

			<OperatorSheet
				open={Boolean(selectedMarket)}
				onClose={() => setSelectedStrategy(undefined)}
				wide
				title={selectedMarket ? `${selectedMarket.token0} ↔ ${selectedMarket.token1}` : "Market"}
				description="Review and update this market without leaving the operator workspace."
			>
				{selectedMarket ? (
					<StrategyMarketEditor
						key={selectedMarket.index}
						strategy={selectedMarket}
						onApplied={onChanged}
						removable={strategies.length > 1}
						hasVaults={(config?.vaults.length ?? 0) > 0}
					/>
				) : null}
			</OperatorSheet>

			<OperatorSheet
				open={showAddMarket}
				onClose={() => setShowAddMarket(false)}
				wide
				title="Create a market"
				description="Choose the asset pair, order limit, and pricing directions."
			>
				<CreateMarketForm
					symbols={marketSymbols(config)}
					chains={chains}
					chainLabel={(id) => chainLabels?.[String(id)] ?? `Chain ${id}`}
					onAdded={async () => {
						setShowAddMarket(false)
						await onChanged()
					}}
					onCancel={() => setShowAddMarket(false)}
				/>
			</OperatorSheet>
		</>
	)
}
