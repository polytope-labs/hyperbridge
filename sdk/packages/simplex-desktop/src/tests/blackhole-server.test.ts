import { once } from "node:events"
import { createConnection } from "node:net"
import { describe, expect, it } from "vitest"
import { blackholeServer } from "../../scripts/e2e/blackhole-server"

describe("Electron E2E blackhole server", () => {
	it("absorbs an expected client connection reset", async () => {
		const fixture = await blackholeServer()
		try {
			const client = createConnection(fixture.port, "127.0.0.1")
			await once(client, "connect")
			client.resetAndDestroy()
			await new Promise((resolveDelay) => setTimeout(resolveDelay, 25))
			expect(client.destroyed).toBe(true)
		} finally {
			await fixture.close()
		}
	})
})
