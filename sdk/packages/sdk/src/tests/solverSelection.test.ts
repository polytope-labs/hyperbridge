import { createPublicClient, custom, encodeAbiParameters, recoverAddress, toFunctionSelector } from "viem"
import { describe, expect, it } from "vitest"
import { CryptoUtils, LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import { readSelectionFormat } from "@/protocols/intents/selection"
import type { HexString } from "@/types"

// Vectors derived with foundry (`cast abi-encode` + `cast keccak`, cross-checked with
// `cast wallet sign --data` over the EIP-712 JSON), independent of the SDK.
const CHAIN_ID = 8453n
const GATEWAY = "0x6CF42FA9BecbC5b6a26884964956b113530f7cFA" as HexString
const SESSION_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const SESSION_ADDRESS = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
const COMMITMENT = "0x5193b7b26e003ba73618491c243b3e2aa9ec144ce99506d13c38d523b298fa82" as HexString
const USER_OP_HASH = "0xab9c42a6333902fb3a76ad9afe551c6136a717d399208978e99f0c3a437cebe4" as HexString
const SOLVER = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString

const DOMAIN_SEPARATOR = "0x0f99e3135cb4975ba4730000fb380174ac0c44e7ff4a028af7cc448bad9230be"
const USER_OP_HASH_TYPEHASH = "0xe706bdab7d945360dcd9d81d355f856754dd1cfa461edfc0a7502e2583b4e09e"
const ADDRESS_TYPEHASH = "0x4e7a964fde3ec316fdcf2e27971047ec3048eaa33a854c4c0b363e05c61b68d0"
const USER_OP_HASH_DIGEST = "0x7dcb5f1729deab3f4e5522520839b3e5ba2f9e7f0ce3454597cace7ed37f13ed"
const ADDRESS_DIGEST = "0xdb1462cadfba4250c00830d787f8c777f8a18dbc9ad0160de16e7f34ece56b9c"
const USER_OP_HASH_SIGNATURE =
	"0x4252ea55528ee8b8787f76949c5e7741c0ae7ad604c88047e9bacd3a2bcfdda02e181448672502e9d83110fa095ddf8c07c3f889faaa02531bb180543f4fef2a1b"
const ADDRESS_SIGNATURE =
	"0x23bbf20996acba6d2a6c53e35d8e69812f04be01430837104f68ffd8898e4a762b4260f9d0bb1fe43077ac70eb84e936c5bd73bac89f30f9c9a9fd56d5c6c64e1b"

const TYPEHASH_SELECTOR = toFunctionSelector("SELECT_SOLVER_TYPEHASH()")

/** A client over a gateway that answers `SELECT_SOLVER_TYPEHASH()` with `typehash`, or fails while `down`. */
function gatewayClient(state: { typehash: HexString; down?: boolean; calls: { to: string; data: string }[] }) {
	return createPublicClient({
		transport: custom(
			{
				async request({ method, params }) {
					if (method !== "eth_call") throw new Error(`unexpected ${method}`)
					const { to, data } = (params as [{ to: string; data: string }])[0]
					state.calls.push({ to, data })
					if (state.down) throw new Error("fetch failed")
					if (data !== TYPEHASH_SELECTOR) throw new Error(`unexpected call ${data}`)
					return encodeAbiParameters([{ type: "bytes32" }], [state.typehash])
				},
			},
			{ retryCount: 0 },
		),
	})
}

describe("solver selection signatures", () => {
	it("typehashes and domain separator match the foundry vectors", () => {
		expect(SELECT_SOLVER_TYPEHASH).toBe(USER_OP_HASH_TYPEHASH)
		expect(LEGACY_SELECT_SOLVER_TYPEHASH).toBe(ADDRESS_TYPEHASH)
		expect(CryptoUtils.getDomainSeparator("IntentGateway", "2", CHAIN_ID, GATEWAY)).toBe(DOMAIN_SEPARATOR)
	})

	it("signs SelectSolver(bytes32 commitment,bytes32 userOpHash)", async () => {
		const signature = await CryptoUtils.signUserOpHashSelection(
			COMMITMENT,
			USER_OP_HASH,
			DOMAIN_SEPARATOR,
			SESSION_KEY,
		)
		expect(signature).toBe(USER_OP_HASH_SIGNATURE)
		expect(await recoverAddress({ hash: USER_OP_HASH_DIGEST, signature })).toBe(SESSION_ADDRESS)
	})

	it("signs SelectSolver(bytes32 commitment,address solver)", async () => {
		const signature = await CryptoUtils.signLegacySolverSelection(COMMITMENT, SOLVER, DOMAIN_SEPARATOR, SESSION_KEY)
		expect(signature).toBe(ADDRESS_SIGNATURE)
		expect(await recoverAddress({ hash: ADDRESS_DIGEST, signature })).toBe(SESSION_ADDRESS)
	})
})

describe("readSelectionFormat", () => {
	it("reads userOpHash from an upgraded gateway", async () => {
		const state = { typehash: USER_OP_HASH_TYPEHASH as HexString, calls: [] as { to: string; data: string }[] }
		const gateway = "0x0000000000000000000000000000000000000a01" as HexString

		expect(await readSelectionFormat(gatewayClient(state), CHAIN_ID, gateway)).toBe("userOpHash")
		expect(state.calls).toEqual([{ to: gateway, data: TYPEHASH_SELECTOR }])
	})

	it("reads address from a gateway that selects by address", async () => {
		const state = { typehash: ADDRESS_TYPEHASH as HexString, calls: [] as { to: string; data: string }[] }

		expect(
			await readSelectionFormat(gatewayClient(state), CHAIN_ID, "0x0000000000000000000000000000000000000a02"),
		).toBe("address")
	})

	it("throws on an unknown typehash", async () => {
		const state = { typehash: `0x${"11".repeat(32)}` as HexString, calls: [] as { to: string; data: string }[] }

		await expect(
			readSelectionFormat(gatewayClient(state), CHAIN_ID, "0x0000000000000000000000000000000000000a03"),
		).rejects.toThrow(/unknown SELECT_SOLVER_TYPEHASH/)
	})

	it("reads once per chain and gateway", async () => {
		const state = { typehash: USER_OP_HASH_TYPEHASH as HexString, calls: [] as { to: string; data: string }[] }
		const client = gatewayClient(state)
		const gateway = GATEWAY

		await Promise.all([
			readSelectionFormat(client, CHAIN_ID, gateway),
			readSelectionFormat(client, CHAIN_ID, gateway.toLowerCase() as HexString),
		])
		await readSelectionFormat(client, Number(CHAIN_ID), gateway)
		expect(state.calls).toHaveLength(1)

		state.typehash = ADDRESS_TYPEHASH
		expect(await readSelectionFormat(client, 1n, gateway)).toBe("address")
		expect(await readSelectionFormat(client, CHAIN_ID, gateway)).toBe("userOpHash")
		expect(state.calls).toHaveLength(2)
	})

	it("reads again after a failed read", async () => {
		const state = {
			typehash: USER_OP_HASH_TYPEHASH as HexString,
			down: true,
			calls: [] as { to: string; data: string }[],
		}
		const client = gatewayClient(state)
		const gateway = "0x0000000000000000000000000000000000000a05" as HexString

		await expect(readSelectionFormat(client, CHAIN_ID, gateway)).rejects.toThrow()
		state.down = false
		expect(await readSelectionFormat(client, CHAIN_ID, gateway)).toBe("userOpHash")
		expect(state.calls).toHaveLength(2)
	})
})
