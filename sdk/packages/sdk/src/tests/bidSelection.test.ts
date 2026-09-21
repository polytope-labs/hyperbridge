import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { BidManager } from "@/protocols/intents/BidManager"
import { OrderExecutor } from "@/protocols/intents/OrderExecutor"
import type { Bid, FillerBid, HexString, Order, PackedUserOperation, SelectBidResult } from "@/types"
import { describe, expect, it, vi } from "vitest"

const SOLVER_ONE = "0x1111111111111111111111111111111111111111" as HexString
const SOLVER_TWO = "0x2222222222222222222222222222222222222222" as HexString
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
})
