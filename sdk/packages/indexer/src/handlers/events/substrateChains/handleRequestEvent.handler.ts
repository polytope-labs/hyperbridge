import { SubstrateEvent } from "@subql/types"
import fetch from "node-fetch"
import { bytesToHex, hexToBytes, toHex } from "viem"

import { RequestService } from "../../../services/request.service"
import { Status } from "../../../../configs/src/types"
import { formatChain, getHostStateMachine, isSubstrateChain } from "../../../utils/substrate.helpers"
import { SUBSTRATE_RPC_URL } from "../../../constants"
import { RequestMetadata } from "../../../utils/state-machine.helper"

export async function handleSubstrateRequestEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw Ismp.Request Event on ${getHostStateMachine(chainId)}`)

	if (!event.event.data) return

	const [dest_chain, source_chain, request_nonce, commitment] = event.event.data

	logger.info(
		`Handling ISMP Request Event: ${JSON.stringify({
			source_chain,
			dest_chain,
			request_nonce,
			commitment,
		})}`,
	)

	const sourceId = formatChain(source_chain.toString())
	const destId = formatChain(dest_chain.toString())

	logger.info(
		`Chain Ids: ${JSON.stringify({
			sourceId,
			destId,
		})}`,
	)

	if (!isSubstrateChain(sourceId)) {
		logger.error(`Skipping hyperbridge aggregated request`)
		return
	}

	if (!SUBSTRATE_RPC_URL[sourceId]) {
		logger.error(`No WS URL found for chain ${sourceId}`)
		return
	}

	const method = {
		id: 1,
		jsonrpc: "2.0",
		method: "ismp_queryRequests",
		params: [[{ commitment: commitment.toString() }]],
	}

	const response = await fetch(SUBSTRATE_RPC_URL[sourceId], {
		method: "POST",
		headers: {
			accept: "application/json",
			"content-type": "application/json",
		},
		body: JSON.stringify(method),
	})
	const data = await response.json()

	if (data.result.length === 0) {
		logger.error(`No requests found for commitment ${commitment.toString()}`)
		return
	}

	// todo: support GET requests
	const postRequest = data.result[0].Post

	if (!postRequest) {
		logger.error(`Request not found for commitment ${commitment.toString()}`)
		return
	}

	const { body, from, to, nonce, timeoutTimestamp } = postRequest
	const prefix = toHex(":child_storage:default:ISMP")
	const key = bytesToHex(
		new Uint8Array([
			...new TextEncoder().encode("RequestCommitments"),
			...hexToBytes(commitment.toString() as any),
		]),
	)

	const metadataResponse = await fetch(SUBSTRATE_RPC_URL[sourceId], {
		method: "POST",
		headers: {
			accept: "application/json",
			"content-type": "application/json",
		},
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "childstate_getStorage",
			params: [prefix, key],
		}),
	})
	const storageValue = (await metadataResponse.json()).result as `0x${string}` | undefined

	let fee = BigInt(0)
	if (typeof storageValue === "string") {
		const metadata = RequestMetadata.dec(hexToBytes(storageValue))
		fee = BigInt(Number(metadata.fee.fee))
	}

	const host = getHostStateMachine(chainId)
	await RequestService.findOrCreate({
		chain: host,
		commitment: commitment.toString(),
		body,
		dest: destId,
		fee,
		from,
		nonce: BigInt(nonce),
		source: sourceId,
		timeoutTimestamp: BigInt(Number(timeoutTimestamp)),
		to,
		status: Status.SOURCE,
		blockNumber: event.block.block.header.number.toString(),
		blockHash: event.block.block.header.hash.toString(),
		transactionHash: event.extrinsic?.extrinsic.hash.toString() || "",
		blockTimestamp: BigInt(event.block?.timestamp!.getTime()),
	})
}
