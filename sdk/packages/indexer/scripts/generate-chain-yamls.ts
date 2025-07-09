#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

import Handlebars from "handlebars"
import { RpcWebSocketClient } from "rpc-websocket-client"
import { Hex, hexToNumber } from "viem"

import { type Configuration, getEnv, getValidChains } from "../src/configs"

const root = process.cwd()
const currentEnv = getEnv()
const validChains = getValidChains()

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Load and compile templates
const templatesDir = path.join(__dirname, "templates")
const partialsDir = path.join(templatesDir, "partials")

// Register partials
Handlebars.registerPartial("handlers", fs.readFileSync(path.join(partialsDir, "handlers.hbs"), "utf8"))
Handlebars.registerPartial("metadata", fs.readFileSync(path.join(partialsDir, "metadata.hbs"), "utf8"))
Handlebars.registerPartial("network-config", fs.readFileSync(path.join(partialsDir, "network-config.hbs"), "utf8"))

// Compile templates
const substrateTemplate = Handlebars.compile(
	fs.readFileSync(path.join(templatesDir, "substrate-chain.yaml.hbs"), "utf8"),
)
const evmTemplate = Handlebars.compile(fs.readFileSync(path.join(templatesDir, "evm-chain.yaml.hbs"), "utf8"))
const multichainTemplate = Handlebars.compile(fs.readFileSync(path.join(templatesDir, "multichain.yaml.hbs"), "utf8"))

const EVM_TRACKED = [
	// Envrionment Variable Tracked
	"COIN_GECKGO_API_KEY",
] as const

const getChainTypesPath = (chain: string) => {
	// Extract base chain name before the hyphen
	const baseChainName = chain.split("-")[0]
	const potentialPath = `./dist/substrate-chaintypes/${baseChainName}.js`

	// Check if file exists
	if (fs.existsSync(potentialPath)) {
		return potentialPath
	}
	return null
}

const generateEndpoints = (chain: string) => {
	const envKey = chain.replace(/-/g, "_").toUpperCase()
	// Expect comma-separated endpoints in env var
	return process.env[envKey]?.split(",") || []
}

// Generate chain-specific YAML files
const generateSubstrateYaml = async (chain: string, config: Configuration) => {
	const chainTypesConfig = getChainTypesPath(chain)
	const endpoints = generateEndpoints(chain)

	// Expect comma-separated endpoints in env var
	const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
	const rpc = new RpcWebSocketClient()
	await rpc.connect(rpcUrl as string)
	const header = (await rpc.call("chain_getHeader", [])) as { number: Hex }
	const blockNumber = currentEnv === "local" ? hexToNumber(header.number) : config.startBlock

	// Check if this is a Hyperbridge chain (stateMachineId is KUSAMA-4009 or POLKADOT-3367)
	const isHyperbridgeChain = ["KUSAMA-4009", "POLKADOT-3367"].includes(config.stateMachineId)

	const templateData = {
		name: `${chain}-chain`,
		description: `${chain.charAt(0).toUpperCase() + chain.slice(1)} Chain Indexer`,
		runner: {
			node: {
				name: "@subql/node",
				version: ">=4.0.0",
			},
		},
		config,
		endpoints,
		chainTypesConfig,
		blockNumber,
		isHyperbridgeChain,
		handlerKind: "substrate/EventHandler",
		handlers: [
			{ handler: "handleIsmpStateMachineUpdatedEvent", module: "ismp", method: "StateMachineUpdated" },
			{ handler: "handleSubstrateRequestEvent", module: "ismp", method: "Request" },
			{ handler: "handleSubstrateResponseEvent", module: "ismp", method: "Response" },
			{ handler: "handleSubstratePostRequestHandledEvent", module: "ismp", method: "PostRequestHandled" },
			{ handler: "handleSubstratePostResponseHandledEvent", module: "ismp", method: "PostResponseHandled" },
			{
				handler: "handleSubstratePostRequestTimeoutHandledEvent",
				module: "ismp",
				method: "PostRequestTimeoutHandled",
			},
			{ handler: "handleSubstrateGetRequestHandledEvent", module: "ismp", method: "GetRequestHandled" },
			{
				handler: "handleSubstrateGetRequestTimeoutHandledEvent",
				module: "ismp",
				method: "GetRequestTimeoutHandled",
			},
			{
				handler: "handleSubstratePostResponseTimeoutHandledEvent",
				module: "ismp",
				method: "PostResponseTimeoutHandled",
			},
		],
	}

	return substrateTemplate(templateData)
}

