import { fromPricePoints, toPricePoints, type EditorPoint } from "../components/CurveEditor"
import type { ChainDefault, CurvePoint, FillerConfig, Network, PairConfig, SetupDefaults } from "../types"

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

/**
 * One trading market. Same-asset transfer markets (token0 == token1) are fixed
 * prefab rows toggled by `enabled`; cross-asset rows are user-added.
 */
export interface PairDraft {
	kind: "sameAsset" | "crossAsset"
	enabled: boolean
	token0: string
	token1: string
	maxOrderSize: string
	bidEnabled: boolean
	askEnabled: boolean
	bid: EditorPoint[]
	ask: EditorPoint[]
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
	pairs: PairDraft[]
	/** `[assets]` entries for custom token symbols: symbol → state machine id → address. */
	customAssets: Record<string, Record<string, string>>
	/** Price source for the cross-asset pairs; same-asset markets always use their ask curve. */
	fxPricing: "curves" | "uniswapV4"
	fxSpreadBps: string
	fxPositions: V4PositionDraft[]
	fxSide: "" | "ask" | "bid"
	vaults: VaultDraft[]
	allowlistUsers: string[]
	maxConcurrentOrders: string
	maxRechecks: string
	recheckDelayMs: string
	logging: string
}

function sameAssetPrefabs(defaults: SetupDefaults): PairDraft[] {
	return ["USDC", "USDT"].map((symbol) => ({
		kind: "sameAsset" as const,
		enabled: symbol === "USDC",
		token0: symbol,
		token1: symbol,
		maxOrderSize: "100000",
		bidEnabled: false,
		askEnabled: true,
		bid: [],
		ask: fromPricePoints(defaults.sameAssetAskCurve),
	}))
}

export function newCrossAssetDraft(): PairDraft {
	return {
		kind: "crossAsset",
		enabled: true,
		token0: "USDC",
		token1: "CNGN",
		maxOrderSize: "5000",
		bidEnabled: true,
		askEnabled: true,
		bid: [{ amount: "100", value: "" }],
		ask: [{ amount: "100", value: "" }],
	}
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
			})),
		pairs: sameAssetPrefabs(defaults),
		customAssets: {},
		fxPricing: "curves",
		fxSpreadBps: "",
		fxPositions: [],
		fxSide: "",
		vaults: [],
		allowlistUsers: [],
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
			})),
		// Everything keyed by the previous network's chain ids must reset with it.
		pairs: sameAssetPrefabs(defaults),
		customAssets: {},
		vaults: [],
		fxPositions: [],
		alchemyStatus: undefined,
		alchemyError: undefined,
	}
}

export function enabledChains(state: WizardState): ChainDraft[] {
	return state.chains.filter((c) => c.enabled)
}

export function enabledPairs(state: WizardState): PairDraft[] {
	return state.pairs.filter((p) => p.enabled)
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
	const usingPool = state.fxPricing === "uniswapV4"

	const pairs: PairConfig[] = enabledPairs(state).map((draft) => {
		const sameAsset = draft.kind === "sameAsset"
		const withBid = !sameAsset && !usingPool && draft.bidEnabled
		const withAsk = sameAsset || (!usingPool && draft.askEnabled)
		return {
			token0: draft.token0,
			token1: draft.token1,
			maxOrderSize: draft.maxOrderSize.trim(),
			...(withBid ? { bidPriceCurve: toPricePoints(draft.bid) } : {}),
			...(withAsk ? { askPriceCurve: toPricePoints(draft.ask) } : {}),
		}
	})

	// Only [assets] entries actually referenced by a pair are emitted.
	const usedSymbols = new Set(pairs.flatMap((p) => [p.token0, p.token1]))
	const assets = Object.fromEntries(
		Object.entries(state.customAssets)
			.filter(([symbol]) => usedSymbols.has(symbol))
			.map(([symbol, byChain]) => [
				symbol,
				Object.fromEntries(Object.entries(byChain).filter(([, address]) => address.trim())),
			]),
	)

	// Testnet chain ids have no built-in confirmation defaults; write explicit ones.
	const confirmationPolicies: Record<string, { points: CurvePoint[] }> | undefined =
		state.network === "testnet"
			? Object.fromEntries(
					chains.map((c) => [String(c.meta.chainId), { points: defaults.testnetConfirmationPoints }]),
				)
			: undefined

	const uniswapV4 = usingPool
		? {
				positions: state.fxPositions.map((p) => ({
					chain: p.chain,
					tokenId: p.tokenId.trim(),
					...(p.referencePrice.trim() ? { referencePrice: p.referencePrice.trim() } : {}),
					...(p.maxDeviationBps.trim() ? { maxDeviationBps: Number(p.maxDeviationBps) } : {}),
				})),
				...(state.fxSide ? { side: state.fxSide } : {}),
				...(state.fxSpreadBps.trim() ? { spreadBps: Number(state.fxSpreadBps) } : {}),
			}
		: undefined

	const vaults =
		state.vaults.length > 0
			? state.vaults.map((v) => ({
					chain: v.chain,
					vault: v.vault.trim(),
					...(v.threshold.trim() ? { threshold: v.threshold.trim() } : {}),
					...(v.minBalance.trim() ? { minBalance: v.minBalance.trim() } : {}),
					...(v.redeemOnShutdown ? { redeemOnShutdown: true } : {}),
				}))
			: undefined

	const watchOnlyEntries = chains.filter((c) => c.watchOnly).map((c) => [String(c.meta.chainId), true] as const)

	const allowlistUsers = state.allowlistUsers

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
		...(Object.keys(assets).length > 0 ? { assets } : {}),
		pairs,
		...(confirmationPolicies ? { confirmationPolicies } : {}),
		chains: chains.map((c) => ({
			rpcUrls: c.rpcUrls.map((u) => u.trim()).filter(Boolean),
			bundlerUrl: c.bundlerUrl.trim(),
		})),
		...(vaults || uniswapV4
			? {
					vault: {
						...(vaults ? { vaults } : {}),
						...(uniswapV4 ? { uniswapV4 } : {}),
					},
				}
			: {}),
		...(allowlistUsers.length > 0 ? { allowlist: { users: allowlistUsers } } : {}),
	}
}

export function chainLabels(state: WizardState): string[] {
	return enabledChains(state).map((c) => `${c.meta.label} (chainId ${c.meta.chainId})`)
}
