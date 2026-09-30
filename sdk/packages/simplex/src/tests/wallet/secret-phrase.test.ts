import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { recoverAddress, recoverTransactionAddress, recoverTypedDataAddress } from "viem"
import { hashAuthorization } from "viem/utils"
import type { HexString } from "@hyperbridge/sdk"
import { assertDerivationIndex } from "@/services/wallet/accounts/secretphrase"
import {
	canDerive,
	createSigner,
	privateKeySigner,
	secretPhraseSigner,
	signerFromToml,
	SignerType,
	validateSecretPhrase,
	validateSignerConfig,
	type Signer,
	type SignerConfig,
	type TypedDataPayload,
} from "@/services/wallet"

const PHRASE = "test test test test test test test test test test test junk"
const ADDRESSES = [
	"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
	"0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
	"0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
]

const BAD_CHECKSUM = "legal winner thank year wave sausage worth useful legal winner thank year"

const KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString
const DELEGATE = "0x0000000000000000000000000000000000000001" as HexString

const TYPED_DATA = {
	types: {
		EIP712Domain: [
			{ name: "name", type: "string" },
			{ name: "version", type: "string" },
			{ name: "chainId", type: "uint256" },
		],
		Bid: [{ name: "amount", type: "uint256" }],
	},
	primaryType: "Bid",
	domain: { name: "Simplex", version: "1", chainId: 1 },
	message: { amount: 1n },
} satisfies TypedDataPayload

async function assertSignsForItsAddress(signer: Signer) {
	const typed = await signer.signTypedData(TYPED_DATA)
	const typedSigner = await recoverTypedDataAddress({ ...TYPED_DATA, signature: typed } as never)
	expect(typedSigner).toBe(signer.address)

	const { r, s, yParity } = await signer.signAuthorization({ chainId: 1, contractAddress: DELEGATE, nonce: 7 })
	const authSigner = await recoverAddress({
		hash: hashAuthorization({ address: DELEGATE, chainId: 1, nonce: 7 }),
		signature: { r, s, v: BigInt(yParity) + 27n },
	})
	expect(authSigner).toBe(signer.address)

	const serialized = await signer.signTransaction({
		chainId: 1,
		type: "eip1559",
		to: signer.address,
		value: 1_000n,
		gas: 21_000n,
		maxFeePerGas: 1_000_000_000n,
		maxPriorityFeePerGas: 1n,
		nonce: 0,
	})
	const txSigner = await recoverTransactionAddress({ serializedTransaction: serialized as never })
	expect(txSigner).toBe(signer.address)
}

function errorFrom(fn: () => unknown): Error {
	try {
		fn()
	} catch (error) {
		return error as Error
	}
	throw new Error("expected a throw")
}

function expectNoWordOf(phrase: string, message: string) {
	const text = message.toLowerCase()
	for (const word of new Set(phrase.toLowerCase().split(/\s+/))) {
		expect(text).not.toContain(word)
	}
}

describe("secretPhraseSigner", () => {
	it("derives the standard wallets at m/44'/60'/0'/0/i", () => {
		const signer = secretPhraseSigner({ phrase: PHRASE })
		expect(signer.mode).toBe("secretPhrase")
		expect(signer.accountIndex).toBe(0)
		expect(signer.address).toBe(ADDRESSES[0])
		expect(signer.derive(1).address).toBe(ADDRESSES[1])
		expect(signer.derive(2).address).toBe(ADDRESSES[2])
	})

	it("uses accountIndex to pick the hot wallet", () => {
		const signer = secretPhraseSigner({ phrase: PHRASE, accountIndex: 1 })
		expect(signer.accountIndex).toBe(1)
		expect(signer.address).toBe(ADDRESSES[1])
	})

	it("derives by absolute index whatever the hot wallet's index is", () => {
		for (const accountIndex of [0, 1, 2]) {
			const signer = secretPhraseSigner({ phrase: PHRASE, accountIndex })
			ADDRESSES.forEach((address, index) => expect(signer.derive(index).address).toBe(address))
		}
	})

	it("loads a phrase with extra whitespace and mixed case to the same wallet", () => {
		const untidy = `  ${PHRASE.toUpperCase().split(" ").join("  \t")} \n`
		expect(() => validateSecretPhrase(untidy)).not.toThrow()
		expect(secretPhraseSigner({ phrase: untidy }).address).toBe(ADDRESSES[0])
	})

	it("rejects negative, fractional and hardened-range derive indices", () => {
		const signer = secretPhraseSigner({ phrase: PHRASE })
		for (const index of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, 2 ** 31]) {
			expect(() => signer.derive(index)).toThrow(/must be an integer between 0 and/)
		}
		expect(() => secretPhraseSigner({ phrase: PHRASE, accountIndex: -1 })).toThrow(/accountIndex must be/)
		expect(() => secretPhraseSigner({ phrase: PHRASE, accountIndex: 0.5 })).toThrow(/accountIndex must be/)
	})

	it("rejects an index that is not a number without quoting it", () => {
		const signer = secretPhraseSigner({ phrase: PHRASE })
		const pasted = "zebra walnut" as unknown as number

		for (const error of [
			errorFrom(() => assertDerivationIndex(pasted)),
			errorFrom(() => signer.derive(pasted)),
			errorFrom(() => secretPhraseSigner({ phrase: PHRASE, accountIndex: pasted })),
		]) {
			expect(error.message).toMatch(/must be an integer between 0 and 2147483647$/)
			expectNoWordOf("zebra walnut", error.message)
		}
	})

	it("does not expose the phrase on the signer", () => {
		const signer = secretPhraseSigner({ phrase: PHRASE })
		expect(JSON.stringify(signer)).not.toContain("junk")
		expect(Object.values(signer)).not.toContain(PHRASE)
	})

	it("signs every operation for the hot wallet and for derived wallets", async () => {
		const signer = secretPhraseSigner({ phrase: PHRASE })
		await assertSignsForItsAddress(signer)

		const derived = signer.derive(1)
		expect(derived.address).toBe(ADDRESSES[1])
		await assertSignsForItsAddress(derived)
	})
})

