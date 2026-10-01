// Reads the ISMP messages out of a delivery transaction's calldata. Relayers deliver through the
// handler contract, and what a handler event wants from the calldata is which modules the delivered
// messages concern, so the transaction's ERC-20 transfers can be attributed to them.
import { Interface, Result } from "@ethersproject/abi"

import HandlerV2Abi from "@/configs/abis/HandlerV2.abi.json"

const handlerInterface = new Interface(HandlerV2Abi)

export interface HandlerCall {
	name: string
	args: Result
}

/**
 * The handler calls a transaction's calldata makes, in order. A `batchCall(bytes[])` is unwrapped
 * into the calls it carries — a relayer batches `handleConsensus` with the messages it proves — so
 * a batched delivery and a direct one read the same.
 *
 * Never throws. Calldata that is no handler call — a relayer delivering through a contract of its
 * own, or a handler newer than the bundled ABI — is no calls at all.
 */
export function decodeHandlerCalls(calldata: string | undefined): HandlerCall[] {
	if (!calldata) return []

	let call: HandlerCall
	try {
		call = handlerInterface.parseTransaction({ data: calldata })
	} catch {
		return []
	}
	if (call.name !== "batchCall") return [{ name: call.name, args: call.args }]

	return (call.args[0] as string[]).flatMap((inner) => decodeHandlerCalls(inner))
}

// Each takes the function's message argument.
const MESSAGE_MODULES = {
	// The modules the delivered requests are addressed to.
	handlePostRequests: (message: Result): string[] => message.requests.map((leaf: Result) => leaf.request.to),
	// The modules whose GET requests were answered.
	handleGetResponses: (message: Result): string[] =>
		message.responses.map((leaf: Result) => leaf.response.request.from),
	// The modules whose requests timed out.
	handlePostRequestTimeouts: (message: Result): string[] => message.timeouts.map((request: Result) => request.from),
	handleGetRequestTimeouts: (message: Result): string[] => message.timeouts.map((request: Result) => request.from),
}

export type HandlerMessageFunction = keyof typeof MESSAGE_MODULES

/**
 * The module addresses named by every `fn` call in a delivery transaction's calldata: the recipients
 * of delivered POST requests, or the senders of answered or timed-out requests.
 *
 * Never throws, because callers go on to index the transaction's transfers whether or not the
 * calldata could be read. Unreadable calldata is logged and answers with no modules, which costs
 * the caller only the per-contract attribution of those transfers.
 */
export function getHandlerMessageModules(calldata: string | undefined, fn: HandlerMessageFunction): string[] {
	if (!calldata) return []

	try {
		const calls = decodeHandlerCalls(calldata)
		if (calls.length === 0) {
			logger.warn(`[handler] Calldata with selector ${calldata.slice(0, 10)} is not a known handler call`)
		}

		return calls.filter(({ name }) => name === fn).flatMap(({ args }) => MESSAGE_MODULES[fn](args[1]))
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.warn(`[handler] Could not read ${fn} messages from calldata: ${message}`)
		return []
	}
}
