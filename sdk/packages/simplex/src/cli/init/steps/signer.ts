import { select } from "@clack/prompts"
import type { HexString } from "@hyperbridge/sdk"
import { SignerType, validateSignerConfig, type SignerConfig } from "@/services/wallet"
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
			(value) => (/^(0x)?[0-9a-fA-F]{64}$/.test(value) ? undefined : "Expected 64 hex characters (0x prefix optional)"),
		)
		state.signer = { type: SignerType.PrivateKey, key: (key.startsWith("0x") ? key : `0x${key}`) as HexString }
	} else if (type === SignerType.MpcVault) {
		const prev = existing?.type === SignerType.MpcVault ? existing : undefined
		state.signer = {
			type: SignerType.MpcVault,
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
	} else {
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
	}

	validateSignerConfig(state.signer as SignerConfig)
}
