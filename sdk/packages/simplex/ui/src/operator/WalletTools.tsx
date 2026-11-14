import { useCallback, useLayoutEffect, useRef, useState } from "react"
import { toast } from "sonner"
import { formatChainKey, parseChainKey } from "@/config/interpolated-curve"
import { api } from "../api"
import { OperationLink } from "../components/OperationLink"
import { OperatorSheet } from "../components/OperatorSheet"
import { VaultRowsEditor } from "../components/VaultRowsEditor"
import { useAction, usePolling } from "../lib/hooks"
import { vaultRowsToToml, type VaultRowDraft } from "../lib/vault-rows"
import type { BalanceSnapshot, ConfigDto, VaultSweepDto } from "../types"
import { SendDialog } from "./SendDialog"

/** "2 vaults connected · Base, Arbitrum" for the drawer's summary line. */
function describeConnectedVaults(vaults: ConfigDto["vaults"], chainLabel: (id: number | string) => string): string {
	if (vaults.length === 0) return "No vaults connected"
	const chains = [...new Set(vaults.map((vault) => vault.chain))].map((chain) => {
		const id = parseChainKey(chain)
		return id === null ? chain : chainLabel(id)
	})
	return `${vaults.length} ${vaults.length === 1 ? "vault" : "vaults"} connected · ${chains.join(", ")}`
}

/**
 * The wallet's money-moving tools: outbound transfers and the vault treasury.
 * They live on the Wallet page beside the ledger they write to; configuration
 * tools stay on Operations.
 */
export function WalletTools(props: {
	chains: number[]
	chainLabels?: Record<string, string>
	balances?: BalanceSnapshot
	onBalancesChanged: () => Promise<void> | void
	/** Takes the operator to the chain editor (on Operations) from a locked vault group. */
	onOpenChains: () => void
}) {
	const [config, setConfig] = useState<ConfigDto>()
	const [vaultRows, setVaultRows] = useState<VaultRowDraft[]>()
	const [panel, setPanel] = useState<"send" | "vaults">()
	// Outcome of the last vault action that is not a plain success: a save that
	// needs a restart, or a sweep that found nothing it could deposit and why.
	const [vaultNotice, setVaultNotice] = useState<VaultNotice>()
	const { run: act, message, error } = useAction()
	const latestVaultRows = useRef<VaultRowDraft[] | undefined>(undefined)
	const vaultSaveActive = useRef(false)
	const vaultSaveQueued = useRef(false)
	useLayoutEffect(() => {
		latestVaultRows.current = vaultRows
	}, [vaultRows])

	const chainLabel = (id: number | string) => props.chainLabels?.[String(id)] ?? `chain ${id}`

	const load = useCallback(async () => {
		const configDto = await api.get<ConfigDto>("/api/config")
		setConfig(configDto)
		// seed the editor once from the running config; later edits are local until saved
		setVaultRows(
			(current) =>
				current ??
				configDto.vaults.map((v) => ({
					chain: v.chain,
					vault: v.vault,
					threshold: v.threshold ?? "",
					minBalance: v.minBalance ?? "",
					redeemOnShutdown: v.redeemOnShutdown ?? false,
				})),
		)
	}, [])
	usePolling(useCallback(() => act(load, undefined, "poll"), [act, load]))

	const saveVaults = () => {
		// A save may finish after the operator has made another edit. Remember a
		// second click and drain the latest draft instead of silently discarding it.
		vaultSaveQueued.current = true
		if (vaultSaveActive.current) return
		vaultSaveActive.current = true
		return act(
			async () => {
				setVaultNotice(undefined)
				try {
					let result: { applied: boolean; restartNeeded: boolean; persisted: boolean } | undefined
					let failed = false
					let failure: unknown
					do {
						vaultSaveQueued.current = false
						failed = false
						try {
							result = await api.put("/api/vault", {
								vaults: vaultRowsToToml(latestVaultRows.current ?? []),
							})
							await load()
						} catch (saveError) {
							failed = true
							failure = saveError
						}
					} while (vaultSaveQueued.current)

					if (failed) throw failure
					if (!result) throw new Error("Vault save did not return a result")
					if (!result.persisted) {
						throw new Error(
							"Vault changes were applied for this session but could not be saved to the config file",
						)
					}
					if (result.restartNeeded) {
						// The server only says this when the filler booted without a vault
						// venue: the rows are in the config file, but nothing in this
						// process sweeps into or sources from them until it restarts.
						setVaultNotice({
							tone: "warn",
							text: "Saved to config — restart the filler to activate the vault treasury",
						})
						toast.warning("Vault settings saved", {
							description: "Restart the filler to activate the vault treasury.",
						})
					} else {
						toast.success("Vault settings saved")
					}
				} finally {
					vaultSaveActive.current = false
					vaultSaveQueued.current = false
				}
			},
			"Vault treasury updated",
			"vault-save",
		)
	}

	return (
		<section className="operator-section operator-tools">
			<div className="operator-section-heading">
				<div>
					<span className="eyebrow">Funds</span>
					<h2>Move and grow liquidity</h2>
				</div>
				<small>Each tool opens on its own, over this page.</small>
			</div>
			<div className="operator-tool-list">
				<OperationLink
					title="Send funds"
					description="Transfer native gas tokens or ERC-20 assets from the filler wallet."
					meta={`${props.chains.length} ${props.chains.length === 1 ? "network" : "networks"}`}
					onClick={() => setPanel("send")}
				/>
				<OperationLink
					title="Vault treasury"
					description="Set sweep thresholds, wallet reserves, and redemption behaviour."
					meta={config ? describeVaultMeta(config) : "Optional"}
					onClick={() => setPanel("vaults")}
				/>
			</div>

			<SendDialog
				open={panel === "send"}
				onClose={() => setPanel(undefined)}
				chains={props.chains}
				chainLabel={chainLabel}
				sendTokens={config?.sendTokens}
				balances={props.balances}
				onSent={props.onBalancesChanged}
			/>

			<OperatorSheet
				open={panel === "vaults"}
				onClose={() => setPanel(undefined)}
				wide
				title="Vault treasury"
				description="Keep idle liquidity productive while preserving enough wallet balance for fills and gas."
			>
				<div className="operator-panel-form vault-panel">
					{config && !config.vaultConfigured && (
						<p className="hint">
							The filler started without a vault treasury. Vaults saved here are written to the config and
							activate after a restart.
						</p>
					)}
					<div className="vault-panel-summary">
						<span className="vault-panel-stat">
							{describeConnectedVaults(config?.vaults ?? [], chainLabel)}
						</span>
						{config?.vaultConfigured && (
							<div className="vault-editor-actions">
								<button
									type="button"
									className="vault-panel-action"
									onClick={() =>
										act(
											async () => {
												setVaultNotice(undefined)
												const result = await api.post<VaultSweepDto>("/api/vault/sweep")
												setVaultNotice(describeSweep(result, chainLabel))
												await props.onBalancesChanged()
											},
											undefined,
											"vault-sweep",
										)
									}
								>
									Sweep now
								</button>
								<button
									type="button"
									className="vault-panel-action"
									onClick={() => act(() => api.post("/api/vault/redeem"), "Positions redeemed")}
								>
									Redeem all
								</button>
							</div>
						)}
					</div>
					<VaultRowsEditor
						chains={props.chains.map((id) => ({ key: formatChainKey(id), label: chainLabel(id) }))}
						knownVaults={config?.knownVaults ?? {}}
						onEnableChain={() => {
							setPanel(undefined)
							props.onOpenChains()
						}}
						rows={vaultRows ?? []}
						onChange={setVaultRows}
					/>
					<div className="row" style={{ marginTop: "1.25rem" }}>
						<button
							type="button"
							className="primary"
							disabled={vaultRows === undefined}
							onClick={saveVaults}
						>
							Save vaults
						</button>
					</div>
					{vaultNotice && (
						<p className={vaultNotice.tone === "warn" ? "warning" : "hint"}>
							{vaultNotice.tone === "ok" && "✓ "}
							{vaultNotice.text}
						</p>
					)}
					{message && <p className="hint">✓ {message}</p>}
					{error && <p className="error">{error}</p>}
				</div>
			</OperatorSheet>
		</section>
	)
}

