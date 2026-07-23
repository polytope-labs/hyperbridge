import { confirm, select } from "@clack/prompts"
import { guard } from "./prompt-utils"

/**
 * One provider API key can serve every chain (Alchemy RPCs, Pimlico bundlers).
 * Tracks the key detected in the first manually entered URL and, once the user
 * confirms, supplies derived URLs for the remaining chains.
 */
export class ProviderDerivation<K> {
	private key?: K
	private asked = false
	private confirmed = false

	constructor(
		private options: {
			detect(url: string): K | null
			derive(key: K, chainId: number): string | null
			/** e.g. "Alchemy key detected — derive RPC URLs for {chains} from the same key?" */
			confirmMessage(remaining: string): string
		},
	) {}

	/** Call after each manually entered URL; asks at most once, when a key is detected. */
	async offer(url: string, remainingLabels: string[]): Promise<void> {
		if (this.asked || remainingLabels.length === 0) return
		const detected = this.options.detect(url)
		if (!detected) return
		this.asked = true
		this.confirmed = guard(
			await confirm({ message: this.options.confirmMessage(remainingLabels.join(", ")), initialValue: true }),
		)
		this.key = detected
	}

	/** The derived URL for a chain, when the user accepted derivation. */
	candidate(chainId: number): string | null {
		if (!this.confirmed || this.key === undefined) return null
		return this.options.derive(this.key, chainId)
	}
}

/** Offers a derived provider URL with a manual-entry escape hatch. */
export async function askDerivedOrCustom(
	message: string,
	derived: string,
	derivedLabel: string,
	askCustom: () => Promise<string>,
): Promise<string> {
	const choice = guard(
		await select({
			message,
			options: [
				{ value: "derived", label: derivedLabel, hint: derived },
				{ value: "custom", label: "Enter a different URL" },
			],
		}),
	)
	return choice === "derived" ? derived : askCustom()
}
