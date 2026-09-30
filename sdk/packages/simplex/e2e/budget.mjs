// The on-chain limit on a limit order's size. A solver's account tallies what each of its limit
// orders has paid out and refuses a fill that would take the tally past the order's size. These
// checks read the tally back after a scenario and simulate a payout past the size. Nothing is sent.
import {
	BaseError,
	ContractFunctionZeroDataError,
	decodeErrorResult,
	encodeFunctionData,
	erc20Abi,
	formatUnits,
	keccak256,
	parseAbi,
	parseUnits,
	stringToBytes,
	zeroAddress,
} from "viem"
import { GATEWAY, TOKENS } from "./env.mjs"

// Simplex exports neither of these from its package root, so they are restated here.
export const ACCOUNT_ABI = parseAbi([
	"function debitOrder(bytes32 orderId, uint256 cap, address token, uint256 approved, uint256 fee)",
	"function spent(bytes32 orderId) view returns (uint256)",
	"error LimitOrderExceeded(bytes32 orderId, uint256 total, uint256 cap)",
])

/** The key a limit order is tallied under: `keccak256` of its id as simplex stores it. */
export const budgetIdFor = (id) => keccak256(stringToBytes(id))

/** A 1e18 amount in the token's own units, truncated as simplex truncates it. */
export function toRaw(scaled, decimals) {
	const shift = 18 - decimals
	return shift >= 0 ? scaled / 10n ** BigInt(shift) : scaled * 10n ** BigInt(-shift)
}

/** The most a limit order created with `amountOut` may pay out, in the token's own units. */
export const capFor = (amountOut, decimals) => toRaw(parseUnits(String(amountOut), 18), decimals)

/** What the runner keeps of a limit order it posted: enough to find its tally and its cap. */
export function keep(solver, account, id, body) {
	const { address: token, decimals } = TOKENS[body.fillChain][body.tokenOut]
	return {
		solver: solver.name,
		port: solver.port,
		account,
		id,
		fillChain: body.fillChain,
		symbol: body.tokenOut,
		token,
		decimals,
		amountOut: body.amountOut,
		budgetId: budgetIdFor(id),
		cap: capFor(body.amountOut, decimals),
	}
}

/** Calldata for a `debitOrder` that counts `approved`, less the allowance left, against `order`. */
export const debitCall = (order, approved) =>
	encodeFunctionData({
		abi: ACCOUNT_ABI,
		functionName: "debitOrder",
		args: [order.budgetId, order.cap, order.token, approved, 0n],
	})

/** `LimitOrderExceeded`'s arguments when `data` is that revert, undefined otherwise. */
export function decodeRefusal(data) {
	try {
		const { errorName, args } = decodeErrorResult({ abi: ACCOUNT_ABI, data })
		if (errorName !== "LimitOrderExceeded") return undefined
		const [orderId, total, cap] = args
		return { orderId, total, cap }
	} catch {
		return undefined
	}
}

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
const show = (order, raw) => `${formatUnits(raw, order.decimals)} ${order.symbol}`
const label = (order) => `limit order ${order.id} of ${order.solver} on ${order.fillChain}`
const brief = (error) =>
	String(error?.shortMessage ?? error?.message ?? error)
		.split("\n")
		.filter(Boolean)
		.slice(0, 2)
		.join(" ")

/** Nodes put an `eth_call`'s revert data at `data` or at `data.data`. */
function revertData(error) {
	const cause = error instanceof BaseError ? error.walk() : error
	return typeof cause?.data === "object" ? cause.data?.data : cause?.data
}

const readSpent = (client, order, blockNumber) =>
	client.readContract({
		address: order.account,
		abi: ACCOUNT_ABI,
		functionName: "spent",
		args: [order.budgetId],
		blockNumber,
	})

const over = (order, spent) =>
	spent > order.cap
		? `${label(order)} has spent ${show(order, spent)}, over its size ${show(order, order.cap)}`
		: undefined

