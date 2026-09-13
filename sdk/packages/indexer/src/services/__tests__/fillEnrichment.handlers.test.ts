import { Interface } from "@ethersproject/abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { handleOrderFilledEventV3 } from "@/handlers/events/intentGatewayV3/orderFilledV3.event.handler"
import { handlePartialFilledEventV3 } from "@/handlers/events/intentGatewayV3/partialFilledV3.event.handler"
import { handleOrderPlacedEventV3 } from "@/handlers/events/intentGatewayV3/orderPlacedV3.event.handler"
import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import { getHostFeeToken } from "@/utils/host.helpers"

jest.mock("@/utils/rpc.helpers", () => ({ getBlockTimestamp: async () => 1000n, getContractCallInputs: jest.fn() }))
jest.mock("@/utils/host.helpers", () => ({ getHostFeeToken: jest.fn() }))
jest.mock("@/utils/substrate.helpers", () => ({ getHostStateMachine: () => "EVM-56" }))
jest.mock("@/services/points.service", () => ({ PointsService: { awardPoints: jest.fn() } }))
jest.mock("@/services/volume.service", () => ({ VolumeService: { updateVolume: jest.fn() } }))
jest.mock("@/services/userActivity.services", () => ({
	getOrCreateUser: async () => ({ totalOrdersPlaced: 0n, totalOrderPlacedVolumeUSD: "0", save: jest.fn() }),
}))

const records = new Map<string, any>()
const gateway = "0x1111111111111111111111111111111111111111"
const filler = "0x2222222222222222222222222222222222222222"
const beneficiary = "0x3333333333333333333333333333333333333333"
const token = "0x4444444444444444444444444444444444444444"
const zero = "0x0000000000000000000000000000000000000000"
const hash = "0x" + "ab".repeat(32)
const commitment = "0x" + "cd".repeat(32)
const pad = (address: string) => `0x${address.slice(2).padStart(64, "0")}`
const abi = new Interface(IntentGatewayV3Abi)
const logs = [
	{
		address: token,
		logIndex: 2,
		topics: ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef", pad(filler), pad(beneficiary)],
		data: "0x" + 105n.toString(16).padStart(64, "0"),
	},
	{
		address: "0x4337084d9e255ff0702461cf8895ce9e3b5ff108",
		logIndex: 6,
		topics: ["0x49628fd1471006c1482da88028e9ce4dbb080b815c9b0344d39e5a8e6ec1419f", hash, pad(filler), pad(zero)],
		data: "0x",
	},
]

beforeEach(() => {
	records.clear()
	;(global as any).chainId = "56"
	;(global as any).logger = { info: jest.fn(), warn: jest.fn(), error: jest.fn() }
	;(global as any).store = {
		get: async (entity: string, id: string) => records.get(`${entity}:${id}`),
		set: async (entity: string, id: string, data: any) => records.set(`${entity}:${id}`, { ...data }),
	}
	jest.spyOn(IntentGatewayV3Service, "updateOrderStatus").mockResolvedValue(undefined)
	jest.spyOn(IntentGatewayV3Service, "publishInventoryAfterFill").mockResolvedValue(undefined)
	jest.spyOn(IntentGatewayV3Service, "recordOrderVolume").mockResolvedValue(undefined)
	jest.spyOn(IntentGatewayV3Service, "flushPendingStatuses").mockResolvedValue(undefined)
	jest.spyOn(IntentGatewayV3Service as any, "getOrderValue").mockResolvedValue({
		inputUSD: "100",
		outputUSD: "100",
	} as any)
	jest.mocked(getHostFeeToken).mockResolvedValue({ address: token, decimals: 6 })
})
afterEach(() => jest.restoreAllMocks())

it.each([
	["OrderFilled", handleOrderFilledEventV3, "IOrderV3Fill", "IOrderV3FillOutputAsset"],
	["PartialFill", handlePartialFilledEventV3, "IOrderV3PartialFill", "IOrderV3PartialFillOutputAsset"],
] as const)(
	"persists %s receipt data while keeping inventory refresh and event amounts",
	async (name, handle, entity, asset) => {
		records.set(`IOrderV3OutputAsset:${commitment}-output-0`, {
			id: `${commitment}-output-0`,
			orderId: commitment,
			token: pad(token),
			amount: 100n,
			index: 0,
			beneficiary: pad(beneficiary),
		})
		const encoded = abi.encodeEventLog(abi.getEvent(name), [
			commitment,
			filler,
			[{ token: pad(token), amount: 100n }],
			[],
		])
		const event = {
			address: gateway,
			transactionHash: hash,
			blockHash: hash,
			blockNumber: 10,
			logIndex: 5,
			args: abi.decodeEventLog(name, encoded.data, encoded.topics),
			transaction: { input: "0x", receipt: async () => ({ logs }) },
		} as any
		await handle(event)
		expect(records.get(`${entity}:${hash}.5`)).toMatchObject({
			userOpHash: hash,
			transactionHash: hash,
			orderId: commitment,
		})
		expect(records.get(`${asset}:${hash}.5-output-0`)).toMatchObject({ amount: 100n, amountReceived: 105n })
		expect(IntentGatewayV3Service.publishInventoryAfterFill).toHaveBeenCalledTimes(1)
		if (name === "OrderFilled")
			expect(IntentGatewayV3Service.recordOrderVolume).toHaveBeenCalledWith(
				"FILLED",
				[{ token: pad(token), amount: 100n }],
				1000n,
			)
		await handle(event)
		expect([...records.keys()].filter((key) => key.startsWith(`${asset}:`))).toHaveLength(1)
	},
)

