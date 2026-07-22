import { toCurvePoints, toPricePoints, type EditorPoint } from "../components/CurveEditor"
import type { ChainDefault, CurvePoint, FillerConfig, Network, SetupDefaults, StrategyConfig } from "../types"

export interface ChainDraft {
	meta: ChainDefault
	enabled: boolean
	rpcUrls: string[]
	bundlerUrl: string
	viaAlchemy: boolean
	watchOnly: boolean
	rpcStatus?: "ok" | "err" | "checking"
	rpcError?: string
	bundlerWarning?: string
	bundlerOk?: boolean
	token1: string
	tokenSymbol?: string
	tokenError?: string
}

export interface VaultDraft {
	chain: string
	vault: string
	threshold: string
	minBalance: string
	redeemOnShutdown: boolean
}

export type SignerType = "privateKey" | "mpcVault" | "turnkey"

export interface V4PositionDraft {
	chain: string
	tokenId: string
	referencePrice: string
	maxDeviationBps: string
}

export interface WizardState {
	network: Network
	signerType: SignerType
	signerKey: string
	signerAddress?: string
	mpcVault: { apiToken: string; vaultUuid: string; accountAddress: string; callbackClientSignerPublicKey: string; grpcTarget: string }
	turnkey: { organizationId: string; apiPublicKey: string; apiPrivateKey: string; signWith: string }
	substrateKey: string
	substrateAddress?: string
	generatedMnemonic?: string
	hyperbridgeWsUrl: string
	balanceCheck?: { funded: boolean; free: string; decimals: number }
	alchemyKey: string
	alchemyStatus?: "ok" | "err"
	alchemyError?: string
	chains: ChainDraft[]
	stableEnabled: boolean
	stableBps: EditorPoint[]
	fxEnabled: boolean
	fxMaxOrderUsd: string
	fxPricing: "curves" | "uniswapV4"
	fxBidEnabled: boolean
	fxAskEnabled: boolean
	fxBid: EditorPoint[]
	fxAsk: EditorPoint[]
	fxSpreadBps: string
	fxPositions: V4PositionDraft[]
	fxSide: "" | "ask" | "bid"
	rebalancingEnabled: boolean
	rebalancingTrigger: string
	rebalancingUsdc: Record<string, string>
	binanceKey: string
	binanceSecret: string
	vaults: VaultDraft[]
	allowlist: string
	maxConcurrentOrders: string
	maxRechecks: string
	recheckDelayMs: string
	logging: string
}

export function initialState(defaults: SetupDefaults): WizardState {
	return {
		network: "mainnet",
		signerType: "privateKey",
		signerKey: "",
		mpcVault: { apiToken: "", vaultUuid: "", accountAddress: "", callbackClientSignerPublicKey: "", grpcTarget: "" },
		turnkey: { organizationId: "", apiPublicKey: "", apiPrivateKey: "", signWith: "" },
		substrateKey: "",
		hyperbridgeWsUrl: defaults.hyperbridgeWs.mainnet,
		alchemyKey: "",
		chains: defaults.chains
			.filter((c) => c.network === "mainnet")
			.map((meta) => ({
				meta,
				enabled: false,
				rpcUrls: [""],
				bundlerUrl: "",
				viaAlchemy: false,
				watchOnly: false,
				token1: "",
			})),
		stableEnabled: true,
		stableBps: defaults.stableBpsCurve.map((p) => ({ amount: p.amount, value: String(p.value) })),
		fxEnabled: false,
		fxMaxOrderUsd: "5000",
		fxPricing: "curves",
		fxBidEnabled: true,
		fxAskEnabled: true,
		fxBid: [{ amount: "100", value: "" }],
		fxAsk: [{ amount: "100", value: "" }],
		fxSpreadBps: "",
		fxPositions: [],
		fxSide: "",
		rebalancingEnabled: false,
		rebalancingTrigger: "0.5",
		rebalancingUsdc: {},
		binanceKey: "",
		binanceSecret: "",
		vaults: [],
		allowlist: "",
		maxConcurrentOrders: String(defaults.maxConcurrentOrders),
		maxRechecks: String(defaults.queue.maxRechecks),
		recheckDelayMs: String(defaults.queue.recheckDelayMs),
		logging: "info",
	}
}

