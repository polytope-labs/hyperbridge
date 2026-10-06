// EntryPoint v0.9 end to end on a local anvil fork of BSC Chapel, in place of a live testnet run.
//
//  1. Governance, delivered as the host: the IntentGatewayV2 proxy moves to the userOpHash
//     implementation through `Execute`, and the SimplexPaymaster bundler allowlist is set through
//     the nested `UpgradeContract(sameImpl, onAccept(SetBundlers))` route of the sudo script.
//  2. A fresh solver EOA delegates to the v0.9 SolverAccount through EIP-7702.
//  3. A user places a same-chain order with the SDK. The solver prices it with the SDK's gas
//     estimate, takes `paymasterAndData` from simplex's paymaster builder and signs it with
//     `prepareSubmitBid`.
//  4. The user runs the SDK's `buildBids`, `sortBids`, `simulate` and `execute`, which sends the
//     bid to a local rundler that bundles it on the fork.
//  5. The rundler signer is taken off the allowlist, and a second sponsored bid must be refused.
//
// From sdk/, after `pnpm -C packages/sdk build`:
//   FORK_URL=<Chapel RPC> pnpm -C packages/simplex exec tsx e2e/entrypoint-v09-local.mjs
//
// It runs under tsx because simplex does not export its paymaster builder. Environment:
//   FORK_URL                     Chapel RPC to fork (else BSC_TESTNET_RPC_URL, else BSC_CHAPEL in the env file)
//   FORK_BLOCK                   Block to fork at (default: latest)
//   RUNDLER_SIGNER_PRIVATE_KEY   Rundler's signer (else read by exact key from the env file)
//   RUNDLER_BIN                  Default: ../rundler/target/debug/rundler beside this repository
//   E2E_ENV_FILE                 Default: sdk/.env.local, read line by line, never sourced
//   GATEWAY_IMPL                 Default: INTENT_GATEWAY_V2_IMPL of [97.address] in evm/config.testnet.toml
import { execFileSync, spawn } from "node:child_process"
import fs from "node:fs"
import net from "node:net"
import os from "node:os"
import path from "node:path"
import readline from "node:readline"
import { fileURLToPath } from "node:url"
import {
	ChainConfigService,
	CryptoUtils,
	DEFAULT_GRAFFITI,
	EvmChain,
	FILL_ORDER_SELECTOR,
	IntentGateway,
	bytes20ToBytes32,
	encodeERC7821ExecuteBatch,
	encodeFillOrder,
	transformOrderForContract,
} from "@hyperbridge/sdk"
import {
	concat,
	createPublicClient,
	createWalletClient,
	decodeErrorResult,
	encodeAbiParameters,
	encodeFunctionData,
	erc20Abi,
	getAddress,
	http,
	isAddressEqual,
	keccak256,
	parseAbi,
	parseEther,
	parseEventLogs,
	parseUnits,
	toFunctionSelector,
	toHex,
	zeroAddress,
} from "viem"
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"
import { FillerConfigService } from "../src/services/FillerConfigService.ts"
import { buildPaymasterAndData } from "../src/services/paymaster/index.ts"

const HERE = path.dirname(fileURLToPath(import.meta.url))
const SDK_DIR = path.resolve(HERE, "../../..")
const REPO = path.resolve(SDK_DIR, "..")
const INDEXER = path.join(SDK_DIR, "packages/indexer")
const ENV_FILE = process.env.E2E_ENV_FILE || path.join(SDK_DIR, ".env.local")
const RUNDLER_BIN = process.env.RUNDLER_BIN || path.resolve(REPO, "../rundler/target/debug/rundler")

const CHAIN = "EVM-97"
const SIMULATION_ORIGIN = "0x0643866dA50efE0b055Cd15aF95191968c8411b5"
const SELECT_SOLVER_TYPEHASH = "0xe706bdab7d945360dcd9d81d355f856754dd1cfa461edfc0a7502e2583b4e09e"
const IMPLEMENTATION_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
// `IntentsBase.RequestKind.Execute` and `SimplexPaymaster.RequestKind.UpgradeContract`.
const GATEWAY_EXECUTE = 5
const PAYMASTER_UPGRADE = 0
const ORDER_IN = parseUnits("5", 18)
const ORDER_OUT = parseUnits("7500", 6)
const UNAUTHORIZED_BUNDLER = toFunctionSelector("UnauthorizedBundler(address)")
const SELECT = toFunctionSelector("select((bytes32,bytes32,bytes))")
const GET_CURRENT_USER_OP_HASH = toFunctionSelector("getCurrentUserOpHash()")
const VALIDATE_PAYMASTER = toFunctionSelector(
	"validatePaymasterUserOp((address,uint256,bytes,bytes,bytes32,uint256,bytes32,bytes,bytes),bytes32,uint256)",
)

