// The on-chain limit refusing a real fill. Runs in the swap worker when the bids arrive, before the
// SDK picks one: the best bid's solver uses up its limit order's room on chain until it is one unit
// short of what the bid pays, then the bid is simulated as the SDK does and sent to the bundler
// anyway. The runner checks what the chain recorded (`refusalChecks` in e2e/budget.mjs).
import { createWalletClient, encodeFunctionData, erc20Abi, http } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { ACCOUNT_ABI, ENTRY_POINT_ABI, brief, debitOf, readSpent } from "./budget.mjs"
import { ENTRY_POINT, GATEWAY } from "./env.mjs"

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

/** The first `UserOperationEvent` for `userOpHash` from `fromBlock` on, polled while the bundle lands. */
async function bundleOf(client, userOpHash, fromBlock) {
	const until = Date.now() + 120_000
	for (;;) {
		const [event] = await client
			.getContractEvents({
				address: ENTRY_POINT,
				abi: ENTRY_POINT_ABI,
				eventName: "UserOperationEvent",
				args: { userOpHash },
				fromBlock,
			})
			.catch(() => [])
		if (event) return event.transactionHash
		if (Date.now() > until) return undefined
		await pause(5_000)
	}
}

/**
 * Why the SDK's own simulation refuses the bid, or undefined when it keeps passing. A pass is
 * retried a few times, since a node behind the one that mined the debit still has the old tally.
 */
async function refusedInSimulation(bid) {
	for (let tries = 0; tries < 4; tries++) {
		try {
			await bid.simulate()
		} catch (error) {
			return brief(error)
		}
		await pause(3_000)
	}
	return undefined
}

/**
 * Leaves the best bid's limit order one unit short of what the bid pays, then shows the bid is
 * refused. Everything is sent from the solver's own key, as its account calling itself.
 *
 * `orderInputs` are the inputs as the gateway committed them, after its protocol fee.
 *
 * Answers what happened, with amounts as decimal strings, or `{ error }` when the scenario could
 * not be set up. The verdict is the runner's: this only acts and records.
 */
export async function refuseBest({ bids, orderInputs, client, chain, rpc, solverKeys, log }) {
	const candidates = bids
		.map((bid) => ({ bid, debit: debitOf(bid.userOp) }))
		.filter(({ debit }) => debit !== undefined)
		.sort((a, b) => (b.bid.outputs[0].amount > a.bid.outputs[0].amount ? 1 : -1))
	if (candidates.length === 0) return { error: `none of the ${bids.length} bids ends with a debitOrder call` }
	if (bids.length < 2) return { error: "only one bid arrived, so nothing could fill the order once it is refused" }
	const { bid, debit } = candidates[0]
	const key = solverKeys.find((key) => privateKeyToAccount(key).address.toLowerCase() === bid.solverAddress.toLowerCase())
	if (!key) return { error: `the best bid is from ${bid.solverAddress}, which is none of the run's solvers` }

	// The fill pays exactly what the bid approved only when it takes the whole order and no
	// dispatch fee comes out of the same allowance. Anything else would leave the room it is
	// given here larger than the fill needs.
	const whole = bid.inputs.length === orderInputs.length && bid.inputs.every((input, i) => input.amount === orderInputs[i].amount)
	if (debit.fee !== 0n || !whole) {
		const takes = bid.inputs.map((input) => input.amount).join(", ")
		const escrowed = orderInputs.map((input) => input.amount).join(", ")
		return {
			error: `the best bid does not take the whole order with no fee (takes ${takes} of ${escrowed}, fee ${debit.fee}), so its payout is not what it approved`,
		}
	}
	const payout = debit.approved
	const account = bid.solverAddress
	const tally = { account, budgetId: debit.orderId }
	const record = {
		solver: account,
		orderId: debit.orderId,
		cap: debit.cap.toString(),
		token: debit.token,
		payout: payout.toString(),
	}

	try {
		await bid.simulate()
	} catch (error) {
		return { ...record, error: `the best bid failed simulation before any room was taken: ${brief(error)}` }
	}

	const spent = await readSpent(client, tally)
	const room = debit.cap - spent
	if (room < payout) return { ...record, error: `the limit order has ${room} left, less than the ${payout} the bid pays already` }
	// `debitOrder` counts `approved` less the gateway's allowance, so the allowance is added back.
	const allowance = await client.readContract({ address: debit.token, abi: erc20Abi, functionName: "allowance", args: [account, GATEWAY] })
	const drain = room - payout + 1n
	const wallet = createWalletClient({ account: privateKeyToAccount(key), chain, transport: http(rpc) })
	const drainTx = await wallet.sendTransaction({
		to: account,
		data: encodeFunctionData({
			abi: ACCOUNT_ABI,
			functionName: "debitOrder",
			args: [debit.orderId, debit.cap, debit.token, allowance + drain, 0n],
		}),
	})
	const drained = await client.waitForTransactionReceipt({ hash: drainTx })
	record.drainTx = drainTx
	if (drained.status !== "success") return { ...record, error: `the debit that takes the room reverted: ${drainTx}` }
	const spentAfterDrain = await readSpent(client, tally, drained.blockNumber)
	record.drain = drain.toString()
	record.spentAfterDrain = spentAfterDrain.toString()
	log("room-taken", { account, orderId: debit.orderId, spent, drain, spentAfterDrain, cap: debit.cap, payout, drainTx })

	record.simulationError = await refusedInSimulation(bid)
	log("simulated", { refused: record.simulationError !== undefined, error: record.simulationError })

	// Sent past the simulation, as a bundler takes any operation that validates.
	const userOpHash = await client.readContract({
		address: ENTRY_POINT,
		abi: ENTRY_POINT_ABI,
		functionName: "getUserOpHash",
		args: [bid.userOp],
	})
	record.userOpHash = userOpHash
	const fromBlock = await client.getBlockNumber({ cacheTime: 0 })
	try {
		const executed = await bid.execute()
		record.executeResult = `executed in ${executed.txnHash} with fill status ${executed.fillStatus}`
	} catch (error) {
		record.executeError = brief(error)
	}
	record.transactionHash = await bundleOf(client, userOpHash, fromBlock)
	log("forced", { userOpHash, transactionHash: record.transactionHash, executeError: record.executeError })
	return record
}
