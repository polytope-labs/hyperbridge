#!/usr/bin/env node
const dotenv = require("dotenv")
const path = require("path")

const currentEnv = process.env.ENV
if (!currentEnv) throw new Error("$ENV variable not set")

const root = process.cwd()
dotenv.config({ path: path.resolve(root, `../../.env.${currentEnv}`) })

const fs = require("fs")
const { RpcWebSocketClient } = require("rpc-websocket-client")
const { hexToNumber } = require("viem")
const configs = require(root + `/src/configs/config-${currentEnv}.json`)

const getChainTypesPath = (chain) => {
	// Extract base chain name before the hyphen
	const baseChainName = chain.split("-")[0]

	const potentialPath = `./dist/substrate-chaintypes/${baseChainName}.js`

	// Check if file exists
	if (fs.existsSync(potentialPath)) {
		return potentialPath
	}

	return null
}

const generateEndpoints = (chain) => {
	const envKey = chain.replace(/-/g, "_").toUpperCase()
	// Expect comma-separated endpoints in env var
	const endpoints = process.env[envKey]?.split(",") || []

	return endpoints.map((endpoint) => `    - '${endpoint.trim()}'`).join("\n")
}

// Generate chain-specific YAML files
const generateSubstrateYaml = async (chain, config) => {
	const chainTypesConfig = getChainTypesPath(chain)
	const endpoints = generateEndpoints(chain)

	// Expect comma-separated endpoints in env var
	const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
	const rpc = new RpcWebSocketClient()
	await rpc.connect(rpcUrl)
	const header = await rpc.call("chain_getHeader", [])
	const blockNumber = currentEnv === "local" ? hexToNumber(header.number) : config.startBlock
	const chainTypesSection = chainTypesConfig ? `\n  chaintypes:\n    file: ${chainTypesConfig}` : ""
	
	// Check if this is a Hyperbridge chain (stateMachineId is KUSAMA-4009 or POLKADOT-3367)
	const isHyperbridgeChain = config.stateMachineId === "KUSAMA-4009" || config.stateMachineId === "POLKADOT-3367"
	
	// Add AssetTeleported handler only for Hyperbridge chains
	const assetTeleportedHandler = isHyperbridgeChain ? `        - handler: handleSubstrateAssetTeleportedEvent
          kind: substrate/EventHandler
          filter:
            module: xcmGateway
            method: AssetTeleported` : ''

	return `# // Auto-generated , DO NOT EDIT
specVersion: 1.0.0
version: 0.0.1
name: ${chain}-chain
description: ${chain.charAt(0).toUpperCase() + chain.slice(1)} Chain Indexer
runner:
  node:
    name: '@subql/node'
    version: '>=4.0.0'
  query:
    name: '@subql/query'
    version: '*'
schema:
  file: ./schema.graphql
network:
  chainId: '${config.chainId}'
  endpoint:
${endpoints}${chainTypesSection}
dataSources:
  - kind: substrate/Runtime
    startBlock: ${blockNumber}
    mapping:
      file: ./dist/index.js
      handlers:
        - handler: handleIsmpStateMachineUpdatedEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: StateMachineUpdated
        - handler: handleSubstrateRequestEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: Request
        - handler: handleSubstrateResponseEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: Response
        - handler: handleSubstratePostRequestHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostRequestHandled
        - handler: handleSubstratePostResponseHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostResponseHandled
        - handler: handleSubstratePostRequestTimeoutHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostRequestTimeoutHandled
        - handler: handleSubstratePostResponseTimeoutHandledEvent
          kind: substrate/EventHandler
          filter:
            module: ismp
            method: PostResponseTimeoutHandled${assetTeleportedHandler ? '\n' + assetTeleportedHandler : ''}

repository: 'https://github.com/polytope-labs/hyperbridge'`
}

