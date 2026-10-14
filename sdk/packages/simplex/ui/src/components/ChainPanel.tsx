import * as Collapsible from "@radix-ui/react-collapsible"
import { type MouseEvent, useState } from "react"
import { ChevronDownIcon } from "./InterfaceIcons"

/** Which chain cards are expanded. Every card starts closed; enabling a chain does not open it. */
export function useChainPanels() {
	const [open, setOpenState] = useState<Record<number, boolean>>({})
	return {
		isOpen: (chainId: number) => open[chainId] ?? false,
		setOpen: (chainId: number, next: boolean) => setOpenState((current) => ({ ...current, [chainId]: next })),
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
 * The keyboard and screen-reader way to open a chain card. On the operator page,
 * clicking anywhere on the row does the same, but a row cannot be a button: it
 * holds the switch.
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
