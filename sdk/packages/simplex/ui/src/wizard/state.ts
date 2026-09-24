import { DEFAULT_ORDERBOOK_URLS } from "@/config/defaults"
import { normalizeSymbol } from "@/config/asset-registry"
import { vaultRowsToToml, type VaultRowDraft } from "../lib/vault-rows"
import type { EndpointVerificationState } from "../components/EndpointVerificationStatus"
import type { ChainDefault, FillerConfig, PairConfig, SetupDefaults, SetupOrderbook } from "../types"

export interface ChainDraft {
	meta: ChainDefault
	enabled: boolean
	rpcUrls: string[]
	bundlerUrl: string
	viaAlchemy: boolean
	watchOnly: boolean
	verificationState?: EndpointVerificationState
	verificationMessage?: string
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
	/** The orderbook's books, read when the wizard opens; the markets come from these. */
	orderbook?: SetupOrderbook
	orderbookError?: string
	vaults: VaultDraft[]
	allowlistUsers: string[]
	maxConcurrentOrders: string
	logging: string
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
				// Public endpoints where the registry has them, so the operator only
				// has to supply a bundler. One empty field otherwise, to type into.
				rpcUrls: meta.defaultRpcUrls?.length ? [...meta.defaultRpcUrls] : [""],
				bundlerUrl: "",
				viaAlchemy: false,
				watchOnly: false,
			})),
		vaults: [],
		allowlistUsers: [],
		maxConcurrentOrders: String(defaults.maxConcurrentOrders),
		logging: "info",
	}
}

export function enabledChains(state: WizardState): ChainDraft[] {
	return state.chains.filter((c) => c.enabled)
}

/**
 * The markets the config declares: every book the orderbook lists whose two
 * assets are each deployed on some enabled chain.
 *
 * A pair only says a market exists; what the filler pays there comes from a
 * limit order, so declaring every book costs nothing. Declaring none would
 * leave boot with no trading engine, and no market could be added at runtime.
 */
export function orderbookPairs(state: WizardState, defaults: SetupDefaults): PairConfig[] {
	// Registry spelling by normalized symbol: books write "cNGN", the registry "CNGN".
	const available = new Map(
		enabledChains(state).flatMap((chain) =>
			(defaults.knownTokens[chain.meta.stateMachineId] ?? []).map(
				(token) => [normalizeSymbol(token.symbol), token.symbol] as const,
			),
		),
	)
	return (state.orderbook?.books ?? []).flatMap((book) => {
		const token0 = available.get(normalizeSymbol(book.base))
		const token1 = available.get(normalizeSymbol(book.quote))
		return token0 && token1 ? [{ token0, token1 }] : []
	})
}

/** Users paste keys with and without the 0x prefix; normalize instead of nagging. */
export function normalizeHexKey(key: string): string {
	const trimmed = key.trim()
	return trimmed && !trimmed.startsWith("0x") ? `0x${trimmed}` : trimmed
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

	const pairs = orderbookPairs(state, defaults)

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
		pairs,
		orderbook: { url: state.orderbook?.url ?? DEFAULT_ORDERBOOK_URLS.mainnet },
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
