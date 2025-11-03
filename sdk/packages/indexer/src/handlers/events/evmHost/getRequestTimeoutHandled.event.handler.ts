import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status, Transfer } from "@/configs/src/types"
import { GetRequestTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { Interface } from "@ethersproject/abi"
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { GetTimeoutMessage } from "@/types/ismp"
import { safeArray } from "@/utils/data.helper"
import { extractAddressFromTopic, getPriceDataFromEthereumLog, isERC20TransferEvent } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { VolumeService } from "@/services/volume.service"

/**
 * Handles the GetRequestTimeoutHandled event from EVMHost
 */
export const handleGetRequestTimeoutHandled = wrap(async (event: GetRequestTimeoutHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transactionHash, blockNumber, blockHash, transaction } = event
	const { commitment } = args

	logger.info(
		`Handling GetRequestTimeoutHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

	await GetRequestService.updateStatus({
		commitment,
		chain,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		blockTimestamp,
		status: Status.TIMED_OUT,
		transactionHash,
	})

	let fromAddresses = [] as string[]

	if (transaction?.input) {
		const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })

		try {
			if (name === "handleGetRequestTimeouts" && args && args.length > 1) {
				const { timeouts } = args[1] as GetTimeoutMessage
				for (const getRequest of timeouts) {
					const { from: getRequestFrom } = getRequest
					fromAddresses.push(getRequestFrom)
				}
			}
		} catch (e: any) {
			logger.error(
				`Error decoding Post Request Handled event: ${stringify({
					error: e as unknown as Error,
				})}`,
			)
		}
	}

	for (const [index, log] of safeArray(transaction.logs).entries()) {
		if (!isERC20TransferEvent(log)) {
			continue
		}

		const value = BigInt(log.data)
		const transferId = `${log.transactionHash}-index-${index}`
		const transfer = await Transfer.get(transferId)

		if (!transfer) {
			const [_, fromTopic, toTopic] = log.topics
			const from = extractAddressFromTopic(fromTopic)
			const to = extractAddressFromTopic(toTopic)

			// Compute USD value first; skip zero-USD transfers
			const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(log.address, value, blockTimestamp)
			if (amountValueInUSD === "0") {
				continue
			}

			await TransferService.storeTransfer({
				transactionHash: transferId,
				chain,
				value,
				from,
				to,
			})

			await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)

			const matchingContract = fromAddresses.find(
				(addr) => addr.toLowerCase() === from.toLowerCase() || addr.toLowerCase() === to.toLowerCase(),
			)

			if (matchingContract) {
				await VolumeService.updateVolume(`Contract.${matchingContract}`, amountValueInUSD, blockTimestamp)
			}
		}
	}
})
