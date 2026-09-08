import {
	encodePacked,
	formatEther,
	maxUint256,
	erc20Abi,
	BaseError,
	ContractFunctionRevertedError,
	ContractFunctionZeroDataError,
	type PublicClient,
	type WalletClient,
} from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import {
	RECOMMENDED_AMOUNT_USD,
	VERIFICATION_GAS_LIMIT_PERMIT2,
	PERMIT2_DEADLINE_SECONDS,
	POST_OP_GAS_LIMIT_SIMPLEX,
	type FeeTokenBalance,
	type PaymasterResult,
} from "../types"
import { randomPermit2Nonce, signPermit2Transfer } from "../permit2"
import { SIMPLEX_PAYMASTER_ABI } from "@/config/abis/SimplexPaymaster"
import type { Signer } from "@/services/wallet/types"

interface TokenOption {
	symbol: FeeTokenBalance["symbol"]
	address: HexString
	decimals: number
}

/** Gas ceiling for the one-time ERC-20 approve tx (typical approves need ~45-55k). */
const APPROVE_TX_GAS = 60_000n

/**
 * Builds the paymaster fields for a PackedUserOperation using the SimplexPaymaster.
 *
 * Selects the first configured stablecoin (USDC, then USDT) with a balance of at least
 * one token and authorizes it in PERMIT2 mode (0x02): a per-op, single-use Permit2
 * signature, so nothing is exposed to the paymaster at rest. Permit2 nonces are an
 * unordered bitmap, so concurrent ops on one chain each carry their own permit without
 * coordinating on a shared counter — the reason this is the only mode used. A token not
 * yet approved to Permit2 costs one funded bootstrap tx, approve(Permit2, max), for the
 * account's lifetime.
 *
 * Returns the balances it read instead, in selection order, when the solver holds less
 * than one whole unit of every configured token, so the caller can name each shortfall,
 * and throws when Permit2 is unusable (not configured on the chain, or the paymaster
 * deployment does not expose PERMIT2()). The caller decides whether to fall back to the
 * EntryPoint deposit; a standing allowance to the paymaster is never used or created.
 */
export async function buildSimplexPaymasterData(
	client: PublicClient,
	walletClient: WalletClient,
	signer: Pick<Signer, "signTypedData">,
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
): Promise<(PaymasterResult & { token: HexString }) | { insufficient: FeeTokenBalance[] }> {
	const chainId = configService.getChainId(chain)

	const { selected, balances } = await selectToken(client, solverAccount, configuredFeeTokens(chain, configService))
	if (!selected) {
		return { insufficient: balances }
	}

	const { address: tokenAddress, decimals: tokenDecimals } = selected
	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(tokenDecimals)

	const permit2 = configService.getPermit2Address(chain)
	if (!isConfigured(permit2)) {
		throw new Error(
			`SimplexPaymaster cannot sponsor ${tokenAddress} on ${chain}: Permit2 is not configured for this chain`,
		)
	}
	if (!(await paymasterSupportsPermit2(client, chainId, paymasterAddress))) {
		throw new Error(
			`SimplexPaymaster cannot sponsor ${tokenAddress} on ${chain}: the paymaster at ${paymasterAddress} ` +
				`does not expose PERMIT2() (predates PERMIT2 mode or is not deployed)`,
		)
	}

	const permit2Allowance = await readAllowance(client, tokenAddress, solverAccount, permit2)
	if (permit2Allowance < recommended) {
		await sendFundedApprove(client, walletClient, solverAccount, tokenAddress, permit2, maxUint256)
	}

	return {
		...(await buildPermit2Mode(signer, {
			permit2,
			chainId,
			token: tokenAddress,
			amount: recommended,
			spender: paymasterAddress,
			deadlineSeconds: PERMIT2_DEADLINE_SECONDS,
		})),
		token: tokenAddress,
	}
}

/**
 * The fee token a native EIP-7702 delegation can approve to Permit2 in the same transaction,
 * so the account's one-time bootstrap rides the delegation rather than costing a separate
 * native tx. Every fee token needs this approval — sponsorship is Permit2-only — so it is
 * the main way a fresh solver reaches a sponsored op without a second funded tx. Returns
 * null when nothing batchable is pending: any existing non-zero allowance (at the
 * recommendation PERMIT2 mode already works; below it the USDT rule needs a zero-first reset
 * the batched tx cannot carry), no Permit2 configured, a paymaster without PERMIT2 mode, or
 * the solver holding no fee token yet. Deferred cases are handled by the first sponsored
 * op's own funded approve.
 */
