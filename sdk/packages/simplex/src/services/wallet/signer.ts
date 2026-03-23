import { SignerType, type SignerConfig, type SigningAccount } from "./types"
import { createMpcVaultSigningAccount } from "./accounts/mpc"
import { createPrivateKeySigningAccount } from "./accounts/privatekey"

export function createSimplexSigner(config: SignerConfig): SigningAccount {
	if (config.type === SignerType.PrivateKey) {
		return createPrivateKeySigningAccount(config.privateKey)
	}

	if (config.type === SignerType.MpcVault) {
		return createMpcVaultSigningAccount(config.mpcVault)
	}

	throw new Error(`Unsupported signer mode: ${(config as { type?: string }).type ?? "unknown"}`)
}

export function validateSignerConfig(config: SignerConfig): void {
	if (config.type === SignerType.PrivateKey) {
		if (!config.privateKey) {
			throw new Error("simplex.signer.privateKey is required when simplex.signer.type=privateKey")
		}
		return
	}

	if (config.type === SignerType.MpcVault) {
		const mpcVault = config.mpcVault
		if (!mpcVault?.apiToken) throw new Error("simplex.signer.mpcVault.apiToken is required")
		if (!mpcVault?.vaultUuid) throw new Error("simplex.signer.mpcVault.vaultUuid is required")
		if (!mpcVault?.accountAddress) throw new Error("simplex.signer.mpcVault.accountAddress is required")
		if (!mpcVault?.callbackClientSignerPublicKey) {
			throw new Error("simplex.signer.mpcVault.callbackClientSignerPublicKey is required")
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
