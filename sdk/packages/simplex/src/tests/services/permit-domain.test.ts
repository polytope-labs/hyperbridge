import { describe, expect, it } from "vitest"
import {
	ContractFunctionExecutionError,
	ContractFunctionRevertedError,
	HttpRequestError,
	pad,
	recoverTypedDataAddress,
	toHex,
	type PublicClient,
	type TypedDataDefinition,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import type { HexString } from "@hyperbridge/sdk"
import { resolvePermitDomain, signEip2612Permit } from "@/services/paymaster/permit"

/**
 * The domain separators below were read from mainnet on 2026-10-14. They pin the three EIP-2612
 * shapes Simplex's fee tokens use, so a bootstrap permit is signed over the domain each token checks.
 */
const POLYGON_USDT0 = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F" as HexString
const ARBITRUM_USDT0 = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9" as HexString
const POLYGON_USDC = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359" as HexString

type TokenViews = { DOMAIN_SEPARATOR?: HexString | Error; name?: string; version?: string; nonces?: bigint }

const TOKENS: Record<string, TokenViews> = {
	[POLYGON_USDT0]: {
		DOMAIN_SEPARATOR: "0x7b43b7deae87806d0ace67d6c8e9e347fc85db8ad198e756e5c17d126fef9a05",
		name: "USDT0",
		nonces: 3n,
	},
	[ARBITRUM_USDT0]: {
		DOMAIN_SEPARATOR: "0x566af68fb471b22d6421762f84aa7bd761c670a2e4d5c8a47d4085d5957b127c",
		name: "USD₮0",
	},
	[POLYGON_USDC]: {
		DOMAIN_SEPARATOR: "0xcaa2ce1a5703ccbe253a34eb3166df60a705c561b44b192061e28f2a985be2ca",
		name: "USD Coin",
		version: "2",
	},
}

function reverted(functionName: string): Error {
	return new ContractFunctionExecutionError(
		new ContractFunctionRevertedError({ abi: [], functionName, message: "execution reverted" }),
		{ abi: [], functionName } as never,
	)
}

/** A client whose token exposes exactly `views`; anything else reverts, as a missing function does. */
function clientFor(views: Record<string, TokenViews>): PublicClient {
	return {
		readContract: async ({ address, functionName }: { address: HexString; functionName: keyof TokenViews }) => {
			const value = views[address]?.[functionName]
			if (value instanceof Error) throw value
			if (value === undefined) throw reverted(functionName)
			return value
		},
	} as unknown as PublicClient
}

describe("EIP-2612 permit domain", () => {
	const client = clientFor(TOKENS)

	it("matches Polygon USDT0's salt domain, which has no chainId and no version()", async () => {
		const resolved = await resolvePermitDomain(client, POLYGON_USDT0, 137)
		expect(resolved?.domain).toEqual({
			name: "USDT0",
			version: "1",
			verifyingContract: POLYGON_USDT0,
			salt: pad(toHex(137), { size: 32 }),
		})
		expect(resolved?.fields.map((field) => field.name)).toEqual(["name", "version", "verifyingContract", "salt"])
	})

	it("matches Arbitrum USDT0's standard domain even though it has no version()", async () => {
		const resolved = await resolvePermitDomain(client, ARBITRUM_USDT0, 42161)
		expect(resolved?.domain).toEqual({
			name: "USD₮0",
			version: "1",
			chainId: 42161,
			verifyingContract: ARBITRUM_USDT0,
		})
	})

	it("matches Polygon USDC's standard domain using its own version()", async () => {
		const resolved = await resolvePermitDomain(client, POLYGON_USDC, 137)
		expect(resolved?.domain).toMatchObject({ version: "2", chainId: 137 })
	})

	it("finds no permit on a token without DOMAIN_SEPARATOR, such as Ethereum USDT", async () => {
		const ethereumUsdt = "0xdAC17F958D2ee523a2206206994597C13D831ec7" as HexString
		const tether = clientFor({ [ethereumUsdt]: { name: "Tether USD" } })
		expect(await resolvePermitDomain(tether, ethereumUsdt, 1)).toBeNull()
	})

	it("finds no permit when the separator matches none of the known shapes", async () => {
		const odd = clientFor({
			[POLYGON_USDT0]: { ...TOKENS[POLYGON_USDT0], DOMAIN_SEPARATOR: `0x${"ab".repeat(32)}` },
		})
		expect(await resolvePermitDomain(odd, POLYGON_USDT0, 137)).toBeNull()
	})

	it("propagates a transport error rather than reading it as no permit", async () => {
		const rateLimited = new ContractFunctionExecutionError(
			new HttpRequestError({ url: "https://rpc.example", status: 429, details: "Too Many Requests" }),
			{ abi: [], functionName: "DOMAIN_SEPARATOR" } as never,
		)
		const flaky = clientFor({ [POLYGON_USDT0]: { ...TOKENS[POLYGON_USDT0], DOMAIN_SEPARATOR: rateLimited } })
		await expect(resolvePermitDomain(flaky, POLYGON_USDT0, 137)).rejects.toThrow(/HTTP request failed/)
	})

	it("signs a Polygon USDT0 permit over the salt domain, recoverable to the owner", async () => {
		const account = privateKeyToAccount(`0x${"42".repeat(32)}`)
		let signed: TypedDataDefinition | undefined
		const signer = {
			signTypedData: async (typedData: TypedDataDefinition) => {
				signed = typedData
				return account.signTypedData(typedData as never)
			},
		}
		const spender = "0x1111111111111111111111111111111111111111" as HexString

		const signature = await signEip2612Permit(
			client,
			signer as never,
			account.address,
			spender,
			POLYGON_USDT0,
			5_000_000n,
			137,
		)

		expect(signed?.types?.EIP712Domain).toEqual([
			{ name: "name", type: "string" },
			{ name: "version", type: "string" },
			{ name: "verifyingContract", type: "address" },
			{ name: "salt", type: "bytes32" },
		])
		expect(signed?.message).toMatchObject({ owner: account.address, spender, value: 5_000_000n, nonce: 3n })
		expect(await recoverTypedDataAddress({ ...(signed as TypedDataDefinition), signature })).toBe(account.address)
	})
})
