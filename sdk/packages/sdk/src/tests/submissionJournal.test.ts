import { afterEach, describe, expect, it, vi } from "vitest"
import { encodeAbiParameters, encodeEventTopics, parseAbi, stringToHex } from "viem"
import { ABI } from "@/abis/IntentGatewayV2"
import { BidExecutionPendingError, BidExecutionRejectedError, BidImpl } from "@/protocols/intents/Bid"
import { BidManager } from "@/protocols/intents/BidManager"
import { CryptoUtils } from "@/protocols/intents/CryptoUtils"
import { SubmissionJournal } from "@/protocols/intents/submissionJournal"
import { OrderExecutor } from "@/protocols/intents/OrderExecutor"
import type { HexString, Order, PackedUserOperation } from "@/types"
import { encodeFillOrder } from "@/protocols/intents/fillOrderCodec"
const sender = `0x${"22".repeat(20)}` as HexString
const gateway = `0x${"33".repeat(20)}` as HexString
const entryPoint = `0x${"44".repeat(20)}` as HexString
const token = `0x${"55".repeat(32)}` as HexString
const commitment = `0x${"66".repeat(32)}` as HexString
const transactionHash = `0x${"77".repeat(32)}` as HexString
const eventAbi = parseAbi([
	"event BeforeExecution()",
	"event UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster, uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)",
])
const order: Order = {
	id: commitment,
	user: token,
	source: "EVM-1",
	destination: "EVM-8453",
	deadline: 100n,
	nonce: 1n,
	fees: 0n,
	session: sender,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token, amount: 100n }],
	output: { beneficiary: token, assets: [{ token, amount: 100n }], call: "0x" },
}
let opCallData: HexString = "0x1234"
function op(nonce = 1n): PackedUserOperation {
	return {
		sender,
		nonce,
		initCode: "0x",
		callData: opCallData,
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x12",
	}
}
function beforeExecutionLog() {
	return {
		address: entryPoint,
		topics: encodeEventTopics({ abi: eventAbi, eventName: "BeforeExecution" }),
		data: "0x",
	}
}
function fixture() {
	const storage = new Map<string, string>()
	const sent: any[] = []
	let receipt: any = null
	let failure: unknown
	let filled = false
	let onSend = () => {}
	const ctx: any = {
		bundlerUrl: "https://bundler.invalid",
		receiptPolling: { maxRetries: 2, backoffMs: 0 },
		intentsCoprocessor: { getBidsForOrder: vi.fn(async () => [{ filler: "solver", userOp: op(), deposit: 0n }]) },
		sessionKeyStorage: { getSessionKeyByAddress: async () => ({ privateKey: `0x${"01".repeat(32)}` }) },
		usedUserOpsStorage: {
			getItem: async (key: string) => storage.get(key) ?? null,
			setItem: async (key: string, value: string) => {
				storage.set(key, value)
			},
		},
		dest: {
			config: { stateMachineId: "EVM-8453" },
			configService: { getEntryPointV08Address: () => entryPoint, getIntentGatewayAddress: () => gateway },
			client: {
				chain: { id: 8453 },
				getBlockNumber: async () => 0n,
				getBlock: async () => ({ number: 0n }),
				call: async () => ({}),
				readContract: async ({ functionName }: any) =>
					functionName === "_filled"
						? filled
							? sender
							: `0x${"00".repeat(20)}`
						: functionName === "getNonce"
							? 1n
							: filled
								? 100n
								: 0n,
				waitForTransactionReceipt: async () => receipt,
			},
		},
	}
	const crypto = new CryptoUtils(ctx)
	vi.stubGlobal(
		"fetch",
		vi.fn(async (_url, request) => {
			const body = JSON.parse(request.body)
			if (body.method === "eth_sendUserOperation") {
				sent.push(body.params[0])
				onSend()
				if (failure instanceof Error) throw failure
				if (failure) return { json: async () => ({ jsonrpc: "2.0", id: 1, error: failure }) }
				return {
					json: async () => ({
						jsonrpc: "2.0",
						id: 1,
						result: CryptoUtils.computeUserOpHash(op(BigInt(body.params[0].nonce)), entryPoint, 8453n),
					}),
				}
			}
			return {
				json: async () => ({
					jsonrpc: "2.0",
					id: 1,
					result: receipt ? { success: true, receipt: { transactionHash } } : null,
				}),
			}
		}),
	)
	const manager = new BidManager(ctx, crypto)
	function bid(nonce = 1n) {
		const b = new BidImpl({
			ctx,
			crypto,
			order,
			fillerBid: { filler: "solver", userOp: op(nonce), deposit: 0n },
			fillOptions: {
				outputs: [{ token, amount: nonce === 1n ? 120n : 100n }],
				inputs: [],
				relayerFee: 0n,
				nativeDispatchFee: 0n,
				validUntil: 0n,
			},
			priceOutputs: async () => null,
		})
		vi.spyOn(b, "simulate").mockResolvedValue()
		return b
	}
	vi.spyOn(manager, "buildBids").mockImplementation(() => [bid()])
	const stream = () =>
		new OrderExecutor(ctx, manager).executeOrder({ order, automatic: true, auctionTimeMs: 0, pollIntervalMs: 0 })
	function confirm(success = true, emitter = entryPoint, nonce = 1n) {
		filled = success
		receipt = {
			status: "success",
			logs: [
				beforeExecutionLog(),
				{
					address: gateway,
					topics: encodeEventTopics({ abi: ABI, eventName: "OrderFilled", args: { commitment } }),
					data: encodeAbiParameters(
						[
							{ type: "address" },
							{
								type: "tuple[]",
								components: [
									{ name: "token", type: "bytes32" },
									{ name: "amount", type: "uint256" },
								],
							},
							{
								type: "tuple[]",
								components: [
									{ name: "token", type: "bytes32" },
									{ name: "amount", type: "uint256" },
								],
							},
						],
						[sender, [], []],
					),
				},
				{
					address: emitter,
					topics: encodeEventTopics({
						abi: eventAbi,
						eventName: "UserOperationEvent",
						args: {
							userOpHash: CryptoUtils.computeUserOpHash(op(nonce), entryPoint, 8453n),
							sender,
							paymaster: sender,
						},
					}),
					data: encodeAbiParameters(
						[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
						[nonce, success, 1n, 1n],
					),
				},
			].map((log, logIndex) => ({ ...log, logIndex })),
		}
	}
	return {
		ctx,
		crypto,
		manager,
		bid,
		stream,
		storage,
		sent,
		confirm,
		setOnSend: (fn: () => void) => {
			onSend = fn
		},
		setReceipt: (value: unknown) => {
			receipt = value
		},
		setFailure: (value: unknown) => {
			failure = value
		},
	}
}
afterEach(() => {
	vi.unstubAllGlobals()
	vi.restoreAllMocks()
	opCallData = "0x1234"
})
async function attempt(f: ReturnType<typeof fixture>) {
	const s = f.stream()
	expect((await s.next()).value).toMatchObject({ status: "AWAITING_BIDS" })
	expect((await s.next()).value).toMatchObject({ status: "BIDS_RECEIVED" })
	const next = await s.next()
	await s.return()
	return next.value
}
describe("durable submissions", () => {
	it("recovers a crash after the pending write before broadcast", async () => {
		const f = fixture()
		const original = f.ctx.usedUserOpsStorage.setItem
		f.ctx.usedUserOpsStorage.setItem = async (key: string, value: string) => {
			await original(key, value)
			throw new Error("process death")
		}
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(f.sent).toHaveLength(0)
		f.ctx.usedUserOpsStorage.setItem = original
		f.setFailure(new Error("timeout"))
		const replay = f.stream()
		expect((await replay.next()).value).toMatchObject({ status: "FAILED" })
		await replay.return()
		expect(f.sent).toHaveLength(1)
		f.confirm()
		const resumed = f.stream()
		expect((await resumed.next()).value).toMatchObject({ status: "FILLED" })
		await resumed.return()
		expect([...f.storage.values()].some((value) => value.includes('"submission"'))).toBe(false)
		expect([...f.storage.keys()].some((key) => key.startsWith("pending-submission:v1:"))).toBe(true)
	})
	it("falls back after a definitive first-send AA21 rejection", async () => {
		const f = fixture()
		f.setFailure({ code: -32500, message: "AA21 didn't pay prefund", data: { reason: "prefund" } })
		const first = f.bid()
		const second = f.bid(2n)
		vi.spyOn(second, "simulate").mockImplementation(async () => {
			f.setFailure(undefined)
			f.confirm(true, entryPoint, 2n)
		})
		await expect(f.manager.selectAndExecuteBest(order, [first, second])).resolves.toMatchObject({
			fillStatus: "full",
		})
		expect(f.sent.map((x) => x.nonce)).toEqual(["0x1", "0x2"])
	})
	it("preserves structured RPC error code and data", async () => {
		const f = fixture()
		f.setFailure({ code: -32500, message: "AA21 didn't pay prefund", data: { reason: "prefund" } })
		await expect(
			f.crypto.sendBundler("eth_sendUserOperation" as never, [{ nonce: "0x1" }, entryPoint]),
		).rejects.toMatchObject({ code: -32500, data: { reason: "prefund" } })
	})
})

function pending(f: ReturnType<typeof fixture>) {
	return [...f.storage.entries()].find(([key, value]) => key.startsWith("pending-submission:") && value !== "null")
}
async function resume(f: ReturnType<typeof fixture>) {
	const s = f.stream()
	const value = (await s.next()).value
	await s.return()
	return value
}
describe("submission recovery safety", () => {
	it("replays the same signed nonce across repeated restarts and reconciles a late receipt", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		const record = JSON.parse(pending(f)![1])
		expect(record.submission.userOp.signature.length).toBe(134)
		expect(f.storage.has(`used-userops:${commitment}`)).toBe(false)
		for (let i = 0; i < 2; i++) expect(await resume(f)).toMatchObject({ status: "FAILED" })
		expect(f.sent).toHaveLength(3)
		expect(f.sent[1]).toEqual(f.sent[0])
		expect(f.sent[2]).toEqual(f.sent[0])
		f.confirm()
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(pending(f)).toBeUndefined()
		expect(f.sent).toHaveLength(3)
	})
	it("waits for a rebroadcast receipt instead of failing on the first empty poll", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		f.setFailure(undefined)
		f.confirm()
		vi.spyOn(BidImpl, "receipt").mockResolvedValueOnce(null).mockResolvedValueOnce(null)
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(f.sent).toHaveLength(2)
		expect(pending(f)).toBeUndefined()
	})
	it("retires a journaled operation once finalized state passes its validUntil", async () => {
		const f = fixture()
		const canonical = `0x${"00".repeat(12)}${"55".repeat(20)}` as HexString
		const legs = [{ token: canonical, amount: 100n }]
		opCallData = f.crypto.encodeERC7821Execute([
			{
				target: gateway,
				value: 0n,
				data: encodeFillOrder(
					{
						...order,
						source: stringToHex(order.source),
						destination: stringToHex(order.destination),
						inputs: legs,
						output: { ...order.output, assets: legs },
					},
					{ relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 50n, outputs: legs, inputs: legs },
					3,
				),
			},
		])
		f.setFailure(new Error("timeout"))
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(pending(f)).toBeDefined()

		f.ctx.dest.client.getBlock = async () => ({ number: 49n })
		expect(await resume(f)).toMatchObject({ status: "FAILED" })
		expect(f.sent).toHaveLength(2)

		f.ctx.dest.client.getBlock = async () => ({ number: 51n })
		expect(await resume(f)).toMatchObject({ status: "AWAITING_BIDS" })
		expect(f.sent).toHaveLength(2)
		expect(pending(f)).toBeUndefined()
	})
	it("does not fall back after timeout then restart then explicit rejection", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		f.setFailure({ code: -32500, message: "AA21 didn't pay prefund" })
		expect(await resume(f)).toMatchObject({ status: "FAILED" })
		expect(pending(f)).toBeDefined()
		expect(f.sent).toHaveLength(2)
		expect(f.sent[1]).toEqual(f.sent[0])
		f.confirm()
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(f.sent).toHaveLength(2)
	})
	it.each([
		{ code: -32500, message: "already known" },
		{ code: -32500, message: "AA25 invalid account nonce" },
		{ code: -32000, message: "unknown server error" },
	])("keeps ambiguous structured errors pending: $message", async (error) => {
		const f = fixture()
		f.setFailure(error)
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(pending(f)).toBeDefined()
		expect(f.sent).toHaveLength(1)
	})
	it("blocks send when pending persistence fails", async () => {
		const f = fixture()
		f.ctx.usedUserOpsStorage.setItem = async () => {
			throw new Error("disk full")
		}
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(f.sent).toHaveLength(0)
	})
	it("blocks fallback when definitive rejection cannot be retired durably", async () => {
		const f = fixture()
		f.setFailure({ code: -32500, message: "AA21 didn't pay prefund" })
		const original = f.ctx.usedUserOpsStorage.setItem
		f.ctx.usedUserOpsStorage.setItem = async (key: string, value: string) => {
			if (key.startsWith("used-userops:")) throw new Error("disk full")
			await original(key, value)
		}
		vi.mocked(f.manager.buildBids).mockReturnValue([f.bid(), f.bid(2n)])
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(f.sent).toHaveLength(1)
		expect(pending(f)).toBeDefined()
	})
	it("recovers a crash after terminal persistence without rebroadcast", async () => {
		const f = fixture()
		f.setOnSend(() => f.confirm())
		const original = f.ctx.usedUserOpsStorage.setItem
		f.ctx.usedUserOpsStorage.setItem = async (key: string, value: string) => {
			if (value === "null") throw new Error("crash")
			await original(key, value)
		}
		expect(await attempt(f)).toMatchObject({ status: "FAILED" })
		expect(pending(f)).toBeDefined()
		expect(f.storage.has(`used-userops:${commitment}`)).toBe(true)
		f.ctx.usedUserOpsStorage.setItem = original
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(f.sent).toHaveLength(1)
		expect(pending(f)).toBeUndefined()
	})
	it.each(["json", "nonce", "hash", "scope", "signature"])("fails closed on a corrupt %s journal", async (field) => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		const [key, raw] = pending(f)!
		const record = JSON.parse(raw)
		if (field === "nonce") record.submission.userOp.nonce = "-1"
		if (field === "hash") record.submission.userOpHash = `0x${"00".repeat(32)}`
		if (field === "scope") record.entryPoint = sender
		if (field === "signature") record.submission.userOp.signature = "not hex"
		f.storage.set(key, field === "json" ? "{bad" : JSON.stringify(record))
		expect(await resume(f)).toMatchObject({ status: "FAILED", error: expect.stringContaining("corrupt") })
		expect(f.sent).toHaveLength(1)
	})
	it("reconciles an included operation before the expired-order early return", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		f.confirm()
		f.ctx.dest.client.getBlockNumber = async () => 200n
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(pending(f)).toBeUndefined()
		expect(f.sent).toHaveLength(1)
	})
	it("waits for finalized nonce consumption before selecting another candidate", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		const original = f.ctx.dest.client.readContract
		f.ctx.dest.client.readContract = async (args: any) =>
			args.functionName === "getNonce" ? (args.blockNumber === undefined ? 2n : 1n) : original(args)
		expect(await resume(f)).toMatchObject({ status: "FAILED", error: expect.stringContaining("finalized") })
		expect(f.sent).toHaveLength(1)
		expect(pending(f)).toBeDefined()
		f.ctx.dest.client.readContract = async (args: any) =>
			args.functionName === "getNonce" ? 2n : args.functionName === "_filled" ? sender : 100n
		expect(await resume(f)).toMatchObject({ status: "FILLED" })
		expect(pending(f)).toBeUndefined()
		expect(f.sent).toHaveLength(1)
	})
	it("retires an expired operation only after finalized state reconciliation", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		f.ctx.dest.client.getBlock = async () => ({ number: 101n })
		f.ctx.dest.client.getBlockNumber = async () => 100n
		const s = f.stream()
		let value: any
		for (let i = 0; i < 3; i++) {
			value = (await s.next()).value
			if (value?.status === "EXPIRED") break
		}
		await s.return()
		expect(value).toMatchObject({ status: "EXPIRED" })
		expect(pending(f)).toBeUndefined()
		expect(f.sent).toHaveLength(1)
	})
	it("fails closed on corrupt legacy terminal storage", async () => {
		const f = fixture()
		f.storage.set(`used-userops:${commitment}`, "{}")
		expect(await resume(f)).toMatchObject({ status: "FAILED", error: expect.stringContaining("corrupt") })
		expect(f.sent).toHaveLength(0)
	})
	it("keeps pre-journal legacy terminal hashes terminal", async () => {
		const f = fixture()
		f.storage.set(
			`used-userops:${commitment}`,
			JSON.stringify([CryptoUtils.computeUserOpHash(op(), entryPoint, 8453n)]),
		)
		f.ctx.dest.client.getBlockNumber = async () => 100n
		const s = f.stream()
		for (let i = 0; i < 3; i++) {
			const value = (await s.next()).value
			if (value?.status === "EXPIRED") break
		}
		await s.return()
		expect(f.sent).toHaveLength(0)
	})
	it("round-trips the complete signed operation using decimal bigint fields", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		const journal = new SubmissionJournal(f.ctx.usedUserOpsStorage, {
			chainId: 8453,
			gateway,
			entryPoint,
			commitment,
		})
		const record = await journal.read()
		expect(record?.submission.userOp.nonce).toBe(1n)
		expect(record?.submission.userOp.preVerificationGas).toBe(1n)
		expect(record?.submission.userOp.signature).toBe(f.sent[0].signature)
	})
	it("does not trust a bundler receipt without an EntryPoint event", async () => {
		const f = fixture()
		f.confirm(true, sender)
		await expect(f.bid().execute()).rejects.toBeInstanceOf(BidExecutionPendingError)
	})
	it.each(["another filler", "a fill after the operation event"])(
		"retains the pending journal during recovery with %s",
		async (mismatch) => {
			const f = fixture()
			f.setFailure(new Error("timeout"))
			await attempt(f)
			const stored = pending(f)![1]
			f.confirm()
			const receipt = await f.ctx.dest.client.waitForTransactionReceipt()
			const [before, fill, terminal] = receipt.logs
			if (mismatch === "another filler") {
				// The filler is the first non-indexed address in the event data.
				fill.data = `0x${gateway.slice(2).padStart(64, "0")}${fill.data.slice(66)}`
			} else {
				receipt.logs = [before, terminal, fill].map((log, logIndex) => ({ ...log, logIndex }))
			}
			expect(await resume(f)).toMatchObject({ status: "FAILED" })
			expect(pending(f)?.[1]).toBe(stored)
			expect(f.sent).toHaveLength(1)
		},
	)
	it("uses matching on-chain failure even when the bundler reports success", async () => {
		const f = fixture()
		f.confirm(false)
		await expect(f.bid().execute()).rejects.toBeInstanceOf(BidExecutionRejectedError)
	})
	it.each(["sender", "nonce", "hash"])("requires the operation event to match %s", async (field) => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		const badOp = op()
		const nonce = field === "nonce" ? 2n : 1n
		const logs = [
			{
				address: entryPoint,
				topics: encodeEventTopics({
					abi: eventAbi,
					eventName: "UserOperationEvent",
					args: {
						userOpHash: field === "hash" ? token : CryptoUtils.computeUserOpHash(badOp, entryPoint, 8453n),
						sender: field === "sender" ? gateway : sender,
						paymaster: sender,
					},
				}),
				data: encodeAbiParameters(
					[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
					[nonce, false, 1n, 1n],
				),
			},
		]
		f.setReceipt({ status: "success", logs })
		expect(await resume(f)).toMatchObject({ status: "FAILED" })
		expect(pending(f)).toBeDefined()
		expect(f.sent).toHaveLength(1)
	})
})

