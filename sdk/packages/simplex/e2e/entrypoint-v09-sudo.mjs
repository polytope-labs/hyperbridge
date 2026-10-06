// EntryPoint v0.9 testnet rollout, governance side: the Gargantua sudo calls that register the
// SimplexPaymaster, upgrade the IntentGatewayV2 proxies and set or clear the paymaster's bundler
// allowlist. Every step that reaches an EVM chain polls it until the relayer has delivered.
//
// Usage: node e2e/entrypoint-v09-sudo.mjs <command> [--dry-run] [options]   (run with --help)
// The EVM steps and the full order: evm/script/testnet/entrypoint-v09.sh and the README beside it.
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { parseArgs } from "node:util"
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api"
import { u8aToHex } from "@polkadot/util"
import { cryptoWaitReady, keccakAsU8a } from "@polkadot/util-crypto"
import {
	concat,
	createPublicClient,
	encodeAbiParameters,
	encodeFunctionData,
	getAddress,
	hexToString,
	http,
	isAddress,
	toHex,
} from "viem"

const HERE = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(HERE, "../../../..")

const HYPERBRIDGE = "KUSAMA-4009"
const RELAYER = "0xc8809DD0b00370be097382d741A43347Ad582757"
// The rundler signer and the origin rundler simulates validation from.
const BUNDLERS = ["0x2E0E23636E27826193EA4A585a05CD1808d3c96c", "0x0643866dA50efE0b055Cd15aF95191968c8411b5"]
const ENTRYPOINT_V09 = "0x433709009B8330FDa32311DF1C2AFA402eD8D009"
const SELECT_SOLVER_TYPEHASH = "0xe706bdab7d945360dcd9d81d355f856754dd1cfa461edfc0a7502e2583b4e09e"
const IMPLEMENTATION_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
// `SimplexPaymaster.RequestKind.SetBundlers`
const SET_BUNDLERS = 8
const POLL_MS = 15_000

const CHAINS = {
	97: { name: "BSC Chapel", rpc: "BSC_TESTNET_RPC_URL", fallback: "BSC_CHAPEL" },
	80002: { name: "Polygon Amoy", rpc: "POLYGON_AMOY_RPC_URL", fallback: "POLYGON_AMOY" },
}

const CALLS = [
	"addPaymasterDeployment",
	"executeOnGateway",
	"upgradePaymaster",
	"upgradeGateway",
	"setPaymasterBundlers",
]

const postRequest = {
	name: "request",
	type: "tuple",
	components: [
		{ name: "source", type: "bytes" },
		{ name: "dest", type: "bytes" },
		{ name: "nonce", type: "uint64" },
		{ name: "from", type: "bytes" },
		{ name: "to", type: "bytes" },
		{ name: "timeoutTimestamp", type: "uint64" },
		{ name: "body", type: "bytes" },
	],
}

const view = (name, output) => ({
	type: "function",
	name,
	stateMutability: "view",
	inputs: [],
	outputs: [{ type: output }],
})

const paymasterAbi = [
	{
		type: "function",
		name: "onAccept",
		stateMutability: "nonpayable",
		inputs: [{ name: "incoming", type: "tuple", components: [postRequest, { name: "relayer", type: "address" }] }],
		outputs: [],
	},
	view("getBundlers", "address[]"),
	view("version", "uint64"),
	view("relayer", "address"),
	view("entryPoint", "address"),
	view("host", "address"),
]

const gatewayAbi = [
	{
		type: "function",
		name: "upgradeToAndCall",
		stateMutability: "nonpayable",
		inputs: [
			{ name: "newImplementation", type: "address" },
			{ name: "data", type: "bytes" },
		],
		outputs: [],
	},
	view("version", "uint64"),
	view("SELECT_SOLVER_TYPEHASH", "bytes32"),
]

const hostAbi = [view("hyperbridge", "bytes")]

