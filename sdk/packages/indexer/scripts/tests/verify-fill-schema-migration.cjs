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
const entities = [
	"IOrderV3",
	"IOrderV3Fill",
	"IOrderV3PartialFill",
	"IOrderV3FillOutputAsset",
	"IOrderV3PartialFillOutputAsset",
]
const additions = {
	IOrderV3: ["feeToken", "feeTokenDecimals", "userOpHash"],
	IOrderV3Fill: ["userOpHash"],
	IOrderV3PartialFill: ["userOpHash"],
	IOrderV3FillOutputAsset: ["amountReceived"],
	IOrderV3PartialFillOutputAsset: ["amountReceived"],
}
;(async () => {
	const admin = connection()
	await admin.createSchema(schemaName)
	await admin.close()
	const first = await boot(baseline)
	for (const entity of entities) {
		const model = first.model(entity)
		const seed = { id: "existing" }
		for (const [field, attr] of Object.entries(model.rawAttributes)) {
			if (field === "id" || attr.allowNull !== false) continue
			const kind = [
				"deadline",
				"nonce",
				"fees",
				"inputUSD",
				"blockNumber",
				"blockTimestamp",
				"timestamp",
				"amount",
				"index",
			].includes(field)
				? "BIGINT"
				: field === "createdAt"
					? "DATE"
					: field === "status"
						? "ENUM"
						: "STRING"
			seed[field] = field.endsWith("Id")
				? "existing"
				: kind === "ENUM"
					? "PLACED"
					: kind === "DATE"
						? new Date("2026-09-01")
						: ["BIGINT", "INTEGER", "DECIMAL"].includes(kind)
							? "100"
							: kind === "BOOLEAN"
								? false
								: "existing"
		}
		await model.create(seed)
	}
	const originals = {}
	for (const entity of entities) originals[entity] = await first.model(entity).findByPk("existing", { raw: true })
	await first.close()
	const second = await boot(next)
	for (const entity of entities) {
		const model = second.model(entity)
		const row = await model.findByPk("existing", { raw: true })
		for (const [field, value] of Object.entries(originals[entity]))
			assert.deepEqual(row[field], value, `${entity}.${field} preserved`)
		for (const field of additions[entity]) assert.equal(row[field], null, `${entity}.${field} defaults null`)
		const added = Object.fromEntries(
			additions[entity].map((field) => [
				field,
				field === "feeTokenDecimals"
					? 6
					: field === "amountReceived"
						? "105"
						: "0x" + "ab".repeat(field === "userOpHash" ? 32 : 20),
			]),
		)
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
