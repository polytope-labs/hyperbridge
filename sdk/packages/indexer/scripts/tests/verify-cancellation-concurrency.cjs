// Run inside the deployed SubQuery image; see docs/ai/decisions/2026-09-12-cancellation-metadata-only.md.
const { createRequire } = require("node:module")
const runtime = createRequire("/package.json")
global.__TEST__ = true
global.logger = { info() {}, warn() {}, error() {}, debug() {} }
const { EventEmitter2 } = runtime("@nestjs/event-emitter")
const { buildSchemaFromString } = runtime("@subql/utils")
const { Sequelize } = runtime("@subql/x-sequelize")
const { NodeConfig, StoreCacheService, StoreService } = runtime("@subql/node-core")
const { CachedModel } = runtime("@subql/node-core/dist/indexer/storeModelProvider/model/cacheModel")
const { IntentGatewayV3Service: service } = require("/service.cjs")
const fs = require("node:fs")
const assert = require("node:assert/strict")

const sdl = fs.readFileSync("/schema.graphql", "utf8")
const schemaName = "cancellation_review_" + Date.now()
const historical = "timestamp"
const config = new NodeConfig({ allowSchemaMigration: true, historical, multiChain: true })
const connections = []
function connection() {
	const sequelize = new Sequelize(process.env.MIGRATION_DATABASE_URL, { logging: false })
	connections.push(sequelize)
	return sequelize
}
async function boot() {
	const sequelize = connection()
	const project = { schema: buildSchemaFromString(sdl), schemaSDL: sdl, network: { chainId: "test" } }
	const cache = new StoreCacheService(sequelize, config, new EventEmitter2())
	const store = new StoreService(sequelize, config, cache, project)
	await store.initCoreTables(schemaName)
	const tx = await sequelize.transaction()
	await store.init(schemaName, tx)
	await tx.commit()
	await store.setBlockHeader({ blockHeight: 100, blockHash: "test", timestamp: new Date(1000000) })
	return sequelize
}
function nodeStore(sequelize, unit = 1000000) {
	const models = new Map()
	let operation = 0
	function model(entity) {
		if (!models.has(entity))
			models.set(entity, new CachedModel(sequelize.model(entity), historical, config, () => ++operation))
		return models.get(entity)
	}
	return {
		get: (entity, id) => model(entity).get(id),
		set: (entity, id, data) => model(entity).set(id, data, unit),
		getByField: (entity, field, value, options) => model(entity).getByFields([[field, "=", value]], options),
		remove: (entity, id) => model(entity).remove(id, unit),
		async flush() {
			const tx = await sequelize.transaction()
			try {
				for (const model of models.values()) {
					if (model.isFlushable) await model.runFlush(tx, unit)
				}
				await tx.commit()
				for (const model of models.values()) model.clear(unit)
			} catch (error) {
				await tx.rollback()
				throw error
			}
		},
	}
}
const seed = (id) => ({
	id,
	user: "user",
	sourceChain: "EVM-8453",
	destChain: "EVM-56",
	commitment: id,
	deadline: 100n,
	nonce: 1n,
	fees: 0n,
	inputUSD: 0n,
	status: "PLACED",
	predispatchCalldata: "0x",
	postDispatchCalldata: "0x",
	createdAt: new Date("2026-09-12"),
	blockNumber: 1n,
	blockTimestamp: 1n,
	transactionHash: "placement",
})
const event = { transactionHash: "cancellation", blockNumber: 100, timestamp: 200n, logIndex: 7 }

async function place(sequelize, id) {
	const cache = nodeStore(sequelize, 900000)
	await cache.set("IOrderV3", id, seed(id))
	await cache.flush()
}

async function verify() {
	const admin = connection()
	await admin.query("CREATE EXTENSION IF NOT EXISTS btree_gist")
	await admin.createSchema(schemaName)
	try {
		const source = await boot()
		const destination = await boot()
		// Establish the failure mechanism using the runtime's real cache and DB upserts.
		await place(source, "unsafe")
		const stale = nodeStore(destination)
		const row = await stale.get("IOrderV3", "unsafe")
		row.status = "CANCELLED"
		await stale.set("IOrderV3", "unsafe", row)
		await source.model("IOrderV3").update({ status: "REFUNDED" }, { where: { id: "unsafe" } })
		await stale.flush()
		assert.equal((await source.model("IOrderV3").findOne({ where: { id: "unsafe" } })).status, "CANCELLED")

		for (const first of ["source", "destination"]) {
			const id = "flush-" + first
			await place(source, id)
			const sourceStore = nodeStore(source)
			const destStore = nodeStore(destination)
			// Destination keeps its own stale PLACED snapshot while the source refunds.
			await destStore.get("IOrderV3", id)
			global.store = sourceStore
			global.chainId = "8453"
			await service.updateOrderStatus(id, "REFUNDED", { ...event, transactionHash: "refund" })
			if (first === "source") await sourceStore.flush()
			assert.equal((await destStore.get("IOrderV3", id)).status, "PLACED")
			global.store = destStore
			global.chainId = "56"
			await service.recordOrderCancellation(id, "canceller", { ...event, transactionHash: id })
			await destStore.flush()
			if (first === "destination") await sourceStore.flush()
			assert.equal((await source.model("IOrderV3").findOne({ where: { id } })).status, "REFUNDED")
			assert.ok(await source.model("IOrderV3Cancellation").findOne({ where: { id: id + ".7" } }))
			assert.ok(await source.model("IOrderV3StatusMetadata").findOne({ where: { id: id + ".CANCELLED" } }))
			assert.ok(await source.model("IOrderV3StatusMetadata").findOne({ where: { id: id + ".REFUNDED" } }))
		}
		// A cancellation before placement keeps the existing pending-metadata policy.
		global.store = nodeStore(destination, 900000)
		await service.recordOrderCancellation("pending", "canceller", { ...event, transactionHash: "pending" })
		await global.store.flush()
		assert.ok(await source.model("PendingStatusMetadata").findOne({ where: { id: "pending.IOrderV3.CANCELLED" } }))
		await place(source, "pending")
		global.store = nodeStore(source)
		await service.flushPendingStatuses("pending")
		await global.store.flush()
		assert.equal((await source.model("IOrderV3").findOne({ where: { id: "pending" } })).status, "PLACED")
		assert.ok(await source.model("IOrderV3StatusMetadata").findOne({ where: { id: "pending.CANCELLED" } }))
		assert.equal(
			await source.model("PendingStatusMetadata").findOne({ where: { id: "pending.IOrderV3.CANCELLED" } }),
			null,
		)
		console.log(
			"PASS: stale whole-row write reproduces regression; real cancellation service preserves REFUNDED in both cache flush orders; pending metadata materializes without parent changes.",
		)
	} finally {
		await admin.dropSchema(schemaName, { cascade: true })
		await Promise.all(connections.map((connection) => connection.close()))
	}
}
verify().catch((error) => {
	console.error(error)
	process.exitCode = 1
})