describe("validateSecretPhrase", () => {
	it("accepts a valid phrase", () => {
		expect(() => validateSecretPhrase(PHRASE)).not.toThrow()
	})

	it("rejects a bad checksum without quoting the phrase", () => {
		const error = errorFrom(() => validateSecretPhrase(BAD_CHECKSUM))
		expect(error.message).toMatch(/checksum/)
		expectNoWordOf(BAD_CHECKSUM, error.message)

		const fromSigner = errorFrom(() => secretPhraseSigner({ phrase: BAD_CHECKSUM }))
		expectNoWordOf(BAD_CHECKSUM, fromSigner.message)
	})

	it("rejects a word outside the wordlist without quoting it", () => {
		const phrase = PHRASE.replace("junk", "hyperbridge")
		const error = errorFrom(() => validateSecretPhrase(phrase))
		expect(error.message).toMatch(/wordlist/)
		expectNoWordOf(phrase, error.message)
	})

	it("rejects a wrong word count", () => {
		const eleven = PHRASE.split(" ").slice(1).join(" ")
		const thirteen = `${PHRASE} junk`
		for (const phrase of [eleven, thirteen, "", "   "]) {
			const error = errorFrom(() => validateSecretPhrase(phrase))
			expect(error.message).toMatch(/must have 12, 15, 18, 21, 24 words/)
			expectNoWordOf(phrase.trim() || "junk", error.message)
		}
	})
})

describe("canDerive", () => {
	it("is true for a phrase signer and false for a private key signer", () => {
		expect(canDerive(secretPhraseSigner({ phrase: PHRASE }))).toBe(true)
		expect(canDerive(privateKeySigner(KEY))).toBe(false)
		expect(canDerive(secretPhraseSigner({ phrase: PHRASE }).derive(1))).toBe(false)
	})
})

describe("secretPhrase signer config", () => {
	it("createSigner builds a deriving signer", async () => {
		const signer = await createSigner({ type: SignerType.SecretPhrase, phrase: PHRASE, accountIndex: 1 })
		expect(signer.address).toBe(ADDRESSES[1])
		expect(canDerive(signer)).toBe(true)
	})

	it("signerFromToml loads a [simplex.signer] secretPhrase table", async () => {
		const file = parse(`
[simplex.signer]
type = "secretPhrase"
phrase = "${PHRASE}"
accountIndex = 1
`) as { simplex: { signer: SignerConfig } }

		const signer = await signerFromToml(file.simplex.signer)
		expect(signer?.address).toBe(ADDRESSES[1])
		expect(signer && canDerive(signer) && signer.derive(0).address).toBe(ADDRESSES[0])
	})

	it("signerFromToml defaults accountIndex to 0", async () => {
		const file = parse(`
[simplex.signer]
type = "secretPhrase"
phrase = "${PHRASE}"
`) as { simplex: { signer: SignerConfig } }

		expect((await signerFromToml(file.simplex.signer))?.address).toBe(ADDRESSES[0])
	})

	it("validateSignerConfig rejects a missing or invalid phrase and a bad accountIndex", () => {
		const type = SignerType.SecretPhrase
		expect(() => validateSignerConfig({ type, phrase: "" })).toThrow("simplex.signer.phrase is required")
		expect(() => validateSignerConfig({ type, phrase: BAD_CHECKSUM })).toThrow(/checksum/)
		expect(() => validateSignerConfig({ type, phrase: PHRASE, accountIndex: -1 })).toThrow(
			/simplex.signer.accountIndex must be/,
		)
		expect(() => validateSignerConfig({ type, phrase: PHRASE, accountIndex: 3 })).not.toThrow()
	})

	it("validateSignerConfig does not quote an accountIndex that is not a number", () => {
		const accountIndex = "zebra walnut" as unknown as number
		const error = errorFrom(() =>
			validateSignerConfig({ type: SignerType.SecretPhrase, phrase: PHRASE, accountIndex }),
		)
		expect(error.message).toBe("simplex.signer.accountIndex must be an integer between 0 and 2147483647")
	})
})
