// The indexer's own userOpHash extraction, run on a bundle receipt for e2e/entrypoint-v09-local.mjs.
// It needs the indexer's tsconfig paths, so it runs as its own tsx process from that package.
// Reads `{ logs, sender, logIndex }` as JSON on stdin and prints `{ userOpHash }`.
const { findUserOpHash } = require("../../indexer/src/utils/userOp.helpers.ts")

let input = ""
process.stdin.on("data", (chunk) => {
	input += chunk
})
process.stdin.on("end", () => {
	const { logs, sender, logIndex } = JSON.parse(input)
	process.stdout.write(JSON.stringify({ userOpHash: findUserOpHash(logs, sender, logIndex) ?? null }))
})
