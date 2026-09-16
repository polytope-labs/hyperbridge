import { Interface } from "@ethersproject/abi"
import { resolvePlacementUserOpHash } from "@/utils/userOp.helpers"

const entryPoint = "0x4337084d9e255ff0702461cf8895ce9e3b5ff108"
const account = "0x2222222222222222222222222222222222222222"
const other = "0x3333333333333333333333333333333333333333"
const hash = "0x" + "ab".repeat(32)
const laterHash = "0x" + "cd".repeat(32)
const abi = new Interface([
	"event UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster, uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)",
	"event BeforeExecution()",
])
const boundary = (logIndex: number, sender = account, userOpHash = hash, address = entryPoint) => ({
	...abi.encodeEventLog(abi.getEvent("UserOperationEvent"), [userOpHash, sender, other, 1, true, 1, 1]),
	address,
	logIndex,
})
const start = (logIndex = 0, address = entryPoint) => ({
	...abi.encodeEventLog(abi.getEvent("BeforeExecution"), []),
	address,
	logIndex,
})
const resolve = (logs: unknown[], logIndex = 5, sender = account) =>
	resolvePlacementUserOpHash(
		{ logIndex, transactionHash: hash, transaction: { receipt: async () => ({ logs }) } } as any,
		sender,
	)

beforeEach(() => {
	;(global as any).logger = { warn: jest.fn() }
})

it.each(["0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789", "0x0000000071727de22e5e9d8baf0edac6f37da032", entryPoint])(
	"matches a placement from canonical EntryPoint %s",
	async (address) => {
		expect(await resolve([start(0, address), boundary(6, account, hash, address)])).toBe(hash)
	},
)

it("selects each operation for repeated senders in an unordered bundled receipt", async () => {
	const logs = [boundary(12, account, laterHash), boundary(6), start()]
	expect(await resolve(logs, 5)).toBe(hash)
	expect(await resolve(logs, 10)).toBe(laterHash)
})

it("assigns multiple placements in one operation the same hash", async () => {
	const logs = [start(), boundary(6)]
	expect(await resolve(logs, 3)).toBe(hash)
	expect(await resolve(logs, 5)).toBe(hash)
})

it("does not skip a different sender to find a later matching operation", async () => {
	expect(await resolve([start(), boundary(6, other), boundary(10)])).toBeUndefined()
})

it("ignores spoofed events and matches the placing account case-insensitively", async () => {
	expect(
		await resolve([start(), boundary(6, account, laterHash, other), boundary(10)], 5, account.toUpperCase()),
	).toBe(hash)
})

it.each([
	{ name: "direct transaction", logs: [] },
	{ name: "operation before placement", logs: [start(), boundary(4)] },
	{ name: "missing execution boundary", logs: [boundary(6)] },
	{ name: "spoofed execution boundary", logs: [start(0, other), boundary(6)] },
	{ name: "placement during validation", logs: [start(7), boundary(10)] },
	{ name: "placement between bundles", logs: [start(), boundary(2), start(7), boundary(10)] },
])("leaves $name unset", async ({ logs }) => {
	expect(await resolve(logs)).toBeUndefined()
})

it("leaves the field unset when the transaction or receipt is unavailable", async () => {
	expect(await resolvePlacementUserOpHash({ logIndex: 5, transactionHash: hash }, account)).toBeUndefined()
	expect(
		await resolvePlacementUserOpHash(
			{
				logIndex: 5,
				transactionHash: hash,
				transaction: {
					receipt: async () => {
						throw new Error("RPC unavailable")
					},
				},
			} as any,
			account,
		),
	).toBeUndefined()
	expect(logger.warn).toHaveBeenCalledTimes(1)
})
