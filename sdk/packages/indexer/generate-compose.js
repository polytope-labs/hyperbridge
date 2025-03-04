#!/usr/bin/env node
const dotenv = require("dotenv")
const path = require("path")
const os = require("os")

const currentEnv = process.env.ENV
if (!currentEnv) throw new Error("$ENV variable not set")

const root = process.cwd()
dotenv.config({ path: path.resolve(root, `../../.env.${currentEnv}`) })

const fs = require("fs-extra")
const configs = require(`./configs/config-${currentEnv}.json`)

const SUBSTRATE_IMAGE = "subquerynetwork/subql-node-substrate:v5.9.1"
const EVM_IMAGE = "subquerynetwork/subql-node-ethereum:v5.5.0"

const generateNodeServices = () => {
	const unfinalized = `
      - --historical=timestamp
      - --block-confirmations=0
      - --unfinalized-blocks`

	Object.entries(configs)
		.filter(([chain]) => {
			const envKey = chain.replace(/-/g, "_").toUpperCase()
			return !!process.env[envKey]
		})
		.map(([chain, config]) => {
			const image = config.type === "substrate" ? SUBSTRATE_IMAGE : EVM_IMAGE
			const file = `services:
  subquery-${chain}:
    image: ${image}
    restart: unless-stopped
    environment:
      DB_USER: \${DB_USER}
      DB_PASS: \${DB_PASS}
      DB_DATABASE: \${DB_DATABASE}
      DB_HOST: \${DB_HOST}
      DB_PORT: \${DB_PORT}
    network_mode: host
    volumes:
      - ../../configs:/app
      - ../../dist:/app/dist
    command:
      - \${SUB_COMMAND:-}
      - -f=/app/${chain}.yaml
      - --db-schema=app
      - --workers=\${SUBQL_WORKERS:-${config.type === "substrate" ? "32" : "16"}}
      - --batch-size=\${SUBQL_BATCH_SIZE:-100}
      - --multi-chain
      - --unsafe
      - --log-level=info${config.type === "substrate" ? "" : unfinalized}
      - --store-cache-async=false
      - --store-cache-threshold=5
    healthcheck:
      test: ['CMD', 'curl', '-f', 'http://subquery-node-${chain}:3000/ready']
      interval: 3s
      timeout: 5s
      retries: 10`

			fs.outputFileSync(`docker/${currentEnv}/${chain}.yml`, file)
			console.log(`Generated docker/${currentEnv}/${chain}.yml`)
		})
}

generateNodeServices()
