import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { AddressListEditor } from "../components/AddressListEditor"
import { OperationLink } from "../components/OperationLink"
import { OperatorSheet } from "../components/OperatorSheet"
import { Chains } from "./Chains"
import { useAction, usePolling } from "../lib/hooks"
import type { ConfigDto } from "../types"

export type OperationsPanel = "allowlist" | "chains"

/**
 * Live configuration tools. Moving funds (Send, Vault treasury) lives on the
 * Wallet page next to the ledger; this page keeps the access policy and the
 * chain set.
 */
export function Operations(props: {
	chains: number[]
	/** A sheet to open on arrival (the Wallet's vault editor sends operators here to enable a chain). */
	initialPanel?: OperationsPanel
	/** Called once `initialPanel` has been opened, so it is not re-applied on the next visit. */
	onInitialPanelShown?: () => void
}) {
	const [config, setConfig] = useState<ConfigDto>()
	const [allowlist, setAllowlist] = useState<string[]>([])
	const [panel, setPanel] = useState<OperationsPanel | undefined>(props.initialPanel)
	const { run: act, message, error } = useAction()
	const { initialPanel, onInitialPanelShown } = props
	useEffect(() => {
		if (initialPanel) {
			setPanel(initialPanel)
			onInitialPanelShown?.()
		}
	}, [initialPanel, onInitialPanelShown])

	const load = useCallback(async () => {
		const configDto = await api.get<ConfigDto>("/api/config")
		setConfig(configDto)
		setAllowlist(configDto.allowlistUsers)
	}, [])
	usePolling(useCallback(() => act(load, undefined, "poll"), [act, load]))

	return (
		<div className="operator-page-content">
			<section className="operator-section operator-tools">
				<div className="operator-section-heading">
					<div>
						<span className="eyebrow">Workflows</span>
						<h2>Choose an operation</h2>
					</div>
					<small>Each tool opens in a focused side panel.</small>
				</div>
				<div className="operator-tool-list">
					<OperationLink
						title="Order allowlist"
						description="Restrict filling to orders submitted by approved addresses."
						meta={allowlist.length ? `${allowlist.length} addresses` : "Open to all"}
						onClick={() => setPanel("allowlist")}
					/>
					<OperationLink
						title="Chains & endpoints"
						description="Maintain RPC providers, bundlers, and watch-only behaviour."
						meta={`${props.chains.length} enabled`}
						onClick={() => setPanel("chains")}
					/>
				</div>
			</section>

			{config?.configPath ? (
				<section className="operator-section operator-config-summary">
					<div>
						<span className="eyebrow">Local configuration</span>
						<h2>Runtime source</h2>
						<p>Operator changes are persisted to this file.</p>
					</div>
					<code>{config.configPath}</code>
				</section>
			) : null}

			<OperatorSheet
				open={panel === "allowlist"}
				onClose={() => setPanel(undefined)}
				title="Order allowlist"
				description="Choose who Simplex is permitted to fill for."
			>
				<div className="operator-panel-form">
					<h2>Allowlist</h2>
					<p className="hint">
						Only fill orders placed by these addresses. Changes apply immediately and are saved to the
						config. An empty list accepts orders from everyone.
					</p>
					<AddressListEditor
						addresses={allowlist}
						onChange={(users) =>
							act(async () => {
								await api.put("/api/allowlist", { users })
								setAllowlist(users)
							}, "Allowlist updated")
						}
					/>
				</div>
			</OperatorSheet>

			<OperatorSheet
				open={panel === "chains"}
				onClose={() => setPanel(undefined)}
				wide
				title="Chains & endpoints"
				description="Manage the networks and providers used by this running filler."
			>
				<Chains />
			</OperatorSheet>

			{message && <p className="hint">✓ {message}</p>}
			{error && <p className="error">{error}</p>}
		</div>
	)
}
