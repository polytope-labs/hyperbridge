// Run in the deployed substrate image against a disposable database; see docs/ai/decisions/2026-09-13-placement-userop-attribution.md.
const { createRequire } = require("node:module")
// The released node image exposes its dependencies from /node_modules. The
// synthetic parent is intentional: this script is mounted into that image.
const requireRuntime = createRequire("/package.json")
const { EventEmitter2 } = requireRuntime("@nestjs/event-emitter")
const { buildSchemaFromString } = requireRuntime("@subql/utils")
const { Sequelize } = requireRuntime("@subql/x-sequelize")
global.__TEST__ = true
const { NodeConfig, StoreCacheService, StoreService } = requireRuntime("@subql/node-core")
const fs = require("node:fs")
const assert = require("node:assert/strict")
const baseline = fs.readFileSync(process.env.BASELINE_SCHEMA || "/evidence/baseline.graphql", "utf8")
const next = fs.readFileSync(process.env.NEXT_SCHEMA || "/schema.graphql", "utf8")
const schemaName = "pr1112_" + Date.now()
const config = new NodeConfig({ allowSchemaMigration: true, historical: false })
const connection = () =>
	new Sequelize(process.env.MIGRATION_DATABASE_URL || "postgresql://postgres:pr1112@pr1112-postgres:5432/pr1112", {
		logging: false,
	})
async function boot(sdl) {
	const sequelize = connection()
	const project = { schema: buildSchemaFromString(sdl), schemaSDL: sdl, network: { chainId: "test" } }
	const cache = new StoreCacheService(sequelize, config, new EventEmitter2())
	const store = new StoreService(sequelize, config, cache, project)
	await store.initCoreTables(schemaName)
	const tx = await sequelize.transaction()
	await store.init(schemaName, tx)
	await tx.commit()
	return sequelize
}
const address = "0x" + "ab".repeat(20)
const transactionHash = "0x" + "cd".repeat(32)
const fill = {
	id: "existing",
	orderId: "existing",
	chain: "56",
	filler: address,
	timestamp: "100",
	blockNumber: "100",
	transactionHash,
	createdAt: new Date("2026-09-01"),
}
const output = { id: "existing", token: address, amount: "100", index: 0 }
const seeds = {
	IOrderV3: {
		id: "existing",
		user: address,
		sourceChain: "EVM-1",
		destChain: "EVM-56",
		commitment: "existing",
		deadline: "100",
		nonce: "1",
		fees: "100",
		inputUSD: "100",
		status: "PLACED",
		predispatchCalldata: "0x",
		postDispatchCalldata: "0x",
		createdAt: new Date("2026-09-01"),
		blockNumber: "100",
		blockTimestamp: "100",
		transactionHash,
	},
	IOrderV3Fill: fill,
	IOrderV3PartialFill: fill,
	IOrderV3FillOutputAsset: { ...output, fillId: "existing" },
	IOrderV3PartialFillOutputAsset: { ...output, partialFillId: "existing", beneficiary: address },
}
const userOpHash = "0x" + "ef".repeat(32)
const additions = {
	IOrderV3: { feeToken: address, feeTokenDecimals: 6, userOpHash },
	IOrderV3Fill: { userOpHash },
	IOrderV3PartialFill: { userOpHash },
	IOrderV3FillOutputAsset: { amountReceived: "105" },
	IOrderV3PartialFillOutputAsset: { amountReceived: "105" },
}
const entities = Object.keys(seeds)
;(async () => {
	const admin = connection()
	await admin.createSchema(schemaName)
	await admin.close()
	const first = await boot(baseline)
	for (const entity of entities) await first.model(entity).create(seeds[entity])
	const originals = {}
	for (const entity of entities) originals[entity] = await first.model(entity).findByPk("existing", { raw: true })
	await first.close()
	const second = await boot(next)
	for (const entity of entities) {
		const model = second.model(entity)
		const row = await model.findByPk("existing", { raw: true })
		for (const [field, value] of Object.entries(originals[entity]))
			assert.deepEqual(row[field], value, `${entity}.${field} preserved`)
		const added = additions[entity]
		for (const field of Object.keys(added)) assert.equal(row[field], null, `${entity}.${field} defaults null`)
		await model.create({ ...originals[entity], id: "new", ...added })
		const fresh = await model.findByPk("new", { raw: true })
		for (const [field, value] of Object.entries(added)) assert.equal(String(fresh[field]), String(value))
	}
	await second.close()
	const third = await boot(next)
	for (const entity of entities) assert.equal(await third.model(entity).count(), 2)
	const [metadata] = await third.query(`SELECT value FROM ${schemaName}._metadata WHERE key='appliedSchemaSDL'`)
	assert.equal(metadata[0].value, next)
	await third.close()
	console.log(
		"PASS: baseline schema -> PR schema -> unchanged restart; all 5 affected tables preserve old rows, 7 nullable fields accept new data, appliedSchemaSDL advances.",
	)
})().catch((e) => {
	console.error(e)
	process.exit(1)
})
