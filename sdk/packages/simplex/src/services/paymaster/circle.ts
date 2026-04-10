import { encodePacked, maxUint256, getContract, erc20Abi, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { EIP2612_ABI } from "@/config/abis/EIP2612"
import type { FillerConfigService } from "@/services/FillerConfigService"

// ── Constants ────────────────────────────────────────────────────────

export const PAYMASTER_VERIFICATION_GAS_LIMIT_PERMIT = 250_000n
export const PAYMASTER_VERIFICATION_GAS_LIMIT_APPROVE = 150_000n
export const PAYMASTER_POST_OP_GAS_LIMIT = 100_000n

/** Capped approval amount for non-permit tokens ($100) */
const APPROVAL_CAP_USD = 100n
/** Threshold below which we top up the approval ($10) */
const APPROVAL_THRESHOLD_USD = 10n

// ── Types ────────────────────────────────────────────────────────────

export interface PaymasterResult {
	paymaster: HexString
	paymasterData: HexString
	paymasterVerificationGasLimit: bigint
	paymasterPostOpGasLimit: bigint
}

interface TokenOption {
	address: HexString
	decimals: number
}

// ── Main entry point ─────────────────────────────────────────────────

/**
 * Self-contained paymaster data builder for SimplexPaymaster.
 *
 * Resolves USDC/USDT token addresses from the config service, then:
 * 1. Selects the first token with ≥1 balance (priority: USDC → USDT)
 * 2. If the token supports EIP-2612 permit → signs permit, encodes PERMIT mode
 * 3. If not → ensures a capped on-chain approval exists, encodes APPROVE mode
 *
 * @param client          Public client for reading balances/allowances
 * @param walletClient    Wallet client for sending approve tx (only for non-permit tokens)
 * @param signer          Signer with signTypedData capability (for permit)
 * @param solverAccount   The solver's smart account address
 * @param paymasterAddress SimplexPaymaster contract address
 * @param chain           State machine ID (e.g. "EVM-8453")
 * @param configService   Config service to resolve token addresses and decimals
 */
export async function buildPaymasterData(
	client: PublicClient,
	walletClient: WalletClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	chain: string,
	configService: FillerConfigService,
): Promise<PaymasterResult> {
	const chainId = configService.getChainId(chain)

	// ── Resolve tokens from config ───────────────────────────────────
	const tokens: TokenOption[] = []

	const usdcAddress = configService.getUsdcAsset(chain)
	const usdcDecimals = configService.getUsdcDecimals(chain)
	if (usdcAddress) {
		tokens.push({ address: usdcAddress, decimals: usdcDecimals })
	}

	const usdtAddress = configService.getUsdtAsset(chain)
	const usdtDecimals = configService.getUsdtDecimals(chain)
	if (usdtAddress) {
		tokens.push({ address: usdtAddress, decimals: usdtDecimals })
	}

	// ── 1. Select token by balance ───────────────────────────────────
	const selected = await selectToken(client, solverAccount, tokens)
	if (!selected) {
		throw new Error(
			`SimplexPaymaster: solver ${solverAccount} has insufficient balance in all configured tokens ` +
				`(${tokens.map((t) => t.address).join(", ")}). Need ≥1 token in at least one.`,
		)
	}

	const { address: tokenAddress, decimals: tokenDecimals } = selected
	const permitAmount = defaultPermitAmount(tokenDecimals)

	// ── 2. Check permit support ──────────────────────────────────────
	const hasPermit = await tokenSupportsPermit(client, tokenAddress)

	if (hasPermit) {
		return buildPermitMode(client, signer, solverAccount, paymasterAddress, tokenAddress, permitAmount, chainId)
	}

	// ── 3. Approve mode: ensure capped allowance ─────────────────────
	await ensureCappedApproval(client, walletClient, solverAccount, paymasterAddress, tokenAddress, tokenDecimals)

	const paymasterData = encodePacked(["uint8", "address"], [1, tokenAddress]) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: PAYMASTER_VERIFICATION_GAS_LIMIT_APPROVE,
		paymasterPostOpGasLimit: PAYMASTER_POST_OP_GAS_LIMIT,
	}
}

// ── Token selection ──────────────────────────────────────────────────

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

// ── Permit mode ──────────────────────────────────────────────────────

