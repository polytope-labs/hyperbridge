// Testnet swap end-to-end run: three simplex solvers post limit orders to a live HyperFX
// orderbook, and users place orders that the SDK executes against their bids.
//
// Usage: node e2e/run.mjs [scenario ...]    (built package; see e2e/env.mjs for the variables)
//
// Each scenario gets a freshly posted book, so one scenario's fills never starve the next.
// A scenario passes when its order is filled on the destination chain in at least `minFills`
// transactions. Settlement through Hyperbridge is not awaited.
import { spawn } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { createPublicClient, createWalletClient, erc20Abi, formatEther, formatUnits, http, parseUnits } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { EXTRA_LEVELS, SCENARIOS, SOLVERS, TOKENS, chains, readEnv, standingOrders } from "./env.mjs"

const HERE = path.dirname(fileURLToPath(import.meta.url))
const PACKAGE = path.resolve(HERE, "..")
const SIMPLEX = path.join(PACKAGE, "dist/bin/simplex.js")
const TIMEOUT_MIN = Number(process.env.E2E_SCENARIO_TIMEOUT_MIN || 10)

const env = readEnv()
const CHAINS = chains(env)
const workdir = process.env.E2E_WORKDIR || fs.mkdtempSync(path.join(os.tmpdir(), "simplex-e2e-"))
fs.mkdirSync(workdir, { recursive: true })
const selected = process.argv.slice(2).length > 0 ? process.argv.slice(2) : (process.env.E2E_SCENARIOS || Object.keys(SCENARIOS).join(",")).split(",")
for (const name of selected) if (!SCENARIOS[name]) throw new Error(`Unknown scenario ${name}`)

// Keys and endpoints never reach a report, whatever an error message quotes.
const SECRETS = Object.values(env).filter((value) => value && value.length > 8)
const redact = (text) => SECRETS.reduce((out, secret) => out.split(secret).join("***"), String(text))
const log = (line) => console.log(redact(`${new Date().toISOString()} ${line}`))
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

const solverKeys = [
	{ key: env.solver1Key, substrate: env.solver1Substrate },
	{ key: env.solver2Key, substrate: env.solver2Substrate },
	{ key: env.solver3Key, substrate: env.solver3Substrate },
]

function solverConfig(index) {
	const q = JSON.stringify
	return `[simplex]
maxConcurrentOrders = 5
logging = "info"
bidValiditySeconds = 600
substratePrivateKey = ${q(solverKeys[index].substrate)}
hyperbridgeWsUrl = ${q(env.hyperbridge)}

[simplex.signer]
type = "privateKey"
key = ${q(solverKeys[index].key)}

[orderbook]
url = ${q(env.orderbook)}

[assets.CNGN]
"EVM-97" = ${q(TOKENS["EVM-97"].cNGN.address)}
"EVM-80002" = ${q(TOKENS["EVM-80002"].cNGN.address)}

[[pairs]]
token0 = "USDC"
token1 = "CNGN"

[confirmationPolicies."97"]
points = [{ amount = "1", value = 1 }, { amount = "100000", value = 2 }]

[confirmationPolicies."80002"]
points = [{ amount = "1", value = 1 }, { amount = "100000", value = 2 }]

[[chains]]
rpcUrls = [${q(env.bscRpc)}]
bundlerUrl = ${q(env.bscBundler)}

[[chains]]
rpcUrls = [${q(env.amoyRpc)}]
bundlerUrl = ${q(env.amoyBundler)}
`
}

async function api(port, method, route, body) {
	const response = await fetch(`http://127.0.0.1:${port}${route}`, {
		method,
		headers: { "content-type": "application/json", "X-Simplex-UI": "1" },
		body: body ? JSON.stringify(body) : undefined,
	})
	const text = await response.text()
	let json
	try {
		json = JSON.parse(text)
	} catch {
		json = { raw: text }
	}
	return { status: response.status, ok: response.ok, json }
}

const processes = []

function startSolvers() {
	SOLVERS.forEach((solver, index) => {
		const dir = path.join(workdir, solver.name)
		fs.mkdirSync(path.join(dir, "data"), { recursive: true })
		const config = path.join(dir, "config.toml")
		fs.writeFileSync(config, solverConfig(index), { mode: 0o600 })
		const out = fs.openSync(path.join(dir, "solver.log"), "a")
		const child = spawn(
			process.execPath,
			[SIMPLEX, "run", "-c", config, "-d", path.join(dir, "data"), "--ui", `127.0.0.1:${solver.port}`, "--no-open", "--log-format", "json"],
			{ stdio: ["ignore", out, out] },
		)
		child.on("exit", (code, signal) => log(`${solver.name} exited (${code ?? signal})`))
		processes.push(child)
		log(`${solver.name} started, pid ${child.pid}`)
	})
}

