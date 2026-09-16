import { decodeEventLog, encodeAbiParameters, parseAbiParameters } from "viem"
import { expect, it } from "vitest"
import { ABI } from "../abis/IntentGatewayV2"

it("decodes the solver and released tokens from an EscrowReleased log", () => {
	const commitment = "0x1111111111111111111111111111111111111111111111111111111111111111"
	const solver = "0x2222222222222222222222222222222222222222"
	const token = "0x3333333333333333333333333333333333333333333333333333333333333333"

	// Wire format from IntentsBase.EscrowReleased, independent of the ABI under test.
	const decoded = decodeEventLog({
		abi: ABI,
		topics: ["0x295adf27451ecabbf0c3858d66bf3d23fd8df2a3e075a03fbdb4aee85a69d51a", commitment],
		data: encodeAbiParameters(parseAbiParameters("address solver, (bytes32 token, uint256 amount)[] tokens"), [
			solver,
			[{ token, amount: 123n }],
		]),
	})

	expect(decoded.eventName).toBe("EscrowReleased")
	expect(decoded.args).toEqual({
		commitment,
		solver,
		tokens: [{ token, amount: 123n }],
	})
})
