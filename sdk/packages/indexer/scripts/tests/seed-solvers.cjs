// Gives the E2E's solvers a balance and a delegation on an anvil fork of Base, then reads both
// back through the same calls the indexer will make.
//
// Balances are written straight into USDC's balance mapping rather than transferred from a whale:
// the slot is already config (`tokenSlots` in config-mainnet.json), and a write leaves no Transfer
// log, so what the indexer reports afterwards can only have come from its genesis read.
//
// Delegation is the EIP-7702 designator itself — `0xef0100 ‖ address` as the account's code — which
// is what the chain holds after an authorization and what `parseDelegation` reads.
//
// Run: node scripts/tests/seed-solvers.cjs [--rpc http://127.0.0.1:8545]
const { encodeAbiParameters, encodeFunctionData, keccak256, pad, toHex } = require("viem")

const { SOLVERS, USDC, VAULT } = require("./solver-fixtures.cjs")

/** Multicall3, which the indexer's batched reads go through. */
const MULTICALL3 = "0xcA11bde05977b3631167028862bE2a173976CA11"

const rpcUrl = process.env.ANVIL_URL || argValue("--rpc") || "http://127.0.0.1:8545"
/** Gas money, so the plain EOA can send the Transfer the assertions wait for. */
const GAS_BUDGET = 10n ** 18n

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

/** Where `mapping(address => uint256)` at `slot` keeps `holder`, the layout Solidity generates. */
const balanceKey = (holder, slot) =>
	keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [holder, BigInt(slot)]))

const balanceOf = (holder) => encodeFunctionData({ abi: erc20Abi, args: [holder] })

const erc20Abi = [
	{ name: "balanceOf", type: "function", inputs: [{ type: "address" }], outputs: [{ type: "uint256" }] },
]
const vaultAbi = [
	...erc20Abi,
	{ name: "convertToAssets", type: "function", inputs: [{ type: "uint256" }], outputs: [{ type: "uint256" }] },
]
const aggregate3Abi = [
	{
		name: "aggregate3",
		type: "function",
		inputs: [
			{
				type: "tuple[]",
				components: [{ type: "address" }, { type: "bool" }, { type: "bytes" }],
			},
		],
		outputs: [{ type: "tuple[]", components: [{ type: "bool" }, { type: "bytes" }] }],
	},
]

/**
 * Makes every read the indexer's genesis pass will make, now, while the fork is seconds old.
 *
 * anvil resolves any account it has not seen from the fork base upstream, and pins that request to
 * the fork block. By the time discovery finishes — a Hyperbridge block, an EVM block, a head
 * advance — that block is a couple of hundred behind the chain, past the window a non-archive
 * endpoint will serve, and anvil blocks on it forever: it stops answering, stops mining, and the
 * whole run times out with nothing indexed. Reading the same accounts here caches them while the
 * request is still cheap, so the genesis pass never reaches upstream at all.
 */
async function warmFork() {
	const calls = []
	for (const solver of SOLVERS) {
		calls.push({ target: USDC.address, callData: balanceOf(solver.address) })
		calls.push({ target: VAULT, callData: balanceOf(solver.address) })
	}
	const data = encodeFunctionData({
		abi: aggregate3Abi,
		args: [calls.map((call) => [call.target, true, call.callData])],
	})
	await rpc("eth_getCode", [MULTICALL3, "latest"])
	await rpc("eth_call", [{ to: MULTICALL3, data }, "latest"])
	await rpc("eth_call", [
		{
			to: VAULT,
			data: encodeFunctionData({ abi: vaultAbi, functionName: "convertToAssets", args: [10n ** 6n] }),
		},
		"latest",
	])
	console.log(`[seed] warmed Multicall3, ${USDC.address} and ${VAULT} into the fork's cache`)
}

async function main() {
	const chainId = await rpc("eth_chainId")
	const blockNumber = await rpc("eth_blockNumber")
	if (BigInt(chainId) !== 8453n) {
		throw new Error(`expected a Base fork (chain id 8453), got ${BigInt(chainId)}`)
	}
	console.log(`[seed] anvil at ${rpcUrl}, chain ${BigInt(chainId)}, head ${BigInt(blockNumber)}`)

	for (const solver of SOLVERS) {
		await rpc("anvil_setBalance", [solver.address, toHex(GAS_BUDGET)])
		await rpc("anvil_setStorageAt", [
			USDC.address,
			balanceKey(solver.address, USDC.balanceSlot),
			pad(toHex(solver.usdc), { size: 32 }),
		])
		if (solver.delegateTo) {
			await rpc("anvil_setCode", [solver.address, `0xef0100${solver.delegateTo.slice(2)}`])
		}

		// Read both back exactly as the indexer's genesis read will, so a wrong slot or a silently
		// ignored override fails here rather than as a mystified assertion twenty minutes later.
		const balance = BigInt(await rpc("eth_call", [{ to: USDC.address, data: balanceOf(solver.address) }, "latest"]))
		if (balance !== solver.usdc) {
			throw new Error(`${solver.name}: balanceOf says ${balance}, seeded ${solver.usdc}`)
		}
		const code = (await rpc("eth_getCode", [solver.address, "latest"])).toLowerCase()
		const wanted = solver.delegateTo ? `0xef0100${solver.delegateTo.slice(2)}`.toLowerCase() : "0x"
		if (code !== wanted) throw new Error(`${solver.name}: code is ${code}, expected ${wanted}`)

		console.log(
			`[seed] ${solver.address} (${solver.name}): ${solver.usdc} USDC units, ` +
				(solver.delegateTo ? `delegated to ${solver.delegateTo}` : "no delegation"),
		)
	}
	await warmFork()
	console.log(`[seed] ${SOLVERS.length} solvers ready`)
}

main().catch((error) => {
	console.error(`[seed] ${error.message}`)
	process.exit(1)
})