async function stopSolvers() {
	for (const child of processes) if (child.exitCode === null) child.kill("SIGTERM")
	for (let i = 0; i < 20 && processes.some((child) => child.exitCode === null); i++) await pause(500)
	for (const child of processes) if (child.exitCode === null) child.kill("SIGKILL")
}

async function waitReady() {
	for (const solver of SOLVERS) {
		const until = Date.now() + 180_000
		for (;;) {
			const ready = await api(solver.port, "GET", "/api/limit-orders?status=open").catch(() => undefined)
			if (ready?.ok) break
			if (Date.now() > until) throw new Error(`${solver.name} never answered on port ${solver.port}`)
			await pause(2_000)
		}
		log(`${solver.name} ready`)
	}
}

async function liveOrders(solver) {
	const live = []
	for (const status of ["open", "resizing"]) {
		const listed = await api(solver.port, "GET", `/api/limit-orders?status=${status}`)
		if (listed.ok) live.push(...(listed.json.orders ?? []))
	}
	return live
}

/** Withdraws every limit order the solvers hold. */
async function clearBook() {
	for (let attempt = 0; attempt < 6; attempt++) {
		let left = 0
		for (const solver of SOLVERS) {
			for (const order of await liveOrders(solver)) {
				const cancelled = await api(solver.port, "DELETE", `/api/limit-orders/${order.id}`)
				if (!cancelled.ok) left++
			}
		}
		if (left === 0) return
		// An order still being resized after a fill cannot be withdrawn yet.
		await pause(10_000)
	}
	log("warning: some limit orders could not be withdrawn")
}

async function create(solver, body) {
	const created = await api(solver.port, "POST", "/api/limit-orders", body)
	const line = `${body.amountIn} ${body.tokenIn} -> ${body.amountOut} ${body.tokenOut} on ${body.fillChain}`
	if (!created.ok) throw new Error(`${solver.name} could not post ${line}: ${created.status} ${JSON.stringify(created.json).slice(0, 300)}`)
	log(`${solver.name} posted ${line}`)
}

/** Posts the standing book, plus solver 1's extra levels the scenario names. */
async function postBook(levels = []) {
	await clearBook()
	for (const solver of SOLVERS) for (const body of standingOrders(solver)) await create(solver, body)
	for (const level of levels) for (const body of EXTRA_LEVELS[level]) await create(SOLVERS[0], body)
}

/** What the selected scenarios spend, by user, chain and token. */
function spending() {
	const need = {}
	for (const name of selected) {
		const s = SCENARIOS[name]
		for (const leg of s.legs) {
			const key = `${s.user}:${s.source}:${leg.tokenIn}`
			need[key] = (need[key] ?? 0) + Number(leg.amountIn)
		}
	}
	return need
}

/** What a solver keeps back to post its own limit orders when it tops up a user. */
const SOLVER_RESERVE = { USDC: "400", cNGN: "800000" }

/**
 * Tops each user up from a solver, and fails early, and legibly, on a wallet that still cannot
 * pay for the run. Users pay the solvers one token and are paid the other, so the solvers hold
 * what the users spent; moving it back keeps the wallets going with nothing but gas.
 */
async function preflight() {
	const need = spending()
	const users = [env.user1Key, env.user2Key].map((key) => privateKeyToAccount(key))
	const solvers = solverKeys.map(({ key }) => privateKeyToAccount(key))
	const problems = []
	for (const [chain, config] of Object.entries(CHAINS)) {
		const client = createPublicClient({ chain: config.viem, transport: http(config.rpc) })
		const balanceOf = (token, address) => client.readContract({ address: token.address, abi: erc20Abi, functionName: "balanceOf", args: [address] })
		for (const [index, user] of users.entries()) {
			for (const [symbol, token] of Object.entries(TOKENS[chain])) {
				const spends = need[`${index}:${chain}:${symbol}`]
				if (!spends) continue
				const required = parseUnits(String(spends), token.decimals)
				const held = await balanceOf(token, user.address)
				if (held >= required) continue
				// Half as much again, so a top-up is not needed on every run.
				const amount = (required * 3n) / 2n - held
				const reserve = parseUnits(SOLVER_RESERVE[symbol], token.decimals)
				let donor
				let most = 0n
				for (const solver of solvers) {
					const balance = await balanceOf(token, solver.address)
					if (balance - amount >= reserve && balance > most) [donor, most] = [solver, balance]
				}
				if (!donor) {
					problems.push(`user${index + 1} ${user.address} holds ${formatUnits(held, token.decimals)} ${symbol} on ${chain}, the run spends ${spends}, and no solver can spare it`)
					continue
				}
				const wallet = createWalletClient({ account: donor, chain: config.viem, transport: http(config.rpc) })
				const hash = await wallet.writeContract({ address: token.address, abi: erc20Abi, functionName: "transfer", args: [user.address, amount] })
				await client.waitForTransactionReceipt({ hash })
				log(`topped up user${index + 1} with ${formatUnits(amount, token.decimals)} ${symbol} on ${chain} from ${donor.address}: ${hash}`)
			}
		}
		for (const [role, accounts] of [
			["user", users],
			["solver", solvers],
		]) {
			for (const [index, { address }] of accounts.entries()) {
				const gas = await client.getBalance({ address })
				if (gas === 0n) problems.push(`${role}${index + 1} ${address} has no gas on ${chain}`)
				const held = []
				for (const [symbol, token] of Object.entries(TOKENS[chain])) {
					held.push(`${formatUnits(await balanceOf(token, address), token.decimals)} ${symbol}`)
				}
				log(`${role}${index + 1} ${address} on ${chain}: ${formatEther(gas)} gas, ${held.join(", ")}`)
			}
		}
	}
	if (problems.length > 0) throw new Error(`Fund these wallets first:\n${problems.join("\n")}`)
}

