import { createServer, type Socket } from "node:net"

/** Hold client connections open so configured startup remains in progress. */
export async function blackholeServer(): Promise<{ port: number; close: () => Promise<void> }> {
	const sockets = new Set<Socket>()
	const server = createServer((socket) => {
		sockets.add(socket)
		socket.on("close", () => sockets.delete(socket))
		// Windows reports a client that abandons a pending connection as a read
		// reset. That is expected for this deliberately non-responsive fixture.
		socket.on("error", (error: NodeJS.ErrnoException) => {
			if (error.code !== "ECONNRESET") throw error
		})
	})
	await new Promise<void>((resolveListen, reject) => {
		server.once("error", reject)
		server.listen(0, "127.0.0.1", resolveListen)
	})
	const address = server.address()
	if (!address || typeof address === "string") throw new Error("Blackhole server did not bind TCP")
	return {
		port: address.port,
		close: async () => {
			for (const socket of sockets) socket.destroy()
			await new Promise<void>((resolveClose) => server.close(() => resolveClose()))
		},
	}
}