describe("remaining submission boundaries", () => {
	it("retains ambiguity when the same concrete bid is executed again after timeout", async () => {
		const f = fixture()
		const bid = f.bid()
		f.setFailure(new Error("timeout"))
		await expect(bid.execute()).rejects.toBeInstanceOf(BidExecutionPendingError)
		f.setFailure({ code: -32500, message: "AA21 didn't pay prefund" })
		await expect(bid.execute()).rejects.toBeInstanceOf(BidExecutionPendingError)
	})
	it.each([{ code: "-32500", message: "AA21" }, { code: -32500 }, {}])(
		"keeps malformed RPC errors ambiguous: %j",
		async (error) => {
			const f = fixture()
			f.setFailure(error)
			expect(await attempt(f)).toMatchObject({ status: "FAILED" })
			expect(pending(f)).toBeDefined()
			expect(f.sent).toHaveLength(1)
		},
	)
	it("requires the send response hash to identify the stored operation", async () => {
		const f = fixture()
		vi.stubGlobal(
			"fetch",
			vi.fn(async () => ({ json: async () => ({ jsonrpc: "2.0", id: 1, result: token }) })),
		)
		expect(await attempt(f)).toMatchObject({
			status: "FAILED",
			error: expect.stringContaining("unexpected operation hash"),
		})
		expect(pending(f)).toBeDefined()
	})
	it("recovers partial progress without counting the receipt credit twice", async () => {
		const f = fixture()
		f.setFailure(new Error("timeout"))
		await attempt(f)
		const submission = JSON.parse(pending(f)![1]).submission
		const operationLog = {
			address: entryPoint,
			topics: encodeEventTopics({
				abi: eventAbi,
				eventName: "UserOperationEvent",
				args: { userOpHash: submission.userOpHash, sender, paymaster: sender },
			}),
			data: encodeAbiParameters(
				[{ type: "uint256" }, { type: "bool" }, { type: "uint256" }, { type: "uint256" }],
				[1n, true, 1n, 1n],
			),
		}
		const partialLog = {
			address: gateway,
			topics: encodeEventTopics({ abi: ABI, eventName: "PartialFill", args: { commitment } }),
			data: encodeAbiParameters(
				[
					{ type: "address" },
					{
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
						],
					},
					{
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
						],
					},
				],
				[sender, [{ token, amount: 40n }], []],
			),
		}
		f.setReceipt({
			status: "success",
			logs: [beforeExecutionLog(), partialLog, operationLog].map((log, logIndex) => ({ ...log, logIndex })),
		})
		const original = f.ctx.dest.client.readContract
		f.ctx.dest.client.readContract = async (args: any) =>
			args.functionName === "_partialFills" ? 40n : original(args)
		expect(await resume(f)).toMatchObject({
			status: "PARTIAL_FILL",
			totalFilledAssets: [{ token, amount: 40n }],
			remainingAssets: [{ token, amount: 60n }],
		})
		expect(pending(f)).toBeUndefined()
		expect(f.sent).toHaveLength(1)
	})
	it("writes terminal state before clearing a successful first submission", async () => {
		const f = fixture()
		const writes: string[] = []
		const original = f.ctx.usedUserOpsStorage.setItem
		f.ctx.usedUserOpsStorage.setItem = async (key: string, value: string) => {
			writes.push(key.startsWith("used-userops:") ? "terminal" : value === "null" ? "clear" : "pending")
			await original(key, value)
		}
		f.setOnSend(() => {
			expect(writes).toEqual(["pending"])
			f.confirm()
		})
		expect(await attempt(f)).toMatchObject({ status: "BID_SELECTED" })
		expect(writes.slice(0, 3)).toEqual(["pending", "terminal", "clear"])
		expect(pending(f)).toBeUndefined()
	})
})

