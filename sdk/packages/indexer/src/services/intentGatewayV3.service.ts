import Decimal from "decimal.js"
import { ethers } from "ethers"
import type { Hex } from "viem"
import { keccak256, encodeAbiParameters, toHex, type AbiParameter } from "viem"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"

// Derive the Order tuple type from the canonical ABI so this stays in sync with
// the on-chain struct (which the contract hashes via keccak256(abi.encode(order))).
const ORDER_TUPLE_TYPE: AbiParameter = (() => {
	const placeOrder = (IntentGatewayV3Abi as readonly any[]).find(
		(item) => item.type === "function" && item.name === "placeOrder",
	)
	const order = placeOrder?.inputs?.[0]
	if (!order) throw new Error("placeOrder.order not found in IntentGatewayV3 ABI")
	return order as AbiParameter
})()

import { OrderStatus, PendingStatusMetadata, ProtocolParticipantType, PointsActivityType } from "@/configs/src/types"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { IOrderV3 as OrderV3Placed } from "@/configs/src/types/models/IOrderV3"
import { IOrderV3StatusMetadata } from "@/configs/src/types/models/IOrderV3StatusMetadata"
import { IOrderV3PredispatchAsset } from "@/configs/src/types/models/IOrderV3PredispatchAsset"
import { IOrderV3InputAsset } from "@/configs/src/types/models/IOrderV3InputAsset"
import { IOrderV3OutputAsset } from "@/configs/src/types/models/IOrderV3OutputAsset"
import { IOrderV3PartialFill } from "@/configs/src/types/models/IOrderV3PartialFill"
import { IOrderV3PartialFillInputAsset } from "@/configs/src/types/models/IOrderV3PartialFillInputAsset"
import { IOrderV3PartialFillOutputAsset } from "@/configs/src/types/models/IOrderV3PartialFillOutputAsset"
import { IOrderV3Fill } from "@/configs/src/types/models/IOrderV3Fill"
import { IOrderV3FillInputAsset } from "@/configs/src/types/models/IOrderV3FillInputAsset"
import { IOrderV3FillOutputAsset } from "@/configs/src/types/models/IOrderV3FillOutputAsset"
import { IOrderV3EscrowRelease } from "@/configs/src/types/models/IOrderV3EscrowRelease"
import { IOrderV3EscrowReleaseToken } from "@/configs/src/types/models/IOrderV3EscrowReleaseToken"
import { IOrderV3EscrowRefund } from "@/configs/src/types/models/IOrderV3EscrowRefund"
import { IOrderV3EscrowRefundToken } from "@/configs/src/types/models/IOrderV3EscrowRefundToken"
import { timestampToDate } from "@/utils/date.helpers"

import { PointsService } from "./points.service"
import { VolumeService } from "./volume.service"
import PriceHelper from "@/utils/price.helpers"
import { TokenPriceService } from "./token-price.service"
import stringify from "safe-stable-stringify"
import { getOrCreateUser } from "./userActivity.services"
export interface TokenInfo {
	token: Hex
	amount: bigint
}

const ENTITY_TYPE = "IOrderV3"

const decodeChain = (value: string): string =>
	value.startsWith("0x") ? ethers.utils.toUtf8String(value) : value

export interface DispatchInfo {
	assets: TokenInfo[]
	call: Hex
}
export interface PaymentInfoV3 {
	beneficiary: Hex
	assets: TokenInfo[]
	call: Hex
}
export interface OrderV3 {
	id?: string
	user: Hex
	sourceChain: string
	destChain: string
	deadline: bigint
	nonce: bigint
	fees: bigint
	session: Hex
	predispatch: DispatchInfo
	inputs: TokenInfo[]
	outputs: PaymentInfoV3
}

export const DEFAULT_REFERRER = "0x0000000000000000000000000000000000000000000000000000000000000000" as Hex

