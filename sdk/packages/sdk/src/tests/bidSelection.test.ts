import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { BidManager } from "@/protocols/intents/BidManager"
import { OrderExecutor } from "@/protocols/intents/OrderExecutor"
import type { Bid, FillerBid, HexString, Order, PackedUserOperation, SelectBidResult } from "@/types"
import { describe, expect, it, vi } from "vitest"

const SOLVER_ONE = "0x1111111111111111111111111111111111111111" as HexString
const SOLVER_TWO = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER_THREE = "0x7777777777777777777777777777777777777777" as HexString
const TOKEN = "0x3333333333333333333333333333333333333333" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const COMMITMENT = `0x${"ab".repeat(32)}` as HexString
const SESSION = "0x5555555555555555555555555555555555555555" as HexString
const GATEWAY = "0x6666666666666666666666666666666666666666" as HexString
const ZERO_ADDRESS = `0x${"00".repeat(20)}` as HexString

/** Destination gateway reads: `_filled` answers `finalizer`, `_partialFills` answers `credited` by leg. */
function gatewayReads(state: { finalizer?: HexString; credited?: bigint[] }) {
	return vi.fn(async ({ functionName, args }: { functionName: string; args: [HexString, bigint] }) =>
		functionName === "_filled" ? (state.finalizer ?? ZERO_ADDRESS) : (state.credited?.[Number(args[1])] ?? 0n),
	)
}

/** A deadline watcher that stays pending until `expire()`, which lets the executor tear down. */
function pendingDeadline() {
	let expire: () => void = () => undefined
	const block = new Promise<bigint>((resolve) => {
		expire = () => resolve(makeOrder().deadline)
	})
	return { getBlockNumber: () => block, expire }
}

function makeContext(params: {
	getBidsForOrder: () => Promise<FillerBid[]>
	readContract: ReturnType<typeof gatewayReads>
	getBlockNumber: () => Promise<bigint>
	setItem?: (key: string, value: string) => Promise<void>
}) {
	return {
		bundlerUrl: "http://bundler.test",
		intentsCoprocessor: { getBidsForOrder: params.getBidsForOrder },
		dest: {
			config: { stateMachineId: "EVM-8453" },
			configService: { getEntryPointV08Address: () => ENTRY_POINT, getIntentGatewayAddress: () => GATEWAY },
			client: {
				chain: { id: 8453, blockTime: 1 },
				getBlockNumber: vi.fn(params.getBlockNumber),
				readContract: params.readContract,
			},
		},
		usedUserOpsStorage: {
			getItem: vi.fn(async () => null),
			setItem: params.setItem ?? vi.fn(async () => undefined),
		},
	} as never
}

function makeUserOp(sender = SOLVER_ONE): PackedUserOperation {
	return {
		sender,
		nonce: 1n,
		initCode: "0x",
		callData: "0x1234",
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 50_000n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x12",
	}
}

function makeOrder(): Order {
	return {
		id: COMMITMENT,
		user: SOLVER_ONE,
		source: "EVM-1",
		destination: "EVM-8453",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: SESSION,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 100n }],
		output: {
			beneficiary: SOLVER_ONE,
			assets: [{ token: TOKEN, amount: 100n }],
			call: "0x",
		},
	}
}

function makeResult(userOp: PackedUserOperation, solverAddress: HexString): SelectBidResult {
	return {
		userOp,
		userOpHash: `0x${"ef".repeat(32)}`,
		solverAddress,
		commitment: COMMITMENT,
	}
}

function makeBid(params: { solverAddress: HexString; amount: bigint; execute: Bid["execute"] }): Bid {
	return {
		solverAddress: params.solverAddress,
		outputs: [{ token: TOKEN, amount: params.amount }],
		inputs: [{ token: TOKEN, amount: 100n }],
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		userOp: makeUserOp(params.solverAddress),
		simulate: vi.fn(async () => undefined),
		execute: params.execute,
		outputUsdValue: vi.fn(async () => null),
	}
}