const config = new ChainConfigService()
const HOST = config.getHostAddress(CHAIN)
const GATEWAY = config.getIntentGatewayAddress(CHAIN)
const ENTRY_POINT = config.getEntryPointAddress(CHAIN)
const SOLVER_ACCOUNT = config.getSolverAccountAddress(CHAIN)
const PAYMASTER = config.getSimplexPaymasterAddress(CHAIN)
const USDH = config.getUsdcAsset(CHAIN)
const CNGN = config.getCNgnAsset(CHAIN)

const abi = parseAbi([
	"struct PostRequest { bytes source; bytes dest; uint64 nonce; bytes from; bytes to; uint64 timeoutTimestamp; bytes body; }",
	"struct IncomingPostRequest { PostRequest request; address relayer; }",
	"function onAccept(IncomingPostRequest incoming)",
	"function upgradeToAndCall(address newImplementation, bytes data)",
	"function hyperbridge() view returns (bytes)",
	"function host() view returns (bytes)",
	"function version() view returns (uint64)",
	"function SELECT_SOLVER_TYPEHASH() view returns (bytes32)",
	"function params() view returns ((address host, address dispatcher, bool solverSelection, uint256 surplusShareBps, uint256 protocolFeeBps, address priceOracle))",
	"function _filled(bytes32) view returns (address)",
	"function _orders(bytes32, uint256) view returns (uint256)",
	"function getBundlers() view returns (address[])",
	"function relayer() view returns (address)",
	"function treasury() view returns (address)",
	"function deposit() payable",
	"function addStake(uint32 unstakeDelaySec) payable",
	"function getNonce(address sender, uint192 key) view returns (uint256)",
	"function balanceOf(address account) view returns (uint256)",
	"function getDepositInfo(address account) view returns ((uint256 deposit, bool staked, uint112 stake, uint32 unstakeDelaySec, uint48 withdrawTime))",
	"event UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster, uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)",
	"event OrderFilled(bytes32 indexed commitment, address filler, (bytes32 token, uint256 amount)[] outputs, (bytes32 token, uint256 amount)[] inputs)",
	"error FailedOp(uint256 opIndex, string reason)",
	"error FailedOpWithRevert(uint256 opIndex, string reason, bytes inner)",
	"error UnauthorizedBundler(address origin)",
])

// ── Output ───────────────────────────────────────────────────────────

// Keys and endpoints never reach the output, whatever an error message quotes. Longest first.
const SECRETS = []
const secret = (value) => {
	if (value && value.length > 8) {
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
			...args.map((arg) => (typeof arg === "string" || arg instanceof Error ? redact(arg.stack ?? arg) : arg)),
		)
}
const json = (value) => JSON.stringify(value, (_key, v) => (typeof v === "bigint" ? v.toString() : v))
const log = (line) => console.log(`${new Date().toISOString()} ${line}`)
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

const checks = []
function check(name, ok, observed) {
	checks.push({ name, ok: Boolean(ok) })
	log(`${ok ? "PASS" : "FAIL"} ${name}: ${typeof observed === "string" ? observed : json(observed)}`)
}

function ensure(value, message) {
	if (!value) throw new Error(message)
	return value
}

