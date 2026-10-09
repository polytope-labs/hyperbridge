import { describe, expect, it } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import { validateConfig } from "@/config/filler-toml"
import { DEFAULT_ORDERBOOK_URLS } from "@/config/defaults"
import type { SetupDefaults } from "../types"
import {
	accountIndexFormatError,
	assembleConfig,
	initialState,
	secretPhraseCredentials,
	secretPhraseFormatError,
	switchSignerType,
	type WizardState,
} from "./state"
import { signerRequirements } from "./Wizard"

describe("wizard secret phrase signer", () => {
	const PHRASE = "test test test test test test test test test test test junk"
	const PRIVATE_KEY = `0x${"11".repeat(32)}`

	const defaults = {
		chains: chainsForNetwork("mainnet"),
		hyperbridgeWs: { mainnet: "wss://nexus.rpc.polytope.technology" },
		usdStables: ["USDC", "USDT"],
		maxConcurrentOrders: 5,
		configPath: "/tmp/filler-config.toml",
		knownTokens: {
			"EVM-8453": [
				{ symbol: "USDC", address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" },
				{ symbol: "cNGN", address: "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" },
			],
		},
		knownVaults: {},
	} as unknown as SetupDefaults

	function wizardWithPhrase(signerAccountIndex = ""): WizardState {
		const state = initialState(defaults)
		return {
			...state,
			signerType: "secretPhrase",
			signerPhrase: `  ${PHRASE} `,
			signerAccountIndex,
			substrateKey: `0x${"22".repeat(32)}`,
			orderbook: {
				url: DEFAULT_ORDERBOOK_URLS.mainnet,
				books: [{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" }],
			},
			chains: state.chains.map((chain) => (chain.meta.chainId === 8453 ? { ...chain, enabled: true } : chain)),
		}
	}

	it("writes a secret phrase signer the config check accepts, leaving out the default index", () => {
		for (const index of ["", "0"]) {
			const config = assembleConfig(wizardWithPhrase(index), defaults)
			expect(config.simplex.signer).toEqual({ type: "secretPhrase", phrase: PHRASE })
			expect(() => validateConfig(config)).not.toThrow()
		}
	})

	it("writes a chosen account index as a number", () => {
		const config = assembleConfig(wizardWithPhrase("7"), defaults)
		expect(config.simplex.signer).toEqual({ type: "secretPhrase", phrase: PHRASE, accountIndex: 7 })
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("asks for the address with the phrase alone, never beside a private key field", () => {
		expect(secretPhraseCredentials(PHRASE, "")).toEqual({ phrase: PHRASE })
		expect(secretPhraseCredentials(PHRASE, "3")).toEqual({ phrase: PHRASE, accountIndex: 3 })
		expect(Object.keys(secretPhraseCredentials(PHRASE, "3"))).not.toContain("privateKey")
	})

	it("drops the secret of the signer being left", () => {
		const verified: WizardState = {
			...wizardWithPhrase("4"),
			signerAddress: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
			signerKeyValidation: "valid",
		}
		const onPrivateKey = switchSignerType(verified, "privateKey")
		expect(onPrivateKey).toMatchObject({
			signerType: "privateKey",
			signerPhrase: "",
			signerAccountIndex: "",
			signerKeyValidation: "empty",
		})
		expect(onPrivateKey.signerAddress).toBeUndefined()

		const backOnPhrase = switchSignerType({ ...onPrivateKey, signerKey: PRIVATE_KEY }, "secretPhrase")
		expect(backOnPhrase.signerKey).toBe("")
		expect(JSON.stringify(assembleConfig(backOnPhrase, defaults))).not.toContain(PRIVATE_KEY)
	})

	it("keeps what was typed when the same signer is chosen again", () => {
		const state = wizardWithPhrase("4")
		expect(switchSignerType(state, "secretPhrase")).toBe(state)
	})

	it("blocks Continue until the server has verified the phrase and returned its address", () => {
		const address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
		const blocked: Array<Partial<WizardState>> = [
			{ signerKeyValidation: "empty" },
			{ signerKeyValidation: "checking" },
			{ signerKeyValidation: "checking", signerAddress: address },
			{ signerKeyValidation: "error", signerKeyValidationMessage: "Could not reach the server." },
			{ signerKeyValidation: "error", signerAddress: address },
			{ signerKeyValidation: "invalid", signerKeyValidationMessage: "Invalid secret phrase." },
			{ signerKeyValidation: "invalid", signerAddress: address },
			{ signerKeyValidation: "valid" },
			{ signerKeyValidation: "valid", signerAddress: "" },
			{ signerKeyValidation: "valid", signerAddress: address, signerPhrase: "" },
			{ signerKeyValidation: "valid", signerAddress: address, signerPhrase: "   " },
			{ signerKeyValidation: "valid", signerAddress: address, signerAccountIndex: "-1" },
		]
		for (const patch of blocked) {
			expect(signerRequirements({ ...wizardWithPhrase(), ...patch })).not.toEqual([])
		}

		expect(
			signerRequirements({ ...wizardWithPhrase("4"), signerKeyValidation: "valid", signerAddress: address }),
		).toEqual([])
	})

	it("checks the word count and the account index before asking the server", () => {
		expect(secretPhraseFormatError("")).toBe("Enter the secret phrase.")
		expect(secretPhraseFormatError("test test test")).toBe(
			"Secret phrase must have 12, 15, 18, 21 or 24 words; got 3.",
		)
		expect(secretPhraseFormatError(PHRASE)).toBeUndefined()
		expect(accountIndexFormatError("")).toBeUndefined()
		expect(accountIndexFormatError("12")).toBeUndefined()
		for (const index of ["-1", "1.5", "abc", "2147483648"]) {
			expect(accountIndexFormatError(index)).toBe(
				"Account index must be a whole number between 0 and 2147483647.",
			)
		}
	})
})
