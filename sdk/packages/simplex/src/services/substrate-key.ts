import type { Keyring } from "@polkadot/api"

type KeyringPair = ReturnType<Keyring["addFromUri"]>

/**
 * Derives an sr25519 keypair from any of the accepted substratePrivateKey forms:
 * a `//Uri` derivation path, a mnemonic phrase, or a hex seed (with or without 0x).
 * Mirrors IntentsCoprocessor.getKeyPair so every consumer resolves the same address.
 */
export async function deriveSubstrateKeyPair(key: string): Promise<KeyringPair> {
	const { Keyring } = await import("@polkadot/api")
	const { cryptoWaitReady } = await import("@polkadot/util-crypto")
	await cryptoWaitReady()

	const keyring = new Keyring({ type: "sr25519" })
	if (key.startsWith("//")) {
		return keyring.addFromUri(key)
	}
	if (key.includes(" ")) {
		return keyring.addFromMnemonic(key)
	}
	const hex = key.startsWith("0x") ? key.slice(2) : key
	return keyring.addFromSeed(Buffer.from(hex, "hex"))
}

/** Generates a fresh mnemonic + sr25519 keypair (used by the setup wizard). */
export async function generateSubstrateKey(): Promise<{ mnemonic: string; address: string }> {
	const { mnemonicGenerate } = await import("@polkadot/util-crypto")
	const mnemonic = mnemonicGenerate()
	const pair = await deriveSubstrateKeyPair(mnemonic)
	return { mnemonic, address: pair.address }
}
