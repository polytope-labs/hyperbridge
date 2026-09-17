import { BidManager } from "@/protocols/intents/BidManager"
import { BidExecutionPendingError } from "@/protocols/intents/Bid"
import { OrderExecutor, isUserOperationNonceConsumed } from "@/protocols/intents/OrderExecutor"
import type { Bid, FillerBid, HexString, Order, PackedUserOperation, SelectBidResult } from "@/types"
import { describe, expect, it, vi } from "vitest"

const TOKEN = `0x${"11".repeat(32)}` as HexString
const SOLVER = "0x2222222222222222222222222222222222222222" as HexString
const COMMITMENT = `0x${"33".repeat(32)}` as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const SECOND_TOKEN = `0x${"44".repeat(32)}` as HexString

function order(): Order {
	return {
		id: COMMITMENT,
		user: TOKEN,
		source: "EVM-1",
		destination: "EVM-8453",
		deadline: 100n,
		nonce: 1n,
		fees: 0n,
		session: SOLVER,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 100n }],
		output: { beneficiary: TOKEN, assets: [{ token: TOKEN, amount: 100n }], call: "0x" },
	}
}

function userOp(nonce: bigint): PackedUserOperation {
	return {
		sender: SOLVER,
		nonce,
		initCode: "0x",
		callData: `0x${nonce.toString(16).padStart(4, "0")}` as HexString,
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x12",
	}
}

function bid(op: PackedUserOperation): Bid {
	return {
		solverAddress: SOLVER,
		inputs: [{ token: TOKEN, amount: 60n }],
		outputs: [{ token: TOKEN, amount: 66n }],
		relayerFee: 0n,
		nativeDispatchFee: 0n,
		userOp: op,
		simulate: vi.fn(),
		execute: vi.fn(),
		outputUsdValue: vi.fn(async () => null),
	}
}

function result(op: PackedUserOperation, amount: bigint, status: "partial" | "full"): SelectBidResult {
	return {
		userOp: op,
		userOpHash: `0x${op.nonce.toString(16).padStart(64, "0")}` as HexString,
		solverAddress: SOLVER,
		commitment: COMMITMENT,
		fillStatus: status,
		filledAssets: status === "partial" ? [{ token: TOKEN, amount }] : undefined,
	}
}

