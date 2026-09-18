import http from "http"
import type { AddressInfo } from "net"

import { safeFetch } from "@/utils/safeFetch"

test("abandons a request the server accepts but never answers", async () => {
	const server = http.createServer(() => {})
	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve))
	const { port } = server.address() as AddressInfo
	try {
		await expect(safeFetch(`http://127.0.0.1:${port}/`, { timeoutMs: 50 })).rejects.toThrow("timed out")
	} finally {
		server.closeAllConnections()
		await new Promise((resolve) => server.close(resolve))
	}
})