export function switchNetwork(state: WizardState, defaults: SetupDefaults, network: Network): WizardState {
	return {
		...state,
		network,
		hyperbridgeWsUrl: defaults.hyperbridgeWs[network],
		chains: defaults.chains
			.filter((c) => c.network === network)
			.map((meta) => ({
				meta,
				enabled: false,
				rpcUrls: [""],
				bundlerUrl: "",
				viaAlchemy: false,
				watchOnly: false,
				token1: "",
			})),
		// Everything keyed by the previous network's chain ids must reset with it.
		vaults: [],
		fxPositions: [],
		rebalancingUsdc: {},
		alchemyStatus: undefined,
		alchemyError: undefined,
	}
}

export function enabledChains(state: WizardState): ChainDraft[] {
	return state.chains.filter((c) => c.enabled)
}

/** Users paste keys with and without the 0x prefix; normalize instead of nagging. */
export function normalizeHexKey(key: string): string {
	const trimmed = key.trim()
	return trimmed && !trimmed.startsWith("0x") ? `0x${trimmed}` : trimmed
}

export function patchAt<T>(list: T[], index: number, patch: Partial<T>): T[] {
	return list.map((item, i) => (i === index ? { ...item, ...patch } : item))
}

export function removeAt<T>(list: T[], index: number): T[] {
	return list.filter((_, i) => i !== index)
}

export function patchChain(state: WizardState, chainId: number, patch: Partial<ChainDraft>): WizardState {
	return {
		...state,
		chains: state.chains.map((c) => (c.meta.chainId === chainId ? { ...c, ...patch } : c)),
	}
}

