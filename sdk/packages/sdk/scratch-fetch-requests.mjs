import { WsProvider } from "@polkadot/api"

const provider = new WsProvider("wss://gargantua.rpc.polytope.technology")
await provider.isReady

const commitments = [
	"0x6f8cc3e98194ca72e82bddb92c6c2f5c473c335192704c981c5e28fe49e1ee36",
	"0x4399d6a3590d9a7c770a3b4e531ab399f378a10a15199f7fa9831d3814f43f8c",
]
const res = await provider.send("ismp_queryRequests", [commitments.map((c) => ({ commitment: c }))])
console.log(JSON.stringify(res, null, 1))
await provider.disconnect()
process.exit(0)
