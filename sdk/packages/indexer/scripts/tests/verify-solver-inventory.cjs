// The solver-inventory E2E's assertions, run against a live indexer.
//
// It proves the whole path the orderbook depends on, end to end and in order:
//
//   1. Discovery. The Hyperbridge node polled the orderbook's `GET /solvers` and queued each solver,
//      so every row's `discoveredBy` is WATCHLIST — not FILL, which is the other way in and would
//      mean the watchlist was never read.
//   2. Genesis. The EVM node's pinned read found the balance and the EIP-7702 designator the fork
//      was seeded with, with delegation counted only for a known SolverAccount.
//   3. Events. A real USDC Transfer then moves two tracked rows in opposite directions, from the log
//      alone. A balance seeded by a storage write emits nothing, so a row that moved by exactly the
//      transferred amount can only have been event-sourced.
//
// Run: node scripts/tests/verify-solver-inventory.cjs
const { encodeFunctionData } = require("viem")

const { CHAIN, SOLVERS, TRANSFER, USDC, expectedWallets } = require("./solver-fixtures.cjs")

const graphqlUrl = process.env.GRAPHQL_URL || "http://127.0.0.1:3100"
const anvilUrl = process.env.ANVIL_URL || "http://127.0.0.1:8545"
/** Discovery waits on a Hyperbridge block, the genesis read on the next Base block, then the head. */
const GENESIS_TIMEOUT_MS = Number(process.env.GENESIS_TIMEOUT_MS || 900_000)
/** Once tracked, a Transfer is applied by the log handler that indexes its block. */
const TRANSFER_TIMEOUT_MS = Number(process.env.TRANSFER_TIMEOUT_MS || 300_000)
const POLL_INTERVAL_MS = 5_000

const addresses = SOLVERS.map((solver) => solver.address)
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

/** An error worth giving up on immediately: waiting cannot fix it. */
class FatalError extends Error {}

async function graphql(query) {
	const response = await fetch(graphqlUrl, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ query }),
	})
	if (!response.ok) throw new Error(`query service answered ${response.status}`)
	const body = await response.json()
	if (body.errors) {
		const message = body.errors.map((error) => error.message).join("; ")
		// The query service reads the schema once at startup. If it booted before the indexer
		// created its tables, the entities are missing from the Query type for the rest of the run,
		// and every retry from here would report this same thing until the timeout.
		if (message.includes("Unknown field")) {
			throw new FatalError(
				`the query service has no solver entities: ${message}. It was started before the ` +
					`indexer created them — restart it, or gate it on the node that owns the schema.`,
			)
		}
		throw new Error(message)
	}
	return body.data
}

let nextId = 1
async function rpc(method, params = []) {
	const response = await fetch(anvilUrl, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", id: nextId++, method, params }),
	})
	const body = await response.json()
	if (body.error) throw new Error(`${method}: ${body.error.message}`)
	return body.result
}

/** The query the orderbook itself runs, plus the discovery trigger this test turns on. */
const STATE_QUERY = `query {
	solverInventoryHead(id: "${CHAIN}") { blockNumber observedAt }
	trackedSolvers(filter: { chain: { equalTo: "${CHAIN}" }, solver: { in: ${JSON.stringify(addresses)} } }) {
		nodes { solver status discoveredBy genesisBlock }
	}
	solverInventories(filter: { chain: { equalTo: "${CHAIN}" }, solver: { in: ${JSON.stringify(addresses)} } }) {
		nodes { solver tokenAddress balance wallet vaults vaultShares blockNumber }
	}
	solverDelegations(filter: { chain: { equalTo: "${CHAIN}" }, solver: { in: ${JSON.stringify(addresses)} } }) {
		nodes { solver delegated delegate }
	}
}`

const bySolver = (nodes) => new Map(nodes.map((node) => [node.solver.toLowerCase(), node]))

async function state() {
	const data = await graphql(STATE_QUERY)
	return {
		head: data.solverInventoryHead,
		tracked: bySolver(data.trackedSolvers.nodes),
		inventories: bySolver(data.solverInventories.nodes.filter((row) => row.tokenAddress === USDC.address)),
		delegations: bySolver(data.solverDelegations.nodes),
	}
}

/**
 * Polls until `check` stops throwing. Its last complaint is what the failure reports, so the
 * timeout says which solver was missing what rather than that something timed out.
 */
async function waitFor(label, timeoutMs, check) {
	const deadline = Date.now() + timeoutMs
	let lastError = new Error("never ran")
	while (Date.now() < deadline) {
		try {
			const value = await check()
			console.log(`[verify] ${label}: ok`)
			return value
		} catch (error) {
			if (error instanceof FatalError) throw error
			lastError = error
			const remaining = Math.round((deadline - Date.now()) / 1000)
			console.log(`[verify] ${label}: waiting (${remaining}s left) — ${error.message}`)
			await sleep(POLL_INTERVAL_MS)
		}
	}
	throw new Error(`${label}: timed out after ${Math.round(timeoutMs / 1000)}s — ${lastError.message}`)
}