/** One `KEY=value` line of the env file, by exact key; nothing else is kept. */
function envFileValue(key) {
	if (!fs.existsSync(ENV_FILE)) return undefined
	const line = fs
		.readFileSync(ENV_FILE, "utf8")
		.split(/\r?\n/)
		.find((entry) => entry.startsWith(`${key}=`))
	return line
		?.slice(key.length + 1)
		.trim()
		.replace(/^(["'])(.*)\1$/, "$2")
}

function gatewayImplementation() {
	if (process.env.GATEWAY_IMPL) return getAddress(process.env.GATEWAY_IMPL)
	const toml = fs.readFileSync(path.join(REPO, "evm/config.testnet.toml"), "utf8")
	const table = /^\[97\.address\]$([\s\S]*?)(?=^\[)/m.exec(toml)?.[1] ?? ""
	const impl = /^INTENT_GATEWAY_V2_IMPL\s*=\s*"(0x[0-9a-fA-F]{40})"/m.exec(table)?.[1]
	return getAddress(ensure(impl, "INTENT_GATEWAY_V2_IMPL is missing from [97.address] in evm/config.testnet.toml"))
}

// ── Processes ────────────────────────────────────────────────────────

const workdir = fs.mkdtempSync(path.join(os.tmpdir(), "entrypoint-v09-local-"))
const children = []
const running = (child) => child.exitCode === null && child.signalCode === null

function start(name, command, args, env = {}) {
	const logFile = path.join(workdir, `${name}.log`)
	const out = fs.openSync(logFile, "w")
	const child = spawn(command, args, { env: { ...process.env, ...env }, stdio: ["ignore", "pipe", "pipe"] })
	// Anvil's errors quote the fork URL from its argv, so lines are redacted before they reach the file.
	for (const input of [child.stdout, child.stderr])
		readline.createInterface({ input }).on("line", (line) => fs.writeSync(out, `${redact(line)}\n`))
	// Unlike "exit", "close" fires only once every line has been written.
	Object.assign(child, { label: name, logFile, exited: new Promise((resolve) => child.on("close", resolve)) })
	child.on("error", (error) => log(`${name}: ${error.message}`))
	children.push(child)
	log(`${name} started, pid ${child.pid}, log ${logFile}`)
	return child
}

/** Polls `probe` until it answers, failing as soon as `child` exits. */
async function ready(child, probe) {
	const deadline = Date.now() + 60_000
	for (;;) {
		if (!running(child)) throw new Error(`${child.label} exited; see ${child.logFile}`)
		const value = await probe().catch(() => undefined)
		if (value) return value
		if (Date.now() > deadline) throw new Error(`${child.label} not ready after 60s; see ${child.logFile}`)
		await pause(500)
	}
}

async function stopAll() {
	await Promise.all(
		children.filter(running).map(async (child) => {
			child.kill("SIGTERM")
			const timer = setTimeout(() => child.kill("SIGKILL"), 5_000)
			await child.exited
			clearTimeout(timer)
		}),
	)
	await Promise.all(children.map((child) => child.exited))
	for (const child of children)
		log(`${child.label} pid ${child.pid}: ${running(child) ? "STILL RUNNING" : "stopped"}`)
}

for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => stopAll().finally(() => process.exit(130)))

function freePort() {
	return new Promise((resolve, reject) => {
		const server = net.createServer()
		server.on("error", reject)
		server.listen(0, "127.0.0.1", () => {
			const { port } = server.address()
			server.close(() => resolve(port))
		})
	})
}

async function jsonRpc(url, method, params = []) {
	const response = await fetch(url, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
	})
	const body = await response.json()
	if (body.error) throw new Error(`${method}: ${body.error.message ?? json(body.error)}`)
	return body.result
}

// ── Chain helpers ────────────────────────────────────────────────────

let client
let transport

const rpc = (method, params) => client.request({ method, params })
const read = (address, functionName, args = []) => client.readContract({ address, abi, functionName, args })
const balanceOf = (token, account) =>
	client.readContract({ address: token, abi: erc20Abi, functionName: "balanceOf", args: [account] })
const fund = (address) => rpc("anvil_setBalance", [address, toHex(parseEther("100"))])

async function confirmed(hash, label) {
	const receipt = await client.waitForTransactionReceipt({ hash })
	if (receipt.status !== "success") throw new Error(`${label}: ${hash} reverted`)
	return receipt
}

/** Sends `request` as `from`, which anvil impersonates. */
async function impersonated(from, request, label) {
	await rpc("anvil_impersonateAccount", [from])
	await fund(from)
	const wallet = createWalletClient({ account: from, chain: bscTestnet, transport })
	const receipt = await confirmed(await wallet.sendTransaction(request), label)
	await rpc("anvil_stopImpersonatingAccount", [from])
	return receipt
}

/** Writes `account`'s balance of `token` into the first storage slot that `balanceOf` reads it from. */
async function deal(token, account, amount) {
	for (let slot = 0n; slot < 16n; slot++) {
		const key = keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [account, slot]))
		const previous = await client.getStorageAt({ address: token, slot: key })
		await rpc("anvil_setStorageAt", [token, key, toHex(amount, { size: 32 })])
		if ((await balanceOf(token, account)) === amount) return
		await rpc("anvil_setStorageAt", [token, key, previous ?? toHex(0n, { size: 32 })])
	}
	throw new Error(`No balance slot found for ${token}`)
}

async function approve(wallet, token, spender) {
	const args = [spender, 2n ** 255n]
	await confirmed(
		await wallet.writeContract({ address: token, abi: erc20Abi, functionName: "approve", args }),
		"approve",
	)
}

async function implementationOf(proxy) {
	const word = await client.getStorageAt({ address: proxy, slot: IMPLEMENTATION_SLOT })
	return getAddress(`0x${word.slice(-40)}`)
}

