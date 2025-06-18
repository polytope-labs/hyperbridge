import fs from "node:fs"
import path from "node:path"

import dotenv from "dotenv"
import { z } from "zod"

// EVM contract addresses schema
const evmContractsSchema = z.object({
	ethereumHost: z.string().min(3, "Invalid Ethereum address"),
	handlerV1: z.string().min(3, "Invalid Ethereum address"),
	erc6160ext20: z.string().min(3, "Invalid Ethereum address"),
	intentGateway: z.string(),
	tokenGateway: z.string(),
})

// Base chain configuration schema
const baseChainConfigSchema = z.object({
	chainId: z.string(),
	startBlock: z.number().int().min(0),
	stateMachineId: z.string().min(3, "Invalid state machine ID format"),
})

// Configuration object schema (maps chain name to chain config)
export const schemaConfiguration = z.record(
	z.string(),
	z.discriminatedUnion("type", [
		baseChainConfigSchema.extend({ type: z.literal("substrate") }),
		baseChainConfigSchema.extend({ type: z.literal("evm"), contracts: evmContractsSchema.optional() }),
	]),
)

export type ConfigObject = z.infer<typeof schemaConfiguration>
export type Configuration = ConfigObject[0]
export type Environment = "local" | "testnet" | "mainnet"

/**
 * Validate configuration for the configuration structure and validate the schema.
 * @param config
 * @returns
 */
function validateConfig(config: unknown): ConfigObject {
	return schemaConfiguration.parse(config)
}

/**
 * Load configuration for the specified environment
 * @param env - The environment to load configuration for
 * @param rootPath - Optional root path, defaults to process.cwd()
 * @returns The validated configuration
 * @throws {z.ZodError} If the configuration is invalid
 * @throws {Error} If the configuration file cannot be read
 */
function loadConfig(env: Environment): ConfigObject {
	const root = process.cwd()
	dotenv.config({ path: path.resolve(root, `../../.env.${env}`) })

	const configPath = path.join(root, `src/configs/config-${env}.json`)

	try {
		const rawConfig = JSON.parse(fs.readFileSync(configPath, "utf8"))
		const config = validateConfig(rawConfig)
		// const environment = env
		return config
	} catch (error) {
		if (error instanceof z.ZodError) {
			throw new Error(
				`Invalid configuration for environment '${env}':\n${error.errors
					.map((e) => `  - ${e.path.join(".")}: ${e.message}`)
					.join("\n")}`,
			)
		}
		if (error instanceof Error && "code" in error && error.code === "ENOENT") {
			throw new Error(`Configuration file not found: ${configPath}`)
		}
		throw error
	}
}

export function getEnv(): Environment {
	const currentEnv = process.env.ENV as Environment
	if (!currentEnv) throw new Error("$ENV variable not set")

	return currentEnv
}

/**
 * Convenience function to get configuration for current environment
 * @returns ConfigObject
 */
export function getConfigs(): ConfigObject {
	const env = getEnv()
	return loadConfig(env)
}

export function getValidChains(): Map<string, Configuration> {
	const configs = getConfigs()

	let validChains = new Map<string, Configuration>()

	for (const [chain, config] of Object.entries(configs)) {
		const envKey = chain.replace(/-/g, "_").toUpperCase()
		const endpoint = process.env[envKey]

		if (!endpoint) {
			console.log(`Skipping ${chain}.yaml - No endpoint configured in environment`)
			continue
		}

		validChains.set(chain, config)
	}

	return validChains
}
