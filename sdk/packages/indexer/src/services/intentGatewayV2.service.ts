import Decimal from "decimal.js"
import { ethers } from "ethers"
import type { Hex } from "viem"
import { keccak256, encodeAbiParameters } from "viem"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"

import { OrderStatus, ProtocolParticipantType, PointsActivityType } from "@/configs/src/types"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { OrderV2 as OrderV2Placed } from "@/configs/src/types/models/OrderV2"
import { OrderV2StatusMetadata } from "@/configs/src/types/models/OrderV2StatusMetadata"
import { OrderV2PredispatchAsset } from "@/configs/src/types/models/OrderV2PredispatchAsset"
import { OrderV2InputAsset } from "@/configs/src/types/models/OrderV2InputAsset"
import { OrderV2OutputAsset } from "@/configs/src/types/models/OrderV2OutputAsset"
import { timestampToDate } from "@/utils/date.helpers"

import { PointsService } from "./points.service"
import { VolumeService } from "./volume.service"
import PriceHelper from "@/utils/price.helpers"
import { TokenPriceService } from "./token-price.service"
import stringify from "safe-stable-stringify"
import { getOrCreateUser } from "./userActivity.services"
import { TokenInfo } from "./intentGateway.service"

export interface DispatchInfo {
	assets: TokenInfo[]
	call: Hex
}
export interface PaymentInfoV2 {
	beneficiary: Hex
	assets: TokenInfo[]
	call: Hex
}
export interface OrderV2 {
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
	outputs: PaymentInfoV2
}

export const DEFAULT_REFERRER = "0x0000000000000000000000000000000000000000000000000000000000000000" as Hex