it("retains a pending operation at the inclusive order deadline", async () => {
	const f = fixture()
	f.setFailure(new Error("timeout"))
	await attempt(f)
	f.ctx.dest.client.getBlock = async () => ({ number: 100n })
	expect(await resume(f)).toMatchObject({ status: "FAILED" })
	expect(pending(f)).toBeDefined()
	expect(f.sent).toHaveLength(2)
	expect(f.sent[1]).toEqual(f.sent[0])
})

it("keeps a contradictory RPC result and error pending without selecting another bid", async () => {
	const f = fixture()
	const originalFetch = globalThis.fetch
	const sentNonces: string[] = []
	vi.stubGlobal(
		"fetch",
		vi.fn(async (url, request) => {
			const body = JSON.parse(request.body)
			if (body.method === "eth_sendUserOperation") {
				sentNonces.push(body.params[0].nonce)
				if (body.params[0].nonce === "0x1") {
					return {
						json: async () => ({
							jsonrpc: "2.0",
							id: 1,
							result: CryptoUtils.computeUserOpHash(op(), entryPoint, 8453n),
							error: { code: -32500, message: "AA21 didn't pay prefund" },
						}),
					}
				}
				f.confirm(true, entryPoint, 2n)
			}
			return originalFetch(url, request)
		}),
	)
	const journal = new SubmissionJournal(f.ctx.usedUserOpsStorage, { chainId: 8453, gateway, entryPoint, commitment })
	await expect(
		f.manager.selectAndExecuteBest(
			order,
			[f.bid(), f.bid(2n)],
			(submission) => journal.write(submission),
			async (submission) => {
				await f.ctx.usedUserOpsStorage.setItem(
					`used-userops:${commitment}`,
					JSON.stringify([submission.userOpHash]),
				)
				await journal.clear()
			},
		),
	).rejects.toBeInstanceOf(BidExecutionPendingError)
	expect(sentNonces).toEqual(["0x1"])
	expect(pending(f)).toBeDefined()
	expect(f.storage.has(`used-userops:${commitment}`)).toBe(false)
})
