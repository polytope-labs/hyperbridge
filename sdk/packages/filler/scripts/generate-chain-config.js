import fs from "fs"
import path from "path"
import { fileURLToPath } from "url"
import toml from "@iarna/toml"

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const root = path.resolve(__dirname, "..")
const configPath = path.resolve(root, "src/config/chains.toml")
const outputPath = path.resolve(root, "src/config/chain.ts")

const config = toml.parse(fs.readFileSync(configPath, "utf-8"))

const chainsEnum = Object.keys(config.chains)
	.map((chain) => `\t${chain.toUpperCase().replace(/-/g, "_")} = "${config.chains[chain].stateMachineId}"`)
	.join(",\n")

const chainIds = Object.entries(config.chains)
	.map(([chain, data]) => {
		const chainId = parseInt(data.stateMachineId.split("-")[1])
		return `\t[Chains.${chain.toUpperCase().replace(/-/g, "_")}]: ${chainId}`
	})
	.join(",\n")

const chainNameMap = {
	bsc: "bscTestnet",
	gnosis: "gnosisChiado",
	sepolia: "sepolia",
}

const viemChains = Object.entries(config.chains)
	.filter(([_, data]) => data.type === "evm")
	.map(([chain, data]) => {
		const chainType = chain.split("-")[0]
		const chainId = parseInt(data.stateMachineId.split("-")[1])
		return `\t"${chainId}": ${chainNameMap[chainType]}`
	})
	.join(",\n")

const wrappedNativeDecimals = Object.entries(config.chains)
	.filter(([_, data]) => data.wrappedNativeDecimals)
	.map(([chain, data]) => `\t[Chains.${chain.toUpperCase().replace(/-/g, "_")}]: ${data.wrappedNativeDecimals}`)
	.join(",\n")

const assets = Object.entries(config.chains)
	.map(([chain, data]) => {
		if (!data.assets) return ""
		const assetEntries = Object.entries(data.assets)
			.map(([asset, address]) => `\t\t${asset}: "${address}".toLowerCase()`)
			.join(",\n")
		return `\t[Chains.${chain.toUpperCase().replace(/-/g, "_")}]: {\n${assetEntries}\n\t}`
	})
	.filter(Boolean)
	.join(",\n")

const addressesByContract = {}
Object.entries(config.chains).forEach(([chain, data]) => {
	if (data.addresses) {
		Object.entries(data.addresses).forEach(([contract, address]) => {
			if (!addressesByContract[contract]) {
				addressesByContract[contract] = {}
			}
			addressesByContract[contract][chain] = address
		})
	}
})

const addresses = Object.entries(addressesByContract)
	.map(([contract, chainAddresses]) => {
		const addressEntries = Object.entries(chainAddresses)
			.map(([chain, address]) => `\t\t[Chains.${chain.toUpperCase().replace(/-/g, "_")}]: "${address}"`)
			.join(",\n")
		return `\t${contract}: {\n${addressEntries}\n\t}`
	})
	.join(",\n")

const rpcUrls = Object.entries(config.chains)
	.map(([chain, data]) => {
		const chainEnum = `Chains.${chain.toUpperCase().replace(/-/g, "_")}`
		if (typeof data.rpcUrl === "object") {
			return `\t[${chainEnum}]: env.${data.rpcUrl.env} || "${data.rpcUrl.url}"`
		}
		return `\t[${chainEnum}]: env.${data.rpcUrl} || ""`
	})
	.join(",\n")

const consensusStateIds = Object.entries(config.chains)
	.map(([chain, data]) => `\t[Chains.${chain.toUpperCase().replace(/-/g, "_")}]: "${data.consensusStateId}"`)
	.join(",\n")

const tsContent = `
import { Chain, bscTestnet, gnosisChiado, sepolia } from "viem/chains"

/**
 * Enum representing different chains.
 */
export enum Chains {
${chainsEnum}
}

type AddressMap = {
	[key: string]: {
		[K in Chains]?: \`0x\${string}\`
	}
}

type RpcMap = {
	[K in Chains]?: string
}

/**
 * Mapping of chain IDs for different chains.
 */
export const chainIds = {
${chainIds}
} as const

export type ChainId = typeof chainIds

/**
 * Mapping of Viem Chain objects for different chains.
 */
export const viemChains: Record<string, Chain> = {
${viemChains}
}

export const WrappedNativeDecimals = {
${wrappedNativeDecimals}
}

/**
 * Mapping of assets for different chains.
 */
export const assets = {
${assets}
}

/**
 * Mapping of addresses for different contracts.
 */
export const addresses: AddressMap = {
${addresses}
}

/**
 * Creates an RPC URLs map from environment variables
 */
export const createRpcUrls = (env: NodeJS.ProcessEnv) => ({
${rpcUrls}
})

export const consensusStateIds = {
${consensusStateIds}
}
`

// Write the generated file
fs.writeFileSync(outputPath, tsContent)
console.log(`Generated ${outputPath}`)
