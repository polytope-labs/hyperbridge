import { useCallback, useState, type ComponentType, type SVGProps } from "react"
import { api } from "../api"
import hyperfxLogo from "../assets/hyperfx-logo.webp"
import { CopyHash } from "../components/CopyHash"
import { ActivityIcon, OperationsIcon, OverviewIcon, SettingsIcon, WalletIcon } from "../components/InterfaceIcons"
import { OperatorSheet } from "../components/OperatorSheet"
import { useAction, usePolling } from "../lib/hooks"
import type { AdminStrategyDto, BalanceSnapshot, ConfigDto, StatusOperator } from "../types"
import { Activity } from "./Activity"
import { Operations } from "./Operations"
import { OperatorOverview } from "./OperatorOverview"
import { Wallet } from "./Wallet"

type Tab = "overview" | "activity" | "wallet" | "operations"

const PAGE_TABS: Array<{
	value: Tab
	label: string
	description: string
	icon: ComponentType<SVGProps<SVGSVGElement>>
}> = [
	{ value: "overview", label: "Overview", description: "Health and liquidity", icon: OverviewIcon },
	{ value: "activity", label: "Activity", description: "Orders and bids", icon: ActivityIcon },
	{ value: "wallet", label: "Wallet", description: "Transaction history", icon: WalletIcon },
	{ value: "operations", label: "Operations", description: "Funds and configuration", icon: OperationsIcon },
]

const PAGE_COPY: Record<Tab, { eyebrow: string; title: string; description: string }> = {
	overview: {
		eyebrow: "Live workspace",
		title: "Overview",
		description: "Monitor liquidity, market coverage, and the health of your running filler.",
	},
	activity: {
		eyebrow: "Execution feed",
		title: "Activity",
		description: "Follow orders from detection through bidding and execution.",
	},
	wallet: {
		eyebrow: "Treasury ledger",
		title: "Wallet",
		description: "Review transactions submitted by the filler wallet.",
	},
	operations: {
		eyebrow: "Operator tools",
		title: "Operations",
		description: "Move funds and maintain live configuration without crowding the dashboard.",
	},
}

function formatUptime(seconds: number): string {
	const h = Math.floor(seconds / 3600)
	const m = Math.floor((seconds % 3600) / 60)
	return h > 0 ? `${h}h ${m}m` : `${m}m ${seconds % 60}s`
}

