import { encodePacked, formatEther, maxUint256, erc20Abi, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import {
	RECOMMENDED_AMOUNT_USD,
	THRESHOLD_USD,
	VERIFICATION_GAS_LIMIT_PERMIT,
	VERIFICATION_GAS_LIMIT_APPROVE,
	VERIFICATION_GAS_LIMIT_PERMIT2,
	PERMIT2_DEADLINE_SECONDS,
	POST_OP_GAS_LIMIT_SIMPLEX,
	type PaymasterResult,
} from "../types"
import { signEip2612Permit } from "../permit"
import { randomPermit2Nonce, signPermit2Transfer } from "../permit2"
import { SIMPLEX_PAYMASTER_ABI } from "@/config/abis/SimplexPaymaster"
import type { Signer } from "@/services/wallet/types"

export interface SimplexPaymasterOptions {
	/** Skip EIP-2612 permit detection (PERMIT2 and APPROVE stay available). */
	skipPermit?: boolean
	permit2DeadlineSeconds?: bigint
}

interface TokenOption {
	address: HexString
	decimals: number
}

/** Gas ceiling for the one-time ERC-20 approve tx (typical approves need ~45-55k). */
const APPROVE_TX_GAS = 60_000n

/**
 * Builds the paymaster fields for a PackedUserOperation using the SimplexPaymaster.
 *
 * Selects the first configured stablecoin (USDC, then USDT) with a balance of at
 * least one token, then picks the authorization mode:
 * - PERMIT (0x00) when the token supports EIP-2612: a permit executed during validation
 * - PERMIT2 (0x02) when the token is already approved to Permit2: a per-op, single-use
 *   Permit2 signature; nothing is exposed to the paymaster at rest
 * - APPROVE (0x01) while a legacy allowance to the paymaster is still in place
 * - otherwise one funded bootstrap tx: approve(Permit2, max) where Permit2 exists and
 *   the paymaster deployment supports it (then PERMIT2 for the account's lifetime),
 *   else a capped approve(paymaster)
 *
 * Returns null when the solver has no balance in any configured token — the caller
 * decides whether to fall back to another paymaster or to the EntryPoint deposit.
 */
export async function buildSimplexPaymasterData(
	client: PublicClient,
	walletClient: WalletClient,
	signer: Pick<Signer, "signTypedData">,
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
	options: SimplexPaymasterOptions = {},
): Promise<(PaymasterResult & { token: HexString }) | null> {
	const chainId = configService.getChainId(chain)

	const tokens: TokenOption[] = []

	const usdcAddress = configService.getUsdcAsset(chain)
	if (isConfigured(usdcAddress)) {
		tokens.push({ address: usdcAddress, decimals: configService.getUsdcDecimals(chain) })
	}

	const usdtAddress = configService.getUsdtAsset(chain)
	if (isConfigured(usdtAddress)) {
		tokens.push({ address: usdtAddress, decimals: configService.getUsdtDecimals(chain) })
	}

	const selected = await selectToken(client, solverAccount, tokens)
	if (!selected) {
		return null
	}

	const { address: tokenAddress, decimals: tokenDecimals } = selected
	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(tokenDecimals)

	const hasPermit = !options.skipPermit && (await tokenSupportsPermit(client, tokenAddress))

	if (hasPermit) {
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

	const permit2Address = configService.getPermit2Address(chain)
	const permit2 =
		isConfigured(permit2Address) && (await paymasterSupportsPermit2(client, paymasterAddress))
			? permit2Address
			: undefined

	const [paymasterAllowance, permit2Allowance] = await Promise.all([
		readAllowance(client, tokenAddress, solverAccount, paymasterAddress),
		permit2 ? readAllowance(client, tokenAddress, solverAccount, permit2) : Promise.resolve(0n),
	])

	const permit2Mode = async () => ({
		...(await buildPermit2Mode(signer, {
			permit2: permit2!,
			chainId,
			token: tokenAddress,
			amount: recommended,
			spender: paymasterAddress,
			deadlineSeconds: options.permit2DeadlineSeconds ?? PERMIT2_DEADLINE_SECONDS,
		})),
		token: tokenAddress,
	})
	const approveMode = () => ({
		paymaster: paymasterAddress,
		paymasterData: encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
		token: tokenAddress,
	})

	if (permit2 && permit2Allowance >= recommended) {
		return permit2Mode()
	}

	if (paymasterAllowance >= THRESHOLD_USD * 10n ** BigInt(tokenDecimals)) {
		return approveMode()
	}

	if (permit2) {
		await sendFundedApprove(client, walletClient, solverAccount, tokenAddress, permit2, maxUint256)
		return permit2Mode()
	}

	await sendFundedApprove(client, walletClient, solverAccount, tokenAddress, paymasterAddress, recommended)
	return approveMode()
}

/**
 * The fee token a native EIP-7702 delegation can approve to Permit2 in the same transaction,
 * so a no-permit chain's one-time bootstrap rides the delegation rather than costing a
 * separate native tx. Returns null when no such approval is pending: a permit-capable token,
 * an approval already in place, no Permit2 configured, a paymaster without PERMIT2 mode, or
 * the solver holding no fee token yet (the approval then defers to the first sponsored op).
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
	if (!(await paymasterSupportsPermit2(client, paymasterAddress))) return null

	const tokens: TokenOption[] = []
	const usdc = configService.getUsdcAsset(chain)
	if (isConfigured(usdc)) tokens.push({ address: usdc, decimals: configService.getUsdcDecimals(chain) })
	const usdt = configService.getUsdtAsset(chain)
	if (isConfigured(usdt)) tokens.push({ address: usdt, decimals: configService.getUsdtDecimals(chain) })

	const selected = await selectToken(client, solverAccount, tokens)
	if (!selected) return null
	// A permit-capable token uses PERMIT mode, never Permit2.
	if (await tokenSupportsPermit(client, selected.address)) return null

	const recommended = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(selected.decimals)
	const allowance = await readAllowance(client, selected.address, solverAccount, permit2)
	if (allowance >= recommended) return null

	return { token: selected.address, spender: permit2 }
}

// ── Helpers ──────────────────────────────────────────────────────────

/** Missing assets come back from the config service as the literal "0x". */
function isConfigured(address: HexString): boolean {
	return !!address && address !== "0x" && address !== "0x0000000000000000000000000000000000000000"
}

async function selectToken(
	client: PublicClient,
	solverAccount: HexString,
	tokens: TokenOption[],
): Promise<TokenOption | null> {
	for (const token of tokens) {
		const balance = (await client.readContract({
			address: token.address,
			abi: erc20Abi,
			functionName: "balanceOf",
			args: [solverAccount],
		})) as bigint

		const minBalance = 10n ** BigInt(token.decimals)
		if (balance >= minBalance) {
			return token
		}
	}
	return null
}

async function buildPermitMode(
	client: PublicClient,
	signer: Pick<Signer, "signTypedData">,
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	permitAmount: bigint,
	chainId: number,
): Promise<PaymasterResult> {
	// An existing allowance makes the permit redundant — use the cheaper approve mode.
	const existingAllowance = (await client.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	if (existingAllowance >= permitAmount) {
		const paymasterData = encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString
		return {
			paymaster: paymasterAddress,
			paymasterData,
			paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
			paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
		}
	}

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

	// mode(1) + token(20) + permitAmount(32) + deadline(32) + v(1) + r(32) + s(32) = 150 bytes,
	// matching SimplexPaymaster._executePermit. Deadline is maxUint256 because paymasters
	// cannot read block.timestamp under ERC-4337 validation rules.
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

const PERMIT2_SUPPORT_NEGATIVE_TTL_MS = 5 * 60_000
const permit2Support = new Map<string, { supported: boolean; checkedAt: number }>()

/**
 * Only paymaster implementations that expose PERMIT2() accept mode 0x02; older
 * deployments reject it with InvalidMode. Probing keeps the client safe while the
 * redeploy lands chain by chain. Positive results are cached for the process
 * lifetime, negative ones briefly so an upgrade is picked up without a restart.
 */
async function paymasterSupportsPermit2(client: PublicClient, paymasterAddress: HexString): Promise<boolean> {
	const key = paymasterAddress.toLowerCase()
	const cached = permit2Support.get(key)
	if (cached && (cached.supported || Date.now() - cached.checkedAt < PERMIT2_SUPPORT_NEGATIVE_TTL_MS)) {
		return cached.supported
	}
	let supported = false
	try {
		await client.readContract({ address: paymasterAddress, abi: SIMPLEX_PAYMASTER_ABI, functionName: "PERMIT2" })
		supported = true
	} catch {
		supported = false
	}
	permit2Support.set(key, { supported, checkedAt: Date.now() })
	return supported
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

	// Split into explicit v, r, s (mode 0x02 layout mirrors the EIP-2612 mode 0x00 fields).
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
 * Sends `approve(spender, amount)` from the solver EOA and waits for one
 * confirmation. This is a plain EOA tx paid in native — the very thing the
 * paymaster exists to avoid needing — so the balance is pre-checked to fail an
 * unfunded solver with one actionable line instead of viem's estimateGas chain.
 */
async function sendFundedApprove(
	client: PublicClient,
	walletClient: WalletClient,
	solverAccount: HexString,
	tokenAddress: HexString,
	spender: HexString,
	amount: bigint,
): Promise<void> {
	const [nativeBalance, gasPrice] = await Promise.all([
		client.getBalance({ address: solverAccount }),
		client.getGasPrice(),
	])
	const requiredNative = APPROVE_TX_GAS * gasPrice
	if (nativeBalance < requiredNative) {
		throw new Error(
			`SimplexPaymaster needs a one-time funded approval on this chain — the fee token has no permit; ` +
				`send native dust (>= ${formatEther(requiredNative)}) to ${solverAccount}`,
		)
	}

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
	} catch {
		return false
	}
}