async function buildPermitMode(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	permitAmount: bigint,
	chainId: number,
): Promise<PaymasterResult> {
	// Check existing allowance — skip permit if already sufficient
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
			paymasterVerificationGasLimit: PAYMASTER_VERIFICATION_GAS_LIMIT_APPROVE,
			paymasterPostOpGasLimit: PAYMASTER_POST_OP_GAS_LIMIT,
		}
	}

	const permitSignature = await signTokenPermit(
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
	const v = parseInt(permitSignature.slice(130, 132), 16)

	const paymasterData = encodePacked(
		["uint8", "address", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
		[0, tokenAddress, permitAmount, maxUint256, v, r, s],
	) as HexString

	return {
		paymaster: paymasterAddress,
		paymasterData,
		paymasterVerificationGasLimit: PAYMASTER_VERIFICATION_GAS_LIMIT_PERMIT,
		paymasterPostOpGasLimit: PAYMASTER_POST_OP_GAS_LIMIT,
	}
}

// ── Approve mode: capped on-chain approval ───────────────────────────

async function ensureCappedApproval(
	client: PublicClient,
	walletClient: WalletClient,
	solverAccount: HexString,
	paymasterAddress: HexString,
	tokenAddress: HexString,
	tokenDecimals: number,
): Promise<void> {
	const currentAllowance = (await client.readContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "allowance",
		args: [solverAccount, paymasterAddress],
	})) as bigint

	const threshold = APPROVAL_THRESHOLD_USD * 10n ** BigInt(tokenDecimals)

	if (currentAllowance >= threshold) {
		return
	}

	const approvalAmount = APPROVAL_CAP_USD * 10n ** BigInt(tokenDecimals)

	const hash = await walletClient.writeContract({
		address: tokenAddress,
		abi: erc20Abi,
		functionName: "approve",
		args: [paymasterAddress, approvalAmount],
		chain: walletClient.chain,
		account: walletClient.account!,
	})

	await client.waitForTransactionReceipt({ hash, confirmations: 1 })
}

// ── EIP-2612 support detection ───────────────────────────────────────

async function tokenSupportsPermit(client: PublicClient, tokenAddress: HexString): Promise<boolean> {
	try {
		await client.readContract({
			address: tokenAddress,
			abi: [{ inputs: [], name: "version", outputs: [{ type: "string" }], stateMutability: "view", type: "function" }],
			functionName: "version",
		})
		return true
	} catch {
		return false
	}
}

// ── EIP-2612 Permit signing ──────────────────────────────────────────

async function signTokenPermit(
	client: PublicClient,
	signer: { signTypedData: (typedData: unknown, chainId?: number) => Promise<HexString> },
	owner: HexString,
	spender: HexString,
	tokenAddress: HexString,
	value: bigint,
	chainId: number,
): Promise<HexString> {
	const token = getContract({
		client,
		address: tokenAddress,
		abi: EIP2612_ABI,
	})

	const [name, version, nonce] = await Promise.all([
		token.read.name(),
		token.read.version(),
		token.read.nonces([owner]),
	])

	const typedData = {
		types: {
			EIP712Domain: [
				{ name: "name", type: "string" },
				{ name: "version", type: "string" },
				{ name: "chainId", type: "uint256" },
				{ name: "verifyingContract", type: "address" },
			],
			Permit: [
				{ name: "owner", type: "address" },
				{ name: "spender", type: "address" },
				{ name: "value", type: "uint256" },
				{ name: "nonce", type: "uint256" },
				{ name: "deadline", type: "uint256" },
			],
		},
		primaryType: "Permit" as const,
		domain: {
			name,
			version,
			chainId,
			verifyingContract: tokenAddress,
		},
		message: {
			owner,
			spender,
			value,
			nonce,
			deadline: maxUint256,
		},
	}

	return signer.signTypedData(typedData, chainId)
}

// ── Helpers ──────────────────────────────────────────────────────────

function defaultPermitAmount(tokenDecimals: number): bigint {
	return 10n * 10n ** BigInt(tokenDecimals) // $10 cap
}

/**
 * For EntryPoint v0.8, the `paymasterAndData` field in PackedUserOperation
 * is encoded as:
 *   paymaster (20 bytes) || paymasterVerificationGasLimit (uint128, 16 bytes)
 *   || paymasterPostOpGasLimit (uint128, 16 bytes) || paymasterData (variable)
 */
export function packPaymasterAndData(pm: PaymasterResult): HexString {
	return encodePacked(
		["address", "uint128", "uint128", "bytes"],
		[pm.paymaster, pm.paymasterVerificationGasLimit, pm.paymasterPostOpGasLimit, pm.paymasterData],
	) as HexString
}