export function Operator(props: { status: StatusOperator; refresh: () => void }) {
	const { status, refresh } = props
	const [tab, setTab] = useState<Tab>("overview")
	const [balances, setBalances] = useState<BalanceSnapshot>()
	const [strategies, setStrategies] = useState<AdminStrategyDto[]>([])
	const [config, setConfig] = useState<ConfigDto>()
	const [showEnvironment, setShowEnvironment] = useState(false)
	const [showRuntime, setShowRuntime] = useState(false)
	const [loadError, setLoadError] = useState<string>()
	const [stopped, setStopped] = useState(false)
	const { run, pending, error } = useAction()
	const page = PAGE_COPY[tab]

	const load = useCallback(async () => {
		try {
			// Status is polled too so runtime changes (overfill self-halt, an
			// external pause) surface without a manual action.
			const [balanceSnapshot, strategyList, configDto] = await Promise.all([
				api.get<BalanceSnapshot>("/api/balances"),
				api.get<{ strategies: AdminStrategyDto[] }>("/api/strategies"),
				api.get<ConfigDto>("/api/config"),
				refresh(),
			])
			setBalances(balanceSnapshot)
			setStrategies(strategyList.strategies)
			setConfig(configDto)
			setLoadError(undefined)
		} catch (err) {
			setLoadError(err instanceof Error ? err.message : String(err))
		}
	}, [refresh])
	usePolling(load, 30_000)

	const togglePause = () =>
		run(async () => {
			await api.post(status.paused ? "/api/resume" : "/api/pause")
			refresh()
		})

	const resetHalt = () =>
		run(async () => {
			await api.post("/api/reset-halt")
			refresh()
		})

	const stopFiller = () => {
		if (
			!window.confirm(
				"Stop the filler? In-flight fills drain, vault positions may unwind, and the process exits.",
			)
		) {
			return
		}
		return run(async () => {
			await api.post("/api/stop")
			setStopped(true)
		})
	}
	if (stopped) {
		return (
			<div className="card">
				<h2>Filler stopping</h2>
				<p className="hint">
					In-flight fills are draining and the process will exit. Restart it with `simplex run` — a persisted
					pause state is honored on the next boot.
				</p>
			</div>
		)
	}

	return (
		<div className="operator-shell">
			<header className="operator-brandbar">
				<div className="operator-brand">
					<img className="hyperfx-logo" src={hyperfxLogo} alt="HyperFX" />
					<span>Simplex</span>
				</div>
				<button type="button" className="operator-environment-trigger" onClick={() => setShowEnvironment(true)}>
					<span className={`operator-status-dot ${status.paused ? "warn" : ""}`} />
					<span>{status.paused ? "Filling paused" : "Filler running"}</span>
					<SettingsIcon aria-hidden="true" />
				</button>
			</header>

			<div className="operator-layout">
				<aside className="operator-sidebar" aria-label="Dashboard navigation">
					<nav>
						{PAGE_TABS.map((item) => {
							const Icon = item.icon
							return (
								<button
									type="button"
									key={item.value}
									className="operator-nav-item"
									data-active={tab === item.value || undefined}
									onClick={() => setTab(item.value)}
								>
									<Icon aria-hidden="true" />
									<span>
										<strong>{item.label}</strong>
										<small>{item.description}</small>
									</span>
								</button>
							)
						})}
					</nav>
					<div className="operator-sidebar-footer">
						<span>Version {status.version}</span>
						<span>Up {formatUptime(status.uptimeSec)}</span>
					</div>
				</aside>

				<main className="operator-main">
					<header className="operator-page-header">
						<div>
							<span className="eyebrow">{page.eyebrow}</span>
							<h1>{page.title}</h1>
							<p>{page.description}</p>
						</div>
						{tab === "overview" ? (
							<button type="button" className="secondary" onClick={() => setShowRuntime(true)}>
								Runtime controls
							</button>
						) : null}
					</header>

					{tab === "overview" ? (
						<OperatorOverview
							status={status}
							balances={balances}
							strategies={strategies}
							config={config}
							onResetHalt={resetHalt}
							onMarketsChanged={load}
						/>
					) : null}

					{tab === "activity" ? <Activity /> : null}
					{tab === "wallet" ? <Wallet chainLabels={status.chainLabels} /> : null}
					{tab === "operations" ? (
						<Operations chains={status.chains} chainLabels={status.chainLabels} />
					) : null}
					{(error ?? loadError) ? <p className="error">{error ?? loadError}</p> : null}
				</main>
			</div>

			<OperatorSheet
				open={showEnvironment}
				onClose={() => setShowEnvironment(false)}
				title="Environment"
				description="Runtime identity and the local configuration used by this filler."
			>
				<div className="operator-detail-list">
					<div>
						<span>Status</span>
						<strong className={status.paused ? "text-warn" : "text-ok"}>
							{status.paused ? "Filling paused" : "Running normally"}
						</strong>
					</div>
					<div>
						<span>Uptime</span>
						<strong>{formatUptime(status.uptimeSec)}</strong>
					</div>
				</div>
				{status.addresses ? (
					<div className="operator-identity-list">
						<IdentityRow label="Filler wallet" value={status.addresses.evm} />
						{status.addresses.substrate ? (
							<IdentityRow label="Hyperbridge account" value={status.addresses.substrate} />
						) : null}
					</div>
				) : null}
				{status.configPath ? (
					<div className="operator-config-block">
						<span>Configuration file</span>
						<code>{status.configPath}</code>
					</div>
				) : null}
			</OperatorSheet>

			<OperatorSheet
				open={showRuntime}
				onClose={() => setShowRuntime(false)}
				title="Runtime controls"
				description="Pause new fills or safely stop the current Simplex process."
			>
				<div className="operator-runtime-state">
					<span className={`operator-status-dot ${status.paused ? "warn" : ""}`} />
					<div>
						<strong>{status.paused ? "New fills are paused" : "Filling is active"}</strong>
						<p>
							{status.paused
								? "Order monitoring continues, but new fills are not analysed."
								: "Simplex is monitoring and filling eligible orders."}
						</p>
					</div>
				</div>
				<div className="operator-action-section">
					<h3>{status.paused ? "Resume filling" : "Pause filling"}</h3>
					<p>In-flight fills complete. The pause state persists across restarts.</p>
					<button type="button" className="primary" onClick={togglePause} disabled={pending}>
						{status.paused ? "Resume filling" : "Pause new fills"}
					</button>
				</div>
				<div className="operator-action-section danger">
					<h3>Stop Simplex</h3>
					<p>Drain in-flight fills, unwind eligible vault positions, and exit the process.</p>
					<button type="button" onClick={stopFiller} disabled={pending}>
						Stop filler
					</button>
				</div>
			</OperatorSheet>
		</div>
	)
}

function IdentityRow(props: { label: string; value: string }) {
	return (
		<div className="operator-identity-row">
			<span>{props.label}</span>
			<code>
				<CopyHash value={props.value} chars={64} />
			</code>
		</div>
	)
}