const USAGE = `Usage: node e2e/entrypoint-v09-sudo.mjs <command> [options]

Commands:
  metadata            Arg shapes of the intentsCoprocessor calls on the live runtime, and what it stores
  init-data           The nested onAccept init_data for the bundler allowlist (no network)
  register-paymaster  4. addPaymasterDeployment(Evm(chain), SIMPLEX_PAYMASTER)
  upgrade-gateway     5. executeOnGateway(Evm(chain), upgradeToAndCall(INTENT_GATEWAY_V2_IMPL, "")),
                         then polls the gateway's implementation slot and SELECT_SOLVER_TYPEHASH()
  allow-bundlers      8. upgradePaymaster(Evm(chain), current implementation, onAccept(SetBundlers(list, true)))
  clear-bundlers      9. the same with allowed = false, then expects an empty getBundlers()

Options:
  --dry-run               Print each decoded call and its hex; sign and send nothing
  --chains 97,80002       Chain ids (default: both)
  --paymaster 0x..        Paymaster proxy (default: SIMPLEX_PAYMASTER in the config)
  --paymaster-impl 0x..   Implementation to re-point the proxy at (default: read from the proxy)
  --impl 0x..             Gateway implementation (default: INTENT_GATEWAY_V2_IMPL in the config)
  --bundlers 0x..,0x..    Allowlist (default: ${BUNDLERS.join(",")})
  --relayer 0x..          Relayer named in the nested request (default: ${RELAYER})
  --timeout-min N         How long to poll an EVM chain for the relayer (default: 30)
  --env-file PATH         Default: sdk/.env.local, read line by line, never sourced
  --config PATH           Default: evm/config.testnet.toml

Environment: HYPERBRIDGE_GARGANTUA, GARGANTUA_SUDO_SEED (seed of sudo.key(); live runs only),
BSC_TESTNET_RPC_URL or BSC_CHAPEL, POLYGON_AMOY_RPC_URL or POLYGON_AMOY. The process environment wins
over the env file.`

const { values: options, positionals } = parseArgs({
	allowPositionals: true,
	options: {
		"dry-run": { type: "boolean", default: false },
		chains: { type: "string", default: "97,80002" },
		paymaster: { type: "string" },
		"paymaster-impl": { type: "string" },
		impl: { type: "string" },
		bundlers: { type: "string" },
		relayer: { type: "string", default: RELAYER },
		"timeout-min": { type: "string", default: "30" },
		"env-file": { type: "string", default: path.join(REPO, "sdk/.env.local") },
		config: { type: "string", default: path.join(REPO, "evm/config.testnet.toml") },
		help: { type: "boolean", short: "h", default: false },
	},
})

// Keys and endpoints never reach the output, whatever an error message quotes. Longest first, so
// a value is never half-redacted by one it contains.
const SECRETS = []
const secret = (value) => {
	if (value && value.length > 8 && !SECRETS.includes(value)) {
		SECRETS.push(value)
		SECRETS.sort((a, b) => b.length - a.length)
	}
	return value
}
const redact = (text) => SECRETS.reduce((out, value) => out.split(value).join("***"), String(text))
for (const method of ["log", "info", "warn", "error"]) {
	const original = console[method].bind(console)
	console[method] = (...args) =>
		original(
			...args.map((arg) =>
				typeof arg === "string" || arg instanceof Error
					? redact(arg instanceof Error ? (arg.stack ?? arg.message) : arg)
					: arg,
			),
		)
}
const log = (line) => console.log(`${new Date().toISOString()} ${line}`)
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
const same = (a, b) => Boolean(a) && Boolean(b) && a.toLowerCase() === b.toLowerCase()
const dryRun = options["dry-run"]