describe("automatic rate bid execution", () => {
	it("treats a candidate below the EntryPoint nonce as already consumed", async () => {
		const op = userOp((9n << 64n) | 2n)
		const readContract = vi.fn(async () => (9n << 64n) | 3n)

		await expect(isUserOperationNonceConsumed({ readContract } as never, ENTRY_POINT, op)).resolves.toBe(true)
		expect(readContract).toHaveBeenCalledWith(
			expect.objectContaining({ functionName: "getNonce", args: [SOLVER, 9n] }),
		)
	})

	it("does not replay a fetched bid whose nonce was consumed before restart", async () => {
		const op = userOp((9n << 64n) | 2n)
		let resolveBlock: (block: bigint) => void = () => undefined
		const block = new Promise<bigint>((resolve) => {
			resolveBlock = resolve
		})
		const readContract = vi.fn(async ({ functionName }: { functionName: string }) => {
			if (functionName === "getNonce") return op.nonce + 1n
			if (functionName === "_filled") return "0x0000000000000000000000000000000000000000"
			return 0n
		})
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: {
				getBidsForOrder: vi.fn(async () => [{ filler: "solver", userOp: op, deposit: 0n }]),
			},
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getEntryPointV08Address: () => ENTRY_POINT,
					getIntentGatewayAddress: () => SOLVER,
				},
				client: { chain: { id: 8453, blockTime: 1 }, readContract, getBlockNumber: vi.fn(() => block) },
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const manager = new BidManager(ctx, {} as never)
		const buildBids = vi.spyOn(manager, "buildBids")
		const stream = new OrderExecutor(ctx, manager).executeOrder({
			order: { ...order(), deadline: 1n },
			auctionTimeMs: 0,
			pollIntervalMs: 1,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		const terminal = stream.next()
		await vi.waitFor(() =>
			expect(readContract).toHaveBeenCalledWith(expect.objectContaining({ functionName: "getNonce" })),
		)
		resolveBlock(1n)
		expect((await terminal).value).toMatchObject({ status: "EXPIRED" })
		expect(buildBids).not.toHaveBeenCalled()
	})

	it("stops as cancelled when destination finalization has frozen incomplete progress", async () => {
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => []) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getEntryPointV08Address: () => ENTRY_POINT,
					getIntentGatewayAddress: () => SOLVER,
				},
				client: {
					chain: { id: 8453, blockTime: 1 },
					readContract: vi.fn(async ({ functionName }: { functionName: string }) =>
						functionName === "_filled" ? SOLVER : 40n,
					),
					getBlockNumber: vi.fn(async () => 0n),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: order(),
			auctionTimeMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({
			status: "CANCELLED",
			totalFilledAssets: [{ token: TOKEN, amount: 40n }],
			remainingAssets: [{ token: TOKEN, amount: 60n }],
		})
	})

	it("rereads credited progress after finalization to observe a concurrent completion", async () => {
		let progressReads = 0
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => []) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getEntryPointV08Address: () => ENTRY_POINT,
					getIntentGatewayAddress: () => SOLVER,
				},
				client: {
					chain: { id: 8453, blockTime: 1 },
					readContract: vi.fn(async ({ functionName }: { functionName: string }) => {
						if (functionName === "_filled") return SOLVER
						progressReads += 1
						return progressReads === 1 ? 40n : 100n
					}),
					getBlockNumber: vi.fn(async () => 0n),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: order(),
			auctionTimeMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({
			status: "FILLED",
			totalFilledAssets: [{ token: TOKEN, amount: 100n }],
			remainingAssets: [{ token: TOKEN, amount: 0n }],
		})
		expect(progressReads).toBe(2)
	})

	it("fails closed when destination state cannot be reconciled", async () => {
		const getBidsForOrder = vi.fn(async () => [])
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getEntryPointV08Address: () => ENTRY_POINT,
					getIntentGatewayAddress: () => SOLVER,
				},
				client: {
					chain: { id: 8453, blockTime: 1 },
					readContract: vi.fn(async () => {
						throw new Error("rpc unavailable")
					}),
					getBlockNumber: vi.fn(async () => 0n),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: order(),
			auctionTimeMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "FAILED", error: "rpc unavailable" })
		expect(getBidsForOrder).not.toHaveBeenCalled()
	})

	it("seeds resumed progress from destination credited state", async () => {
		let resolveBlock: (block: bigint) => void = () => undefined
		const block = new Promise<bigint>((resolve) => {
			resolveBlock = resolve
		})
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => []) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: {
					getEntryPointV08Address: () => ENTRY_POINT,
					getIntentGatewayAddress: () => SOLVER,
				},
				client: {
					chain: { id: 8453, blockTime: 1 },
					readContract: vi.fn(async ({ functionName }: { functionName: string }) =>
						functionName === "_filled" ? "0x0000000000000000000000000000000000000000" : 40n,
					),
					getBlockNumber: vi.fn(() => block),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const stream = new OrderExecutor(ctx, new BidManager(ctx, {} as never)).executeOrder({
			order: order(),
			auctionTimeMs: 0,
		})

		expect((await stream.next()).value).toMatchObject({
			status: "AWAITING_BIDS",
			totalFilledAssets: [{ token: TOKEN, amount: 40n }],
			remainingAssets: [{ token: TOKEN, amount: 60n }],
		})
		resolveBlock(100n)
		await stream.return()
	})

	it("executes sequential bids and advances progress only by credited event amounts", async () => {
		const firstOp = userOp(1n)
		const secondOp = userOp(2n)
		const raw = (op: PackedUserOperation): FillerBid => ({ filler: "solver", userOp: op, deposit: 0n })
		const getBidsForOrder = vi
			.fn()
			.mockResolvedValueOnce([raw(firstOp)])
			.mockResolvedValue([raw(firstOp), raw(secondOp)])
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: {
					chain: { id: 8453, blockTime: 1 },
					getBlockNumber: vi.fn(() => new Promise(() => undefined)),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const manager = new BidManager(ctx, {} as never)
		vi.spyOn(manager, "buildBids").mockImplementation((_order, bids) => bids.map((item) => bid(item.userOp)))
		const execute = vi
			.spyOn(manager, "selectAndExecuteBest")
			.mockResolvedValueOnce(result(firstOp, 50n, "partial"))
			.mockResolvedValueOnce(result(secondOp, 50n, "full"))
		const stream = new OrderExecutor(ctx, manager).executeOrder({
			order: order(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
		expect((await stream.next()).value).toMatchObject({ status: "BID_SELECTED" })
		expect((await stream.next()).value).toMatchObject({
			status: "PARTIAL_FILL",
			totalFilledAssets: [{ token: TOKEN, amount: 50n }],
			remainingAssets: [{ token: TOKEN, amount: 50n }],
		})
		expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
		expect((await stream.next()).value).toMatchObject({ status: "BID_SELECTED" })
		expect((await stream.next()).value).toMatchObject({ status: "FILLED" })
		expect(execute).toHaveBeenCalledTimes(2)
		await stream.return()
	})

	it("persists an accepted UserOp before reporting an uncertain submission outcome", async () => {
		const op = userOp(1n)
		const raw: FillerBid = { filler: "solver", userOp: op, deposit: 0n }
		const setItem = vi.fn(async () => undefined)
		const ctx = {
			bundlerUrl: "http://bundler.test",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => [raw]) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: {
					chain: { id: 8453, blockTime: 1 },
					getBlockNumber: vi.fn(() => new Promise(() => undefined)),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem },
		} as never
		const manager = new BidManager(ctx, {} as never)
		vi.spyOn(manager, "buildBids").mockReturnValue([bid(op)])
		vi.spyOn(manager, "selectAndExecuteBest").mockImplementation(async (_order, _bids, onSubmitted) => {
			const accepted = result(op, 0n, "partial")
			await onSubmitted?.(accepted)
			throw new BidExecutionPendingError(accepted.userOpHash, "receipt timed out")
		})
		const stream = new OrderExecutor(ctx, manager).executeOrder({
			order: order(),
			auctionTimeMs: 0,
			pollIntervalMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
		expect((await stream.next()).value).toMatchObject({ status: "FAILED", error: "receipt timed out" })
		expect(setItem).toHaveBeenCalledTimes(1)
		expect(JSON.parse(setItem.mock.calls[0][1])).toHaveLength(1)
		await stream.return()
	})

	it("rejects automatic rate ranking for multi-leg orders", async () => {
		const multi = order()
		multi.inputs.push({ token: SECOND_TOKEN, amount: 10n })
		multi.output.assets.push({ token: SECOND_TOKEN, amount: 10n })
		const ctx = { bundlerUrl: "x", intentsCoprocessor: {} } as never
		const manager = new BidManager(ctx, {} as never)

		await expect(manager.selectAndExecuteBest(multi, [bid(userOp(1n))])).rejects.toThrow(
			"Automatic rate execution supports single-leg orders only",
		)

		const op = userOp(1n)
		const executorCtx = {
			bundlerUrl: "x",
			intentsCoprocessor: {
				getBidsForOrder: vi.fn(async () => [{ filler: "solver", userOp: op, deposit: 0n }]),
			},
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: {
					chain: { id: 8453, blockTime: 1 },
					getBlockNumber: vi.fn(() => new Promise(() => undefined)),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const executorManager = new BidManager(executorCtx, {} as never)
		vi.spyOn(executorManager, "buildBids").mockReturnValue([bid(op)])
		const stream = new OrderExecutor(executorCtx, executorManager).executeOrder({
			order: multi,
			auctionTimeMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
		expect((await stream.next()).value).toMatchObject({
			status: "FAILED",
			error: "Automatic rate execution supports single-leg orders only",
		})
		await stream.return()
	})

	it("keeps automatic legacy multi-leg execution available", async () => {
		const multi = order()
		multi.inputs.push({ token: SECOND_TOKEN, amount: 10n })
		multi.output.assets.push({ token: SECOND_TOKEN, amount: 10n })
		const op = userOp(1n)
		const raw: FillerBid = { filler: "solver", userOp: op, deposit: 0n }
		const rate = bid(op)
		const legacy = { ...rate, inputs: [] }
		const executorCtx = {
			bundlerUrl: "x",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => [raw]) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: {
					chain: { id: 8453, blockTime: 1 },
					getBlockNumber: vi.fn(() => new Promise(() => undefined)),
				},
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const manager = new BidManager(executorCtx, {} as never)
		vi.spyOn(manager, "buildBids").mockReturnValue([rate, legacy])
		const selectBest = vi
			.spyOn(manager, "selectAndExecuteBest")
			.mockResolvedValue(result(op, 0n, "full"))
		const stream = new OrderExecutor(executorCtx, manager).executeOrder({
			order: multi,
			auctionTimeMs: 0,
			automatic: true,
		})

		expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
		expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
		expect((await stream.next()).value).toMatchObject({ status: "BID_SELECTED" })
		expect((await stream.next()).value).toMatchObject({ status: "FILLED" })
		expect(selectBest).toHaveBeenCalledWith(multi, [legacy], expect.any(Function))
		await stream.return()
	})

	it("cancels the far-deadline watcher when a completed iterator is returned", async () => {
		vi.useFakeTimers()
		const op = userOp(1n)
		const raw: FillerBid = { filler: "solver", userOp: op, deposit: 0n }
		const getBlockNumber = vi.fn(async () => 0n)
		const ctx = {
			bundlerUrl: "x",
			intentsCoprocessor: { getBidsForOrder: vi.fn(async () => [raw]) },
			dest: {
				config: { stateMachineId: "EVM-8453" },
				configService: { getEntryPointV08Address: () => ENTRY_POINT },
				client: { chain: { id: 8453, blockTime: 1_000 }, getBlockNumber },
			},
			usedUserOpsStorage: { getItem: vi.fn(async () => null), setItem: vi.fn(async () => undefined) },
		} as never
		const manager = new BidManager(ctx, {} as never)
		vi.spyOn(manager, "buildBids").mockReturnValue([bid(op)])
		vi.spyOn(manager, "selectAndExecuteBest").mockResolvedValue(result(op, 100n, "full"))
		const stream = new OrderExecutor(ctx, manager).executeOrder({
			order: { ...order(), deadline: 10_000n },
			auctionTimeMs: 0,
			automatic: true,
		})

		try {
			expect((await stream.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
			expect((await stream.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
			expect((await stream.next()).value).toMatchObject({ status: "BID_SELECTED" })
			expect((await stream.next()).value).toMatchObject({ status: "FILLED" })

			let returned = false
			const completion = stream.return().then(() => {
				returned = true
			})
			await vi.advanceTimersByTimeAsync(0)
			expect(returned).toBe(true)
			await completion
		} finally {
			vi.clearAllTimers()
			vi.useRealTimers()
		}
	})
})
