import { createServer, type Server } from "node:http"
import { encodeAbiParameters } from "viem"

export interface MockRpcOptions {
	chainId?: number
	/** eth_getCode result; "0x" simulates no contract. */
	code?: string
	/** ERC-20 metadata answered via eth_call. */
	symbol?: string
	decimals?: number
	entryPoints?: string[]
}

export interface MockRpc {
	url: string
	close(): void
	requests: Array<{ method: string; params: unknown[] }>
}

const SYMBOL_SELECTOR = "0x95d89b41"
const DECIMALS_SELECTOR = "0x313ce567"

/** Local JSON-RPC stub answering the calls the setup wizard makes. */
export function startMockRpc(options: MockRpcOptions = {}): Promise<MockRpc> {
	const chainId = options.chainId ?? 1
	const requests: MockRpc["requests"] = []

	const server: Server = createServer((req, res) => {
		let body = ""
		req.on("data", (chunk) => {
			body += chunk
		})
		req.on("end", () => {
			const { method, params, id } = JSON.parse(body) as { method: string; params: unknown[]; id: number }
			requests.push({ method, params })

			let result: unknown
			switch (method) {
				case "eth_chainId":
					result = `0x${chainId.toString(16)}`
					break
				case "eth_getCode":
					result = options.code ?? "0x6001"
					break
				case "eth_call": {
					const data = (params[0] as { data?: string })?.data ?? ""
					if (data.startsWith(SYMBOL_SELECTOR)) {
						result = encodeAbiParameters([{ type: "string" }], [options.symbol ?? "MOCK"])
					} else if (data.startsWith(DECIMALS_SELECTOR)) {
						result = encodeAbiParameters([{ type: "uint8" }], [options.decimals ?? 18])
					} else {
						result = "0x"
					}
					break
				}
				case "eth_supportedEntryPoints":
					result = options.entryPoints ?? ["0x0000000071727De22E5E9d8BAf0edAc6f37da032"]
					break
				default:
					res.writeHead(200, { "Content-Type": "application/json" })
					res.end(JSON.stringify({ jsonrpc: "2.0", id, error: { code: -32601, message: `no ${method}` } }))
					return
			}
			res.writeHead(200, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ jsonrpc: "2.0", id, result }))
		})
	})

	return new Promise((resolve) => {
		server.listen(0, "127.0.0.1", () => {
			const address = server.address()
			const port = typeof address === "object" && address !== null ? address.port : 0
			resolve({
				url: `http://127.0.0.1:${port}`,
				close: () => server.close(),
				requests,
			})
		})
	})
}