/** `KEY=value` lines by exact key; the file holds lines that are not valid shell. */
function readEnvFile(file) {
	const entries = {}
	if (!fs.existsSync(file)) return entries
	for (const line of fs.readFileSync(file, "utf8").split(/\r?\n/)) {
		const match = /^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/.exec(line)
		if (!match || match[1] in entries) continue
		entries[match[1]] = match[2].trim().replace(/^(["'])(.*)\1$/, "$2")
	}
	return entries
}

const fileEnv = readEnvFile(options["env-file"])
const setting = (name, fallback = name) => process.env[name] || fileEnv[fallback]

/** `[<chainId>.address]` tables of the forge config; a zero address counts as unset. */
function readConfig(file) {
	const tables = {}
	let table
	for (const line of fs.readFileSync(file, "utf8").split(/\r?\n/)) {
		const header = /^\[(.+)\]\s*$/.exec(line)
		if (header) {
			table = tables[header[1]] = {}
			continue
		}
		const entry = /^\s*([A-Za-z0-9_]+)\s*=\s*"?([^"]*)"?\s*$/.exec(line)
		if (entry && table) table[entry[1]] = entry[2]
	}
	return (id, key) => {
		const value = tables[`${id}.address`]?.[key]
		return value && !/^0x0{40}$/i.test(value) ? getAddress(value) : undefined
	}
}

function address(value, label) {
	if (!value || !isAddress(value)) throw new Error(`${label} is not an address: ${value}`)
	return getAddress(value)
}

function evm(id) {
	const chain = CHAINS[id]
	const url = secret(setting(chain.rpc, chain.fallback))
	if (!url) throw new Error(`${chain.rpc} is unset and ${chain.fallback} is missing from the env file`)
	return createPublicClient({ transport: http(url) })
}

const read = (client, address, abi, functionName) => client.readContract({ address, abi, functionName })

async function implementationOf(client, proxy) {
	const word = await client.getStorageAt({ address: proxy, slot: IMPLEMENTATION_SLOT })
	return word && BigInt(word) !== 0n ? getAddress(`0x${word.slice(-40)}`) : undefined
}

/** `abi.encodeCall(SimplexPaymaster.onAccept, (IncomingPostRequest))` carrying a `SetBundlers` body. */
function bundlerInitData(bundlers, allowed, relayer) {
	const body = concat([
		toHex(SET_BUNDLERS, { size: 1 }),
		encodeAbiParameters([{ type: "address[]" }, { type: "bool" }], [bundlers, allowed]),
	])
	const request = {
		source: toHex(HYPERBRIDGE),
		dest: "0x",
		nonce: 0n,
		from: toHex("pallet-intents"),
		to: "0x",
		timeoutTimestamp: 0n,
		body,
	}
	return encodeFunctionData({ abi: paymasterAbi, functionName: "onAccept", args: [{ request, relayer }] })
}

async function connect() {
	const url = secret(setting("HYPERBRIDGE_GARGANTUA"))
	if (!url) throw new Error("HYPERBRIDGE_GARGANTUA is unset and missing from the env file")
	await cryptoWaitReady()
	const api = await ApiPromise.create({
		provider: new WsProvider(url),
		noInitWarn: true,
		// Hyperbridge hashes with keccak; the default blake2 registry signs and reads wrongly.
		typesBundle: { spec: { nexus: { hasher: keccakAsU8a }, gargantua: { hasher: keccakAsU8a } } },
	})
	const version = api.runtimeVersion
	log(`Gargantua ${version.specName} spec ${version.specVersion}`)
	return api
}

/** Read before anything connects, so a live run without it stops at once. */
function sudoSeed() {
	const seed = secret(setting("GARGANTUA_SUDO_SEED"))
	if (!seed) {
		throw new Error(
			`GARGANTUA_SUDO_SEED is unset and missing from ${options["env-file"]}; live runs sign with the Gargantua sudo key`,
		)
	}
	return seed
}

/** The pair GARGANTUA_SUDO_SEED derives that is `sudo.key()`, sr25519 first. */
async function sudoSigner(api, seed) {
	const key = await api.query.sudo.key()
	if (key.isNone) throw new Error("Gargantua has no sudo key")
	const sudo = key.unwrap().toHex()
	const tried = []
	for (const type of ["sr25519", "ed25519", "ecdsa"]) {
		let pair
		try {
			pair = new Keyring({ type }).addFromUri(seed)
		} catch (error) {
			tried.push(`${type}: ${error.message}`)
			continue
		}
		if (u8aToHex(pair.addressRaw) === sudo) return pair
		tried.push(`${type} ${pair.address}`)
	}
	throw new Error(`GARGANTUA_SUDO_SEED is not the sudo key ${key.toString()} (${tried.join(", ")})`)
}

function describe(api, error) {
	if (error.isModule) {
		const { section, name, docs } = api.registry.findMetaError(error.asModule)
		return `${section}.${name}: ${docs.join(" ")}`
	}
	return error.toString()
}

/** Prints the call in a dry run; otherwise sends it through `sudo.sudo` and waits for inclusion. */
async function dispatch(api, signer, label, call) {
	const wrapped = api.tx.sudo.sudo(call)
	log(`${label}: ${call.method.section}.${call.method.method}`)
	console.log(`  decoded: ${JSON.stringify(call.method.toHuman())}`)
	console.log(`  call:    ${call.method.toHex()}`)
	console.log(`  sudo:    ${wrapped.method.toHex()}`)
	if (dryRun) return false
	await new Promise((resolve, reject) => {
		let unsubscribe
		const stop = () => unsubscribe?.()
		wrapped
			.signAndSend(signer, { nonce: -1 }, ({ status, events, dispatchError }) => {
				if (dispatchError) {
					stop()
					reject(new Error(`${label}: ${describe(api, dispatchError)}`))
					return
				}
				if (!status.isInBlock) return
				stop()
				const sudid = events.find(({ event }) => api.events.sudo.Sudid.is(event))
				if (!sudid) return reject(new Error(`${label}: no sudo.Sudid event in ${status.asInBlock.toHex()}`))
				const result = sudid.event.data[0]
				if (result.isErr) return reject(new Error(`${label}: sudo call failed: ${describe(api, result.asErr)}`))
				const emitted = events.map(({ event }) => `${event.section}.${event.method}`).join(", ")
				log(`${label}: included in ${status.asInBlock.toHex()} (${emitted})`)
				resolve()
			})
			.then((unsub) => {
				unsubscribe = unsub
			})
			.catch(reject)
	})
	return true
}

/** Re-reads `check` until it reports done, printing progress, or fails after `--timeout-min`. */
async function poll(label, check) {
	const limit = Number(options["timeout-min"]) * 60_000
	const start = Date.now()
	for (;;) {
		const { done, progress } = await check().catch((error) => ({
			done: false,
			progress: `read failed: ${error.shortMessage ?? error.message}`,
		}))
		const elapsed = `${Math.round((Date.now() - start) / 1000)}s`
		if (done) return log(`${label}: landed after ${elapsed} (${progress})`)
		if (Date.now() - start > limit)
			throw new Error(`${label}: nothing after ${options["timeout-min"]} min (${progress})`)
		log(`${label}: waiting for the relayer, ${elapsed}: ${progress}`)
		await pause(POLL_MS)
	}
}

/** A failed precondition stops a live run; a dry run reports it and still prints the call. */
function precondition(condition, message) {
	if (condition) return
	if (!dryRun) throw new Error(message)
	log(`warning: ${message}`)
}

function selectedChains() {
	return options.chains.split(",").map((id) => {
		const chain = Number(id.trim())
		if (!CHAINS[chain]) throw new Error(`Unsupported chain ${id}: use ${Object.keys(CHAINS).join(" or ")}`)
		return chain
	})
}

function bundlerList() {
	const list = options.bundlers ? options.bundlers.split(",") : BUNDLERS
	return list.map((bundler) => address(bundler.trim(), "bundler"))
}

async function paymasterOf(api, configured, id) {
	const registered = (await api.query.intentsCoprocessor.paymasters({ Evm: id })).unwrapOr(undefined)?.toHex()
	const paymaster = options.paymaster ?? configured(id, "SIMPLEX_PAYMASTER") ?? registered
	if (!paymaster) throw new Error(`EVM-${id}: no paymaster in --paymaster, the config or the pallet`)
	return { paymaster: address(paymaster, "paymaster"), registered: registered && getAddress(registered) }
}

/** The checks the paymaster runs on a governance delivery, read from the chain. */
async function checkPaymaster(client, id, paymaster) {
	const [version, entryPoint, relayer, host] = await Promise.all(
		["version", "entryPoint", "relayer", "host"].map((name) => read(client, paymaster, paymasterAbi, name)),
	).catch(() => [])
	precondition(version === 3n, `EVM-${id}: paymaster ${paymaster} reports version ${version}, expected 3`)
	precondition(same(entryPoint, ENTRYPOINT_V09), `EVM-${id}: paymaster entryPoint() is ${entryPoint}, expected v0.9`)
	precondition(
		same(relayer, options.relayer),
		`EVM-${id}: paymaster relayer() is ${relayer}, the nested request names ${options.relayer}`,
	)
	if (host) {
		const source = hexToString(await read(client, host, hostAbi, "hyperbridge"))
		precondition(
			source === HYPERBRIDGE,
			`EVM-${id}: host hyperbridge() is ${source}, the nested request names ${HYPERBRIDGE}`,
		)
	}
}

// ── Commands ─────────────────────────────────────────────────────────

async function metadata(api) {
	const pallet = api.tx.intentsCoprocessor
	for (const name of CALLS) {
		const call = pallet[name]
		if (!call) {
			console.log(`intentsCoprocessor.${name}: not in this runtime`)
			continue
		}
		const args = call.meta.args.map(
			(arg) => `${arg.name}: ${arg.typeName.toString() || arg.type.toString()} (${arg.type.toString()})`,
		)
		console.log(`intentsCoprocessor.${name} [call index ${u8aToHex(call.callIndex)}]`)
		for (const arg of args) console.log(`  ${arg}`)
	}
	console.log(
		`StateMachine Evm(97) encodes as ${api.createType(pallet.addPaymasterDeployment.meta.args[0].type.toString(), { Evm: 97 }).toHex()}`,
	)
	console.log(`sudo.key: ${(await api.query.sudo.key()).toString()}`)
	for (const id of selectedChains()) {
		const gateway = (await api.query.intentsCoprocessor.gateways({ Evm: id })).unwrapOr(undefined)
		const paymaster = (await api.query.intentsCoprocessor.paymasters({ Evm: id })).unwrapOr(undefined)
		console.log(
			`EVM-${id}: gateway ${gateway ? gateway.gateway.toHex() : "unset"}, paymaster ${paymaster ? paymaster.toHex() : "unset"}`,
		)
	}
}

function initData() {
	const bundlers = bundlerList()
	console.log(`bundlers: ${bundlers.join(", ")}`)
	console.log(`relayer:  ${getAddress(options.relayer)}`)
	console.log(`allow:    ${bundlerInitData(bundlers, true, getAddress(options.relayer))}`)
	console.log(`clear:    ${bundlerInitData(bundlers, false, getAddress(options.relayer))}`)
}

async function registerPaymaster(api, signer, configured) {
	for (const id of selectedChains()) {
		const label = `EVM-${id} register-paymaster`
		const { paymaster, registered } = await paymasterOf(api, configured, id)
		await checkPaymaster(evm(id), id, paymaster)
		if (same(registered, paymaster)) {
			log(`${label}: ${paymaster} already registered`)
			continue
		}
		if (registered) log(`${label}: replaces ${registered}`)
		const sent = await dispatch(
			api,
			signer,
			label,
			api.tx.intentsCoprocessor.addPaymasterDeployment({ Evm: id }, paymaster),
		)
		if (!sent) continue
		const stored = (await api.query.intentsCoprocessor.paymasters({ Evm: id })).unwrapOr(undefined)?.toHex()
		if (!same(stored, paymaster)) throw new Error(`${label}: the pallet stores ${stored}, not ${paymaster}`)
		log(`${label}: Paymasters(Evm(${id})) = ${paymaster}`)
	}
}

async function upgradeGateway(api, signer, configured) {
	const pending = []
	for (const id of selectedChains()) {
		const label = `EVM-${id} upgrade-gateway`
		const client = evm(id)
		const impl = address(options.impl ?? configured(id, "INTENT_GATEWAY_V2_IMPL"), "gateway implementation")
		const info = (await api.query.intentsCoprocessor.gateways({ Evm: id })).unwrapOr(undefined)
		if (!info) throw new Error(`${label}: no gateway registered for Evm(${id})`)
		const gateway = getAddress(info.gateway.toHex())

		const typehash = await read(client, impl, gatewayAbi, "SELECT_SOLVER_TYPEHASH").catch(() => undefined)
		precondition(
			typehash === SELECT_SOLVER_TYPEHASH,
			`${label}: ${impl} reports SELECT_SOLVER_TYPEHASH ${typehash}`,
		)
		const version = await read(client, gateway, gatewayAbi, "version").catch(() => undefined)
		precondition(
			version === 3n,
			`${label}: gateway ${gateway} is at version ${version}; the empty init data assumes 3`,
		)

		const landed = async () => {
			const current = await implementationOf(client, gateway)
			const live = await read(client, gateway, gatewayAbi, "SELECT_SOLVER_TYPEHASH").catch(() => undefined)
			return {
				done: same(current, impl) && live === SELECT_SOLVER_TYPEHASH,
				progress: `implementation ${current}, typehash ${live}`,
			}
		}
		const before = await landed()
		if (before.done) {
			log(`${label}: gateway ${gateway} already on ${impl}`)
			continue
		}
		const data = encodeFunctionData({ abi: gatewayAbi, functionName: "upgradeToAndCall", args: [impl, "0x"] })
		console.log(`  gateway ${gateway}: ${before.progress}, target ${impl}`)
		if (await dispatch(api, signer, label, api.tx.intentsCoprocessor.executeOnGateway({ Evm: id }, data))) {
			pending.push(poll(label, landed))
		}
	}
	await Promise.all(pending)
}

async function setBundlers(api, signer, configured, allowed) {
	const bundlers = bundlerList()
	const relayer = getAddress(options.relayer)
	const pending = []
	for (const id of selectedChains()) {
		const label = `EVM-${id} ${allowed ? "allow" : "clear"}-bundlers`
		const client = evm(id)
		const { paymaster, registered } = await paymasterOf(api, configured, id)
		precondition(
			same(registered, paymaster),
			`${label}: Paymasters(Evm(${id})) is ${registered}, not ${paymaster}; run register-paymaster first`,
		)
		await checkPaymaster(client, id, paymaster)

		const fromProxy = await implementationOf(client, paymaster).catch(() => undefined)
		const impl = options["paymaster-impl"]
			? address(options["paymaster-impl"], "paymaster implementation")
			: fromProxy
		if (!impl) throw new Error(`${label}: cannot read the implementation of ${paymaster}; pass --paymaster-impl`)
		precondition(!fromProxy || same(fromProxy, impl), `${label}: ${paymaster} runs ${fromProxy}, not ${impl}`)

		const landed = async () => {
			const listed = await read(client, paymaster, paymasterAbi, "getBundlers")
			const done = allowed
				? bundlers.every((bundler) => listed.some((entry) => same(entry, bundler)))
				: listed.length === 0
			return { done, progress: `getBundlers() [${listed.join(", ")}]` }
		}
		const before = await landed().catch((error) => ({
			done: false,
			progress: `unread: ${error.shortMessage ?? error.message}`,
		}))
		if (before.done) {
			log(`${label}: already ${before.progress}`)
			continue
		}
		const init = bundlerInitData(bundlers, allowed, relayer)
		console.log(`  paymaster ${paymaster} on ${impl}, ${before.progress}`)
		console.log(`  init_data ${init}`)
		if (await dispatch(api, signer, label, api.tx.intentsCoprocessor.upgradePaymaster({ Evm: id }, impl, init))) {
			pending.push(poll(label, landed))
		}
	}
	await Promise.all(pending)
}

async function main() {
	const [command] = positionals
	if (options.help || !command) {
		console.log(USAGE)
		return
	}
	options.relayer = address(options.relayer, "--relayer")
	if (command === "init-data") return initData()

	const commands = {
		metadata: (api) => metadata(api),
		"register-paymaster": (api, signer, configured) => registerPaymaster(api, signer, configured),
		"upgrade-gateway": (api, signer, configured) => upgradeGateway(api, signer, configured),
		"allow-bundlers": (api, signer, configured) => setBundlers(api, signer, configured, true),
		"clear-bundlers": (api, signer, configured) => setBundlers(api, signer, configured, false),
	}
	if (!commands[command]) throw new Error(`Unknown command ${command}\n\n${USAGE}`)

	const live = command !== "metadata" && !dryRun
	const seed = live ? sudoSeed() : undefined
	const configured = readConfig(options.config)
	const api = await connect()
	try {
		const signer = live ? await sudoSigner(api, seed) : undefined
		if (dryRun) log("dry run: nothing is signed or sent")
		await commands[command](api, signer, configured)
	} finally {
		await api.disconnect().catch(() => {})
	}
}

main().then(
	() => process.exit(0),
	(error) => {
		console.error(error)
		process.exit(1)
	},
)
