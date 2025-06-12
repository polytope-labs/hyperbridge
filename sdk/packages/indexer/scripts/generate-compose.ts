#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

import Handlebars from "handlebars"
import { getEnv, getValidChains } from "../src/configs"

const EVM_IMAGE = "subquerynetwork/subql-node-ethereum:v5.5.0"
const SUBSTRATE_IMAGE = "subquerynetwork/subql-node-substrate:v5.9.1"

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
			unfinalizedBlocks: config.type === "evm", // Only EVM chains need unfinalized blocks handling
			config,
			volumesPath: "../../",
		}

		const yaml = serviceTemplate(serviceData)

		const filePath = path.join(dockerDir, `${chainName}.yml`)

		if (!fs.existsSync(filePath)) {
			fs.writeFileSync(filePath, yaml)
			console.log(`Generated ${filePath}`)
		} else {
			console.log(`Skipping ${filePath} - File already exists`)
		}
	})
}

generateNodeServices()