const generateEvmYaml = async (chain, config) => {
	const endpoints = generateEndpoints(chain)

	// Expect comma-separated endpoints in env var
	const rpcUrl = process.env[chain.replace(/-/g, "_").toUpperCase()]?.split(",")[0]
	const response = await fetch(rpcUrl, {
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

	return `# // Auto-generated , DO NOT EDIT
specVersion: 1.0.0
version: 0.0.1
name: ${chain}
description: ${chain.charAt(0).toUpperCase() + chain.slice(1)} Indexer
runner:
  node:
    name: '@subql/node-ethereum'
    version: '>=3.0.0'
  query:
    name: '@subql/query'
    version: '*'
schema:
  file: ./schema.graphql
network:
  chainId: '${config.chainId}'
  endpoint:
${endpoints}
dataSources:
  - kind: ethereum/Runtime
    startBlock: ${blockNumber}
    options:
      abi: ethereumHost
      address: '${config.contracts.ethereumHost}'
    assets:
      ethereumHost:
        file: ./abis/EthereumHost.abi.json
      chainLinkAggregatorV3:
        file: ./abis/ChainLinkAggregatorV3.abi.json
    mapping:
      file: ./dist/index.js
      handlers:
        - kind: ethereum/LogHandler
          handler: handleStateMachineUpdatedEvent
          filter:
            topics:
              - 'StateMachineUpdated(string,uint256)'
        - kind: ethereum/LogHandler
          handler: handlePostRequestEvent
          filter:
            topics:
              - 'PostRequestEvent(string,string,address,bytes,uint256,uint256,bytes,uint256)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseEvent
          filter:
            topics:
              - 'PostResponseEvent(string,string,address,bytes,uint256,uint256,bytes,bytes,uint256,uint256)'
        - kind: ethereum/LogHandler
          handler: handlePostRequestHandledEvent
          filter:
            topics:
              - 'PostRequestHandled(bytes32,address)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseHandledEvent
          filter:
            topics:
              - 'PostResponseHandled(bytes32,address)'
        - kind: ethereum/LogHandler
          handler: handlePostRequestTimeoutHandledEvent
          filter:
            topics:
              - 'PostRequestTimeoutHandled(bytes32,string)'
        - kind: ethereum/LogHandler
          handler: handlePostResponseTimeoutHandledEvent
          filter:
            topics:
              - 'PostResponseTimeoutHandled(bytes32,string)'
  - kind: ethereum/Runtime
    startBlock: ${blockNumber}
    options:
      abi: erc6160ext20
      address: '${config.contracts.erc6160ext20}'
    assets:
      erc6160ext20:
        file: ./abis/ERC6160Ext20.abi.json
    mapping:
      file: ./dist/index.js
      handlers:
        - kind: ethereum/LogHandler
          handler: handleTransferEvent
          filter:
            topics:
              - 'Transfer(address indexed from, address indexed to, uint256 amount)'
  # - kind: ethereum/Runtime
  #   startBlock: 21535312
  #   options:
  #     abi: handlerV1
  #     address: '0xA801da100bF16D07F668F4A49E1f71fc54D05177'
  #   assets:
  #     handlerV1:
  #       file: ./abis/HandlerV1.abi.json
  #   mapping:
  #     file: ./dist/index.js
  #     handlers:
  #       - handler: handlePostRequestTransactionHandler
  #         kind: ethereum/TransactionHandler
  #         function: >-
  #           handlePostRequests(address,(((uint256,uint256),bytes32[],uint256),((bytes,bytes,uint64,bytes,bytes,uint64,bytes),uint256,uint256)[]))
  #       - handler: handlePostResponseTransactionHandler
  #         kind: ethereum/TransactionHandler
  #         function: >-
  #           handlePostResponses(address,(((uint256,uint256),bytes32[],uint256),(((bytes,bytes,uint64,bytes,bytes,uint64,bytes),bytes,uint64),uint256,uint256)[]))

repository: 'https://github.com/polytope-labs/hyperbridge'`
}

const validChains = Object.entries(configs).filter(([chain, config]) => {
	const envKey = chain.replace(/-/g, "_").toUpperCase()
	const endpoint = process.env[envKey]

	if (!endpoint) {
		console.log(`Skipping ${chain}.yaml - No endpoint configured in environment`)
		return false
	}
	return true
})

async function generateAllChainYamls() {
	for (const [chain, config] of validChains) {
		const yaml =
			config.type === "substrate"
				? await generateSubstrateYaml(chain, config)
				: await generateEvmYaml(chain, config)

		const filePath = root + `/src/configs/${chain}.yaml`
		fs.writeFileSync(filePath, yaml)
		console.log(`Generated ${root}/src/configs/${chain}.yaml`)
	}
}

const generateMultichainYaml = () => {
	const projects = validChains.map(([chain]) => `  - ./${chain}.yaml`).join("\n")

	const yaml = `specVersion: 1.0.0
query:
  name: '@subql/query'
  version: '*'
projects:
${projects}`

	fs.writeFileSync(root + "/src/configs/subquery-multichain.yaml", yaml)
	console.log("Generated subquery-multichain.yaml")
}

const generateSubstrateWsJson = () => {
	const substrateWsConfig = {}

	validChains.forEach(([chain, config]) => {
		if (config.type === "substrate") {
			const envKey = chain.replace(/-/g, "_").toUpperCase()
			const endpoints = process.env[envKey]?.split(",") || []

			if (endpoints.length > 0) {
				substrateWsConfig[config.stateMachineId] = endpoints[0].trim()
			}
		}
	})

	fs.writeFileSync(root + "/src/substrate-ws.json", JSON.stringify(substrateWsConfig, null, 2))
	console.log("Generated substrate-ws.json")
}

generateAllChainYamls()
	.then(() => {
		generateMultichainYaml()
		generateSubstrateWsJson()
		process.exit(0)
	})
	.catch((err) => {
		console.error("Error generating YAMLs:", err)
		process.exit(1)
	})
