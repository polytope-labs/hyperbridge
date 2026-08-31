import { SignerType, type SignerConfig, type Signer } from "./types"
import { mpcVaultSigner } from "./accounts/mpc"
import { privateKeySigner } from "./accounts/privatekey"
import { turnkeySigner } from "./accounts/turnkey"

/**
 * Builds the signer a `[simplex.signer]` TOML block describes.
 *
 * This is the bridge from the binary's config file to the {@link Signer}
 * interface `Simplex.start` takes — call it yourself if you load simplex TOML,
 * or call the backend factory directly if you do not.
 */
export async function createSigner(config: SignerConfig): Promise<Signer> {
	if (config.type === SignerType.PrivateKey) return privateKeySigner(config.key)
	if (config.type === SignerType.MpcVault) return mpcVaultSigner(config)
	if (config.type === SignerType.Turnkey) return turnkeySigner(config)
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

	if (config.type === SignerType.Turnkey) {
		if (!config.organizationId) throw new Error("simplex.signer.organizationId is required")
		if (!config.apiPublicKey) throw new Error("simplex.signer.apiPublicKey is required")
		if (!config.apiPrivateKey) throw new Error("simplex.signer.apiPrivateKey is required")
		if (!config.signWith) throw new Error("simplex.signer.signWith is required")
		return
	}

	throw new Error(`Unsupported signer mode: ${(config as { type?: string }).type ?? "unknown"}`)
}

/**
 * Validates a `[simplex.signer]` block and builds it. Returns undefined when
 * there is no block — a watch-only solver signs nothing.
 */
export async function signerFromToml(config?: SignerConfig): Promise<Signer | undefined> {
	if (!config) return undefined
	validateSignerConfig(config)
	return createSigner(config)
}
