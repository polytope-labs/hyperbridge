import { useEffect, useMemo, useRef, useState } from "react"
import { ChainLogo } from "./ChainLogo"
import { CheckIcon, SearchIcon } from "./InterfaceIcons"
import { TokenIcon } from "./TokenIcon"
import { ResponsiveDialog, ResponsiveDialogContent, ResponsiveDialogTitle } from "./ui/ResponsiveDialog"

/** One token the picker offers, on one network. */
export interface PickerToken {
	key: string
	chainId: number
	symbol: string
	name: string
	/** Wallet balance in whole tokens; null when the network could not be read. */
	balance: number | null
	/** Dollar value of the balance, when the token has a known price. */
	usd?: number | null
}

export interface PickerNetwork {
	chainId: number
	label: string
}

function formatBalance(value: number): string {
	return value.toLocaleString(undefined, { maximumFractionDigits: 6 })
}

function formatUsd(value: number): string {
	return value.toLocaleString(undefined, { style: "currency", currency: "USD", maximumFractionDigits: 2 })
}

/**
 * Two-pane token picker: networks on the left, that network's tokens with wallet balances on
 * the right, each pane searchable. Tab moves between panes, Enter selects, Esc closes.
 */
export function TokenNetworkPicker(props: {
	open: boolean
	onClose: () => void
	title: string
	networks: PickerNetwork[]
	tokens: PickerToken[]
	selectedKey?: string
	onSelect: (token: PickerToken) => void
	/** Shown while balances are still loading. */
	loading?: boolean
}) {
	const { open, onClose, title, networks, tokens, selectedKey, onSelect, loading } = props
	const selected = tokens.find((token) => token.key === selectedKey)
	const [chainId, setChainId] = useState<number | undefined>(selected?.chainId ?? networks[0]?.chainId)
	const [networkQuery, setNetworkQuery] = useState("")
	const [tokenQuery, setTokenQuery] = useState("")
	const tokenSearch = useRef<HTMLInputElement>(null)

	useEffect(() => {
		if (!open) return
		setChainId(selected?.chainId ?? networks[0]?.chainId)
		setNetworkQuery("")
		setTokenQuery("")
	}, [open, selected?.chainId, networks])

	const visibleNetworks = useMemo(() => {
		const query = networkQuery.trim().toLowerCase()
		return query ? networks.filter((network) => network.label.toLowerCase().includes(query)) : networks
	}, [networks, networkQuery])

	const networkLabel = (id: number) => networks.find((network) => network.chainId === id)?.label ?? `Chain ${id}`

	const visibleTokens = useMemo(() => {
		const query = tokenQuery.trim().toLowerCase()
		return tokens
			.filter((token) => token.chainId === chainId)
			.filter(
				(token) => !query || token.symbol.toLowerCase().includes(query) || token.name.toLowerCase().includes(query),
			)
			.sort((a, b) => (b.balance ?? -1) - (a.balance ?? -1))
	}, [tokens, chainId, tokenQuery])

	const chooseNetwork = (id: number) => {
		setChainId(id)
		tokenSearch.current?.focus()
	}

	return (
		<ResponsiveDialog
			open={open}
			onOpenChange={(next) => {
				if (!next) onClose()
			}}
		>
			<ResponsiveDialogContent className="token-picker" overlayClassName="dialog-overlay" wide>
				<ResponsiveDialogTitle className="sr-only">{title}</ResponsiveDialogTitle>
				<div className="token-picker-panes">
					<section className="token-picker-networks" aria-label="Networks">
						<label className="token-picker-search">
							<SearchIcon aria-hidden="true" />
							<input
								type="search"
								placeholder="Search networks"
								value={networkQuery}
								onChange={(e) => setNetworkQuery(e.target.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter" && visibleNetworks[0]) {
										e.preventDefault()
										chooseNetwork(visibleNetworks[0].chainId)
									}
								}}
							/>
						</label>
						<h3>Networks</h3>
						<ul>
							{visibleNetworks.map((network) => {
								const active = network.chainId === chainId
								return (
									<li key={network.chainId}>
										<button
											type="button"
											className="token-picker-network"
											aria-pressed={active}
											onClick={() => chooseNetwork(network.chainId)}
										>
											<ChainLogo label={network.label} />
											<span>{network.label}</span>
											{active && (
												<span className="token-picker-check" aria-hidden="true">
													<CheckIcon />
												</span>
											)}
										</button>
									</li>
								)
							})}
							{visibleNetworks.length === 0 && <li className="token-picker-empty">No networks match</li>}
						</ul>
					</section>

					<section className="token-picker-tokens" aria-label="Tokens">
						<label className="token-picker-search">
							<SearchIcon aria-hidden="true" />
							<input
								ref={tokenSearch}
								type="search"
								placeholder="Search tokens"
								value={tokenQuery}
								onChange={(e) => setTokenQuery(e.target.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter" && visibleTokens[0]) {
										e.preventDefault()
										onSelect(visibleTokens[0])
									}
								}}
							/>
						</label>
						<h3>Available tokens</h3>
						<ul>
							{visibleTokens.map((token) => (
								<li key={token.key}>
									<button
										type="button"
										className="token-picker-token"
										aria-pressed={token.key === selectedKey}
										onClick={() => onSelect(token)}
									>
										<span className="send-review-asset">
											<TokenIcon symbol={token.symbol} size="lg" />
											<span className="send-review-chain-badge">
												<ChainLogo label={networkLabel(token.chainId)} />
											</span>
										</span>
										<span className="token-picker-token-name">
											<strong>
												{token.name} <small>{token.symbol}</small>
											</strong>
											<span>{networkLabel(token.chainId)}</span>
										</span>
										<span className="token-picker-token-balance">
											<strong className="mono">
												{token.balance === null ? "—" : formatBalance(token.balance)}
											</strong>
											<span>{token.usd == null ? "" : formatUsd(token.usd)}</span>
										</span>
									</button>
								</li>
							))}
							{visibleTokens.length === 0 && (
								<li className="token-picker-empty">{loading ? "Loading balances…" : "No tokens match"}</li>
							)}
						</ul>
					</section>
				</div>
				<footer className="token-picker-footer">
					<span>Use keyboard shortcuts for faster navigation</span>
					<span className="token-picker-keys">
						Next <kbd>tab</kbd> Select <kbd>enter</kbd> Close <kbd>esc</kbd>
					</span>
				</footer>
			</ResponsiveDialogContent>
		</ResponsiveDialog>
	)
}
