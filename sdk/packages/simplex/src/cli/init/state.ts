import type { FillerConfigFile, FillerTomlConfig } from "@/config/filler-toml"
import type { PairConfig } from "@/config/pairs"
import type { SignerConfig } from "@/services/wallet"
import type { UserProvidedChainConfig } from "@/services/FillerConfigService"
import type { InitChainMeta, InitNetwork } from "./chains"

export interface WizardChain {
	meta: InitChainMeta
	rpcUrls: string[]
	bundlerUrl?: string
}

export interface WizardState {
	network: InitNetwork
	chains: WizardChain[]
	/** Chains from an existing config the wizard doesn't manage; appended to the output unchanged. */
	passthroughChains: UserProvidedChainConfig[]
	/**
	 * The parsed existing config on an update run. assembleConfig overlays the
	 * wizard-managed fields onto a copy of this, so sections the wizard never
	 * prompts for (binance, keeper, targetGasUnits, watchOnly, …) survive verbatim.
	 */
	prefillConfig?: FillerConfigFile
	signer?: SignerConfig
	substratePrivateKey?: string
	hyperbridgeWsUrl?: string
	pairs: PairConfig[]
	/** `[assets]` escape hatch entries created for custom exotic tokens. */
	assets?: FillerTomlConfig["assets"]
	/** Top-level per-chain confirmation policies. */
	confirmationPolicies?: FillerTomlConfig["confirmationPolicies"]
	/**
	 * Uniswap V4 venue block chosen in the pairs step. Owned by the wizard:
	 * assembleConfig replaces any prefill `[vault.uniswapV4]` with this (or
	 * drops it when the operator switched to curve pricing).
	 */
	vaultUniswapV4?: NonNullable<FillerTomlConfig["vault"]>["uniswapV4"]
	maxConcurrentOrders: number
	logging?: string
	gasFeeBump?: FillerTomlConfig["simplex"]["gasFeeBump"]
	overfillProtection?: FillerTomlConfig["simplex"]["overfillProtection"]
	rebalancing?: FillerTomlConfig["rebalancing"]
	vault?: FillerTomlConfig["vault"]
	allowlist?: FillerTomlConfig["allowlist"]
}

/** Existing config being updated, with chain ids resolved from its RPCs. */
export interface Prefill {
	config: FillerConfigFile
	/** chainId per config.chains entry; null when the RPC could not be resolved. */
	chainIds: Array<number | null>
}

import { DEFAULT_MAX_CONCURRENT_ORDERS } from "@/config/defaults"
export { DEFAULT_MAX_CONCURRENT_ORDERS }

/** Ask prices below par by order size — the gap to 1 is the spread on every fill. */
export const DEFAULT_SAME_ASSET_ASK_CURVE = [
	{ amount: "100", price: "0.99" },
	{ amount: "1000", price: "0.995" },
	{ amount: "10000", price: "0.9975" },
	{ amount: "100000", price: "0.999" },
]

/** Low-value testnet default; testnet chain ids have no built-in confirmation policy. */
export const TESTNET_CONFIRMATION_POINTS = [
	{ amount: "100", value: 1 },
	{ amount: "10000", value: 2 },
]

export function newWizardState(): WizardState {
	return {
		network: "mainnet",
		chains: [],
		passthroughChains: [],
		pairs: [],
		maxConcurrentOrders: DEFAULT_MAX_CONCURRENT_ORDERS,
	}
}
