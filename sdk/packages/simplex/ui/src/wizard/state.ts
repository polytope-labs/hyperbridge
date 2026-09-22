import { isRegistrySymbol, normalizeSymbol } from "@/config/asset-registry"
import { toPricePoints, type EditorPoint } from "../components/curveModel"
import { vaultRowsToToml, type VaultRowDraft } from "../lib/vault-rows"
import type { ChainDefault, FillerConfig, PairConfig, SetupDefaults } from "../types"

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

export type VaultDraft = VaultRowDraft

export type SignerType = "privateKey" | "mpcVault" | "turnkey"

export type SignerKeyValidation = "empty" | "invalid" | "checking" | "valid" | "error"

export const EVM_PRIVATE_KEY_FORMAT_ERROR = "Enter a valid 64-character hexadecimal EVM private key."
export const EVM_PRIVATE_KEY_INVALID_ERROR = "Invalid EVM private key."

export function privateKeyFormatError(value: string): string | undefined {
	const trimmed = value.trim()
	if (!trimmed) return "Enter the EVM private key."
	return /^(0x)?[0-9a-fA-F]{64}$/.test(trimmed) ? undefined : EVM_PRIVATE_KEY_FORMAT_ERROR
}

/** One cross-asset trading market or reference-only price feed. */
export interface PairDraft {
	enabled: boolean
	token0: string
	token1: string
	maxOrderSize: string
	/** Price feed only: anchors token1 in USD without opening a market. */
	referenceOnly?: boolean
	/** UI mode of the symbol pickers — lives on the draft so it survives row reordering. */
	custom0?: boolean
	custom1?: boolean
	bidEnabled: boolean
	askEnabled: boolean
	bid: EditorPoint[]
	ask: EditorPoint[]
}

export const normSymbol = normalizeSymbol

export interface WizardState {
	network: "mainnet"
	signerType: SignerType
	signerKey: string
	signerKeyValidation: SignerKeyValidation
	signerKeyValidationMessage?: string
	signerAddress?: string
	mpcVault: {
		apiToken: string
		vaultUuid: string
		accountAddress: string
		callbackClientSignerPublicKey: string
		grpcTarget: string
	}
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
	/** The markets step seeds one FX market draft on first visit; removal must stick. */
	fxSeeded?: boolean
	/** `[assets]` entries for custom token symbols: symbol → state machine id → address. */
	customAssets: Record<string, Record<string, string>>
	vaults: VaultDraft[]
	allowlistUsers: string[]
	maxConcurrentOrders: string
	logging: string
}

export function newCrossAssetDraft(token1: string, token0 = "USDC"): PairDraft {
	return {
		enabled: true,
		token0,
		token1,
		maxOrderSize: "",
		bidEnabled: true,
		askEnabled: true,
		bid: [{ amount: "1", value: "" }],
		ask: [{ amount: "1", value: "" }],
	}
}

/** A curve editor whose filled points parse into a policy the engine accepts. */
export function curveFilled(points: EditorPoint[], check: (v: number) => boolean = (v) => v > 0): boolean {
	const filled = points.filter((p) => p.amount.trim() && p.value.trim())
	return filled.length > 0 && filled.every((p) => Number(p.amount) >= 0 && check(Number(p.value)))
}

/**
 * Whether a draft will contribute a curve edge to the emitted config —
 * mirrors what assembleConfig emits, so the anchor check and the step
 * validation agree with the server by construction.
 */
export function draftHasCurve(draft: PairDraft): boolean {
	if (draft.referenceOnly) return curveFilled(draft.ask)
	return (draft.bidEnabled && curveFilled(draft.bid)) || (draft.askEnabled && curveFilled(draft.ask))
}

/** A reference-only <stable>/<symbol> price feed, inserted by the anchor helper. */
export function newReferenceDraft(token1: string, token0: string): PairDraft {
	return {
		enabled: true,
		token0,
		token1,
		maxOrderSize: "",
		referenceOnly: true,
		bidEnabled: false,
		askEnabled: true,
		bid: [],
		ask: [{ amount: "0", value: "" }],
	}
}

