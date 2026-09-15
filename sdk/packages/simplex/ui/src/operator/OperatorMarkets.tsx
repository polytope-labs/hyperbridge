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
	/** The filler's EVM address, which a solver link binds the swap to. */
	solverAddress?: string
	onChanged: () => Promise<void> | void
}



export function OperatorMarkets(props: OperatorMarketsProps) {
	const { strategies, config, chains, chainLabels, solverAddress, onChanged } = props
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

		</>
	)
}
