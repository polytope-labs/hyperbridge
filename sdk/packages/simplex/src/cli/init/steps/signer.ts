import { select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import {
	SignerType,
	normaliseSecretPhrase,
	validateSecretPhrase,
	validateSignerConfig,
	type SignerConfig,
} from "@/services/wallet"
import { assertDerivationIndex } from "@/services/wallet/accounts/secretphrase"
import { guard, why, askText, askAddress, askSecret } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState } from "../state"

export async function stepSigner(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.signer)
	const existing = prefill?.config.simplex.signer

	const type = guard(
		await select<SignerType>({
			message: "How should the filler sign transactions?",
			initialValue: (existing?.type as SignerType) ?? SignerType.PrivateKey,
			options: [
				{
					value: SignerType.PrivateKey,
					label: "Private key",
					hint: "raw key on this machine — simplest, guard the config file",
				},
				{
					value: SignerType.SecretPhrase,
					label: "Secret phrase",
					hint: "BIP-39 phrase on this machine; guard the config file",
				},
				{
					value: SignerType.MpcVault,
					label: "MPCVault",
					hint: "institutional MPC custody; needs a vault + client-signer setup",
				},
				{
					value: SignerType.Turnkey,
					label: "Turnkey",
					hint: "hosted key management; needs a Turnkey org + API keypair",
				},
			],
		}),
	)

	if (type === SignerType.PrivateKey) {
		const key = await askSecret(
			"EVM private key (64 hex chars, 0x prefix optional)",
			existing?.type === SignerType.PrivateKey ? existing.key : undefined,
			(value) =>
				/^(0x)?[0-9a-fA-F]{64}$/.test(value) ? undefined : "Expected 64 hex characters (0x prefix optional)",
		)
		state.signer = { type: SignerType.PrivateKey, key: (key.startsWith("0x") ? key : `0x${key}`) as HexString }
	} else if (type === SignerType.SecretPhrase) {
		why(WHY.secretPhrase)
		const prev = existing?.type === SignerType.SecretPhrase ? existing : undefined
		const phrase = await askSecret("Secret phrase (12 to 24 words, separated by spaces)", prev?.phrase, (value) =>
			errorMessage(() => validateSecretPhrase(value)),
		)
		const accountIndex = await askText("Account index under the phrase (empty for 0)", {
			initial: prev?.accountIndex !== undefined ? String(prev.accountIndex) : undefined,
			required: false,
			validate: (trimmed) =>
				/^\d+$/.test(trimmed)
					? errorMessage(() => assertDerivationIndex(Number(trimmed), "Account index"))
					: "Enter a whole number, 0 or greater",
		})
		state.signer = {
			type: SignerType.SecretPhrase,
			phrase: normaliseSecretPhrase(phrase),
			...(accountIndex ? { accountIndex: Number(accountIndex) } : {}),
		}
	} else if (type === SignerType.MpcVault) {
		const prev = existing?.type === SignerType.MpcVault ? existing : undefined
		const signer = {
			type: SignerType.MpcVault as const,
			apiToken: await askSecret("MPCVault API token", prev?.apiToken),
			vaultUuid: await askText("Vault UUID", { initial: prev?.vaultUuid, required: "Vault UUID is required" }),
			accountAddress: (await askAddress("Wallet address in the vault (0x...)", {
				initial: prev?.accountAddress,
			})) as HexString,
			callbackClientSignerPublicKey: await askText("Callback client-signer public key (ssh-ed25519 ...)", {
				initial: prev?.callbackClientSignerPublicKey,
				required: "Public key is required",
			}),
		}
		const grpcTarget = await askText("gRPC target (empty for api.mpcvault.com:443)", {
			initial: prev?.grpcTarget,
			required: false,
		})
		state.signer = { ...signer, ...(grpcTarget ? { grpcTarget } : {}) }
	} else if (type === SignerType.Turnkey) {
		const prev = existing?.type === SignerType.Turnkey ? existing : undefined
		state.signer = {
			type: SignerType.Turnkey,
			organizationId: await askText("Turnkey organization ID", {
				initial: prev?.organizationId,
				required: "Organization ID is required",
			}),
			apiPublicKey: await askText("Turnkey API public key", {
				initial: prev?.apiPublicKey,
				required: "API public key is required",
			}),
			apiPrivateKey: await askSecret("Turnkey API private key", prev?.apiPrivateKey),
			signWith: await askAddress("Wallet address to sign with (0x...)", { initial: prev?.signWith }),
		}
	} else {
		throw new Error(`Unsupported signer mode: ${String(type)}`)
	}

	validateSignerConfig(state.signer as SignerConfig)
}

function errorMessage(check: () => void): string | undefined {
	try {
		check()
		return undefined
	} catch (error) {
		return error instanceof Error ? error.message : String(error)
	}
}
