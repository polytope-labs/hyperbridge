import { encodeFunctionData, parseEventLogs, zeroAddress, zeroHash } from "viem"
import { IntentsCoprocessor, orderCommitment, normalizeStateMachineId, encodeStateMachineId } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { ChainClientManager } from "@/services/ChainClientManager"
import { FillerConfigService, type PhantomTokenPairConfig } from "@/services/FillerConfigService"
import { getLogger } from "@/services/Logger"
import type { HexString } from "@hyperbridge/sdk"

const logger = getLogger("phantom-generator")

/**
 * Places phantom orders (expired same-chain swaps) on IntentGatewayV2 for each
 * configured token pair, then registers the commitment on Hyperbridge so fillers
 * can submit bids that the indexer collects for price and liquidity data.
 */
export class PhantomOrderGenerator {
	private timer: ReturnType<typeof setTimeout> | undefined
	private running = false

	constructor(
		private readonly clientManager: ChainClientManager,
		private readonly configService: FillerConfigService,
		private readonly coprocessor: IntentsCoprocessor,
	) {}

	start(): void {
		if (this.running) return
		this.running = true
		this.scheduleNext()
	}

	stop(): void {
		this.running = false
		if (this.timer !== undefined) {
			clearTimeout(this.timer)
			this.timer = undefined
		}
	}

	private scheduleNext(): void {
		const cfg = this.configService.getPhantomGeneratorConfig()
		if (!cfg || cfg.enabled === false) return

		const intervalMs = (cfg.interval_hours ?? 1) * 60 * 60 * 1_000

		this.timer = setTimeout(async () => {
			if (!this.running) return
			try {
				await this.run()
			} catch (err) {
				logger.error({ err }, "Phantom order generation round failed")
			}
			if (this.running) this.scheduleNext()
		}, intervalMs)
	}

	/** Run one round: place one phantom order per token pair and register it. */
	async run(): Promise<void> {
		const cfg = this.configService.getPhantomGeneratorConfig()
		if (!cfg || cfg.enabled === false || cfg.token_pairs.length === 0) return

		const chain = cfg.chain
		const gatewayAddress = this.configService.getIntentGatewayAddress(chain)
		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		for (const pair of cfg.token_pairs) {
			try {
				const commitment = await this.placePair(
					chain,
					gatewayAddress,
					walletClient,
					publicClient,
					pair,
				)
				if (!commitment) continue

				const result = await this.coprocessor.registerPhantomOrder(commitment, chain)
				if (result.success) {
					logger.info({ commitment, chain }, "Phantom order registered")
				} else {
					logger.warn({ commitment, chain, err: result.error }, "Failed to register phantom order")
				}
			} catch (err) {
				logger.error({ pair, chain, err }, "Failed to place phantom order for pair")
			}
		}
	}

	/**
	 * Places a single expired same-chain phantom order for a token pair.
	 * Returns the order commitment extracted from the OrderPlaced event.
	 */
	private async placePair(
		chain: string,
		gatewayAddress: `0x${string}`,
		walletClient: ReturnType<typeof this.clientManager.getWalletClient>,
		publicClient: ReturnType<typeof this.clientManager.getPublicClient>,
		pair: PhantomTokenPairConfig,
	): Promise<HexString | undefined> {
		// Pad a 20-byte address to bytes32 (left-zero padded)
		const toBytes32 = (addr: string): `0x${string}` => {
			const hex = addr.toLowerCase().replace("0x", "")
			return `0x${"0".repeat(64 - hex.length)}${hex}` as `0x${string}`
		}

		const senderAddress = walletClient.account.address

		const chainBytes = encodeStateMachineId(chain)

		const order = {
			user: toBytes32(senderAddress),
			// Same chain for source and destination — this is a same-chain swap
			source: chainBytes,
			destination: chainBytes,
			deadline: 0n, // expired immediately
			nonce: 0n,
			fees: 0n,
			session: zeroAddress,
			predispatch: {
				assets: [] as { token: `0x${string}`; amount: bigint }[],
				call: "0x" as `0x${string}`,
			},
			inputs: [{ token: toBytes32(pair.token_a), amount: BigInt(pair.standard_amount) }],
			output: {
				beneficiary: toBytes32(senderAddress),
				assets: [{ token: toBytes32(pair.token_b), amount: BigInt(pair.min_output) }],
				call: "0x" as `0x${string}`,
			},
		}

		const calldata = encodeFunctionData({
			abi: INTENT_GATEWAY_V2_ABI,
			functionName: "placeOrder",
			args: [order, zeroHash],
		})

		const hash = await walletClient.sendTransaction({
			to: gatewayAddress,
			data: calldata,
			chain: walletClient.chain,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash })

		const events = parseEventLogs({
			abi: INTENT_GATEWAY_V2_ABI,
			logs: receipt.logs,
			eventName: "OrderPlaced",
		})

		const event = events[0]
		if (!event) {
			logger.warn({ hash }, "OrderPlaced event not found in phantom order transaction")
			return undefined
		}

		// Reconstruct the order with the nonce assigned by the contract, then compute commitment.
		// The OrderPlaced event emits source/destination as strings (e.g. "EVM-8453").
		const args = (event as any).args
		const normalizedChain = normalizeStateMachineId(args.source ?? chain) as HexString

		const placedOrder = {
			user: args.user as HexString,
			source: normalizedChain,
			destination: normalizedChain,
			deadline: args.deadline as bigint,
			nonce: args.nonce as bigint,
			fees: args.fees as bigint,
			session: args.session as HexString,
			predispatch: {
				assets: (args.predispatch ?? []).map((a: any) => ({ token: a.token, amount: a.amount })),
				call: "0x" as HexString,
			},
			inputs: (args.inputs ?? []).map((a: any) => ({ token: a.token, amount: a.amount })),
			output: {
				beneficiary: args.beneficiary as HexString,
				assets: (args.outputs ?? []).map((a: any) => ({ token: a.token, amount: a.amount })),
				call: "0x" as HexString,
			},
		}

		const commitment = orderCommitment(placedOrder) as HexString
		logger.info({ commitment, tokenA: pair.token_a, tokenB: pair.token_b, chain }, "Phantom order placed")
		return commitment
	}
}