function assert(condition, message) {
	if (!condition) throw new Error(message)
}

async function assertDiscoveredAndSeeded() {
	const { head, tracked, inventories, delegations } = await state()
	assert(head, "no SolverInventoryHead yet")

	for (const solver of SOLVERS) {
		const row = tracked.get(solver.address)
		assert(row, `${solver.name} is not tracked yet`)
		assert(row.status === "TRACKED", `${solver.name} is ${row.status}, genesis read has not landed`)
		// The whole point of the fake orderbook: this is the only way in for a solver that never filled.
		assert(
			row.discoveredBy === "WATCHLIST",
			`${solver.name} was discovered by ${row.discoveredBy}, not the watchlist`,
		)

		const inventory = inventories.get(solver.address)
		assert(inventory, `${solver.name} has no USDC inventory row`)
		assert(
			BigInt(inventory.wallet) === solver.usdc,
			`${solver.name} wallet is ${inventory.wallet}, seeded ${solver.usdc}`,
		)
		assert(
			BigInt(inventory.vaults) === 0n && BigInt(inventory.vaultShares) === 0n,
			`${solver.name} holds no vault shares but reports ${inventory.vaultShares}`,
		)
		assert(
			BigInt(inventory.balance) === solver.usdc,
			`${solver.name} balance is ${inventory.balance}, expected wallet + vaults = ${solver.usdc}`,
		)

		const delegation = delegations.get(solver.address)
		assert(delegation, `${solver.name} has no delegation row`)
		assert(
			delegation.delegated === solver.expect.delegated,
			`${solver.name} delegated=${delegation.delegated}, expected ${solver.expect.delegated}`,
		)
		const delegate = delegation.delegate ? delegation.delegate.toLowerCase() : null
		assert(
			delegate === solver.expect.delegate,
			`${solver.name} delegate=${delegate}, expected ${solver.expect.delegate}`,
		)
	}
	return { tracked, inventories }
}

async function sendTransfer() {
	const data = encodeFunctionData({
		abi: [
			{
				name: "transfer",
				type: "function",
				inputs: [{ type: "address" }, { type: "uint256" }],
				outputs: [{ type: "bool" }],
			},
		],
		args: [TRANSFER.to, TRANSFER.amount],
	})
	await rpc("anvil_impersonateAccount", [TRANSFER.from])
	const hash = await rpc("eth_sendTransaction", [{ from: TRANSFER.from, to: USDC.address, data }])
	await rpc("anvil_stopImpersonatingAccount", [TRANSFER.from])

	const receipt = await waitFor("transfer mined", 120_000, async () => {
		const mined = await rpc("eth_getTransactionReceipt", [hash])
		assert(mined, `${hash} is still pending`)
		assert(BigInt(mined.status) === 1n, `${hash} reverted`)
		return mined
	})
	console.log(
		`[verify] transferred ${TRANSFER.amount} USDC units ${TRANSFER.from} → ${TRANSFER.to} ` +
			`in block ${BigInt(receipt.blockNumber)}`,
	)
	return BigInt(receipt.blockNumber)
}

async function main() {
	console.log(`[verify] indexer ${graphqlUrl}, anvil ${anvilUrl}`)

	const seeded = await waitFor("watchlist discovery and genesis read", GENESIS_TIMEOUT_MS, assertDiscoveredAndSeeded)
	const before = new Map([...seeded.inventories].map(([solver, row]) => [solver, BigInt(row.blockNumber)]))

	const transferBlock = await sendTransfer()
	const wallets = expectedWallets()

	await waitFor("transfer applied to both sides", TRANSFER_TIMEOUT_MS, async () => {
		const { inventories } = await state()
		for (const [address, expected] of wallets) {
			const row = inventories.get(address)
			assert(row, `${address} lost its inventory row`)
			assert(BigInt(row.wallet) === expected, `${address} wallet is ${row.wallet}, expected ${expected}`)
			assert(BigInt(row.balance) === expected, `${address} balance is ${row.balance}, expected ${expected}`)
		}
		// Only the two sides of the Transfer move; the third solver must sit exactly where it was.
		const untouched = SOLVERS.find((solver) => solver.address !== TRANSFER.from && solver.address !== TRANSFER.to)
		const bystander = inventories.get(untouched.address)
		assert(
			BigInt(bystander.blockNumber) === before.get(untouched.address),
			`${untouched.name} moved to block ${bystander.blockNumber} without a Transfer of its own`,
		)
		for (const address of [TRANSFER.from, TRANSFER.to]) {
			assert(
				BigInt(inventories.get(address).blockNumber) >= transferBlock,
				`${address} is still positioned before the Transfer's block ${transferBlock}`,
			)
		}
	})

	console.log("[verify] solver inventory indexed correctly from the watchlist, the fork and its logs")
}

main().catch((error) => {
	console.error(`[verify] FAILED: ${error.message}`)
	process.exit(1)
})