const generateEvmYaml = async (chain: string, config: Configuration) => {
	const endpoints = generateEndpoints(chain)

	// Expect comma-separated endpoints in env var
	const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
	const response = await fetch(rpcUrl as string, {
		method: "POST",
		headers: {
			accept: "application/json",
			"content-type": "application/json",
		},
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "eth_blockNumber",
		}),
	})
	const data = await response.json()
	const blockNumber = currentEnv === "local" ? hexToNumber(data.result) : config.startBlock

	const templateData = {
		name: chain,
		description: `${chain.charAt(0).toUpperCase() + chain.slice(1)} Indexer`,
		runner: {
			node: {
				name: "@subql/node-ethereum",
				version: ">=3.0.0",
			},
		},
		config,
		endpoints,
		blockNumber,
		handlerKind: "ethereum/LogHandler",
		handlers: [
			{ handler: "handleStateMachineUpdatedEvent", topics: ["StateMachineUpdated(string,uint256)"] },
			{
				handler: "handlePostRequestEvent",
				topics: ["PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)"],
			},
			{
				handler: "handlePostResponseEvent",
				topics: ["PostResponseEvent(string,string,address,bytes,uint256,uint256,bytes,bytes,uint256,uint256)"],
			},
			{ handler: "handlePostRequestHandledEvent", topics: ["PostRequestHandled(bytes32,address)"] },
			{ handler: "handlePostResponseHandledEvent", topics: ["PostResponseHandled(bytes32,address)"] },
			{ handler: "handlePostRequestTimeoutHandledEvent", topics: ["PostRequestTimeoutHandled(bytes32,string)"] },
			{
				handler: "handlePostResponseTimeoutHandledEvent",
				topics: ["PostResponseTimeoutHandled(bytes32,string)"],
			},
			{
				handler: "handleGetRequestEvent",
				topics: ["GetRequestEvent(string,string,address,bytes[],uint256,uint256,uint256,bytes,uint256)"],
			},
			{ handler: "handleGetRequestHandledEvent", topics: ["GetRequestHandled(bytes32,address)"] },
			{ handler: "handleGetRequestTimeoutHandledEvent", topics: ["GetRequestTimeoutHandled(bytes32,string)"] },
		],
	}

	return evmTemplate(templateData)
}

async function generateAllChainYamls() {
	for (const [chain, config] of validChains) {
		const yaml =
			config.type === "substrate"
				? await generateSubstrateYaml(chain, config)
				: await generateEvmYaml(chain, config)

		fs.writeFileSync(root + `/src/configs/${chain}.yaml`, yaml)
		console.log(`Generated ${root}/src/configs/${chain}.yaml`)
	}
}

const generateMultichainYaml = () => {
	const projects = Array.from(validChains.keys()).map((chain) => `./${chain}.yaml`)

	const templateData = {
		projects,
	}

	const yaml = multichainTemplate(templateData)
	fs.writeFileSync(root + "/src/configs/subquery-multichain.yaml", yaml)
	console.log("Generated subquery-multichain.yaml")
}

const generateChainIdsByGenesis = () => {
	const chainIdsByGenesis = {}

	validChains.forEach((config) => {
		if (config.chainId) {
			chainIdsByGenesis[config.chainId] = config.stateMachineId
		}
	})

	const chainIdsByGenesisContent = `// Auto-generated, DO NOT EDIT \nexport const CHAIN_IDS_BY_GENESIS = ${JSON.stringify(chainIdsByGenesis, null, 2)}`

	fs.writeFileSync(root + "/src/chain-ids-by-genesis.ts", chainIdsByGenesisContent)
	console.log("Generated chain-ids-by-genesis.ts")
}

const generateChainsByIsmpHost = () => {
	const chainsByIsmpHost = {}

	validChains.forEach((config) => {
		// Only include EVM chains with ethereumHost contract
		if (config.type === "evm" && config.contracts?.ethereumHost) {
			chainsByIsmpHost[config.stateMachineId] = config.contracts.ethereumHost
		}
	})

	const chainsByIsmpHostContent = `// Auto-generated, DO NOT EDIT \nexport const CHAINS_BY_ISMP_HOST = ${JSON.stringify(chainsByIsmpHost, null, 2)}`

	fs.writeFileSync(root + "/src/chains-by-ismp-host.ts", chainsByIsmpHostContent)
	console.log("Generated chains-by-ismp-host.ts")
}

const generateEnvironmentConfig = () => {
	const configurations = {}

	// Set evm and substrate environment configurations
	validChains.forEach((config, chain) => {
		const envKey = chain.replace(/-/g, "_").toUpperCase()
		const endpoints = process.env[envKey]?.split(",") || []

		if (endpoints.length > 0) {
			configurations[config.stateMachineId] = endpoints[0].trim()
		}
	})

	EVM_TRACKED.forEach((e: string) => (configurations[e] = process.env?.[e] ?? null))

	fs.writeFileSync(root + "/src/env-config.json", JSON.stringify(configurations, null, 2))
	console.log("Generated env-config.json")
}

try {
	await generateAllChainYamls()
	generateMultichainYaml()
	generateChainIdsByGenesis()
	generateChainsByIsmpHost()
	generateEnvironmentConfig()
	process.exit(0)
} catch (err) {
	console.error("Error generating YAMLs:", err)
	process.exit(1)
}