/** Every call `hash` made, depth first, from anvil's call tracer. */
async function callsOf(hash) {
	const flatten = (call) => [call, ...(call.calls ?? []).flatMap(flatten)]
	return flatten(await rpc("debug_traceTransaction", [hash, { tracer: "callTracer" }]))
}

/** The origin a handleOps revert names in `FailedOpWithRevert(_, _, UnauthorizedBundler(origin))`. */
function unauthorizedOrigin(data) {
	try {
		const failed = decodeErrorResult({ abi, data })
		const inner =
			failed.errorName === "FailedOpWithRevert" ? decodeErrorResult({ abi, data: failed.args[2] }) : failed
		return inner.errorName === "UnauthorizedBundler" ? inner.args[0] : undefined
	} catch {
		return undefined
	}
}

/** The origin of an `UnauthorizedBundler(address)` revert quoted anywhere in `text`. */
function unauthorizedIn(text) {
	const word = new RegExp(`${UNAUTHORIZED_BUNDLER.slice(2)}(0{24}[0-9a-f]{40})`, "i").exec(text ?? "")?.[1]
	return word && getAddress(`0x${word.slice(24)}`)
}

/** The indexer's own `findUserOpHash`, run in the indexer package (see entrypoint-v09-local-indexer.cjs). */
function indexerUserOpHash(logs, sender, logIndex) {
	const input = json({
		logs: logs.map(({ address, topics, data, logIndex }) => ({ address, topics, data, logIndex })),
		sender,
		logIndex,
	})
	const helper = path.join(HERE, "entrypoint-v09-local-indexer.cjs")
	const tsx = path.join(INDEXER, "node_modules/.bin/tsx")
	const tsconfig = path.join(INDEXER, "tsconfig.json")
	const out = execFileSync(tsx, ["--tsconfig", tsconfig, helper], { cwd: INDEXER, encoding: "utf8", input })
	return JSON.parse(out).userOpHash
}

// ── Governance ───────────────────────────────────────────────────────

let governanceNonce = 0n

/** `onAccept` on `to` as the host delivers a `pallet-intents` request from Hyperbridge. */
async function deliver(to, body, relayer, label) {
	const request = {
		source: await read(HOST, "hyperbridge"),
		dest: await read(HOST, "host"),
		nonce: governanceNonce++,
		from: toHex("pallet-intents"),
		to,
		timeoutTimestamp: 0n,
		body,
	}
	const data = encodeFunctionData({ abi, functionName: "onAccept", args: [{ request, relayer }] })
	const receipt = await impersonated(HOST, { to, data }, label)
	log(`${label}: delivered in ${receipt.transactionHash}`)
	return receipt
}

/** `allow-bundlers` or `clear-bundlers`, with the init data the sudo script prints for it. */
async function setBundlers(bundlers, allowed, relayer) {
	const args = ["init-data", "--bundlers", bundlers.join(","), "--relayer", relayer, "--env-file", os.devNull]
	const out = execFileSync(process.execPath, [path.join(HERE, "entrypoint-v09-sudo.mjs"), ...args], {
		encoding: "utf8",
	})
	const line = allowed ? "allow" : "clear"
	const init = ensure(new RegExp(`^${line}:\\s+(0x[0-9a-fA-F]+)$`, "m").exec(out)?.[1], `no ${line} init data`)
	const upgrade = encodeAbiParameters(
		[{ type: "address" }, { type: "bytes" }],
		[await implementationOf(PAYMASTER), init],
	)
	await deliver(PAYMASTER, concat([toHex(PAYMASTER_UPGRADE, { size: 1 }), upgrade]), relayer, `${line}-bundlers`)
	return read(PAYMASTER, "getBundlers")
}

// ── Main ─────────────────────────────────────────────────────────────

