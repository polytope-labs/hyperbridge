import { beforeEach, describe, expect, it, vi } from "vitest"

const prompts = vi.hoisted(() => ({
	select: vi.fn(),
	password: vi.fn(),
	text: vi.fn(),
	cancel: vi.fn(),
	isCancel: vi.fn(() => false),
	log: { message: vi.fn() },
}))

vi.mock("@clack/prompts", () => prompts)

import { stepSigner } from "@/cli/init/steps/signer"
import { newWizardState, type Prefill } from "@/cli/init/state"
import { SignerType, type SignerConfig } from "@/services/wallet"

const TEST_PHRASE = "test test test test test test test test test test test junk"

// A valid phrase whose words appear in no prompt or help text, so finding one means the phrase leaked.
const DISTINCT_PHRASE = "zebra walnut giraffe umbrella quantum lizard oyster pumpkin volcano kangaroo jaguar tomato"

type Validator = (value: string) => string | undefined

function expectNoPhraseWords(text: string, phrase: string) {
	const lower = text.toLowerCase()
	for (const word of new Set(phrase.split(" "))) {
		expect(lower).not.toContain(word)
	}
}

function prefillWith(signer: SignerConfig): Prefill {
	return {
		config: {
			simplex: { signer, substratePrivateKey: "seed", hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology" },
			chains: [],
		},
		chainIds: [],
	}
}

function promptedMessages(): string[] {
	return [...prompts.password.mock.calls, ...prompts.text.mock.calls].map(([options]) => options.message)
}

describe("CLI wizard signer step, secret phrase", () => {
	beforeEach(() => {
		prompts.select.mockReset().mockResolvedValue(SignerType.SecretPhrase)
		prompts.password.mockReset()
		prompts.text.mockReset()
		prompts.log.message.mockReset()
	})

	it("offers the secret phrase beside the other signers", async () => {
		prompts.password.mockResolvedValue(TEST_PHRASE)
		prompts.text.mockResolvedValue("")
		await stepSigner(newWizardState())

		const options = prompts.select.mock.calls[0][0].options as Array<{ value: SignerType }>
		expect(options.map((option) => option.value)).toEqual([
			SignerType.PrivateKey,
			SignerType.SecretPhrase,
			SignerType.MpcVault,
			SignerType.Turnkey,
		])
	})

	it("stores the normalised phrase, omits accountIndex, and asks nothing Turnkey needs", async () => {
		prompts.password.mockResolvedValue("  Test test test test  test test test test test test test JUNK ")
		prompts.text.mockResolvedValue("")
		const state = newWizardState()
		await stepSigner(state)

		expect(state.signer).toEqual({ type: SignerType.SecretPhrase, phrase: TEST_PHRASE })
		expect(promptedMessages()).toEqual([
			"Secret phrase (12 to 24 words, separated by spaces)",
			"Account index under the phrase (empty for 0)",
		])
	})

	it("stores the account index when one is given", async () => {
		prompts.password.mockResolvedValue(TEST_PHRASE)
		prompts.text.mockResolvedValue("4")
		const state = newWizardState()
		await stepSigner(state)

		expect(state.signer).toEqual({ type: SignerType.SecretPhrase, phrase: TEST_PHRASE, accountIndex: 4 })
	})

	it("rejects an invalid phrase without repeating any of it", async () => {
		prompts.password.mockResolvedValue(TEST_PHRASE)
		prompts.text.mockResolvedValue("")
		await stepSigner(newWizardState())
		const validate = prompts.password.mock.calls[0][0].validate as Validator

		expect(validate(TEST_PHRASE)).toBeUndefined()
		expect(validate("test test test")).toMatch(/words/)
		expect(validate(TEST_PHRASE.replace("junk", "zzzzzz"))).toMatch(/wordlist/)
		expect(validate(TEST_PHRASE.replace("junk", "test"))).toMatch(/checksum/)
		expect(validate(TEST_PHRASE.replace("junk", "zzzzzz"))).not.toContain("zzzzzz")
	})

	it("accepts only a non-negative integer account index", async () => {
		prompts.password.mockResolvedValue(TEST_PHRASE)
		prompts.text.mockResolvedValue("")
		await stepSigner(newWizardState())
		const validate = prompts.text.mock.calls[0][0].validate as Validator

		expect(validate("")).toBeUndefined()
		expect(validate("0")).toBeUndefined()
		expect(validate("7")).toBeUndefined()
		for (const rejected of ["-1", "1.5", "1e3", "abc", "2147483648"]) {
			expect(validate(rejected)).toEqual(expect.any(String))
		}
	})

	it("keeps an existing secret phrase signer when the operator presses Enter through the prompts", async () => {
		const existing: SignerConfig = { type: SignerType.SecretPhrase, phrase: TEST_PHRASE, accountIndex: 3 }
		prompts.password.mockResolvedValue("")
		prompts.text.mockImplementation(async (options: { initialValue?: string }) => options.initialValue)
		const state = newWizardState()
		await stepSigner(state, prefillWith(existing))

		expect(prompts.select.mock.calls[0][0].initialValue).toBe(SignerType.SecretPhrase)
		expect(state.signer).toEqual(existing)
	})

	it("keeps the previous phrase on an update run without putting it in any prompt", async () => {
		const existing: SignerConfig = { type: SignerType.SecretPhrase, phrase: DISTINCT_PHRASE, accountIndex: 3 }
		prompts.password.mockResolvedValue("")
		prompts.text.mockImplementation(async (options: { initialValue?: string }) => options.initialValue)
		const state = newWizardState()
		await stepSigner(state, prefillWith(existing))

		expect(state.signer).toEqual(existing)

		const calls = [...prompts.select.mock.calls, ...prompts.password.mock.calls, ...prompts.text.mock.calls]
		expect(prompts.password).toHaveBeenCalledTimes(1)
		expect(prompts.text).toHaveBeenCalledTimes(1)
		for (const [options] of calls) {
			for (const field of ["message", "placeholder", "initialValue", "defaultValue"]) {
				expectNoPhraseWords(String(options[field] ?? ""), DISTINCT_PHRASE)
			}
			expectNoPhraseWords(JSON.stringify(options), DISTINCT_PHRASE)
		}
		expectNoPhraseWords(JSON.stringify(prompts.log.message.mock.calls), DISTINCT_PHRASE)
	})

	it("never writes the phrase to the terminal", async () => {
		prompts.password.mockResolvedValue(TEST_PHRASE)
		prompts.text.mockResolvedValue("")
		await stepSigner(newWizardState())

		const shown = JSON.stringify([
			prompts.log.message.mock.calls,
			prompts.password.mock.calls,
			prompts.text.mock.calls,
		])
		expect(shown).not.toContain("junk")
	})
})
