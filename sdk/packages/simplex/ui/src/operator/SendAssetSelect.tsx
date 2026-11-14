import { useEffect, useRef, useState } from "react"
import { ChainLogo } from "../components/ChainLogo"
import { ChevronDownIcon, DashedCircleIcon } from "../components/InterfaceIcons"
import { TokenIcon } from "../components/TokenIcon"

/** One token on one network, as the list shows it. */
export interface SendAssetOption {
	key: string
	/** Null for a token sent by contract address, which has no logo or ticker to show. */
	symbol: string | null
	network: string
	/** The balance, or why there is none to show. */
	balance: string
	/** False while the balance is unknown or zero, which the row shows dimmed. */
	funded: boolean
}

function AssetRow({ asset }: { asset: SendAssetOption }) {
	return (
		<>
			<span className="send-review-asset">
				{asset.symbol === null ? (
					<DashedCircleIcon aria-hidden="true" />
				) : (
					<TokenIcon symbol={asset.symbol} size="lg" />
				)}
				{/* The network is named in text beside it, so the logo adds nothing when read out. */}
				<span className="send-review-chain-badge" aria-hidden="true">
					<ChainLogo label={asset.network} />
				</span>
			</span>
			<span className="send-asset-name">
				<strong>{asset.symbol ?? "Custom token"}</strong>
				<span>{asset.network}</span>
			</span>
			<span className="send-asset-balance" data-funded={asset.funded || undefined}>
				{asset.balance}
			</span>
		</>
	)
}

/**
 * The asset a send goes out in: a row showing the current one, which opens the list to choose
 * another from.
 *
 * The list is not a popover of its own. It is laid over the fields under the row, inside the
 * dialog, so the dialog keeps its height and a phone scrolls the list, not the drawer.
 */
export function SendAssetSelect(props: {
	current: SendAssetOption
	/** In the order shown. */
	options: SendAssetOption[]
	onSelect: (key: string) => void
}) {
	const { current, options, onSelect } = props
	const [open, setOpen] = useState(false)
	const container = useRef<HTMLDivElement>(null)
	const trigger = useRef<HTMLButtonElement>(null)

	useEffect(() => {
		if (!open) return
		const closeOnOutsidePointer = (event: PointerEvent) => {
			if (!container.current?.contains(event.target as Node)) setOpen(false)
		}
		// Escape closes the list alone. The dialog around it listens on the document, so the key is
		// taken one step earlier, on the window.
		const closeOnEscape = (event: KeyboardEvent) => {
			if (event.key !== "Escape") return
			event.stopPropagation()
			setOpen(false)
			trigger.current?.focus()
		}
		document.addEventListener("pointerdown", closeOnOutsidePointer)
		window.addEventListener("keydown", closeOnEscape, true)
		return () => {
			document.removeEventListener("pointerdown", closeOnOutsidePointer)
			window.removeEventListener("keydown", closeOnEscape, true)
		}
	}, [open])

	const name = `${current.symbol ?? "Custom token"} on ${current.network}`

	return (
		<div
			ref={container}
			className="send-asset-select"
			data-open={open || undefined}
			// Focus that moves to another control closes the list. A blur with nowhere to go is a
			// tap on an option in a browser that does not focus buttons, and must not close it.
			onBlur={(event) => {
				if (event.relatedTarget && !event.currentTarget.contains(event.relatedTarget)) setOpen(false)
			}}
		>
			<button
				ref={trigger}
				type="button"
				className="send-asset-trigger"
				aria-label={`Token to send: ${name}${current.balance ? `, balance ${current.balance}` : ""}`}
				aria-expanded={open}
				disabled={options.length === 0}
				onClick={() => setOpen((value) => !value)}
			>
				<AssetRow asset={current} />
				<ChevronDownIcon aria-hidden="true" />
			</button>
			{open ? (
				<ul className="send-asset-list">
					{options.map((option) => (
						<li key={option.key}>
							<button
								type="button"
								className="send-asset-option"
								aria-current={option.key === current.key || undefined}
								onClick={() => {
									setOpen(false)
									onSelect(option.key)
									trigger.current?.focus()
								}}
							>
								<AssetRow asset={option} />
							</button>
						</li>
					))}
				</ul>
			) : null}
		</div>
	)
}
