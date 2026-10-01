;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

import { Interface } from "@ethersproject/abi"

import HandlerV2Abi from "@/configs/abis/HandlerV2.abi.json"
import { decodeHandlerCalls, getHandlerMessageModules } from "@/utils/handler.helpers"

const handler = new Interface(HandlerV2Abi)

const HOST = "0x620128e2b19193d6bd244a3ac8d3bba0541b19c3"
const GATEWAY = "0xae041f7b0cb581876832830baeb6a2aa2a3c9716"
const OTHER_MODULE = "0x1111111111111111111111111111111111111111"

const height = { stateMachineId: 3367n, height: 100n }
const proof = { height, multiproof: ["0x" + "ab".repeat(32)], leafCount: 9n }

const postRequest = (to: string, from = OTHER_MODULE) => ({
	source: "0x504f4c4b41444f542d33333637",
	dest: "0x45564d2d313337",
	nonce: 7n,
	from,
	to,
	timeoutTimestamp: 0n,
	body: "0x1234",
})

const getRequest = (from: string) => ({
	source: "0x45564d2d313337",
	dest: "0x504f4c4b41444f542d33333637",
	nonce: 7n,
	from,
	timeoutTimestamp: 0n,
	keys: ["0x01"],
	height: 5n,
	context: "0x",
})

const consensus = handler.encodeFunctionData("handleConsensus", [HOST, "0xdeadbeef"])

const postRequests = (...to: string[]) =>
	handler.encodeFunctionData("handlePostRequests", [
		HOST,
		{ proof, requests: to.map((module, index) => ({ request: postRequest(module), index })) },
	])

beforeEach(() => jest.clearAllMocks())

describe("decodeHandlerCalls", () => {
	it("reads a direct handler call as one call", () => {
		expect(decodeHandlerCalls(consensus).map(({ name }) => name)).toEqual(["handleConsensus"])
	})

	it("unwraps a batchCall into the calls it carries, in order", () => {
		const batch = handler.encodeFunctionData("batchCall", [[consensus, postRequests(GATEWAY)]])

		expect(batch.slice(0, 10)).toBe("0x68be3cf2")
		expect(decodeHandlerCalls(batch).map(({ name }) => name)).toEqual(["handleConsensus", "handlePostRequests"])
	})

	it("unwraps a batchCall nested in another", () => {
		const inner = handler.encodeFunctionData("batchCall", [[postRequests(GATEWAY)]])
		const outer = handler.encodeFunctionData("batchCall", [[consensus, inner]])

		expect(decodeHandlerCalls(outer).map(({ name }) => name)).toEqual(["handleConsensus", "handlePostRequests"])
	})

	it("answers no calls for calldata that is not a handler call, rather than throwing", () => {
		expect(decodeHandlerCalls(undefined)).toEqual([])
		expect(decodeHandlerCalls("0x")).toEqual([])
		expect(decodeHandlerCalls("0xa9059cbb" + "00".repeat(64))).toEqual([])
	})

	it("skips an unreadable call inside a batch and keeps the rest", () => {
		const batch = handler.encodeFunctionData("batchCall", [["0xa9059cbb", postRequests(GATEWAY)]])

		expect(decodeHandlerCalls(batch).map(({ name }) => name)).toEqual(["handlePostRequests"])
	})
})

describe("getHandlerMessageModules", () => {
	it("names the recipients of the POST requests in a batched delivery", () => {
		const batch = handler.encodeFunctionData("batchCall", [[consensus, postRequests(GATEWAY, OTHER_MODULE)]])

		expect(getHandlerMessageModules(batch, "handlePostRequests")).toEqual([GATEWAY, OTHER_MODULE])
		expect(logger.warn).not.toHaveBeenCalled()
	})

	it("names them across every matching call in the batch", () => {
		const batch = handler.encodeFunctionData("batchCall", [[postRequests(GATEWAY), postRequests(OTHER_MODULE)]])

		expect(getHandlerMessageModules(batch, "handlePostRequests")).toEqual([GATEWAY, OTHER_MODULE])
	})

	it("names them in a direct delivery", () => {
		const direct = postRequests(GATEWAY)

		expect(direct.slice(0, 10)).toBe("0x698b1267")
		expect(getHandlerMessageModules(direct, "handlePostRequests")).toEqual([GATEWAY])
	})

	it("names the senders of answered GET requests, batched or direct", () => {
		const values = [{ key: "0x01", value: "0x02" }]
		const responses = handler.encodeFunctionData("handleGetResponses", [
			HOST,
			{ proof, responses: [{ response: { request: getRequest(GATEWAY), values }, index: 0n }] },
		])
		const batch = handler.encodeFunctionData("batchCall", [[consensus, responses]])

		expect(getHandlerMessageModules(responses, "handleGetResponses")).toEqual([GATEWAY])
		expect(getHandlerMessageModules(batch, "handleGetResponses")).toEqual([GATEWAY])
	})

	it("names the senders of timed-out POST and GET requests", () => {
		const postTimeouts = handler.encodeFunctionData("handlePostRequestTimeouts", [
			HOST,
			{ timeouts: [postRequest(OTHER_MODULE, GATEWAY)], height, proof: ["0x01"] },
		])
		const getTimeouts = handler.encodeFunctionData("handleGetRequestTimeouts", [
			HOST,
			{ timeouts: [getRequest(GATEWAY)], height, proof: ["0x01"] },
		])
		const batch = handler.encodeFunctionData("batchCall", [[consensus, postTimeouts, getTimeouts]])

		expect(getHandlerMessageModules(batch, "handlePostRequestTimeouts")).toEqual([GATEWAY])
		expect(getHandlerMessageModules(batch, "handleGetRequestTimeouts")).toEqual([GATEWAY])
	})

	it("answers no modules for a call of another kind", () => {
		expect(getHandlerMessageModules(consensus, "handlePostRequests")).toEqual([])
		expect(getHandlerMessageModules(postRequests(GATEWAY), "handleGetResponses")).toEqual([])
		expect(logger.warn).not.toHaveBeenCalled()
	})

	it("answers no modules and says so when the calldata is not a handler call", () => {
		expect(getHandlerMessageModules("0xa9059cbb" + "00".repeat(64), "handlePostRequests")).toEqual([])
		expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("0xa9059cbb"))
	})

	it("answers no modules for a transaction without calldata", () => {
		expect(getHandlerMessageModules(undefined, "handlePostRequests")).toEqual([])
		expect(logger.warn).not.toHaveBeenCalled()
	})
})