export async function resolvePendingPermit2Approval(
	client: PublicClient,
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
): Promise<{ token: HexString; spender: HexString } | null> {
	const permit2 = configService.getPermit2Address(chain)
	if (!isConfigured(permit2)) return null
	if (!(await paymasterSupportsPermit2(client, configService.getChainId(chain), paymasterAddress))) return null

	const { selected } = await selectToken(client, solverAccount, configuredFeeTokens(chain, configService))
	if (!selected) return null

	// Only a clean zero allowance is batchable. At or above the recommendation PERMIT2
	// mode already works; a stale non-zero allowance below it cannot ride the delegation
	// either — the batched tx approves max directly, and tokens with the USDT rule revert
	// a non-zero → non-zero change (deterministically: the delegation tx skips simulation).
	const allowance = await readAllowance(client, selected.address, solverAccount, permit2)
	if (allowance !== 0n) return null

	return { token: selected.address, spender: permit2 }
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Missing assets come back from the config service as the literal "0x". */
function isConfigured(address: HexString): boolean {
	return !!address && address !== "0x" && address !== "0x0000000000000000000000000000000000000000"
}

/** The paymaster's fee tokens on `chain` in selection order, USDC then USDT, skipping unconfigured ones. */
function configuredFeeTokens(chain: string, configService: FillerConfigService): TokenOption[] {
	const tokens: TokenOption[] = []
	const usdc = configService.getUsdcAsset(chain)
	if (isConfigured(usdc)) {
		tokens.push({ symbol: "USDC", address: usdc, decimals: configService.getUsdcDecimals(chain) })
	}
	const usdt = configService.getUsdtAsset(chain)
	if (isConfigured(usdt)) {
		tokens.push({ symbol: "USDT", address: usdt, decimals: configService.getUsdtDecimals(chain) })
	}
	return tokens
}

/**
 * First token the solver holds at least one whole unit of, reading balances in order and
 * stopping at the first hit. `balances` carries every read that fell short, so a caller
 * left with no token can say how short each one was instead of a bare "insufficient".
 */
async function selectToken(
	client: PublicClient,
	solverAccount: HexString,
	tokens: TokenOption[],
): Promise<{ selected: TokenOption | null; balances: FeeTokenBalance[] }> {
	const balances: FeeTokenBalance[] = []
	for (const token of tokens) {
		const balance = (await client.readContract({
			address: token.address,
			abi: erc20Abi,
			functionName: "balanceOf",
			args: [solverAccount],
		})) as bigint

		const required = 10n ** BigInt(token.decimals)
		if (balance >= required) {
			return { selected: token, balances }
		}
		balances.push({ symbol: token.symbol, balance, required })
	}
	return { selected: null, balances }
}

const PERMIT2_SUPPORT_NEGATIVE_TTL_MS = 5 * 60_000
const permit2Support = new Map<string, { supported: boolean; checkedAt: number }>()

/**
 * Only paymaster implementations that expose PERMIT2() accept mode 0x02; older
 * deployments reject it with InvalidMode. Probing keeps the client safe while the
 * redeploy lands chain by chain. Positive results are cached for the process
 * lifetime, negative ones briefly so an upgrade is picked up without a restart.
 */
async function paymasterSupportsPermit2(
	client: PublicClient,
	chainId: number,
	paymasterAddress: HexString,
): Promise<boolean> {
	// Keyed by chain as well as address: one process serves every chain, and a CREATE2
	// redeploy can share an address across them, so an address-only key would let the
	// first chain upgraded mark that address supported everywhere.
	const key = `${chainId}:${paymasterAddress.toLowerCase()}`
	const cached = permit2Support.get(key)
	if (cached && (cached.supported || Date.now() - cached.checkedAt < PERMIT2_SUPPORT_NEGATIVE_TTL_MS)) {
		return cached.supported
	}
	try {
		await client.readContract({ address: paymasterAddress, abi: SIMPLEX_PAYMASTER_ABI, functionName: "PERMIT2" })
		permit2Support.set(key, { supported: true, checkedAt: Date.now() })
		return true
	} catch (error) {
		// Only a real contract revert (an old implementation without PERMIT2()) is evidence
		// of no support. viem wraps EVERY readContract failure — a 429 or timeout included —
		// in ContractFunctionExecutionError, so the thrown type cannot separate a revert from
		// a transport error; only the cause chain can (ContractFunctionRevertedError for a
		// revert, HttpRequestError for transport). A transport error must propagate uncached
		// — caching it would make the builder refuse a healthy paymaster for the TTL.
		const isRevert =
			error instanceof BaseError &&
			error.walk((e) => e instanceof ContractFunctionRevertedError || e instanceof ContractFunctionZeroDataError)
		if (isRevert) {
			permit2Support.set(key, { supported: false, checkedAt: Date.now() })
			return false
		}
		throw error
	}
}

async function readAllowance(
	client: PublicClient,
	token: HexString,
	owner: HexString,
	spender: HexString,
): Promise<bigint> {
	return (await client.readContract({
		address: token,
		abi: erc20Abi,
		functionName: "allowance",
		args: [owner, spender],
	})) as bigint
}

/**
 * Signs a per-op Permit2 transfer permit naming the paymaster as spender and encodes
 * PERMIT2 mode. mode(1) + token(20) + permitAmount(32) + nonce(32) + deadline(32) +
 * signature(65) = 182 bytes, matching SimplexPaymaster._parsePermit2Data.
 */
async function buildPermit2Mode(
	signer: Pick<Signer, "signTypedData">,
	p: {
		permit2: HexString
		chainId: number
		token: HexString
		amount: bigint
		spender: HexString
		deadlineSeconds: bigint
	},
): Promise<PaymasterResult> {
	const nonce = randomPermit2Nonce()
	const deadline = BigInt(Math.floor(Date.now() / 1000)) + p.deadlineSeconds
	const signature = await signPermit2Transfer(signer, {
		permit2: p.permit2,
		chainId: p.chainId,
		token: p.token,
		amount: p.amount,
		spender: p.spender,
		nonce,
		deadline,
	})

	// Split into explicit v, r, s: the packed mode 0x02 layout carries them as separate fields.
	const r = `0x${signature.slice(2, 66)}` as HexString
	const s = `0x${signature.slice(66, 130)}` as HexString
	const v = Number.parseInt(signature.slice(130, 132), 16)

	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
		[2, p.token, p.amount, nonce, deadline, v, r, s],
	) as HexString

	return {
		paymaster: p.spender,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT2,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
	}
}

