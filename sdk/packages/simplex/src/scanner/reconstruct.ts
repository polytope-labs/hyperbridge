import {
	Order,
	orderCommitment,
	normalizeStateMachineId,
	DecodedOrderPlacedLog,
	HexString,
} from "@hyperbridge/sdk"

export interface ReconstructDeps {
	onError?: (err: unknown, log: DecodedOrderPlacedLog) => void
}

/**
 * Pure reconstruction of `OrderPlaced` logs into `Order` structs with
 * commitments. Every log carries the complete order — including both call
 * payloads — so each order is rebuilt from its log alone.
 */
/** An order rebuilt from a log, with the coordinates that locate it in the chain. */
export interface ReconstructedOrder {
	order: Order
	transactionHash: string
	blockNumber: bigint
	blockHash: string
	logIndex: number
}

export function reconstructOrdersFromLogs(
	logs: DecodedOrderPlacedLog[],
	deps: ReconstructDeps = {},
): ReconstructedOrder[] {
	const out: ReconstructedOrder[] = []

	for (const decodedLog of logs) {
		try {
			const { predispatchCall, outputCall } = decodedLog.args
			if (predispatchCall === undefined || outputCall === undefined) {
				throw new Error("OrderPlaced log is missing its call payloads; pre-#1092 gateways are not supported")
			}

			const order: Order = {
				user: decodedLog.args.user,
				source: normalizeStateMachineId(decodedLog.args.source) as HexString,
				destination: normalizeStateMachineId(decodedLog.args.destination) as HexString,
				deadline: decodedLog.args.deadline,
				nonce: decodedLog.args.nonce,
				fees: decodedLog.args.fees,
				session: decodedLog.args.session,
				predispatch: {
					assets: decodedLog.args.predispatch.map(
						(predispatch: { token: HexString; amount: bigint }) => ({
							token: predispatch.token,
							amount: predispatch.amount,
						}),
					),
					call: predispatchCall,
				},
				output: {
					beneficiary: decodedLog.args.beneficiary,
					assets: decodedLog.args.outputs.map((output: { token: HexString; amount: bigint }) => ({
						token: output.token,
						amount: output.amount,
					})),
					call: outputCall,
				},
				inputs: decodedLog.args.inputs.map((input: { token: HexString; amount: bigint }) => ({
					token: input.token,
					amount: input.amount,
				})),
			}

			order.id = orderCommitment(order)

			// Coordinates ride along: any cursor-based feed needs them, and viem's log
			// already carries them — they were simply being dropped here.
			const coords = decodedLog as unknown as { blockNumber?: bigint; blockHash?: string; logIndex?: number }
			out.push({
				order,
				transactionHash: decodedLog.transactionHash as string,
				blockNumber: coords.blockNumber ?? 0n,
				blockHash: coords.blockHash ?? "",
				logIndex: coords.logIndex ?? 0,
			})
		} catch (error) {
			if (deps.onError) deps.onError(error, decodedLog)
		}
	}

	return out
}
