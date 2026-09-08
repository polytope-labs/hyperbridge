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
	VERIFICATION_GAS_LIMIT_PERMIT,
	VERIFICATION_GAS_LIMIT_PERMIT2,
	PERMIT2_DEADLINE_SECONDS,
	POST_OP_GAS_LIMIT_SIMPLEX,
	type FeeTokenBalance,
	type PaymasterResult,
} from "../types"
import { signEip2612Permit } from "../permit"
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
 * coordinating on a shared counter — the reason this is the mode nearly every op uses.
 * A token not yet approved to Permit2 costs one bootstrap approve(Permit2, max) for the
 * account's lifetime, normally funded with native dust.
 *
 * `permitBootstrap` is the exception, set only by `DelegationService`'s first-time
 * delegation op. With no Permit2 allowance in place and a fee token
 * that implements EIP-2612, it signs a permit and packs PERMIT mode (0x00) instead, so a
 * solver holding zero native can pay for the very op that installs its Permit2 approval.
 * A 2612 nonce is a single sequential counter, so this is deliberately confined to the
 * one op per chain that cannot have a concurrent sibling.
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
	permitBootstrap = false,
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
	// Gates the bootstrap too, even though PERMIT mode itself works on an older deployment:
	// that op exists to install a Permit2 allowance, and an allowance the paymaster can never
	// spend is worth nothing. Better to refuse and surface the stale deployment.
	if (!(await paymasterSupportsPermit2(client, chainId, paymasterAddress))) {
		throw new Error(
			`SimplexPaymaster cannot sponsor ${tokenAddress} on ${chain}: the paymaster at ${paymasterAddress} ` +
				`does not expose PERMIT2() (predates PERMIT2 mode or is not deployed)`,
		)
	}

	const permit2Allowance = await readAllowance(client, tokenAddress, solverAccount, permit2)
	if (permit2Allowance < recommended) {
		// The bootstrap op pays for itself with a permit when the token has one, so the
		// approve it carries in its own callData needs no native. Only when that is
		// unavailable does the allowance have to be installed by a funded tx first.
		if (permitBootstrap && (await tokenSupportsPermit(client, tokenAddress))) {
			const pm = await buildPermitMode(
				client,
				signer,
				solverAccount,
				paymasterAddress,
				tokenAddress,
				recommended,
				chainId,
			)
			return { ...pm, token: tokenAddress }
		}
		await ensureFundedApprove(client, walletClient, chainId, solverAccount, tokenAddress, permit2)
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
 * The fee token an EIP-7702 delegation can approve to Permit2 in the same operation, so the
 * account's one-time bootstrap rides the delegation rather than costing a separate native tx.
 * Every fee token needs this approval — every op but the bootstrap authorizes through Permit2
 * — so it is how a fresh solver reaches its first sponsored op.
 *
 * `permitCapable` says whether that operation can be the *sponsored* one: a token with an
 * EIP-2612 permit lets the delegation UserOp pay for itself in PERMIT mode while installing
 * the allowance in its own callData, needing no native at all. Without it the approve can
 * only ride a native type-0x04 tx.
 *
 * Returns null when nothing batchable is pending: any existing non-zero allowance (at the
 * recommendation PERMIT2 mode already works; below it the USDT rule needs a zero-first reset
 * neither carrier can express), no Permit2 configured, a paymaster without PERMIT2 mode, or
 * the solver holding no fee token yet. Deferred cases are handled by the first sponsored
 * op's own funded approve.
 */
export async function resolvePendingPermit2Approval(
	client: PublicClient,
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
): Promise<{ token: HexString; spender: HexString; permitCapable: boolean } | null> {
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

	return {
		token: selected.address,
		spender: permit2,
		permitCapable: await tokenSupportsPermit(client, selected.address),
	}
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
 * Signs an EIP-2612 permit naming the paymaster as spender and encodes PERMIT mode.
 * mode(1) + token(20) + permitAmount(32) + deadline(32) + v(1) + r(32) + s(32) = 150
 * bytes, matching SimplexPaymaster._executePermit. Deadline is maxUint256 because
 * paymasters cannot read block.timestamp under ERC-4337 validation rules.
 *
 * Reachable only through the `permitBootstrap` flag: the sequential 2612 nonce this
 * consumes would serialize concurrent ops, which is harmless for the one-per-chain
 * delegation and unacceptable for fills.
 */
async function buildPermitMode(
	client: PublicClient,
	signer: Pick<Signer, "signTypedData">,
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	permitAmount: bigint,
	chainId: number,
): Promise<PaymasterResult> {
	const permitSignature = await signEip2612Permit(
		client,
		signer,
		solverAccount,
		paymasterAddress,
		tokenAddress,
		permitAmount,
		chainId,
	)

	const r = `0x${permitSignature.slice(2, 66)}` as HexString
	const s = `0x${permitSignature.slice(66, 130)}` as HexString
	const v = Number.parseInt(permitSignature.slice(130, 132), 16)

	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
		[0, tokenAddress, permitAmount, maxUint256, v, r, s],
	) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
	}
}

/**
 * In-flight `approve(spender, max)` per (chain, token, owner). The bid path and the vault /
 * token-send path are scheduled independently, so both can find a missing allowance at once;
 * two concurrent `writeContract` calls would resolve the same pending nonce and one tx would
 * be dropped or replaced. A second caller rides the first call's promise instead.
 */
const approvalsInFlight = new Map<string, Promise<void>>()

/**
 * Deduplicating wrapper around {@link sendFundedApprove}. Rethrows the in-flight call's error
 * to a rider — both callers are blocked on the same missing allowance for the same reason, so
 * they should fail identically rather than pile a second tx onto a failure.
 */
async function ensureFundedApprove(
	client: PublicClient,
	walletClient: WalletClient,
	chainId: number,
	solverAccount: HexString,
	tokenAddress: HexString,
	spender: HexString,
): Promise<void> {
	const key = `${chainId}:${tokenAddress.toLowerCase()}:${solverAccount.toLowerCase()}`
	const inFlight = approvalsInFlight.get(key)
	if (inFlight) return inFlight

	const pending = sendFundedApprove(client, walletClient, solverAccount, tokenAddress, spender, maxUint256).finally(
		() => {
			approvalsInFlight.delete(key)
		},
	)
	approvalsInFlight.set(key, pending)
	return pending
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

/** Probes for EIP-2612 support via the version() getter permit tokens expose. */
async function tokenSupportsPermit(client: PublicClient, tokenAddress: HexString): Promise<boolean> {
	try {
		await client.readContract({
			address: tokenAddress,
			abi: [
				{
					inputs: [],
					name: "version",
					outputs: [{ type: "string" }],
					stateMutability: "view",
					type: "function",
				},
			] as const,
			functionName: "version",
		})
		return true
	} catch (error) {
		// Same discrimination as paymasterSupportsPermit2: only a contract revert /
		// empty return proves the token has no version() — a transport error must
		// propagate, or a 429 would masquerade as "no permit" and strand a solver
		// with no native on the funded-approve path it was trying to avoid.
		const isRevert =
			error instanceof BaseError &&
			error.walk((e) => e instanceof ContractFunctionRevertedError || e instanceof ContractFunctionZeroDataError)
		if (isRevert) return false
		throw error
	}
}
