import * as Collapsible from "@radix-ui/react-collapsible"
import { type MouseEvent, useState } from "react"
import { ChevronDownIcon } from "./InterfaceIcons"

/**
 * Which chain cards are expanded. A card follows its chain's enabled state
 * until the operator opens or closes it by hand.
 */
export function useChainPanels() {
	const [overrides, setOverrides] = useState<Record<number, boolean>>({})
	return {
		isOpen: (chainId: number, enabled: boolean) => overrides[chainId] ?? enabled,
		setOpen: (chainId: number, open: boolean) => setOverrides((current) => ({ ...current, [chainId]: open })),
	}
}

/**
 * Whether a click on a chain card's header landed on one of its own controls,
 * which handle it themselves: the enable switch, and the chevron, which Radix
 * already toggles.
 */
export function isHeaderControl(event: MouseEvent): boolean {
	return (event.target as HTMLElement).closest(".chain-enable-toggle, .chain-collapse-trigger") !== null
}

/**
 * The keyboard and screen-reader way to open a chain card. Clicking anywhere
 * on the row does the same, but a row cannot be a button: it holds the switch.
 */
export function ChainCollapseTrigger({ label }: { label: string }) {
	return (
		<Collapsible.Trigger asChild>
			<button type="button" className="chain-collapse-trigger" aria-label={`${label} settings`}>
				<ChevronDownIcon />
			</button>
		</Collapsible.Trigger>
	)
}
