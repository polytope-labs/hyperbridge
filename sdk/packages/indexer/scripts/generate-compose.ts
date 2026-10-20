#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

import Handlebars from "handlebars"
import { getEnv, getValidChains } from "../src/configs"

const EVM_IMAGE = "polytopelabs/subql-node-ethereum:v6.5.0"
const SUBSTRATE_IMAGE = "polytopelabs/subql-node-substrate:v6.4.9-0"

// Setup paths
const root = process.cwd()
const currentEnv = getEnv()
const validChains = getValidChains()

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Load and compile templates
const templatesDir = path.join(__dirname, "templates")
const partialsDir = path.join(templatesDir, "partials")

// Register partials
Handlebars.registerPartial("docker-command", fs.readFileSync(path.join(partialsDir, "docker-command.hbs"), "utf8"))
Handlebars.registerPartial("docker-service", fs.readFileSync(path.join(partialsDir, "docker-service.hbs"), "utf8"))

// Compile templates
const serviceTemplate = Handlebars.compile(
	fs.readFileSync(path.join(templatesDir, "docker-compose-service.yaml.hbs"), "utf8"),
)

const generateNodeServices = () => {
	const dockerDir = path.join(root, "docker", currentEnv)
	if (!fs.existsSync(dockerDir)) {
		fs.mkdirSync(dockerDir, { recursive: true })
	}

	validChains.forEach((config, chainName) => {
		const serviceData = {
			chainName,
			image: config.type === "substrate" ? SUBSTRATE_IMAGE : EVM_IMAGE,
			// Only the substrate node (the fork image) applies in-place schema migrations; it is the
			// health-gated leader every EVM node waits on, so it owns the shared schema's DDL.
			allowSchemaMigration: config.type === "substrate",
			unfinalizedBlocks: config.type === "evm" && config.unfinalizedBlocks !== false,
			blockConfirmations: config.blockConfirmations, // Pass block confirmations from config
			config,
			volumesPath: "../../",
			dbSchema: "app",
		}

		writeService(dockerDir, serviceData)

		if (config.type === "substrate" && config.governanceStartBlock !== undefined) {
			writeService(dockerDir, {
				...serviceData,
				chainName: `${chainName}-governance`,
				dbSchema: "governance",
				// A standalone project with only event handlers over finalized blocks, so the node runs
				// without multichain metadata, extrinsic decoding or historical rows
				governance: true,
				backfill: config.governanceCutoverBlock !== undefined,
			})
		}
	})
}

const writeService = (dockerDir: string, serviceData: Record<string, unknown> & { chainName: string }) => {
	const filePath = path.join(dockerDir, `${serviceData.chainName}.yml`)

	if (!fs.existsSync(filePath)) {
		fs.writeFileSync(filePath, serviceTemplate(serviceData))
		console.log(`Generated ${filePath}`)
	} else {
		console.log(`Skipping ${filePath} - File already exists`)
	}
}

generateNodeServices()
