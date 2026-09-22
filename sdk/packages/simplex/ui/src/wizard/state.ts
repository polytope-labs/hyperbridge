import { isRegistrySymbol, normalizeSymbol } from "@/config/asset-registry"
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

/** One market the filler will quote. What it pays there comes from a limit order. */
export interface PairDraft {
	enabled: boolean
	token0: string
	token1: string
	/** UI mode of the symbol pickers — lives on the draft so it survives row reordering. */
	custom0?: boolean
	custom1?: boolean
}

export const normSymbol = normalizeSymbol

export interface WizardState {
	network: "mainnet"
	signerType: SignerType
	signerKey: string
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
	return { enabled: true, token0, token1 }
}

export function initialState(defaults: SetupDefaults): WizardState {
	return {
		network: "mainnet",
		signerType: "privateKey",
		signerKey: "",
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

	const pairs: PairConfig[] = enabledPairs(state).map((draft) => ({
		token0: draft.token0,
		token1: draft.token1,
	}))

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
