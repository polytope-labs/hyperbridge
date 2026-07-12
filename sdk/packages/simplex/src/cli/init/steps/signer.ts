import { password, select, text } from "@clack/prompts"
import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { SignerType, validateSignerConfig, type SignerConfig } from "@/services/wallet"
import { guard, why } from "../prompt-utils"
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
			"EVM private key (0x-prefixed, 64 hex chars)",
			existing?.type === SignerType.PrivateKey ? existing.key : undefined,
			(value) => (/^0x[0-9a-fA-F]{64}$/.test(value) ? undefined : "Expected 0x followed by 64 hex characters"),
		)
		state.signer = { type: SignerType.PrivateKey, key: key as HexString }
	} else if (type === SignerType.MpcVault) {
		const prev = existing?.type === SignerType.MpcVault ? existing : undefined
		const apiToken = await askSecret("MPCVault API token", prev?.apiToken)
		const vaultUuid = guard(
			await text({ message: "Vault UUID", initialValue: prev?.vaultUuid, validate: required("Vault UUID") }),
		)
		const accountAddress = guard(
			await text({
				message: "Wallet address in the vault (0x...)",
				initialValue: prev?.accountAddress,
				validate: (value) => (isAddress((value ?? "").trim()) ? undefined : "Enter a valid EVM address"),
			}),
		)
		const callbackClientSignerPublicKey = guard(
			await text({
				message: "Callback client-signer public key (ssh-ed25519 ...)",
				initialValue: prev?.callbackClientSignerPublicKey,
				validate: required("Public key"),
			}),
		)
		state.signer = {
			type: SignerType.MpcVault,
			apiToken,
			vaultUuid: vaultUuid.trim(),
			accountAddress: accountAddress.trim() as HexString,
			callbackClientSignerPublicKey: callbackClientSignerPublicKey.trim(),
		}
	} else {
		const prev = existing?.type === SignerType.Turnkey ? existing : undefined
		const organizationId = guard(
			await text({
				message: "Turnkey organization ID",
				initialValue: prev?.organizationId,
				validate: required("Organization ID"),
			}),
		)
		const apiPublicKey = guard(
			await text({
				message: "Turnkey API public key",
				initialValue: prev?.apiPublicKey,
				validate: required("API public key"),
			}),
		)
		const apiPrivateKey = await askSecret("Turnkey API private key", prev?.apiPrivateKey)
		const signWith = guard(
			await text({
				message: "Wallet address to sign with (0x...)",
				initialValue: prev?.signWith,
				validate: (value) => (isAddress((value ?? "").trim()) ? undefined : "Enter a valid EVM address"),
			}),
		)
		state.signer = {
			type: SignerType.Turnkey,
			organizationId: organizationId.trim(),
			apiPublicKey: apiPublicKey.trim(),
			apiPrivateKey,
			signWith: signWith.trim(),
		}
	}

	validateSignerConfig(state.signer as SignerConfig)
}

function required(label: string) {
	return (value: string | undefined) => ((value ?? "").trim() ? undefined : `${label} is required`)
}

/** Masked input; when a previous value exists, pressing Enter keeps it. */
export async function askSecret(
	message: string,
	previous?: string,
	validate?: (value: string) => string | undefined,
): Promise<string> {
	for (;;) {
		const input = guard(
			await password({
				message: previous ? `${message} (press Enter to keep the current value)` : message,
				validate: (value) => {
					const trimmed = (value ?? "").trim()
					if (!trimmed) return previous ? undefined : "This value is required"
					return validate?.(trimmed)
				},
			}),
		)
		const trimmed = (input ?? "").trim()
		if (trimmed) return trimmed
		if (previous) return previous
	}
}
