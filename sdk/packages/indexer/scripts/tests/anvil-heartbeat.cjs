// Keeps every block on the E2E's anvil fork non-empty.
//
// SubQuery's Ethereum node only runs block handlers on blocks it considers "full", and its
// `isFullBlock` returns false for a block with no transactions (`block.ethereum.js`: an empty block
// is indistinguishable from a light one, so it is treated as light). An idle fork mines nothing but
// empty blocks, so `handleSolverInventoryBlock` — the handler that consumes watchlist requests,
// runs genesis reads and advances the head — would never fire, and the whole test would time out
// waiting for rows that were never going to be written.
//
// A real chain never has this problem: its blocks carry transactions. So this sends one cheap
// self-transfer per block from an anvil dev account, which is enough to make each block full.
//
// Run: node scripts/tests/anvil-heartbeat.cjs [--rpc http://127.0.0.1:8545] [--interval 1500]

/** anvil's first dev account, funded and unlocked on every fork. */
const SENDER = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

const rpcUrl = process.env.ANVIL_URL || argValue("--rpc") || "http://127.0.0.1:8545"
/** Comfortably under the 2s block time, so no block goes out empty. */
const intervalMs = Number(process.env.HEARTBEAT_INTERVAL_MS || argValue("--interval") || 1500)

function argValue(flag) {
	const index = process.argv.indexOf(flag)
	return index === -1 ? undefined : process.argv[index + 1]
}

let nextId = 1
async function rpc(method, params = []) {
	const response = await fetch(rpcUrl, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ jsonrpc: "2.0", id: nextId++, method, params }),
	})
	const body = await response.json()
	if (body.error) throw new Error(`${method}: ${body.error.message}`)
	return body.result
}

let sent = 0
let failures = 0

async function beat() {
	try {
		await rpc("eth_sendTransaction", [{ from: SENDER, to: SENDER, value: "0x0" }])
		sent += 1
		if (sent % 20 === 0) console.log(`[heartbeat] ${sent} transactions sent`)
	} catch (error) {
		failures += 1
		// Never exit: the test's own transactions also fill blocks, and a transient failure here is
		// not worth failing the run over. It only becomes visible if it is constant.
		if (failures % 10 === 1) console.warn(`[heartbeat] send failed (${failures} so far): ${error.message}`)
	}
}

async function main() {
	const chainId = await rpc("eth_chainId")
	console.log(`[heartbeat] ${rpcUrl}, chain ${BigInt(chainId)}, one transaction every ${intervalMs}ms`)
	// anvil funds its dev accounts on a fork, but say so plainly if this one is empty.
	const balance = BigInt(await rpc("eth_getBalance", [SENDER, "latest"]))
	if (balance === 0n) throw new Error(`${SENDER} has no ETH to send from`)
	setInterval(() => void beat(), intervalMs)
}

main().catch((error) => {
	console.error(`[heartbeat] ${error.message}`)
	process.exit(1)
})

for (const signal of ["SIGINT", "SIGTERM"]) {
	process.on(signal, () => {
		console.log(`[heartbeat] sent ${sent} transactions, ${failures} failures`)
		process.exit(0)
	})
}