/**
 * Why `spent` could not be read. An account whose code has no `spent` reverts or returns nothing,
 * which is a solver delegated to an older implementation rather than a node that did not answer.
 * Only the node's own revert, JSON-RPC code 3, counts: viem reports a node's internal error as a
 * contract revert too.
 */
async function unreadable(client, order, error) {
	const absent = (cause) => cause instanceof ContractFunctionZeroDataError || cause?.code === 3
	if (!(error instanceof BaseError) || !error.walk(absent)) return { reason: brief(error) }
	const code = await client.getCode({ address: order.account }).catch(() => null)
	const delegate =
		code === null
			? ""
			: code?.startsWith("0xef0100")
				? `: it delegates to 0x${code.slice(8)}`
				: ": it has no delegation"
	return {
		undelegated: true,
		reason: `${order.solver} ${order.account} on ${order.fillChain} is not delegated to an implementation with spent${delegate}`,
	}
}

/** Waits for the solver to have settled the fill, so that what it shows delivered is the tally. */
async function settled({ clients, api, log }, order, first) {
	const until = Date.now() + 90_000
	let spent = first
	let shown
	for (;;) {
		spent = await readSpent(clients[order.fillChain], order).catch(() => spent)
		const got = await api(order.port, "GET", `/api/limit-orders/${order.id}`).catch(() => undefined)
		if (got?.ok && got.json.order) {
			shown = toRaw(BigInt(got.json.order.size) - BigInt(got.json.order.remaining), order.decimals)
			if (shown === spent) {
				log(`${label(order)}: ${order.solver} shows ${show(order, shown)} delivered, as its account has spent`)
				return undefined
			}
		}
		if (Date.now() > until) break
		await pause(5_000)
	}
	const delivered = shown === undefined ? "nothing" : `${show(order, shown)} delivered`
	return `${label(order)}: the solver shows ${delivered}, its account has spent ${show(order, spent)}`
}

/**
 * Simulates, as the account calling itself, a payout that lands on the cap and one a unit past it.
 * `debitOrder` counts `approved` less the gateway's allowance, so the allowance is added back.
 *
 * Everything is read and simulated at one block: the orderbook is shared, and a fill by anyone
 * else between the reads and the calls would move the tally under them.
 */
async function refusal({ clients, log }, order) {
	const client = clients[order.fillChain]
	const blockNumber = await client.getBlockNumber({ cacheTime: 0 })
	const spent = await readSpent(client, order, blockNumber)
	if (over(order, spent)) return over(order, spent)
	const allowance = await client.readContract({
		address: order.token,
		abi: erc20Abi,
		functionName: "allowance",
		args: [order.account, GATEWAY],
		blockNumber,
	})
	const room = order.cap - spent
	const total = spent + room + 1n
	const simulate = (approved) =>
		client.call({ account: order.account, to: order.account, data: debitCall(order, approved), blockNumber }).then(
			() => undefined,
			(error) => error,
		)

	const atCap = await simulate(allowance + room)
	if (atCap) return `${label(order)}: a payout up to its size ${show(order, order.cap)} was refused: ${brief(atCap)}`
	const past = await simulate(allowance + room + 1n)
	if (!past)
		return `${label(order)}: a payout of ${show(order, total)} was not refused, its size is ${show(order, order.cap)}`
	const refused = decodeRefusal(revertData(past))
	if (!refused) return `${label(order)}: a payout past its size failed without LimitOrderExceeded: ${brief(past)}`
	const said = `LimitOrderExceeded(${refused.orderId}, ${refused.total}, ${refused.cap})`
	if (refused.orderId !== order.budgetId || refused.total !== total || refused.cap !== order.cap) {
		return `${label(order)}: refused with ${said}, expected total ${total} and cap ${order.cap} under ${order.budgetId}`
	}
	log(
		`${label(order)}: at block ${blockNumber} a payout up to ${show(order, order.cap)} passes, one unit more is refused with ${said}`,
	)
	return undefined
}