export function initialState(defaults: SetupDefaults): WizardState {
	return {
		network: "mainnet",
		signerType: "privateKey",
		signerKey: "",
		signerKeyValidation: "empty",
		mpcVault: {
			apiToken: "",
			vaultUuid: "",
			accountAddress: "",
			callbackClientSignerPublicKey: "",
			grpcTarget: "",
		},
		turnkey: { organizationId: "", apiPublicKey: "", apiPrivateKey: "", signWith: "" },
		substrateKey: "",
		hyperbridgeWsUrl: defaults.hyperbridgeWs.mainnet,
		alchemyKey: "",
		chains: defaults.chains.map((meta) => ({
				meta,
				enabled: false,
				rpcUrls: [""],
				bundlerUrl: "",
				viaAlchemy: false,
				watchOnly: false,
			})),
		pairs: [],
		customAssets: {},
		vaults: [],
		allowlistUsers: [],
		maxConcurrentOrders: String(defaults.maxConcurrentOrders),
		logging: "info",
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

	const pairs: PairConfig[] = enabledPairs(state).map((draft) => {
		if (draft.referenceOnly) {
			return {
				token0: draft.token0,
				token1: draft.token1,
				referenceOnly: true,
				askPriceCurve: toPricePoints(draft.ask),
			}
		}
		const withBid = draft.bidEnabled
		const withAsk = draft.askEnabled
		return {
			token0: draft.token0,
			token1: draft.token1,
			// Omitted entirely when blank: the cap is optional, and an empty string
			// would fail config validation as a malformed decimal rather than read
			// as "no cap".
			...(draft.maxOrderSize.trim() ? { maxOrderSize: draft.maxOrderSize.trim() } : {}),
			...(withBid ? { bidPriceCurve: toPricePoints(draft.bid) } : {}),
			...(withAsk ? { askPriceCurve: toPricePoints(draft.ask) } : {}),
		}
	})

	// Only [assets] entries actually referenced by a pair are emitted — and
	// never for registry symbols: an accidental override would silently repoint
	// e.g. USDC at an arbitrary contract. The wizard refuses shadowing outright.
	const usedSymbols = new Set(pairs.flatMap((p) => [p.token0, p.token1]))
	const assets = Object.fromEntries(
		Object.entries(state.customAssets)
			.filter(([symbol]) => usedSymbols.has(symbol) && !isRegistrySymbol(symbol))
			.map(([symbol, byChain]) => [
				symbol,
				Object.fromEntries(Object.entries(byChain).filter(([, address]) => address.trim())),
			]),
	)

	const vaultRows = vaultRowsToToml(state.vaults)
	const vaults = vaultRows.length > 0 ? vaultRows : undefined

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
			// The form collects plain strings; the config type wants the signer union / hex addresses.
			signer: signer as FillerConfig["simplex"]["signer"],
			maxConcurrentOrders: Number(state.maxConcurrentOrders) || defaults.maxConcurrentOrders,
			...(state.logging !== "info" ? { logging: state.logging } : {}),
			...(watchOnlyEntries.length > 0 ? { watchOnly: Object.fromEntries(watchOnlyEntries) } : {}),
			substratePrivateKey: state.substrateKey.trim(),
			hyperbridgeWsUrl: state.hyperbridgeWsUrl.trim(),
		},
		...(Object.keys(assets).length > 0 ? { assets: assets as FillerConfig["assets"] } : {}),
		pairs,
		chains: chains.map((c) => ({
			rpcUrls: c.rpcUrls.map((u) => u.trim()).filter(Boolean),
			bundlerUrl: c.bundlerUrl.trim(),
		})),
		...(vaults ? { vault: { vaults } } : {}),
		...(allowlistUsers.length > 0 ? { allowlist: { users: allowlistUsers } } : {}),
	}
}

export function chainLabels(state: WizardState): string[] {
	return enabledChains(state).map((c) => `${c.meta.label} (chainId ${c.meta.chainId})`)
}
