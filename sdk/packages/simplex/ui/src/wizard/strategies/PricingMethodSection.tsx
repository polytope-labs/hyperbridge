import { ChartLineIcon, CheckIcon } from "../../components/InterfaceIcons"
import uniswapIcon from "../../assets/networks/uniswap.svg"
import type { ReactNode } from "react"

export function PricingMethodSection(props: {
	value: "curves" | "uniswapV4"
	onChange: (value: "curves" | "uniswapV4") => void
}) {
	return (
		<section className="card market-flow-section">
			<div className="market-flow-heading">
				<div>
					<span className="market-flow-step">1 · Pricing method</span>
					<h2>How should Simplex determine prices?</h2>
					<p className="hint">This method applies to every trading market below.</p>
				</div>
			</div>
			<div className="market-method-options">
				<MethodOption
					selected={props.value === "curves"}
					onSelect={() => props.onChange("curves")}
					icon={<ChartLineIcon />}
					iconClass="market-method-icon-curve"
					title="Set prices manually"
					description="Define how the price changes as order size increases."
				/>
				<MethodOption
					selected={props.value === "uniswapV4"}
					onSelect={() => props.onChange("uniswapV4")}
					icon={<img src={uniswapIcon} alt="" />}
					iconClass="market-method-icon-uniswap"
					title="Use Uniswap v4 positions"
					description="Use live pool prices and position liquidity."
				/>
			</div>
		</section>
	)
}

function MethodOption(props: {
	selected: boolean
	onSelect: () => void
	icon: ReactNode
	iconClass: string
	title: string
	description: string
}) {
	return (
		<label className="market-method-option" data-selected={props.selected}>
			<input type="radio" name="pricing-method" checked={props.selected} onChange={props.onSelect} />
			<span className={`market-method-icon ${props.iconClass}`} aria-hidden="true">
				{props.icon}
			</span>
			<span>
				<strong>{props.title}</strong>
				<small>{props.description}</small>
			</span>
			<span className="market-method-check" aria-hidden="true">
				<CheckIcon />
			</span>
		</label>
	)
}
