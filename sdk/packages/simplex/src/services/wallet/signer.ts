import { SignerType, type SignerConfig, type SigningAccount } from "./types"
import { createMpcVaultSigningAccount } from "./accounts/mpc"
import { createPrivateKeySigningAccount } from "./accounts/privatekey"

export function createSimplexSigner(config: SignerConfig): SigningAccount {
	if (config.type === SignerType.PrivateKey) {
		return createPrivateKeySigningAccount(config.key)
	}

	if (config.type === SignerType.MpcVault) {
		return createMpcVaultSigningAccount(config)
	}

	throw new Error(`Unsupported signer mode: ${(config as { type?: string }).type ?? "unknown"}`)
}

export function validateSignerConfig(config: SignerConfig): void {
	if (config.type === SignerType.PrivateKey) {
		if (!config.key) {
			throw new Error("simplex.signer.key is required when simplex.signer.type=privateKey")
		}
		return
	}

	if (config.type === SignerType.MpcVault) {
		if (!config.apiToken) throw new Error("simplex.signer.apiToken is required")
		if (!config.vaultUuid) throw new Error("simplex.signer.vaultUuid is required")
		if (!config.accountAddress) throw new Error("simplex.signer.accountAddress is required")
		if (!config.callbackClientSignerPublicKey) {
			throw new Error("simplex.signer.callbackClientSignerPublicKey is required")
		}
		return
	}

	throw new Error(`Unsupported signer mode: ${(config as { type?: string }).type ?? "unknown"}`)
}

export function initializeSignerFromToml(signerTomlConfig?: SignerConfig): SigningAccount | undefined {
	if (!signerTomlConfig) return undefined
	validateSignerConfig(signerTomlConfig)
	return createSimplexSigner(signerTomlConfig)
}
