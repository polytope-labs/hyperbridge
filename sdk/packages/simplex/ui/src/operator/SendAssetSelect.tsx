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
				<span className="send-review-chain-badge">
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
 * The asset a send goes out in: a row showing the current one, which opens the list of every
 * token the wallet can send.
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
		document.addEventListener("pointerdown", closeOnOutsidePointer)
		return () => document.removeEventListener("pointerdown", closeOnOutsidePointer)
	}, [open])

	return (
		<div ref={container} className="send-asset-select" data-open={open || undefined}>
			<button
				ref={trigger}
				type="button"
				className="send-asset-trigger"
				aria-label="Choose token to send"
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