/** Row meta for the vault tool: how many vaults are saved, or that none are yet. */
function describeVaultMeta(config: ConfigDto): string {
	if (config.vaults.length === 0) return "Optional"
	return `${config.vaults.length} ${config.vaults.length === 1 ? "vault" : "vaults"}`
}

type VaultNotice = { tone: "ok" | "warn"; text: string }

/**
 * Turns a sweep pass into one sentence the operator can act on. An empty pass is
 * only a warning when a vault turned a due deposit away; a wallet that has not
 * reached its threshold is the sweep working as configured.
 */
function describeSweep(result: VaultSweepDto, chainLabel: (id: number | string) => string): VaultNotice {
	const where = (chain: string) => chainLabel(parseChainKey(chain) ?? chain)
	if (result.submitted.length > 0) {
		const legs = result.submitted.map(
			(tx) => `${tx.deposits.map((d) => `${d.amount} ${d.symbol}`).join(", ")} on ${where(tx.chain)}`,
		)
		return { tone: "ok", text: `Sweep submitted: ${legs.join("; ")}` }
	}
	const closed = result.skipped.filter((skip) => skip.reason === "deposits-closed")
	if (closed.length > 0) {
		const legs = closed.map(
			(skip) =>
				`the ${skip.symbol} vault on ${where(skip.chain)} is not accepting deposits right now (wallet ${skip.walletBalance} ${skip.symbol} is above its ${skip.threshold} threshold)`,
		)
		return {
			tone: "warn",
			text: `Nothing swept — ${legs.join("; ")}. The periodic sweep keeps retrying and deposits as soon as the vault reopens.`,
		}
	}
	const below = result.skipped.filter((skip) => skip.reason === "below-threshold")
	if (below.length > 0) {
		const legs = below.map(
			(skip) =>
				`${skip.symbol} on ${where(skip.chain)} is at ${skip.walletBalance} of its ${skip.threshold} threshold`,
		)
		return { tone: "ok", text: `Nothing to sweep — ${legs.join("; ")}` }
	}
	return { tone: "ok", text: "Nothing to sweep — no vault has a sweep threshold configured" }
}