/** Client-side mirror of the CLI wizard's assembleConfig; the server gate is authoritative. */
export function assembleConfig(state: WizardState, defaults: SetupDefaults): FillerConfig {
	const chains = enabledChains(state)

	const strategies: StrategyConfig[] = []

	// Testnet chain ids have no built-in confirmation defaults; write explicit ones.
	const confirmationPolicies: Record<string, { points: CurvePoint[] }> | undefined =
		state.network === "testnet"
			? Object.fromEntries(
					chains.map((c) => [String(c.meta.chainId), { points: defaults.testnetConfirmationPoints }]),
				)
			: undefined

	if (state.stableEnabled) {
		strategies.push({
			type: "stable",
			bpsCurve: toCurvePoints(state.stableBps),
			...(confirmationPolicies ? { confirmationPolicies } : {}),
		})
	}
	if (state.fxEnabled) {
		const token1 = Object.fromEntries(
			chains.filter((c) => c.token1.trim()).map((c) => [c.meta.stateMachineId, c.token1.trim()]),
		)
		const usingPool = state.fxPricing === "uniswapV4"
		strategies.push({
			type: "hyperfx",
			maxOrderUsd: Number(state.fxMaxOrderUsd),
			token1,
			...(!usingPool && state.fxBidEnabled ? { bidPriceCurve: toPricePoints(state.fxBid) } : {}),
			...(!usingPool && state.fxAskEnabled ? { askPriceCurve: toPricePoints(state.fxAsk) } : {}),
			...(state.fxSpreadBps.trim() ? { spreadBps: Number(state.fxSpreadBps) } : {}),
			...(usingPool
				? {
						vault: {
							uniswapV4: {
								positions: state.fxPositions.map((p) => ({
									chain: p.chain,
									tokenId: p.tokenId.trim(),
									...(p.referencePrice.trim() ? { referencePrice: p.referencePrice.trim() } : {}),
									...(p.maxDeviationBps.trim() ? { maxDeviationBps: Number(p.maxDeviationBps) } : {}),
								})),
								...(state.fxSide ? { side: state.fxSide } : {}),
							},
						},
					}
				: {}),
			...(confirmationPolicies ? { confirmationPolicies } : {}),
		})
	}

	const watchOnlyEntries = chains.filter((c) => c.watchOnly).map((c) => [String(c.meta.chainId), true] as const)

	const usdcBalances = Object.fromEntries(
		Object.entries(state.rebalancingUsdc).filter(([, amount]) => amount.trim()),
	)

	const allowlistUsers = state.allowlist
		.split(/[\s,]+/)
		.map((s) => s.trim())
		.filter(Boolean)

	const signer =
		state.signerType === "privateKey"
			? { type: "privateKey" as const, key: normalizeHexKey(state.signerKey) }
			: state.signerType === "mpcVault"
				? {
						type: "mpcVault" as const,
						apiToken: state.mpcVault.apiToken.trim(),
						vaultUuid: state.mpcVault.vaultUuid.trim(),
						accountAddress: state.mpcVault.accountAddress.trim(),
						callbackClientSignerPublicKey: state.mpcVault.callbackClientSignerPublicKey.trim(),
						...(state.mpcVault.grpcTarget.trim() ? { grpcTarget: state.mpcVault.grpcTarget.trim() } : {}),
					}
				: {
						type: "turnkey" as const,
						organizationId: state.turnkey.organizationId.trim(),
						apiPublicKey: state.turnkey.apiPublicKey.trim(),
						apiPrivateKey: state.turnkey.apiPrivateKey.trim(),
						signWith: state.turnkey.signWith.trim(),
					}

	return {
		simplex: {
			signer,
			maxConcurrentOrders: Number(state.maxConcurrentOrders) || defaults.maxConcurrentOrders,
			queue: {
				maxRechecks: Number(state.maxRechecks) || defaults.queue.maxRechecks,
				recheckDelayMs: Number(state.recheckDelayMs) || defaults.queue.recheckDelayMs,
			},
			...(state.logging !== "info" ? { logging: state.logging } : {}),
			...(watchOnlyEntries.length > 0 ? { watchOnly: Object.fromEntries(watchOnlyEntries) } : {}),
			substratePrivateKey: state.substrateKey.trim(),
			hyperbridgeWsUrl: state.hyperbridgeWsUrl.trim(),
		},
		strategies,
		chains: chains.map((c) => ({
			rpcUrls: c.rpcUrls.map((u) => u.trim()).filter(Boolean),
			bundlerUrl: c.bundlerUrl.trim(),
		})),
		...(state.rebalancingEnabled && Object.keys(usdcBalances).length > 0
			? {
					rebalancing: {
						triggerPercentage: Number(state.rebalancingTrigger),
						baseBalances: { USDC: usdcBalances },
					},
				}
			: {}),
		...(state.binanceKey.trim() && state.binanceSecret.trim()
			? { binance: { apiKey: state.binanceKey.trim(), apiSecret: state.binanceSecret.trim() } }
			: {}),
		...(state.vaults.length > 0
			? {
					vault: {
						vaults: state.vaults.map((v) => ({
							chain: v.chain,
							vault: v.vault.trim(),
							...(v.threshold.trim() ? { threshold: v.threshold.trim() } : {}),
							...(v.minBalance.trim() ? { minBalance: v.minBalance.trim() } : {}),
							...(v.redeemOnShutdown ? { redeemOnShutdown: true } : {}),
						})),
					},
				}
			: {}),
		...(allowlistUsers.length > 0 ? { allowlist: { users: allowlistUsers } } : {}),
	}
}

export function chainLabels(state: WizardState): string[] {
	return enabledChains(state).map((c) => `${c.meta.label} (chainId ${c.meta.chainId})`)
}