async function main() {
	const forkUrl = secret(process.env.FORK_URL || process.env.BSC_TESTNET_RPC_URL || envFileValue("BSC_CHAPEL"))
	ensure(forkUrl, `Set FORK_URL, or BSC_CHAPEL in ${ENV_FILE}`)
	const signerKey = secret(process.env.RUNDLER_SIGNER_PRIVATE_KEY || envFileValue("RUNDLER_SIGNER_PRIVATE_KEY"))
	ensure(signerKey, `Set RUNDLER_SIGNER_PRIVATE_KEY, or add it to ${ENV_FILE}`)
	ensure(fs.existsSync(RUNDLER_BIN), `No rundler at ${RUNDLER_BIN}; run \`cargo build --bin rundler\` in the fork`)
	const bundler = privateKeyToAccount(signerKey).address
	const gatewayImpl = gatewayImplementation()

	const [anvilPort, rundlerPort, metricsPort] = [await freePort(), await freePort(), await freePort()]
	const anvilUrl = `http://127.0.0.1:${anvilPort}`
	const rundlerUrl = `http://127.0.0.1:${rundlerPort}`
	const forkBlock = process.env.FORK_BLOCK ? ["--fork-block-number", process.env.FORK_BLOCK] : []
	const anvil = start("anvil", "anvil", [
		...["--fork-url", forkUrl, ...forkBlock, "--port", String(anvilPort)],
		..."--chain-id 97 --hardfork prague --block-time 1 --silent".split(" "),
	])
	transport = http(anvilUrl)
	client = createPublicClient({ chain: bscTestnet, transport })
	log(`anvil forked Chapel at block ${await ready(anvil, () => client.getBlockNumber())} on ${anvilUrl}`)

	// ── 1. Governance ──
	// The one relayer Hyperbridge delivers through, which both contracts accept.
	const relayer = await read(PAYMASTER, "relayer")
	const gatewayBefore = await implementationOf(GATEWAY)
	const upgrade = encodeFunctionData({ abi, functionName: "upgradeToAndCall", args: [gatewayImpl, "0x"] })
	const upgraded = await deliver(
		GATEWAY,
		concat([toHex(GATEWAY_EXECUTE, { size: 1 }), upgrade]),
		relayer,
		"upgrade-gateway",
	)
	const gatewayAfter = await implementationOf(GATEWAY)
	const typehash = await read(GATEWAY, "SELECT_SOLVER_TYPEHASH")
	check(
		"gateway implementation upgraded through Execute",
		isAddressEqual(gatewayAfter, gatewayImpl) && typehash === SELECT_SOLVER_TYPEHASH,
		{ tx: upgraded.transactionHash, before: gatewayBefore, after: gatewayAfter, typehash },
	)
	const { solverSelection } = await read(GATEWAY, "params")
	check("gateway solver selection is on", solverSelection, { solverSelection })

	const treasury = await read(PAYMASTER, "treasury")
	for (const [functionName, args] of [
		["deposit", []],
		["addStake", [86_400]],
	]) {
		const data = encodeFunctionData({ abi, functionName, args })
		await impersonated(treasury, { to: PAYMASTER, data, value: parseEther("1") }, functionName)
	}
	log(`paymaster funded by ${treasury}: ${json(await read(ENTRY_POINT, "getDepositInfo", [PAYMASTER]))}`)

	const allowed = await setBundlers([bundler, SIMULATION_ORIGIN], true, relayer)
	check(
		"paymaster allowlist holds the rundler signer and simulation origin",
		[bundler, SIMULATION_ORIGIN].every((entry) => allowed.some((listed) => isAddressEqual(listed, entry))),
		allowed,
	)

	// ── Bundler ──
	await fund(bundler)
	const rundler = start(
		"rundler",
		RUNDLER_BIN,
		[
			..."node --network bsc_testnet --enabled_entry_points v0.9 --unsafe".split(" "),
			...["--rpc.port", String(rundlerPort), "--metrics.port", String(metricsPort)],
		],
		{ NODE_HTTP: anvilUrl, SIGNER_PRIVATE_KEYS: signerKey, RUST_LOG: "info", NO_COLOR: "1" },
	)
	const entryPoints = await ready(rundler, () => jsonRpc(rundlerUrl, "eth_supportedEntryPoints"))
	check(
		"rundler serves only EntryPoint v0.9",
		entryPoints.length === 1 && isAddressEqual(entryPoints[0], ENTRY_POINT),
		entryPoints,
	)

	// ── 2. Accounts and delegation ──
	const user = privateKeyToAccount(generatePrivateKey())
	const solver = privateKeyToAccount(generatePrivateKey())
	const userWallet = createWalletClient({ account: user, chain: bscTestnet, transport })
	const solverWallet = createWalletClient({ account: solver, chain: bscTestnet, transport })
	await Promise.all([fund(user.address), fund(solver.address)])

	const authorization = await solverWallet.signAuthorization({ contractAddress: SOLVER_ACCOUNT, executor: "self" })
	const delegation = await confirmed(
		await solverWallet.sendTransaction({ authorizationList: [authorization], to: solver.address, data: "0x" }),
		"delegation",
	)
	const code = await client.getCode({ address: solver.address })
	check(
		"solver EOA delegated to the v0.9 SolverAccount",
		code?.toLowerCase() === concat(["0xef0100", SOLVER_ACCOUNT]).toLowerCase(),
		{ solver: solver.address, tx: delegation.transactionHash, code },
	)

	const sdkChain = EvmChain.fromParams({ chainId: 97, host: HOST, rpcUrl: anvilUrl, bundlerUrl: rundlerUrl })
	// The SDK keeps session keys under ./.hyperbridge-cache.
	process.chdir(workdir)
	const gateway = await IntentGateway.create(sdkChain, sdkChain)
	const feeToken = await sdkChain.getFeeTokenWithDecimals()

	await deal(USDH, user.address, parseUnits("100", 18))
	await deal(feeToken.address, user.address, parseUnits("10", feeToken.decimals))
	await deal(USDH, solver.address, parseUnits("50", 18))
	await deal(CNGN, solver.address, parseUnits("100000", 6))
	await approve(userWallet, USDH, GATEWAY)
	await approve(userWallet, feeToken.address, GATEWAY)

	/** Places a same-chain USD.h for cNGN order through the SDK's `execute`, stopping once it is placed. */
	async function placeOrder(label) {
		const beneficiary = bytes20ToBytes32(user.address)
		const order = {
			user: beneficiary,
			source: toHex(CHAIN),
			destination: toHex(CHAIN),
			deadline: (await client.getBlockNumber()) + 10_000n,
			nonce: 0n,
			// Set, so placement skips the fee quote.
			fees: parseUnits("0.01", feeToken.decimals),
			session: zeroAddress,
			predispatch: { assets: [], call: "0x" },
			inputs: [{ token: bytes20ToBytes32(USDH), amount: ORDER_IN }],
			output: { beneficiary, assets: [{ token: bytes20ToBytes32(CNGN), amount: ORDER_OUT }], call: "0x" },
		}
		const stream = gateway.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 0 })
		try {
			const awaiting = (await stream.next()).value
			ensure(awaiting?.status === "AWAITING_PLACE_ORDER", `${label}: SDK yielded ${awaiting?.status}`)
			const { to, data, value } = awaiting
			const request = await userWallet.prepareTransactionRequest({ to, data, value })
			const placed = (await stream.next(await userWallet.signTransaction(request))).value
			ensure(placed?.status === "ORDER_PLACED", `${label}: SDK yielded ${placed?.status}`)
			log(`${label}: order ${placed.order.id} placed in ${placed.receipt.transactionHash}`)
			return { order: placed.order, sessionPrivateKey: awaiting.sessionPrivateKey }
		} finally {
			await stream.return(undefined)
		}
	}

	/** The solver's sponsored bid, built the way simplex's `prepareBidUserOp` builds one. */
	async function buildBid(order, label) {
		const approveOutput = {
			target: CNGN,
			value: 0n,
			data: encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [GATEWAY, ORDER_OUT] }),
		}
		const estimate = await gateway.estimateFillOrder({
			order,
			prependCalls: [approveOutput],
			requireBundlerEstimate: true,
		})
		const fillOptions = {
			relayerFee: estimate.fillOptions.relayerFee,
			nativeDispatchFee: 0n,
			validUntil: (await client.getBlockNumber()) + 600n,
			outputs: order.output.assets,
			inputs: order.inputs,
		}
		const callData = encodeERC7821ExecuteBatch([
			approveOutput,
			{ target: GATEWAY, value: 0n, data: encodeFillOrder(transformOrderForContract(order), fillOptions) },
		])
		const nonceKey = CryptoUtils.bidNonceKey(order.id, order.session, callData)
		const paymaster = await buildPaymasterAndData({
			chain: CHAIN,
			solverAccount: solver.address,
			publicClient: client,
			walletClient: solverWallet,
			signer: solver,
			configService: new FillerConfigService([]),
			prefund: {
				baseGas: estimate.callGasLimit + estimate.verificationGasLimit + estimate.preVerificationGas,
				maxFeePerGas: estimate.maxFeePerGas,
			},
		})
		ensure(paymaster.type === "simplex", `${label}: simplex chose no paymaster: ${paymaster.reason}`)
		const bidGas = {
			solverAccount: solver.address,
			nonce: await read(ENTRY_POINT, "getNonce", [solver.address, nonceKey]),
			entryPointAddress: ENTRY_POINT,
			callGasLimit: estimate.callGasLimit,
			verificationGasLimit: estimate.verificationGasLimit,
			maxFeePerGas: estimate.maxFeePerGas,
			maxPriorityFeePerGas: estimate.maxPriorityFeePerGas,
			callData,
			paymasterAndData: paymaster.paymasterAndData,
		}
		const preVerificationGas = await gateway.estimateBidPreVerificationGas(bidGas)
		const solverSigner = solver
		const userOp = await gateway.prepareSubmitBid({
			...bidGas,
			order,
			fillOptions,
			solverSigner,
			preVerificationGas,
		})
		const { callGasLimit, verificationGasLimit, maxFeePerGas, maxPriorityFeePerGas } = bidGas
		const mode = paymaster.paymasterAndData.slice(106, 108)
		log(
			`${label}: bid signed ${json({ callGasLimit, verificationGasLimit, preVerificationGas, maxFeePerGas, maxPriorityFeePerGas })}, paymaster mode 0x${mode}`,
		)
		return userOp
	}

	/** The user's side, as the SDK runs it on the bids of an auction. */
	async function executeBid(order, userOp, sessionPrivateKey) {
		const fillerBid = { filler: "local", bid: CryptoUtils.bidId(userOp.callData), userOp, deposit: 0n }
		const bids = gateway.buildBids(order, [fillerBid], sessionPrivateKey)
		ensure((await gateway.sortBids(order, bids)).length === 1, "sortBids dropped the bid")
		await bids[0].simulate()
		return bids[0].execute()
	}

	const snapshot = async () => ({
		userCngn: await balanceOf(CNGN, user.address),
		solverCngn: await balanceOf(CNGN, solver.address),
		solverUsdh: await balanceOf(USDH, solver.address),
		paymasterUsdh: await balanceOf(USDH, PAYMASTER),
		paymasterDeposit: await read(ENTRY_POINT, "balanceOf", [PAYMASTER]),
	})

	// ── 3-6. The sponsored fill ──
	const first = await placeOrder("fill")
	const firstOp = await buildBid(first.order, "fill")
	const before = await snapshot()
	const result = await executeBid(first.order, firstOp, first.sessionPrivateKey)
	const after = await snapshot()

	check(
		"SDK execute filled the order with the v0.9 userOpHash the solver signed",
		result.fillStatus === "full" && result.userOpHash === CryptoUtils.computeUserOpHash(firstOp, ENTRY_POINT, 97n),
		{ userOpHash: result.userOpHash, txnHash: result.txnHash, fillStatus: result.fillStatus },
	)

	const receipt = await client.getTransactionReceipt({ hash: result.txnHash })
	const bundle = await client.getTransaction({ hash: result.txnHash })
	check(
		"bundle sent by the rundler signer to EntryPoint v0.9",
		isAddressEqual(bundle.from, bundler) && isAddressEqual(bundle.to, ENTRY_POINT),
		{ tx: result.txnHash, from: bundle.from, to: bundle.to, gasUsed: receipt.gasUsed, block: receipt.blockNumber },
	)

	const calls = await callsOf(result.txnHash)
	const at = (from, to, selector) =>
		calls.findIndex(
			(call) => isAddressEqual(call.from, from) && isAddressEqual(call.to, to) && call.input.startsWith(selector),
		)
	const steps = {
		select: at(solver.address, GATEWAY, SELECT),
		validatePaymasterUserOp: at(ENTRY_POINT, PAYMASTER, VALIDATE_PAYMASTER),
		fillOrder: at(solver.address, GATEWAY, FILL_ORDER_SELECTOR),
		getCurrentUserOpHash: at(GATEWAY, ENTRY_POINT, GET_CURRENT_USER_OP_HASH),
	}
	check(
		"trace: SolverAccount selects and the paymaster validates, then fillOrder reads getCurrentUserOpHash",
		Object.values(steps).every((index) => index >= 0) &&
			Math.max(steps.select, steps.validatePaymasterUserOp) < steps.fillOrder &&
			steps.fillOrder < steps.getCurrentUserOpHash,
		{ callIndex: steps, txOrigin: bundle.from },
	)

	const [operation] = parseEventLogs({ abi, logs: receipt.logs, eventName: "UserOperationEvent" }).filter(
		(event) => isAddressEqual(event.address, ENTRY_POINT) && event.args.userOpHash === result.userOpHash,
	)
	check(
		"UserOperationEvent from EntryPoint v0.9, sponsored by the paymaster proxy, successful",
		operation?.args.success &&
			isAddressEqual(operation.args.paymaster, PAYMASTER) &&
			isAddressEqual(operation.args.sender, solver.address),
		operation && { address: operation.address, ...operation.args },
	)

	const [filled] = parseEventLogs({ abi, logs: receipt.logs, eventName: "OrderFilled" }).filter(
		(event) => isAddressEqual(event.address, GATEWAY) && event.args.commitment === first.order.id,
	)
	const filledBy = await read(GATEWAY, "_filled", [first.order.id])
	const escrowLeft = await read(GATEWAY, "_orders", [first.order.id, 0n])
	check(
		"OrderFilled by the solver, escrow released and order closed",
		filled && isAddressEqual(filled.args.filler, solver.address) && isAddressEqual(filledBy, solver.address),
		{
			commitment: first.order.id,
			filler: filled?.args.filler,
			filledBy,
			escrowLeft,
			released: filled?.args.inputs,
		},
	)
	const escrow = first.order.inputs[0].amount
	const moved = {
		userCngn: after.userCngn - before.userCngn,
		solverCngn: after.solverCngn - before.solverCngn,
		solverUsdh: after.solverUsdh - before.solverUsdh,
		paymasterUsdhCharged: after.paymasterUsdh - before.paymasterUsdh,
		escrowReleased: escrow,
		paymasterDepositSpent: before.paymasterDeposit - after.paymasterDeposit,
		actualGasCost: operation?.args.actualGasCost,
	}
	check(
		"balances moved as the fill and its sponsorship say",
		moved.userCngn === ORDER_OUT &&
			moved.solverCngn === -ORDER_OUT &&
			moved.paymasterUsdhCharged > 0n &&
			moved.solverUsdh + moved.paymasterUsdhCharged === escrow &&
			moved.paymasterDepositSpent === moved.actualGasCost,
		moved,
	)

	const indexed = indexerUserOpHash(receipt.logs, filled?.args.filler, filled?.logIndex)
	check("indexer findUserOpHash resolves the fill's userOpHash", indexed === result.userOpHash, indexed)

	// ── 7. The rundler signer off the allowlist ──
	const remaining = await setBundlers([bundler], false, relayer)
	check(
		"allowlist cleared of the rundler signer only",
		remaining.length === 1 && isAddressEqual(remaining[0], SIMULATION_ORIGIN),
		remaining,
	)

	const second = await placeOrder("refused")
	const secondOp = await buildBid(second.order, "refused")
	const secondHash = CryptoUtils.computeUserOpHash(secondOp, ENTRY_POINT, 97n)
	const logOffset = fs.statSync(rundler.logFile).size
	const fromBlock = await client.getBlockNumber()
	const refusal = await executeBid(second.order, secondOp, second.sessionPrivateKey).then(
		(executed) => ({ executed }),
		(error) => ({ error: error.message }),
	)
	// Leave rundler a few blocks to report what it did with the op.
	await pause(3_000)

	const bundles = []
	const toBlock = await client.getBlockNumber()
	for (let number = fromBlock; number <= toBlock; number++) {
		const block = await client.getBlock({ blockNumber: number, includeTransactions: true })
		for (const tx of block.transactions.filter((tx) => isAddressEqual(tx.from, bundler))) {
			const { status, gasUsed } = await client.getTransactionReceipt({ hash: tx.hash })
			const [top] = status === "reverted" ? await callsOf(tx.hash) : []
			bundles.push({ tx: tx.hash, block: number, status, gasUsed, origin: top && unauthorizedOrigin(top.output) })
		}
	}
	const logged = fs.readFileSync(rundler.logFile, "utf8").slice(logOffset).split("\n").filter(unauthorizedIn)
	const { where, origin } = [
		...bundles.map(({ tx, origin }) => ({ where: `bundle ${tx} reverted on chain`, origin })),
		{ where: "rundler refused eth_sendUserOperation", origin: unauthorizedIn(refusal.error) },
		{ where: "rundler dropped it before bundling", origin: logged.map(unauthorizedIn).find(Boolean) },
	].find((entry) => entry.origin) ?? { where: "nowhere" }
	const opReceipt = await jsonRpc(rundlerUrl, "eth_getUserOperationReceipt", [secondHash]).catch(() => null)
	const secondFilledBy = await read(GATEWAY, "_filled", [second.order.id])
	check(
		"second sponsored op refused with UnauthorizedBundler(rundler signer)",
		refusal.error &&
			origin &&
			isAddressEqual(origin, bundler) &&
			!opReceipt &&
			isAddressEqual(secondFilledBy, zeroAddress),
		{
			where,
			origin,
			userOpHash: secondHash,
			sdk: refusal.error ?? refusal.executed,
			bundles,
			rundlerLog: logged.map((line) => line.trim().slice(0, 300)),
			opReceipt,
			orderFilledBy: secondFilledBy,
		},
	)
}

let exitCode = 0
try {
	await main()
} catch (error) {
	exitCode = 1
	console.error(error)
	const rundler = children.find((child) => child.label === "rundler")
	if (rundler) console.error(redact(fs.readFileSync(rundler.logFile, "utf8").split("\n").slice(-40).join("\n")))
} finally {
	await stopAll()
}
const failed = checks.filter((entry) => !entry.ok).map((entry) => entry.name)
if (failed.length > 0) exitCode = 1
log(
	`${checks.length - failed.length}/${checks.length} checks passed${failed.length ? `; failed: ${failed.join("; ")}` : ""}`,
)
log(`logs in ${workdir}`)
process.exit(exitCode)
