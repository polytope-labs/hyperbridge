import type { FillerTomlConfig, StrategyConfig, QueueConfig } from "@/config/filler-toml"
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
	prefillConfig?: FillerTomlConfig
	signer?: SignerConfig
	substratePrivateKey?: string
	hyperbridgeWsUrl?: string
	strategies: StrategyConfig[]
	maxConcurrentOrders: number
	queue: QueueConfig
	logging?: string
	gasFeeBump?: FillerTomlConfig["simplex"]["gasFeeBump"]
	overfillProtection?: FillerTomlConfig["simplex"]["overfillProtection"]
	rebalancing?: FillerTomlConfig["rebalancing"]
	vault?: FillerTomlConfig["vault"]
	allowlist?: FillerTomlConfig["allowlist"]
}

/** Existing config being updated, with chain ids resolved from its RPCs. */
export interface Prefill {
	config: FillerTomlConfig
	/** chainId per config.chains entry; null when the RPC could not be resolved. */
	chainIds: Array<number | null>
}

export const DEFAULT_MAX_CONCURRENT_ORDERS = 5
export const DEFAULT_QUEUE: QueueConfig = { maxRechecks: 10, recheckDelayMs: 30000 }

export const DEFAULT_STABLE_BPS_CURVE = [
	{ amount: "100", value: 100 },
	{ amount: "1000", value: 50 },
	{ amount: "10000", value: 25 },
	{ amount: "100000", value: 10 },
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
		strategies: [],
		maxConcurrentOrders: DEFAULT_MAX_CONCURRENT_ORDERS,
		queue: { ...DEFAULT_QUEUE },
	}
}