export class IntentGatewayV3Service {
	/**
	 * Create predispatch asset entities for an order
	 */
	private static async createPredispatchAssets(orderId: string, predispatch: DispatchInfo): Promise<void> {
		await Promise.all(
			predispatch.assets.map(async (asset, index) => {
				const assetId = `${orderId}-predispatch-${index}`
				let assetEntity = await IOrderV3PredispatchAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3PredispatchAsset.create({
						id: assetId,
						orderId,
						token: asset.token,
						amount: asset.amount,
						index,
					})
				} else {
					assetEntity.token = asset.token
					assetEntity.amount = asset.amount
					assetEntity.index = index
				}
				await assetEntity.save()
			}),
		)
	}

	/**
	 * Create input asset entities for an order
	 */
	private static async createInputAssets(orderId: string, inputs: TokenInfo[]): Promise<void> {
		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${orderId}-input-${index}`
				let assetEntity = await IOrderV3InputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3InputAsset.create({
						id: assetId,
						orderId,
						token: input.token,
						amount: input.amount,
						index,
					})
				} else {
					assetEntity.token = input.token
					assetEntity.amount = input.amount
					assetEntity.index = index
				}
				await assetEntity.save()
			}),
		)
	}

	/**
	 * Create output asset entities for an order
	 */
	private static async createOutputAssets(orderId: string, outputs: PaymentInfoV3): Promise<void> {
		// Create/update output asset entities
		await Promise.all(
			outputs.assets.map(async (asset, index) => {
				const assetId = `${orderId}-output-${index}`
				let assetEntity = await IOrderV3OutputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3OutputAsset.create({
						id: assetId,
						orderId,
						token: asset.token,
						amount: asset.amount,
						index,
						beneficiary: outputs.beneficiary as string,
					})
				} else {
					assetEntity.token = asset.token
					assetEntity.amount = asset.amount
					assetEntity.index = index
					assetEntity.beneficiary = outputs.beneficiary as string
				}
				await assetEntity.save()
			}),
		)
	}

	static async getOrCreateOrder(
		order: OrderV3,
		referrer: string,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<OrderV3Placed> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderV3Placed.get(order.id!)

		if (!orderPlaced) {
			const { inputUSD } = await this.getOrderValue(order)
			orderPlaced = await OrderV3Placed.create({
				id: order.id!,
				user: order.user,
				sourceChain: order.sourceChain,
				destChain: order.destChain,
				commitment: order.id!,
				deadline: order.deadline,
				nonce: order.nonce,
				fees: order.fees,
				session: order.session,
				inputUSD: BigInt(new Decimal(inputUSD).truncated().toString()),
				predispatchCalldata: order.predispatch.call as string,
				postDispatchCalldata: order.outputs.call as string,
				status: OrderStatus.PLACED,
				referrer,
				createdAt: timestampToDate(timestamp),
				blockNumber: BigInt(blockNumber),
				blockTimestamp: timestamp,
				transactionHash,
			})
			await orderPlaced.save()

			// Create asset entities
			await this.createPredispatchAssets(order.id!, order.predispatch)
			await this.createInputAssets(order.id!, order.inputs)
			await this.createOutputAssets(order.id!, order.outputs)

			logger.info(
				`OrderV3 Placed Event successfully saved: ${stringify({
					orderPlaced,
				})}`,
			)

			await this.flushPendingStatuses(order.id!)

			logger.info("Now awarding points for the OrderV3 Placed Event")

			// Award points for order placement - using USD value directly
			const orderValue = new Decimal(inputUSD)
			const pointsToAward = orderValue.floor().toNumber()

			await PointsService.awardPoints(
				order.user,
				decodeChain(order.sourceChain),
				BigInt(pointsToAward),
				ProtocolParticipantType.USER,
				PointsActivityType.ORDER_PLACED_POINTS,
				transactionHash,
				`Points awarded for placing orderV3 ${order.id} with value ${inputUSD} USD`,
				timestamp,
			)

			await VolumeService.updateVolume("IntentGatewayV3.USER", inputUSD, timestamp)

			// Convert user to 20 bytes for UserActivityV2 ID, but keep referrer as 32 bytes
			const userAddress20 = bytes32ToBytes20(order.user)
			let user = await getOrCreateUser(userAddress20, referrer, timestamp)
			user.totalOrdersPlaced = user.totalOrdersPlaced + BigInt(1)
			user.totalOrderPlacedVolumeUSD = new Decimal(user.totalOrderPlacedVolumeUSD)
				.plus(new Decimal(inputUSD))
				.toString()
			user.createdAt = user.createdAt === timestampToDate(BigInt(0)) ? timestampToDate(timestamp) : user.createdAt
			await user.save()
		} else {
			// Handle race condition: Order already exists (e.g., was filled first)
			// Update all fields except status and status-related metadata
			logger.info(
				`OrderV3 ${stringify({ order: order.id })} already exists with status ${stringify({ status: orderPlaced.status })}. Updating order details while preserving status.`,
			)

			const existingStatus = orderPlaced.status
			const { inputUSD } = await this.getOrderValue(order)

			orderPlaced.user = order.user
			orderPlaced.sourceChain = order.sourceChain
			orderPlaced.destChain = order.destChain
			orderPlaced.deadline = order.deadline
			orderPlaced.nonce = order.nonce
			orderPlaced.fees = order.fees
			orderPlaced.session = order.session
			orderPlaced.inputUSD = BigInt(new Decimal(inputUSD).truncated().toString())
			orderPlaced.predispatchCalldata = order.predispatch.call as string
			orderPlaced.postDispatchCalldata = order.outputs.call as string
			orderPlaced.referrer = referrer
			// Keep existing status - don't overwrite it

			await orderPlaced.save()

			// Update asset entities
			await this.createPredispatchAssets(order.id!, order.predispatch)
			await this.createInputAssets(order.id!, order.inputs)
			await this.createOutputAssets(order.id!, order.outputs)

			logger.info(
				`OrderV3 ${stringify({ order })} updated with actual data. Status remains: ${stringify({ existingStatus })}`,
			)

			// Award points for order placement - using USD value directly
			// Only award if status is not already FILLED (to avoid double awarding)
			if (existingStatus !== OrderStatus.FILLED) {
				logger.info("Now awarding points for the OrderV3 Placed Event")

				const orderValue = new Decimal(inputUSD)
				const pointsToAward = orderValue.floor().toNumber()

				await PointsService.awardPoints(
					order.user,
					decodeChain(order.sourceChain),
					BigInt(pointsToAward),
					ProtocolParticipantType.USER,
					PointsActivityType.ORDER_PLACED_POINTS,
					transactionHash,
					`Points awarded for placing orderV3 ${order.id} with value ${inputUSD} USD`,
					timestamp,
				)

				await VolumeService.updateVolume("IntentGatewayV3.USER", inputUSD, timestamp)
			}
		}

		return orderPlaced
	}

	static async getByCommitment(commitment: string): Promise<OrderV3Placed | null> {
		const orderPlaced = await OrderV3Placed.get(commitment)

		if (!orderPlaced) return null

		return orderPlaced
	}

	private static async getOrderValue(order: OrderV3): Promise<{ inputUSD: string }> {
		const inputValuesUSD = await this.getInputValuesUSD(order)

		return {
			inputUSD: inputValuesUSD.total,
		}
	}

	private static async getInputValuesUSD(order: OrderV3): Promise<{ total: string; values: string[] }> {
		return this.getTokenValuesUSD(order.inputs)
	}

	private static async getOutputValuesUSD(outputs: TokenInfo[]): Promise<{ total: string; values: string[] }> {
		return this.getTokenValuesUSD(outputs)
	}

	private static async getTokenValuesUSD(
		tokens: { token: string; amount: bigint }[],
	): Promise<{ total: string; values: string[] }> {
		const valuesUSD = await Promise.all(
			tokens.map(async (token) => {
				const tokenAddress = bytes32ToBytes20(token.token)
				let decimals = 18
				let symbol = "eth"

				if (tokenAddress != "0x0000000000000000000000000000000000000000") {
					const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
					decimals = await tokenContract.decimals()
					symbol = await tokenContract.symbol()
				}

				const price = await TokenPriceService.getPrice(symbol)
				return PriceHelper.getAmountValueInUSD(token.amount, decimals, price)
			}),
		)

		const total = valuesUSD.reduce((acc, curr) => {
			return acc.plus(new Decimal(curr.amountValueInUSD))
		}, new Decimal(0))

		return {
			total: total.toFixed(18),
			values: valuesUSD.map((value) => value.amountValueInUSD),
		}
	}

	static async updateOrderStatus(
		commitment: string,
		status: OrderStatus,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
		filler?: string,
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderV3Placed.get(commitment)

		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({ commitment })} does not exist yet, storing in PendingStatusMetadata for status ${status}`,
			)

			let pending = PendingStatusMetadata.create({
				id: `${commitment}.${ENTITY_TYPE}.${status}`,
				commitment,
				entityType: ENTITY_TYPE,
				status,
				chain: chainId,
				timestamp,
				blockNumber: blockNumber.toString(),
				blockHash: "",
				transactionHash,
				filler,
				createdAt: timestampToDate(timestamp),
			})

			await pending.save()
			return
		}

		orderPlaced.status = status === OrderStatus.PLACED ? orderPlaced.status : status
		await orderPlaced.save()

		// Once-per-order accounting on completion. Filler volume/points are NOT awarded
		// here: with partial fills an order can be completed by several solvers, so the
		// filler is credited per fill slice in awardFillRewards instead.
		if (status === OrderStatus.FILLED && filler) {
			const orderValue = new Decimal(orderPlaced.inputUSD.toString())
			const pointsToAward = orderValue.floor().toNumber()

			// User - convert to 20 bytes for UserActivityV2 ID, referrer is already 32 bytes
			const userAddress20 = bytes32ToBytes20(orderPlaced.user)
			let user = await getOrCreateUser(userAddress20, orderPlaced.referrer)
			user.totalOrderFilledVolumeUSD = new Decimal(user.totalOrderFilledVolumeUSD)
				.plus(new Decimal(orderPlaced.inputUSD.toString()))
				.toString()
			user.totalFilledOrders = user.totalFilledOrders + BigInt(1)
			await user.save()

			// Referrer
			if (user.referrer) {
				const referrerPointsToAward = Math.floor(pointsToAward / 2)
				await PointsService.awardPoints(
					user.referrer,
					decodeChain(orderPlaced.sourceChain),
					BigInt(referrerPointsToAward),
					ProtocolParticipantType.REFERRER,
					PointsActivityType.ORDER_REFERRED_POINTS,
					transactionHash,
					`Points awarded for filling orderV3 ${commitment} with value ${orderPlaced.inputUSD} USD`,
					timestamp,
				)
			}
		}

		const orderStatusMetadata = await IOrderV3StatusMetadata.create({
			id: `${commitment}.${status}`,
			orderId: commitment,
			status,
			chain: chainId,
			timestamp,
			blockNumber: blockNumber.toString(),
			filler,
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})

		await orderStatusMetadata.save()
	}

	/**
	 * Flush any pending status metadata entries for an order that was just created
	 */
	static async flushPendingStatuses(commitment: string): Promise<void> {
		const pendingStatuses = await PendingStatusMetadata.getByCommitment(commitment, {
			limit: 10,
		})

		const matching = pendingStatuses.filter((p) => p.entityType === ENTITY_TYPE)

		for (const pending of matching) {
			const orderStatusMetadata = IOrderV3StatusMetadata.create({
				id: `${commitment}.${pending.status}`,
				orderId: commitment,
				status: pending.status as OrderStatus,
				chain: pending.chain,
				timestamp: pending.timestamp,
				blockNumber: pending.blockNumber,
				filler: pending.filler,
				transactionHash: pending.transactionHash,
				createdAt: pending.createdAt,
			})

			await orderStatusMetadata.save()
			await PendingStatusMetadata.remove(pending.id)

			logger.info(
				`Flushed pending status ${pending.status} for IOrderV3 ${commitment}`,
			)
		}
	}

	/**
	 * Credits a solver for one fill slice, valued from that fill's own event outputs.
	 * With partial fills an order can be filled by several solvers, so per-order
	 * crediting would attribute other solvers' slices to the completing filler.
	 */
	private static async awardFillRewards(
		commitment: string,
		filler: string,
		outputs: TokenInfo[],
		transactionHash: string,
		timestamp: bigint,
	): Promise<void> {
		const provided = outputs.filter((output) => output.amount > 0n)
		if (provided.length === 0) return

		const outputUSD = await this.getOutputValuesUSD(provided)
		const sliceUSD = new Decimal(outputUSD.total)
		if (sliceUSD.lte(0)) return

		await VolumeService.updateVolume(`IntentGatewayV3.FILLER.${filler}`, outputUSD.total, timestamp)

		const orderPlaced = await OrderV3Placed.get(commitment)
		if (!orderPlaced) return

		const pointsToAward = sliceUSD.floor().toNumber()
		if (pointsToAward > 0) {
			await PointsService.awardPoints(
				filler,
				decodeChain(orderPlaced.destChain),
				BigInt(pointsToAward),
				ProtocolParticipantType.FILLER,
				PointsActivityType.ORDER_FILLED_POINTS,
				transactionHash,
				`Points awarded for filling orderV3 ${commitment} slice worth ${sliceUSD.toString()} USD`,
				timestamp,
			)
		}
	}

	/**
	 * Accumulates a fill's output amounts into the order's per-output `filled` totals.
	 * The order is placed on the source chain while fills land on the destination, so
	 * the output-asset rows may not exist yet when a fill is indexed — progress for
	 * such fills is still recoverable from the fill entities themselves.
	 */
	private static async accumulateFilled(commitment: string, outputs: TokenInfo[]): Promise<void> {
		for (let index = 0; index < outputs.length; index++) {
			const output = outputs[index]
			if (output.amount === 0n) continue

			const asset = await IOrderV3OutputAsset.get(`${commitment}-output-${index}`)
			if (!asset || asset.token.toLowerCase() !== output.token.toLowerCase()) {
				logger.warn(
					`OrderV3 ${commitment} output asset ${index} missing or token mismatch, skipping fill accumulation`,
				)
				continue
			}

			asset.filled = (asset.filled ?? 0n) + output.amount
			await asset.save()
		}
	}

	static async recordPartialFill(
		commitment: string,
		filler: string,
		outputs: TokenInfo[],
		inputs: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData

		// Ensure we at least log if the order doesn't exist yet (race between PLACED and PartialFill)
		const orderPlaced = await OrderV3Placed.get(commitment)
		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({
					commitment,
				})} does not exist yet but PartialFill event received. Recording partial fill linked by commitment.`,
			)
		}

		const partialFillId = `${transactionHash}.${logIndex}`

		// The partial-fill record doubles as the idempotency marker for the cumulative
		// accounting and rewards below, so a replayed event must not double-count.
		if (await IOrderV3PartialFill.get(partialFillId)) {
			logger.info(`OrderV3 PartialFill ${partialFillId} already recorded, skipping`)
			return
		}

		const partialFill = await IOrderV3PartialFill.create({
			id: partialFillId,
			orderId: commitment,
			chain: chainId,
			filler,
			timestamp,
			blockNumber: blockNumber.toString(),
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})

		await partialFill.save()

		// Create/update input assets for this partial fill
		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${partialFillId}-input-${index}`
				let assetEntity = await IOrderV3PartialFillInputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await IOrderV3PartialFillInputAsset.create({
						id: assetId,
						partialFillId,
						token: input.token,
						amount: input.amount,
						index,
					})
				}

				await assetEntity.save()
			}),
		)

		// Create/update output assets for this partial fill
		await Promise.all(
			outputs.map(async (output, index) => {
				const assetId = `${partialFillId}-output-${index}`
				let assetEntity = await IOrderV3PartialFillOutputAsset.get(assetId)

				// Try to reuse the beneficiary from the original order's output asset
				const orderOutputAssetId = `${commitment}-output-${index}`
				const orderOutputAsset = await IOrderV3OutputAsset.get(orderOutputAssetId)
				const beneficiary = orderOutputAsset?.beneficiary ?? "0x0000000000000000000000000000000000000000"

				if (!assetEntity) {
					assetEntity = await IOrderV3PartialFillOutputAsset.create({
						id: assetId,
						partialFillId,
						token: output.token,
						amount: output.amount,
						index,
						beneficiary,
					})
				}

				await assetEntity.save()
			}),
		)

		await this.accumulateFilled(commitment, outputs)
		await this.awardFillRewards(commitment, filler, outputs, transactionHash, timestamp)

		logger.info(
			`OrderV3 PartialFill recorded: ${stringify({
				commitment,
				partialFillId,
				filler,
			})}`,
		)
	}

	static async recordFill(
		commitment: string,
		filler: string,
		outputs: TokenInfo[],
		inputs: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData

		const orderPlaced = await OrderV3Placed.get(commitment)
		if (!orderPlaced) {
			logger.warn(
				`OrderV3 ${stringify({
					commitment,
				})} does not exist yet but OrderFilled event received. Recording fill linked by commitment.`,
			)
		}

		const fillId = `${transactionHash}.${logIndex}`

		// The fill record doubles as the idempotency marker for the cumulative
		// accounting and rewards below, so a replayed event must not double-count.
		if (await IOrderV3Fill.get(fillId)) {
			logger.info(`OrderV3 Fill ${fillId} already recorded, skipping`)
			return
		}

		const fill = await IOrderV3Fill.create({
			id: fillId,
			orderId: commitment,
			chain: chainId,
			filler,
			timestamp,
			blockNumber: blockNumber.toString(),
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})
		await fill.save()

		await Promise.all(
			inputs.map(async (input, index) => {
				const assetId = `${fillId}-input-${index}`
				let assetEntity = await IOrderV3FillInputAsset.get(assetId)
				if (!assetEntity) {
					assetEntity = await IOrderV3FillInputAsset.create({
						id: assetId,
						fillId,
						token: input.token,
						amount: input.amount,
						index,
					})
				}
				await assetEntity.save()
			}),
		)

		await Promise.all(
			outputs.map(async (output, index) => {
				const assetId = `${fillId}-output-${index}`
				let assetEntity = await IOrderV3FillOutputAsset.get(assetId)
				if (!assetEntity) {
					assetEntity = await IOrderV3FillOutputAsset.create({
						id: assetId,
						fillId,
						token: output.token,
						amount: output.amount,
						index,
					})
				}
				await assetEntity.save()
			}),
		)

		await this.accumulateFilled(commitment, outputs)
		await this.awardFillRewards(commitment, filler, outputs, transactionHash, timestamp)

		logger.info(
			`OrderV3 Fill recorded: ${stringify({
				commitment,
				fillId,
				filler,
			})}`,
		)
	}

	static async recordEscrowRelease(
		commitment: string,
		solver: string | undefined,
		tokens: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData
		const releaseId = `${transactionHash}.${logIndex}`

		// The release record doubles as the idempotency marker for the cumulative
		// accounting below, so a replayed event must not double-count.
		if (await IOrderV3EscrowRelease.get(releaseId)) {
			logger.info(`OrderV3 EscrowRelease ${releaseId} already recorded, skipping`)
			return
		}

		const release = await IOrderV3EscrowRelease.create({
			id: releaseId,
			orderId: commitment,
			chain: chainId,
			solver,
			timestamp,
			blockNumber: blockNumber.toString(),
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})
		await release.save()

		await Promise.all(
			tokens.map(async (token, index) => {
				const tokenId = `${releaseId}-token-${index}`
				const tokenEntity = await IOrderV3EscrowReleaseToken.create({
					id: tokenId,
					releaseId,
					token: token.token,
					amount: token.amount,
					index,
				})
				await tokenEntity.save()
			}),
		)

		// EscrowReleased fires for every redeem — including non-finalizing partial
		// redeems — so the order is REDEEMED only once every escrowed input has been
		// fully released. The contract's release formula sends integer-division dust
		// to the completing fill, so cumulative releases sum to exactly the escrowed
		// amount. Release events fire on the source chain (same chain as OrderPlaced),
		// so the input-asset rows exist by the time a release is indexed.
		const inputAssets: IOrderV3InputAsset[] = []
		for (let index = 0; ; index++) {
			const asset = await IOrderV3InputAsset.get(`${commitment}-input-${index}`)
			if (!asset) break
			inputAssets.push(asset)
		}

		if (inputAssets.length === 0) {
			logger.warn(`OrderV3 ${commitment} has no input assets yet, skipping release accumulation`)
			return
		}

		for (let index = 0; index < tokens.length; index++) {
			const token = tokens[index]
			if (token.amount === 0n) continue

			const asset = inputAssets[index]
			if (!asset || asset.token.toLowerCase() !== token.token.toLowerCase()) {
				logger.warn(
					`OrderV3 ${commitment} input asset ${index} missing or token mismatch, skipping release accumulation`,
				)
				continue
			}

			asset.released = (asset.released ?? 0n) + token.amount
			await asset.save()
		}

		const fullyReleased = inputAssets.every((asset) => (asset.released ?? 0n) >= asset.amount)
		if (fullyReleased) {
			await this.updateOrderStatus(
				commitment,
				OrderStatus.REDEEMED,
				{ transactionHash, blockNumber, timestamp },
				solver,
			)
		}
	}

	static async recordEscrowRefund(
		commitment: string,
		tokens: TokenInfo[],
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
			logIndex: number
		},
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp, logIndex } = logsData
		const refundId = `${transactionHash}.${logIndex}`

		let refund = await IOrderV3EscrowRefund.get(refundId)
		if (!refund) {
			refund = await IOrderV3EscrowRefund.create({
				id: refundId,
				orderId: commitment,
				chain: chainId,
				timestamp,
				blockNumber: blockNumber.toString(),
				transactionHash,
				createdAt: timestampToDate(timestamp),
			})
		}
		await refund.save()

		await Promise.all(
			tokens.map(async (token, index) => {
				const tokenId = `${refundId}-token-${index}`
				let tokenEntity = await IOrderV3EscrowRefundToken.get(tokenId)
				if (!tokenEntity) {
					tokenEntity = await IOrderV3EscrowRefundToken.create({
						id: tokenId,
						refundId,
						token: token.token,
						amount: token.amount,
						index,
					})
				}
				await tokenEntity.save()
			}),
		)
	}

	static computeOrderCommitment(order: OrderV3): string {
		// Legacy DB rows store state-machine ids as hex bytes; new event data
		// arrives as plain strings (e.g. "EVM-97"). Normalise both to the hex
		// form the Order struct's `bytes source/destination` fields expect.
		const toBytes = (value: string): `0x${string}` =>
			value.startsWith("0x") ? (value as `0x${string}`) : toHex(value)

		const encoded = encodeAbiParameters(
			[ORDER_TUPLE_TYPE],
			[
				{
					user: order.user,
					source: toBytes(order.sourceChain),
					destination: toBytes(order.destChain),
					deadline: order.deadline,
					nonce: order.nonce,
					fees: order.fees,
					session: (
						order.session || "0x0000000000000000000000000000000000000000"
					).toLowerCase() as `0x${string}`,
					predispatch: {
						assets: order.predispatch.assets.map((a) => ({ token: a.token, amount: a.amount })),
						call: order.predispatch.call,
					},
					inputs: order.inputs.map((i) => ({ token: i.token, amount: i.amount })),
					output: {
						beneficiary: order.outputs.beneficiary,
						assets: order.outputs.assets.map((a) => ({ token: a.token, amount: a.amount })),
						call: order.outputs.call,
					},
				} as any,
			],
		)

		return keccak256(encoded)
	}
}
