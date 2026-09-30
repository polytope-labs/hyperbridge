import { validateMnemonic } from "@scure/bip39"
import { english, mnemonicToAccount } from "viem/accounts"
import type { DerivingSigner, SecretPhraseSignerConfig, Signer } from "../types"
import { viemSigner } from "./viem"

const WORD_COUNTS = [12, 15, 18, 21, 24]

// BIP-32 reserves indices from 2^31 up for hardened children.
const MAX_DERIVATION_INDEX = 2 ** 31 - 1

export function normaliseSecretPhrase(phrase: string): string {
	return phrase.trim().split(/\s+/).join(" ").toLowerCase()
}

/**
 * Throws unless the phrase is a BIP-39 mnemonic from the English wordlist with a
 * matching checksum. The errors name what is wrong and never quote the phrase.
 */
export function validateSecretPhrase(phrase: string): void {
	if (typeof phrase !== "string") throw new Error("Secret phrase must be a string")

	const normalised = normaliseSecretPhrase(phrase)
	const words = normalised === "" ? [] : normalised.split(" ")
	if (!WORD_COUNTS.includes(words.length)) {
		throw new Error(`Secret phrase must have ${WORD_COUNTS.join(", ")} words; got ${words.length}`)
	}
	if (words.some((word) => !english.includes(word))) {
		throw new Error("Secret phrase contains a word that is not in the BIP-39 English wordlist")
	}
	if (!validateMnemonic(normalised, english)) {
		throw new Error("Secret phrase checksum does not match; check the words and their order")
	}
}

export function assertDerivationIndex(index: number, name = "Derivation index"): void {
	const expected = `${name} must be an integer between 0 and ${MAX_DERIVATION_INDEX}`
	// A config file or request body can carry any type here, and a phrase pasted
	// into the index field must not come back in the error.
	if (typeof index !== "number") throw new Error(expected)
	if (!Number.isInteger(index) || index < 0 || index > MAX_DERIVATION_INDEX) {
		throw new Error(`${expected}; got ${index}`)
	}
}

/**
 * Signs with wallets derived from a BIP-39 phrase held in this process. The
 * hot wallet is `m/44'/60'/0'/0/accountIndex`, and `derive` returns the signer
 * for any sibling under the same parent.
 */
export function secretPhraseSigner(config: SecretPhraseSignerConfig): DerivingSigner {
	validateSecretPhrase(config.phrase)
	const phrase = normaliseSecretPhrase(config.phrase)
	const accountIndex = config.accountIndex ?? 0
	assertDerivationIndex(accountIndex, "accountIndex")

	const derive = (index: number): Signer => {
		assertDerivationIndex(index)
		// The wallet index is the last path component, which viem calls
		// `addressIndex`. viem's own `accountIndex` is the third component.
		return { ...viemSigner(mnemonicToAccount(phrase, { addressIndex: index })), mode: "secretPhrase" }
	}

	return { ...derive(accountIndex), accountIndex, derive }
}

export function canDerive(signer: Signer): signer is DerivingSigner {
	const candidate = signer as Partial<DerivingSigner>
	return typeof candidate.derive === "function" && typeof candidate.accountIndex === "number"
}