function runScenario(name) {
	const resultFile = path.join(workdir, `${name}.result.json`)
	const logFile = path.join(workdir, `${name}.log`)
	fs.rmSync(resultFile, { force: true })
	return new Promise((resolve) => {
		const out = fs.openSync(logFile, "w")
		const child = spawn(process.execPath, [path.join(HERE, "swap.mjs"), name, resultFile], { stdio: ["ignore", out, out], cwd: PACKAGE })
		const killer = setTimeout(() => child.kill("SIGKILL"), (TIMEOUT_MIN + 3) * 60_000)
		child.on("exit", () => {
			clearTimeout(killer)
			const text = fs.readFileSync(logFile, "utf8")
			process.stdout.write(redact(text))
			const result = fs.existsSync(resultFile) ? JSON.parse(fs.readFileSync(resultFile, "utf8")) : { outcome: "NO RESULT", fills: [] }
			resolve(result)
		})
	})
}

function report(results) {
	const lines = [
		"## Simplex testnet swaps",
		"",
		"| Scenario | Outcome | Fills | Bids offered | Fill transactions |",
		"| --- | --- | --- | --- | --- |",
	]
	for (const r of results) {
		const bids = r.bidRounds?.[0]?.length ?? 0
		const txs = r.fills.map((fill) => `\`${fill.transactionHash}\``).join("<br>")
		lines.push(`| ${r.scenario} | ${r.passed ? "✅" : "❌"} ${r.outcome}${r.reason ? ` (${r.reason})` : ""} | ${r.fills.length} | ${bids} | ${txs} |`)
	}
	const text = redact(lines.join("\n"))
	console.log(text)
	if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${text}\n`)
	fs.writeFileSync(path.join(workdir, "summary.json"), redact(JSON.stringify(results, null, 2)))
}

async function main() {
	log(`working in ${workdir}; scenarios: ${selected.join(", ")}`)
	await preflight()
	startSolvers()
	const results = []
	try {
		await waitReady()
		for (const name of selected) {
			const scenario = SCENARIOS[name]
			log(`== ${name}`)
			let result
			try {
				await postBook(scenario.levels)
				result = await runScenario(name)
			} catch (error) {
				result = { scenario: name, outcome: "ERROR", error: String(error?.message ?? error), fills: [] }
			}
			result.scenario = name
			result.passed = result.outcome === "FILLED" && result.fills.length >= scenario.minFills
			if (result.outcome === "FILLED" && !result.passed) result.reason = `expected ${scenario.minFills}+ fills`
			if (result.error) result.reason = result.error.slice(0, 200)
			log(`== ${name}: ${result.passed ? "passed" : "FAILED"} (${result.outcome}, ${result.fills.length} fills)`)
			results.push(result)
			// Let the solvers settle the fills against their limit orders before the book is reposted.
			await pause(20_000)
		}
	} finally {
		await clearBook().catch((error) => log(`could not withdraw the book: ${error}`))
		await stopSolvers()
	}
	report(results)
	const passed = results.every((result) => result.passed)
	if (!passed) printSolverLogs()
	process.exit(passed ? 0 : 1)
}

/** The tail of each solver's log, redacted: they are not uploaded, since they quote endpoints. */
function printSolverLogs() {
	for (const solver of SOLVERS) {
		const file = path.join(workdir, solver.name, "solver.log")
		if (!fs.existsSync(file)) continue
		const tail = fs.readFileSync(file, "utf8").trimEnd().split("\n").slice(-60).join("\n")
		console.log(`::group::${solver.name} log (last 60 lines)\n${redact(tail)}\n::endgroup::`)
	}
}

for (const signal of ["SIGINT", "SIGTERM"]) {
	process.on(signal, async () => {
		await stopSolvers()
		process.exit(130)
	})
}

main().catch(async (error) => {
	log(`run failed: ${error?.message ?? error}`)
	await stopSolvers()
	process.exit(1)
})