export class IntentGatewayV2Service {
	/**
	 * Create predispatch asset entities for an order
	 */
	private static async createPredispatchAssets(orderId: string, predispatch: DispatchInfo): Promise<void> {
		await Promise.all(
			predispatch.assets.map(async (asset, index) => {
				const assetId = `${orderId}-predispatch-${index}`
				let assetEntity = await OrderV2PredispatchAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await OrderV2PredispatchAsset.create({
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
				let assetEntity = await OrderV2InputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await OrderV2InputAsset.create({
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
	private static async createOutputAssets(orderId: string, outputs: PaymentInfoV2): Promise<void> {
		// Create/update output asset entities
		await Promise.all(
			outputs.assets.map(async (asset, index) => {
				const assetId = `${orderId}-output-${index}`
				let assetEntity = await OrderV2OutputAsset.get(assetId)

				if (!assetEntity) {
					assetEntity = await OrderV2OutputAsset.create({
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
		order: OrderV2,
		referrer: string,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<OrderV2Placed> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderV2Placed.get(order.id!)

		if (!orderPlaced) {
			const { inputUSD } = await this.getOrderValue(order)
			orderPlaced = await OrderV2Placed.create({
				id: order.id!,
				user: order.user,
				sourceChain: order.sourceChain,
				destChain: order.destChain,
				commitment: order.id!,
				deadline: order.deadline,
				nonce: order.nonce,
				fees: order.fees,
				session: order.session,
				inputUSD: inputUSD,
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
				`OrderV2 Placed Event successfully saved: ${stringify({
					orderPlaced,
				})}`,
			)

			logger.info("Now awarding points for the OrderV2 Placed Event")

			// Award points for order placement - using USD value directly
			const orderValue = new Decimal(inputUSD)
			const pointsToAward = orderValue.floor().toNumber()

			await PointsService.awardPoints(
				order.user,
				ethers.utils.toUtf8String(order.sourceChain),
				BigInt(pointsToAward),
				ProtocolParticipantType.USER,
				PointsActivityType.ORDER_PLACED_POINTS,
				transactionHash,
				`Points awarded for placing orderV2 ${order.id} with value ${inputUSD} USD`,
				timestamp,
			)

			await VolumeService.updateVolume("IntentGatewayV2.USER", inputUSD, timestamp)

			// Convert user to 20 bytes for UserActivity ID, but keep referrer as 32 bytes
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
				`OrderV2 ${stringify({ order: order.id })} already exists with status ${stringify({ status: orderPlaced.status })}. Updating order details while preserving status.`,
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
			orderPlaced.inputUSD = inputUSD
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
				`OrderV2 ${stringify({ order })} updated with actual data. Status remains: ${stringify({ existingStatus })}`,
			)

			// Award points for order placement - using USD value directly
			// Only award if status is not already FILLED (to avoid double awarding)
			if (existingStatus !== OrderStatus.FILLED) {
				logger.info("Now awarding points for the OrderV2 Placed Event")

				const orderValue = new Decimal(inputUSD)
				const pointsToAward = orderValue.floor().toNumber()

				await PointsService.awardPoints(
					order.user,
					ethers.utils.toUtf8String(order.sourceChain),
					BigInt(pointsToAward),
					ProtocolParticipantType.USER,
					PointsActivityType.ORDER_PLACED_POINTS,
					transactionHash,
					`Points awarded for placing orderV2 ${order.id} with value ${inputUSD} USD`,
					timestamp,
				)

				await VolumeService.updateVolume("IntentGatewayV2.USER", inputUSD, timestamp)
			}
		}

		return orderPlaced
	}

	static async getByCommitment(commitment: string): Promise<OrderV2Placed | null> {
		const orderPlaced = await OrderV2Placed.get(commitment)

		if (!orderPlaced) return null

		return orderPlaced
	}

	private static async getOrderValue(order: OrderV2): Promise<{ inputUSD: string }> {
		const inputValuesUSD = await this.getInputValuesUSD(order)

		return {
			inputUSD: inputValuesUSD.total,
		}
	}

	private static async getInputValuesUSD(order: OrderV2): Promise<{ total: string; values: string[] }> {
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

		let orderPlaced = await OrderV2Placed.get(commitment)

		// For race conditions, we create a placeholder order that will be updated when the PLACED event arrives
		if (!orderPlaced && status != OrderStatus.PLACED) {
			logger.warn(
				`OrderV2 ${stringify({ commitment })} does not exist yet but FILLED event received. Creating placeholder order.`,
			)

			orderPlaced = await OrderV2Placed.create({
				id: commitment,
				user: "0x0000000000000000000000000000000000000000" as Hex,
				sourceChain: "",
				destChain: "",
				commitment: commitment,
				deadline: BigInt(0),
				nonce: BigInt(0),
				fees: BigInt(0),
				session: "0x0000000000000000000000000000000000000000" as Hex,
				inputUSD: "0",
				status: OrderStatus.FILLED,
				referrer: DEFAULT_REFERRER,
				predispatchCalldata: "",
				postDispatchCalldata: "",
				createdAt: timestampToDate(timestamp),
				blockNumber: BigInt(blockNumber),
				blockTimestamp: timestamp,
				transactionHash,
			})
			await orderPlaced.save()

			logger.info(`Placeholder orderV2 with status FILLED created for commitment ${stringify({ commitment })}`)
		}

		if (orderPlaced) {
			orderPlaced.status = status === OrderStatus.PLACED ? orderPlaced.status : status
			await orderPlaced.save()

			// Award points for order filling - using USD value directly
			if (status === OrderStatus.FILLED && filler) {
				// Get output assets from the new entity relationships
				// Query output assets by constructing IDs (we'll query up to a reasonable limit)
				const outputAssets: TokenInfo[] = []
				for (let index = 0; index < 100; index++) {
					const assetId = `${commitment}-output-${index}`
					const asset = await OrderV2OutputAsset.get(assetId)
					if (!asset) break
					outputAssets.push({
						token: asset.token as Hex,
						amount: asset.amount,
					})
				}

				if (outputAssets.length > 0) {
					// Volume
					let outputUSD = await this.getOutputValuesUSD(outputAssets)

					await VolumeService.updateVolume(`IntentGatewayV2.FILLER.${filler}`, outputUSD.total, timestamp)

					const orderValue = new Decimal(orderPlaced.inputUSD)
					const pointsToAward = orderValue.floor().toNumber()

					// Rewards
					await PointsService.awardPoints(
						filler,
						ethers.utils.toUtf8String(orderPlaced.destChain),
						BigInt(pointsToAward),
						ProtocolParticipantType.FILLER,
						PointsActivityType.ORDER_FILLED_POINTS,
						transactionHash,
						`Points awarded for filling orderV2 ${commitment} with value ${orderPlaced.inputUSD} USD`,
						timestamp,
					)

					// User - convert to 20 bytes for UserActivity ID, referrer is already 32 bytes
					const userAddress20 = bytes32ToBytes20(orderPlaced.user)
					let user = await getOrCreateUser(userAddress20, orderPlaced.referrer)
					user.totalOrderFilledVolumeUSD = new Decimal(user.totalOrderFilledVolumeUSD)
						.plus(new Decimal(orderPlaced.inputUSD))
						.toString()
					user.totalFilledOrders = user.totalFilledOrders + BigInt(1)
					await user.save()

					// Referrer
					if (user.referrer) {
						const referrerPointsToAward = Math.floor(pointsToAward / 2)
						await PointsService.awardPoints(
							user.referrer,
							ethers.utils.toUtf8String(orderPlaced.sourceChain),
							BigInt(referrerPointsToAward),
							ProtocolParticipantType.REFERRER,
							PointsActivityType.ORDER_REFERRED_POINTS,
							transactionHash,
							`Points awarded for filling orderV2 ${commitment} with value ${orderPlaced.inputUSD} USD`,
							timestamp,
						)
					}
				}
			}
		}

		const orderStatusMetadata = await OrderV2StatusMetadata.create({
			id: `${commitment}.${status}`,
			orderId: orderPlaced?.id,
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

	static computeOrderCommitment(order: OrderV2): string {
		const encodedOrder = encodeAbiParameters(
			[
				{
					name: "order",
					type: "tuple",
					components: [
						{ name: "user", type: "bytes32" },
						{ name: "source", type: "bytes" },
						{ name: "destination", type: "bytes" },
						{ name: "deadline", type: "uint256" },
						{ name: "nonce", type: "uint256" },
						{ name: "fees", type: "uint256" },
						{ name: "session", type: "address" },
						{
							name: "predispatch",
							type: "tuple",
							components: [
								{
									name: "assets",
									type: "tuple[]",
									components: [
										{ name: "token", type: "bytes32" },
										{ name: "amount", type: "uint256" },
									],
								},
								{ name: "call", type: "bytes" },
							],
						},
						{
							name: "inputs",
							type: "tuple[]",
							components: [
								{ name: "token", type: "bytes32" },
								{ name: "amount", type: "uint256" },
							],
						},
						{
							name: "output",
							type: "tuple",
							components: [
								{ name: "beneficiary", type: "bytes32" },
								{
									name: "assets",
									type: "tuple[]",
									components: [
										{ name: "token", type: "bytes32" },
										{ name: "amount", type: "uint256" },
									],
								},
								{ name: "call", type: "bytes" },
							],
						},
					],
				},
			],
			[
				{
					user: order.user as Hex,
					source: order.sourceChain as Hex,
					destination: order.destChain as Hex,
					deadline: order.deadline,
					nonce: order.nonce,
					fees: order.fees,
					session: order.session || "0x0000000000000000000000000000000000000000",
					predispatch: {
						assets: order.predispatch.assets.map((predispatch) => ({
							token: predispatch.token as Hex,
							amount: predispatch.amount,
						})),
						call: order.predispatch.call as Hex,
					},
					inputs: order.inputs.map((input) => ({
						token: input.token as Hex,
						amount: input.amount,
					})),
					output: {
						beneficiary: order.outputs.beneficiary as Hex,
						assets: order.outputs.assets.map((output) => ({
							token: output.token as Hex,
							amount: output.amount,
						})),
						call: order.outputs.call as Hex,
					},
				},
			],
		)

		return keccak256(encodedOrder)
	}
}