describe("Order execution bid-selection integration", () => {
	it("selects the next ranked bid in the same polling round after an already-known error", async () => {
		const firstExecute = vi.fn(async () => {
			throw new Error("Bundler error: already known")
		})
		const first = makeBid({ solverAddress: SOLVER_ONE, amount: 120n, execute: firstExecute })
		const secondExecute = vi.fn(async () => ({
			...makeResult(makeUserOp(SOLVER_TWO), SOLVER_TWO),
			fillStatus: "full" as const,
		}))
		const second = makeBid({ solverAddress: SOLVER_TWO, amount: 110n, execute: secondExecute })
		const rawBids: FillerBid[] = [first, second].map((bid, index) => ({
			filler: `solver-${index}`,
			bid: CryptoUtils.bidId(bid.userOp.callData),
			userOp: bid.userOp,
			deposit: 0n,
		}))
		const getBidsForOrder = vi.fn(async () => rawBids)
		const persistUsedUserOps = vi.fn(async () => undefined)
		let resolveDeadlineBlock: (block: bigint) => void = () => undefined
		const deadlineBlock = new Promise<bigint>((resolve) => {
			resolveDeadlineBlock = resolve
		})
		const ctx = makeContext({
			getBidsForOrder,
			readContract: gatewayReads({}),
			getBlockNumber: () => deadlineBlock,
			setItem: persistUsedUserOps,
		})
		const bidManager = new BidManager(ctx, {} as never)
		vi.spyOn(bidManager, "buildBids").mockReturnValue([first, second])
		const executor = new OrderExecutor(ctx, bidManager)
		const stream = executor.executeOrder({ order: makeOrder(), auctionTimeMs: 0, pollIntervalMs: 0 })

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS", commitment: COMMITMENT })

		const received = await stream.next()
		expect(received.value).toMatchObject({ status: "BIDS_RECEIVED", bidCount: 2 })
		if (received.done || received.value.status !== "BIDS_RECEIVED") throw new Error("Expected bids to be received")

		const selectedResult = await bidManager.selectAndExecuteBest(makeOrder(), received.value.bids)
		expect((await stream.next(selectedResult)).value).toMatchObject({
			status: "BID_SELECTED",
			selectedSolver: SOLVER_TWO,
		})
		expect((await stream.next()).value).toMatchObject({ status: "FILLED", selectedSolver: SOLVER_TWO })

		resolveDeadlineBlock(makeOrder().deadline)
		expect((await stream.next()).done).toBe(true)

		expect(getBidsForOrder).toHaveBeenCalledOnce()
		expect(firstExecute).toHaveBeenCalledOnce()
		expect(secondExecute).toHaveBeenCalledOnce()
		expect(persistUsedUserOps).toHaveBeenCalledOnce()
	})

	it("resumes from the output already credited on the destination chain", async () => {
		const deadline = pendingDeadline()
		const ctx = makeContext({
			getBidsForOrder: async () => [],
			readContract: gatewayReads({ credited: [40n] }),
			getBlockNumber: deadline.getBlockNumber,
		})
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: makeOrder(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
		})

		expect((await stream.next()).value).toMatchObject({
			status: "AWAITING_BIDS",
			totalFilledAssets: [{ token: TOKEN, amount: 40n }],
			remainingAssets: [{ token: TOKEN, amount: 60n }],
		})
		deadline.expire()
		await stream.return()
	})

	it("reports an order another solver completed as filled", async () => {
		const readContract = gatewayReads({})
		const deadline = pendingDeadline()
		const ctx = makeContext({
			getBidsForOrder: async () => [],
			readContract,
			getBlockNumber: deadline.getBlockNumber,
		})
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: makeOrder(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
		})
		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })

		readContract.mockImplementation(gatewayReads({ finalizer: SOLVER_TWO, credited: [100n] }))

		expect((await stream.next()).value).toEqual({
			status: "FILLED",
			commitment: COMMITMENT,
			selectedSolver: SOLVER_TWO,
			totalFilledAssets: [{ token: TOKEN, amount: 100n }],
			remainingAssets: [{ token: TOKEN, amount: 0n }],
		})
		deadline.expire()
		expect((await stream.next()).done).toBe(true)
	})

	it("reports an order finalized short of its output as cancelled", async () => {
		const ctx = makeContext({
			getBidsForOrder: async () => [],
			readContract: gatewayReads({ finalizer: SOLVER_ONE, credited: [40n] }),
			getBlockNumber: async () => 0n,
		})
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: makeOrder(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
		})

		expect((await stream.next()).value).toEqual({
			status: "CANCELLED",
			commitment: COMMITMENT,
			totalFilledAssets: [{ token: TOKEN, amount: 40n }],
			remainingAssets: [{ token: TOKEN, amount: 60n }],
		})
		expect((await stream.next()).done).toBe(true)
	})

	it("fails when the destination chain cannot be read", async () => {
		const readContract = gatewayReads({})
		readContract.mockRejectedValue(new Error("rpc down"))
		const ctx = makeContext({ getBidsForOrder: async () => [], readContract, getBlockNumber: async () => 0n })
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: makeOrder(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
		})

		expect((await stream.next()).value).toMatchObject({
			status: "FAILED",
			error: expect.stringContaining("rpc down"),
		})
	})

	describe("bids at different rates on one order", () => {
		/** 300 in for at least 300 out: a rate of 1.00, which every bid below clears. */
		function rateOrder(): Order {
			return {
				...makeOrder(),
				inputs: [{ token: TOKEN, amount: 300n }],
				output: { beneficiary: SOLVER_ONE, assets: [{ token: TOKEN, amount: 300n }], call: "0x" },
			}
		}

		/** A partial quote: takes `take` of the input for `output`, so its rate is output / take. */
		function rateBid(solverAddress: HexString, take: bigint, output: bigint, execute: Bid["execute"]): Bid {
			return { ...makeBid({ solverAddress, amount: output, execute }), inputs: [{ token: TOKEN, amount: take }] }
		}

		it("ranks by rate, best first, whatever order the bids arrived in and whatever their size", async () => {
			const ctx = makeContext({
				getBidsForOrder: async () => [],
				readContract: gatewayReads({}),
				getBlockNumber: async () => 0n,
			})
			const noop = vi.fn()
			// Arrive worst first. The largest bid has the worst rate, so size alone would pick it.
			const worst = rateBid(SOLVER_ONE, 200n, 200n, noop) // 1.00
			const best = rateBid(SOLVER_TWO, 100n, 130n, noop) // 1.30
			const middle = rateBid(SOLVER_THREE, 100n, 115n, noop) // 1.15

			const ranked = await new BidManager(ctx, {} as never).sortBids(rateOrder(), [worst, best, middle])

			expect(ranked.map((bid) => bid.solverAddress)).toEqual([SOLVER_TWO, SOLVER_THREE, SOLVER_ONE])
		})

		it("executes the best rate first, then the next best on what is left, until the order is filled", async () => {
			// What the destination gateway has credited so far, as the executor reads it each round.
			const destination: { credited: bigint[]; finalizer?: HexString } = { credited: [0n] }
			const readContract = vi.fn(
				async ({ functionName, args }: { functionName: string; args: [HexString, bigint] }) =>
					functionName === "_filled"
						? (destination.finalizer ?? ZERO_ADDRESS)
						: (destination.credited[Number(args[1])] ?? 0n),
			)
			const executed: HexString[] = []
			/** Executes as the gateway would: clamps to what is outstanding and credits it. */
			function fill(solver: HexString, output: bigint): Bid["execute"] {
				return vi.fn(async () => {
					executed.push(solver)
					const outstanding = 300n - destination.credited[0]
					const delivered = output < outstanding ? output : outstanding
					destination.credited = [destination.credited[0] + delivered]
					const full = destination.credited[0] >= 300n
					if (full) destination.finalizer = solver
					return {
						...makeResult(makeUserOp(solver), solver),
						fillStatus: full ? ("full" as const) : ("partial" as const),
						filledAssets: [{ token: TOKEN, amount: delivered }],
					}
				})
			}

			// Worst rate arrives first; all three together cover more than the order asks for.
			const bids: Bid[] = [
				rateBid(SOLVER_ONE, 100n, 100n, fill(SOLVER_ONE, 100n)), // 1.00
				rateBid(SOLVER_TWO, 100n, 130n, fill(SOLVER_TWO, 130n)), // 1.30
				rateBid(SOLVER_THREE, 100n, 115n, fill(SOLVER_THREE, 115n)), // 1.15
			]
			const bySender = new Map(bids.map((bid) => [bid.userOp.sender.toLowerCase(), bid]))
			const rawBids: FillerBid[] = bids.map((bid, index) => ({
				filler: `solver-${index}`,
				bid: CryptoUtils.bidId(bid.userOp.callData),
				userOp: bid.userOp,
				deposit: 0n,
			}))

			const deadline = pendingDeadline()
			const ctx = makeContext({
				getBidsForOrder: async () => rawBids,
				readContract,
				getBlockNumber: deadline.getBlockNumber,
			})
			const bidManager = new BidManager(ctx, {} as never)
			vi.spyOn(bidManager, "buildBids").mockImplementation((_order, fillerBids) =>
				fillerBids.map((fillerBid) => bySender.get(fillerBid.userOp.sender.toLowerCase())!),
			)
			const stream = new OrderExecutor(ctx, bidManager).executeOrder({
				order: rateOrder(),
				auctionTimeMs: 0,
				pollIntervalMs: 0,
			})

			expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })

			const updates: { status: string; selectedSolver?: HexString; remaining?: bigint }[] = []
			let step = await stream.next()
			while (!step.done) {
				const update = step.value
				if (update.status === "BIDS_RECEIVED") {
					step = await stream.next(await bidManager.selectAndExecuteBest(rateOrder(), update.bids))
					continue
				}
				updates.push({
					status: update.status,
					selectedSolver: "selectedSolver" in update ? (update.selectedSolver as HexString) : undefined,
					remaining: "remainingAssets" in update ? update.remainingAssets?.[0]?.amount : undefined,
				})
				if (update.status === "FILLED") break
				step = await stream.next()
			}

			// Best rate first, then the next best, then the worst takes what is left and completes it.
			expect(executed).toEqual([SOLVER_TWO, SOLVER_THREE, SOLVER_ONE])
			expect(updates.filter((u) => u.status !== "BID_SELECTED")).toEqual([
				{ status: "PARTIAL_FILL", selectedSolver: SOLVER_TWO, remaining: 170n },
				{ status: "PARTIAL_FILL", selectedSolver: SOLVER_THREE, remaining: 55n },
				{ status: "FILLED", selectedSolver: SOLVER_ONE, remaining: 0n },
			])
			expect(destination.credited).toEqual([300n])

			deadline.expire()
			await stream.return()
		})
	})

	it("stops offering bids that only quote a leg the destination has completed", async () => {
		// Two legs of 100 each. One bid fills leg 1, a second quotes only leg 1, a third only
		// leg 0. Once leg 1 is done the second can only revert, so it is not handed on again.
		const twoLegs: Order = {
			...makeOrder(),
			inputs: [
				{ token: TOKEN, amount: 100n },
				{ token: TOKEN, amount: 100n },
			],
			output: {
				beneficiary: SOLVER_ONE,
				assets: [
					{ token: TOKEN, amount: 100n },
					{ token: TOKEN, amount: 100n },
				],
				call: "0x",
			},
		}
		const destination: { credited: bigint[]; finalizer?: HexString } = { credited: [0n, 0n] }
		const readContract = vi.fn(async ({ functionName, args }: { functionName: string; args: [HexString, bigint] }) =>
			functionName === "_filled"
				? (destination.finalizer ?? ZERO_ADDRESS)
				: (destination.credited[Number(args[1])] ?? 0n),
		)
		const legBid = (solver: HexString, leg: number, output: bigint, onExecute: () => void): Bid => {
			const outputs = [0n, 0n].map((amount, i) => ({ token: TOKEN, amount: i === leg ? output : amount }))
			return {
				...makeBid({
					solverAddress: solver,
					amount: 0n,
					execute: vi.fn(async () => {
						onExecute()
						const full = destination.credited.every((amount) => amount >= 100n)
						if (full) destination.finalizer = solver
						return {
							...makeResult(makeUserOp(solver), solver),
							fillStatus: full ? ("full" as const) : ("partial" as const),
							filledAssets: destination.credited.map((amount) => ({ token: TOKEN, amount })),
						}
					}),
				}),
				outputs,
				inputs: [0n, 0n].map((amount, i) => ({ token: TOKEN, amount: i === leg ? 100n : amount })),
			}
		}
		const fillsLegOne = legBid(SOLVER_ONE, 1, 130n, () => {
			destination.credited = [destination.credited[0], 100n]
		})
		const deadLegOne = legBid(SOLVER_TWO, 1, 120n, () => {
			throw new Error("a bid on a completed leg was executed")
		})
		const fillsLegZero = legBid(SOLVER_THREE, 0, 110n, () => {
			destination.credited = [100n, destination.credited[1]]
		})
		const bids = [fillsLegOne, deadLegOne, fillsLegZero]
		const bySender = new Map(bids.map((bid) => [bid.userOp.sender.toLowerCase(), bid]))
		const rawBids: FillerBid[] = bids.map((bid, index) => ({
			filler: `solver-${index}`,
			bid: CryptoUtils.bidId(bid.userOp.callData),
			userOp: bid.userOp,
			deposit: 0n,
		}))

		const deadline = pendingDeadline()
		const ctx = makeContext({ getBidsForOrder: async () => rawBids, readContract, getBlockNumber: deadline.getBlockNumber })
		const bidManager = new BidManager(ctx, {} as never)
		vi.spyOn(bidManager, "buildBids").mockImplementation((_order, fillerBids) =>
			fillerBids.map((fillerBid) => bySender.get(fillerBid.userOp.sender.toLowerCase())!),
		)
		const stream = new OrderExecutor(ctx, bidManager).executeOrder({ order: twoLegs, auctionTimeMs: 0, pollIntervalMs: 0 })

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		const rounds: HexString[][] = []
		let step = await stream.next()
		while (!step.done) {
			const update = step.value
			if (update.status === "BIDS_RECEIVED") {
				rounds.push(update.bids.map((bid) => bid.solverAddress))
				// The first round takes leg 1 first; later rounds take whatever is offered.
				const pick = update.bids.find((bid) => bid === fillsLegOne) ?? update.bids[0]
				step = await stream.next(await pick.execute())
				continue
			}
			if (update.status === "FILLED") break
			step = await stream.next()
		}

		expect(rounds[0]).toEqual([SOLVER_ONE, SOLVER_TWO, SOLVER_THREE])
		// Leg 1 is complete: only the leg-0 bid is left to offer.
		expect(rounds[1]).toEqual([SOLVER_THREE])
		expect(destination.credited).toEqual([100n, 100n])

		deadline.expire()
		await stream.return()
	})
})