/**
 * Approves `spender` from the solver EOA — plain native-funded txs, the very thing the
 * paymaster exists to avoid needing. A stale non-zero allowance (e.g. a leftover Permit2
 * approval from another integration) is reset to zero first, because tokens like Ethereum
 * USDT reject a non-zero → non-zero change — so the sequence is up to two txs, each
 * waiting two confirmations. The native balance is pre-checked against the whole sequence,
 * failing an unfunded solver with one actionable line instead of viem's estimateGas chain
 * — or worse, with the reset landed and the re-approve dead, the allowance stuck at zero.
 */
async function sendFundedApprove(
	client: PublicClient,
	walletClient: WalletClient,
	solverAccount: HexString,
	tokenAddress: HexString,
	spender: HexString,
	amount: bigint,
): Promise<void> {
	const [nativeBalance, gasPrice, current] = await Promise.all([
		client.getBalance({ address: solverAccount }),
		client.getGasPrice(),
		readAllowance(client, tokenAddress, solverAccount, spender),
	])
	const needsReset = current !== 0n && amount !== 0n
	const requiredNative = APPROVE_TX_GAS * gasPrice * (needsReset ? 2n : 1n)
	if (nativeBalance < requiredNative) {
		throw new Error(
			`SimplexPaymaster needs a one-time Permit2 approval for the fee token on this chain; ` +
				`send native dust (>= ${formatEther(requiredNative)}) to ${solverAccount}`,
		)
	}

	if (needsReset) {
		await sendApproveTx(client, walletClient, tokenAddress, spender, 0n)
	}
	await sendApproveTx(client, walletClient, tokenAddress, spender, amount)
}

async function sendApproveTx(
	client: PublicClient,
	walletClient: WalletClient,
	tokenAddress: HexString,
	spender: HexString,
	amount: bigint,
): Promise<void> {
	const hash = await walletClient.writeContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "approve",
		args: [spender, amount],
		chain: walletClient.chain,
		account: walletClient.account!,
	})

	// Bundlers simulate on their own nodes; one confirmation after the approve is not
	// always visible there yet, and the very next op would fail validation.
	await client.waitForTransactionReceipt({ hash, confirmations: 2 })
}
