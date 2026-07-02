import { GetRequestService } from "../getRequest.service"

// Mock the logger to avoid console output during tests
;(global as any).logger = {
	debug: jest.fn(),
	info: jest.fn(),
	warn: jest.fn(),
	error: jest.fn(),
}

describe("GetRequestService.computeRequestCommitment", () => {
	// Real request from https://github.com/polytope-labs/hyperbridge/issues/1013
	// (Sepolia -> Base Sepolia GET, nonce 2). The commitment must match the value the
	// EVM host emits on-chain: keccak256(abi.encode(GetRequest)).
	it("matches the on-chain commitment (abi.encode parity)", () => {
		const source = "EVM-11155111"
		const dest = "EVM-84532"
		const nonce = 2n
		const height = 0n
		const timeoutTimestamp = 0n
		const from = "0x7d6b92fa404123e6398d2de862d967f95533c90d"
		const keys = ["0x397cfd836f98eed9fad1041a07de20fe6abaa389ac33ff75c19e70fe83507db0d683fd3465c996598dc972688b7ace676c89077b"]
		const context = "0x0000000000000000000000000000000000000000000000000000000000000000"

		const commitment = GetRequestService.computeRequestCommitment(
			source,
			dest,
			nonce,
			height,
			timeoutTimestamp,
			from,
			keys,
			context,
		)

		expect(commitment).toBe("0xdaa89209aac40e56d0d53dd5105aef394da36a149dc3e226c10b08897303a379")
	})
})