/** The checks of a `check: "limit"` scenario, over the tallies read on its fill chain. */
async function limitChecks(ctx, scenario, tallies) {
	// A node behind the one that served the fill's receipt still answers zero.
	for (let tries = 0; tries < 5 && tallies.every(({ spent }) => spent === 0n); tries++) {
		await pause(3_000)
		for (const tally of tallies)
			tally.spent = await readSpent(ctx.clients[scenario.dest], tally.order).catch(() => tally.spent)
	}
	const debited = tallies.filter(({ spent }) => spent > 0n)
	if (debited.length === 0)
		return `no limit order on ${scenario.dest} was debited: spent is 0 on all ${tallies.length}`
	for (const { order, spent } of debited) if (over(order, spent)) return over(order, spent)

	for (const symbol of new Set(scenario.legs.map((leg) => leg.tokenOut))) {
		const { decimals } = TOKENS[scenario.dest][symbol]
		const owed = scenario.legs.reduce(
			(sum, leg) => (leg.tokenOut === symbol ? sum + parseUnits(leg.minOut, decimals) : sum),
			0n,
		)
		const paid = tallies.reduce((sum, { order, spent }) => (order.symbol === symbol ? sum + spent : sum), 0n)
		const line = `${formatUnits(paid, decimals)} ${symbol} against the ${formatUnits(owed, decimals)} the order was owed`
		if (paid < owed) return `limit orders on ${scenario.dest} spent ${line}`
		ctx.log(`limit orders on ${scenario.dest} spent ${line}`)
	}

	for (const { order, spent } of debited) {
		const unsettled = await settled(ctx, order, spent)
		if (unsettled) return unsettled
		const unproven = await refusal(ctx, order).catch(
			(error) => `${label(order)}: could not simulate a payout: ${brief(error)}`,
		)
		if (unproven) return unproven
	}
	return undefined
}

/**
 * Checks the limit orders posted for a scenario against their on-chain tallies, and answers why
 * the scenario fails, or undefined when it does not.
 *
 * No order may have paid out more than its size, whatever the scenario. A `check: "limit"`
 * scenario also fails, whatever its outcome, on a solver whose account has no `spent` on the fill
 * chain: its bids revert there, which otherwise shows only as an order nobody filled. Once filled,
 * it fails on a tally that could not be read, having proved nothing, and on the limit checks.
 */
export async function checkBudgets(ctx, scenario, result, orders) {
	const tallies = []
	const skipped = new Set()
	let undelegated
	let unread
	// A native payout moves no allowance, so the account keeps no tally of it.
	for (const order of orders.filter((order) => order.token !== zeroAddress)) {
		const client = ctx.clients[order.fillChain]
		const account = `${order.account}:${order.fillChain}`
		if (skipped.has(account)) continue
		try {
			const spent = await readSpent(client, order)
			if (over(order, spent)) return over(order, spent)
			ctx.log(`${label(order)}: spent ${show(order, spent)} of ${show(order, order.cap)}`)
			if (order.fillChain === scenario.dest) tallies.push({ order, spent })
		} catch (error) {
			const why = await unreadable(client, order, error)
			ctx.log(`could not read what ${label(order)} has spent: ${why.reason}`)
			if (why.undelegated) skipped.add(account)
			if (order.fillChain !== scenario.dest) continue
			if (why.undelegated) undelegated ??= why.reason
			else unread ??= `could not read what ${label(order)} has spent: ${why.reason}`
		}
	}
	if (scenario.check !== "limit") return undefined
	if (undelegated) return undelegated
	if (result.outcome !== "FILLED") return undefined
	return (
		unread ??
		limitChecks(ctx, scenario, tallies).catch((error) => `the limit checks could not run: ${brief(error)}`)
	)
}
