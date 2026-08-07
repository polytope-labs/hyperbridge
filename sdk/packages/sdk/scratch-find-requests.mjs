import { ApiPromise, WsProvider } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"

const api = await ApiPromise.create({
	provider: new WsProvider("wss://gargantua.rpc.polytope.technology"),
	typesBundle: { spec: { gargantua: { hasher: keccakAsU8a } } },
})

const head = (await api.rpc.chain.getHeader()).number.toNumber()
console.log("head:", head)

const SCAN = 2400
const CONC = 25
let found = []
for (let start = head; start > head - SCAN && found.length < 2; start -= CONC) {
	const nums = Array.from({ length: CONC }, (_, i) => start - i).filter((n) => n > 0)
	const results = await Promise.all(
		nums.map(async (n) => {
			const hash = await api.rpc.chain.getBlockHash(n)
			const apiAt = await api.at(hash)
			const events = await apiAt.query.system.events()
			const hits = []
			for (const { event } of events) {
				if (event.section === "intentsCoprocessor" && event.method === "GatewayUpgradeInitiated")
					hits.push({ n, type: "upgrade", data: event.data.toHuman() })
				if (event.section === "ismp" && event.method === "Request") hits.push({ n, type: "request", data: event.data.toHuman() })
			}
			return hits
		}),
	)
	for (const hits of results) for (const h of hits) found.push(h)
	if (found.length) {
		// keep scanning this window fully but stop extending
	}
}
for (const f of found.sort((a, b) => a.n - b.n)) console.log(f.n, f.type, JSON.stringify(f.data))
await api.disconnect()
process.exit(0)