it("records plain EOA fills without receipt enrichment when no transaction is supplied", async () => {
	await IntentGatewayV3Service.recordFill(commitment, filler, [{ token: pad(token) as any, amount: 100n }], [], {
		transactionHash: hash,
		blockNumber: 10,
		timestamp: 1000n,
		logIndex: 5,
	})
	expect(records.get(`IOrderV3Fill:${hash}.5`).userOpHash).toBeUndefined()
	expect(records.get(`IOrderV3FillOutputAsset:${hash}.5-output-0`).amountReceived).toBeUndefined()
})

function placementEvent() {
	const inputs = [{ token: pad(token), amount: 100n }]
	return {
		address: gateway,
		transactionHash: hash,
		blockHash: hash,
		blockNumber: 10,
		logIndex: 5,
		args: {
			user: pad(beneficiary),
			source: "0x45564d2d3536",
			destination: "0x45564d2d3536",
			deadline: 100n,
			nonce: 1n,
			fees: 5n,
			session: zero,
			predispatch: [],
			inputs,
			outputs: inputs,
			beneficiary: pad(beneficiary),
			predispatchCall: "0x",
			outputCall: "0x",
			graffiti: pad(beneficiary),
		},
	} as any
}

it("persists fee denomination from a current OrderPlaced event and fills it on existing rows", async () => {
	const event = placementEvent()
	await handleOrderPlacedEventV3(event)
	const key = [...records.keys()].find((key) => key.startsWith("IOrderV3:"))!
	expect(records.get(key)).toMatchObject({ fees: 5n, feeToken: token, feeTokenDecimals: 6 })
	expect(getHostFeeToken).toHaveBeenCalledWith("EVM-56", hash)
	const existing = records.get(key)
	delete existing.feeToken
	delete existing.feeTokenDecimals
	existing.status = "FILLED"
	await handleOrderPlacedEventV3(event)
	expect(records.get(key)).toMatchObject({ feeToken: token, feeTokenDecimals: 6, status: "FILLED" })
})

const placementOpHash = "0x" + "ef".repeat(32)
const entryPointAbi = new Interface([
	"event BeforeExecution()",
	"event UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster, uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)",
])
const placementReceipt = () => ({
	logs: [
		{
			address: logs[1].address,
			logIndex: 1,
			...entryPointAbi.encodeEventLog(entryPointAbi.getEvent("BeforeExecution"), []),
		},
		{
			address: logs[1].address,
			logIndex: 6,
			...entryPointAbi.encodeEventLog(entryPointAbi.getEvent("UserOperationEvent"), [
				placementOpHash,
				beneficiary,
				zero,
				1,
				true,
				1,
				1,
			]),
		},
	],
})

it.each(["current", "legacy"])("persists the placing operation from a %s OrderPlaced event", async (version) => {
	const event = placementEvent()
	// The receiving beneficiary is independent of the account placing the order.
	event.args.beneficiary = pad(filler)
	event.transaction = { receipt: async () => placementReceipt() }
	if (version === "legacy") {
		event.transaction.input = abi.encodeFunctionData("placeOrder", [
			{
				user: event.args.user,
				source: event.args.source,
				destination: event.args.destination,
				deadline: event.args.deadline,
				nonce: event.args.nonce,
				fees: event.args.fees,
				session: event.args.session,
				predispatch: { assets: [], call: "0x" },
				inputs: event.args.inputs,
				output: { assets: event.args.outputs, beneficiary: event.args.beneficiary, call: "0x" },
			},
			event.args.graffiti,
		])
		delete event.args.predispatchCall
		delete event.args.outputCall
		delete event.args.graffiti
	}
	await handleOrderPlacedEventV3(event)
	const key = [...records.keys()].find((key) => key.startsWith("IOrderV3:"))!
	expect(records.get(key)).toMatchObject({ transactionHash: hash, userOpHash: placementOpHash, user: beneficiary })
	// A successful replay can enrich an older row without altering its lifecycle status.
	records.get(key).status = "REFUNDED"
	delete records.get(key).userOpHash
	await handleOrderPlacedEventV3(event)
	expect(records.get(key)).toMatchObject({ status: "REFUNDED", userOpHash: placementOpHash })
	// An unavailable optional lookup must not erase previously stored enrichment.
	event.transaction.receipt = async () => {
		throw new Error("RPC unavailable")
	}
	await handleOrderPlacedEventV3(event)
	expect(records.get(key)).toMatchObject({ status: "REFUNDED", userOpHash: placementOpHash })
})

it.each(["direct", "missing transaction", "unavailable receipt"])(
	"indexes a %s placement without userOpHash",
	async (kind) => {
		const event = placementEvent()
		if (kind !== "missing transaction")
			event.transaction = {
				receipt: async () => {
					if (kind === "unavailable receipt") throw new Error("RPC unavailable")
					return { logs: [] }
				},
			}
		await handleOrderPlacedEventV3(event)
		const key = [...records.keys()].find((key) => key.startsWith("IOrderV3:"))!
		expect(records.get(key)).toMatchObject({ status: "PLACED", transactionHash: hash, feeToken: token })
		expect(records.get(key).userOpHash).toBeUndefined()
		expect(IntentGatewayV3Service.updateOrderStatus).toHaveBeenCalledWith(
			expect.any(String),
			"PLACED",
			expect.any(Object),
		)
	},
)
