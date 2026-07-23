import { describe, it, expect } from "vitest"
import { parseAlchemyUrl, isAlchemyUrl, deriveAlchemyRpc } from "@/cli/init/derive/alchemy"

describe("parseAlchemyUrl", () => {
	it("parses a valid mainnet URL", () => {
		expect(parseAlchemyUrl("https://eth-mainnet.g.alchemy.com/v2/abc123XYZ")).toEqual({
			subdomain: "eth-mainnet",
			apiKey: "abc123XYZ",
		})
	})

	it("parses testnet subdomains", () => {
		expect(parseAlchemyUrl("https://base-sepolia.g.alchemy.com/v2/key")).toEqual({
			subdomain: "base-sepolia",
			apiKey: "key",
		})
	})

	it("rejects http", () => {
		expect(parseAlchemyUrl("http://eth-mainnet.g.alchemy.com/v2/key")).toBeNull()
	})

	it("rejects non-alchemy hosts", () => {
		expect(parseAlchemyUrl("https://mainnet.infura.io/v3/key")).toBeNull()
		expect(parseAlchemyUrl("https://eth-mainnet.g.alchemy.com.evil.com/v2/key")).toBeNull()
	})

	it("rejects missing or malformed key paths", () => {
		expect(parseAlchemyUrl("https://eth-mainnet.g.alchemy.com/v2/")).toBeNull()
		expect(parseAlchemyUrl("https://eth-mainnet.g.alchemy.com/v2/key/extra")).toBeNull()
		expect(parseAlchemyUrl("https://eth-mainnet.g.alchemy.com/")).toBeNull()
	})

	it("rejects garbage input", () => {
		expect(parseAlchemyUrl("not a url")).toBeNull()
		expect(parseAlchemyUrl("")).toBeNull()
	})
})

describe("isAlchemyUrl", () => {
	it("detects alchemy URLs", () => {
		expect(isAlchemyUrl("https://arb-mainnet.g.alchemy.com/v2/key")).toBe(true)
		expect(isAlchemyUrl("https://rpc.ankr.com/eth")).toBe(false)
	})
})

describe("deriveAlchemyRpc", () => {
	it("derives URLs for every mainnet chain with a subdomain", () => {
		expect(deriveAlchemyRpc("key", 1)).toBe("https://eth-mainnet.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 42161)).toBe("https://arb-mainnet.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 8453)).toBe("https://base-mainnet.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 137)).toBe("https://polygon-mainnet.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 56)).toBe("https://bnb-mainnet.g.alchemy.com/v2/key")
	})

	it("derives URLs for testnet chains", () => {
		expect(deriveAlchemyRpc("key", 11155111)).toBe("https://eth-sepolia.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 421614)).toBe("https://arb-sepolia.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 84532)).toBe("https://base-sepolia.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 80002)).toBe("https://polygon-amoy.g.alchemy.com/v2/key")
		expect(deriveAlchemyRpc("key", 97)).toBe("https://bnb-testnet.g.alchemy.com/v2/key")
	})

	it("returns null for chains Alchemy does not serve", () => {
		expect(deriveAlchemyRpc("key", 999999)).toBeNull()
	})

	it("round-trips with parseAlchemyUrl", () => {
		const derived = deriveAlchemyRpc("myKey", 8453)!
		expect(parseAlchemyUrl(derived)).toEqual({ subdomain: "base-mainnet", apiKey: "myKey" })
	})
})
