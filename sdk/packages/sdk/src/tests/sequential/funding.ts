import { InsufficientFundsError, formatUnits } from "viem"

/**
 * Live tests spend testnet funds from one shared account. When that account cannot pay, the test is
 * skipped with a GitHub Actions warning naming what to fund, instead of failing as if the code were
 * broken. A missing secret is still a hard error: that is a CI misconfiguration, not an empty wallet.
 */

/** The parts of vitest's test context these helpers use; `TestContext` satisfies it. */
export interface SkipContext {
	task: { name: string }
	skip(note?: string): never
}

/** A balance on one chain: native gas or a token. */
export interface Asset {
	chain: string
	symbol: string
	decimals: number
}

export interface FundingRequirement extends Asset {
	min: bigint
	read: () => Promise<bigint>
}

/**
 * Reads every balance the test is about to spend and skips the test when any is below its minimum.
 * Call it before the test sends its first transaction.
 */
export async function requireFunded(
	ctx: SkipContext,
	account: string,
	requirements: FundingRequirement[],
): Promise<void> {
	const balances = await Promise.all(requirements.map((requirement) => requirement.read()))
	const shortfalls = requirements
		.map((requirement, i) => ({ ...requirement, balance: balances[i] }))
		.filter(({ balance, min }) => balance < min)
	if (shortfalls.length === 0) return

	const messages = shortfalls.map((s) =>
		unfundedMessage(ctx.task.name, account, s, formatUnits(s.balance, s.decimals), formatUnits(s.min, s.decimals)),
	)
	skipUnfunded(ctx, messages)
}

/**
 * Runs `fn`, a step that sends transactions on `native`'s chain, and skips the test when the chain
 * rejects one for insufficient funds. Gas prices and relayer fees drift, so an upfront minimum can
 * still fall short. Any other error is rethrown.
 */
export async function skipIfUnfunded<T>(
	ctx: SkipContext,
	account: string,
	native: Asset,
	fn: () => Promise<T>,
): Promise<T> {
	try {
		return await fn()
	} catch (error) {
		const insufficient = findInsufficientFunds(error)
		if (!insufficient) throw error

		// Nodes report "insufficient funds for gas * price + value: balance <wei>, tx cost <wei>".
		const amounts = /balance (\d+), tx cost (\d+)/.exec(insufficient.details ?? insufficient.message)
		const message = amounts
			? unfundedMessage(
					ctx.task.name,
					account,
					native,
					formatUnits(BigInt(amounts[1]), native.decimals),
					formatUnits(BigInt(amounts[2]), native.decimals),
				)
			: `${ctx.task.name}: ${account} cannot pay for a transaction in ${native.symbol} on ${native.chain} (${insufficient.shortMessage}). Fund it to run this test.`
		skipUnfunded(ctx, [message])
	}
}

/** Follows the `cause` chain, which viem's errors and `new Error(..., { cause })` both use. */
function findInsufficientFunds(error: unknown): InsufficientFundsError | undefined {
	let current: unknown = error
	while (current instanceof Error) {
		if (current instanceof InsufficientFundsError) return current
		current = current.cause
	}
	return undefined
}

function unfundedMessage(test: string, account: string, asset: Asset, balance: string, min: string): string {
	return `${test}: ${account} holds ${balance} ${asset.symbol} on ${asset.chain}, needs at least ${min} ${asset.symbol}. Fund it to run this test.`
}

function skipUnfunded(ctx: SkipContext, messages: string[]): never {
	for (const message of messages) {
		// Written raw: a workflow command must start its line, and log-timestamp prefixes console output.
		process.stdout.write(`\n::warning title=Unfunded live test::${escapeAnnotation(message)}\n`)
		console.warn(message)
	}
	return ctx.skip(messages.join(" "))
}

/** GitHub Actions workflow commands end at a newline, and `%` starts an escape. */
function escapeAnnotation(message: string): string {
	return message.replace(/%/g, "%25").replace(/\r/g, "%0D").replace(/\n/g, "%0A")
}
