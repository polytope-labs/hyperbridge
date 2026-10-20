#!/usr/bin/env node
// Copies the governance backfill into the main deployment's database at the cutover. Run it after the
// backfill has indexed up to the block below `governanceCutoverBlock` and before the main deployment
// reaches that block, with the DB_* settings the indexer nodes use and GOVERNANCE_DB_DATABASE naming
// the backfill's database on the same server:
//   ENV=mainnet tsx scripts/copy-governance-backfill.ts
import { blake2AsHex } from "@polkadot/util-crypto"
import pg from "pg"

import { getConfigs } from "../src/configs"

// The schemas generate-compose.ts gives the backfill and the main deployment
const BACKFILL_SCHEMA = "governance"
const MAIN_SCHEMA = "app"
const ROWS_PER_INSERT = 500

// The metadata table a multichain deployment keeps for a chain, named as SubQuery's getMetadataTableName does
const metadataTableName = (chainId: string) => `_metadata_${blake2AsHex(chainId, 64)}`.substring(0, 63)

const connect = async (database?: string) => {
	const db = new pg.Client({
		user: process.env.DB_USER,
		password: process.env.DB_PASS,
		database,
		host: process.env.DB_HOST,
		port: Number(process.env.DB_PORT),
	})
	await db.connect()
	return db
}

const lastProcessedHeight = async (db: pg.Client, schema: string, table: string) => {
	const { rows } = await db.query(`SELECT value FROM "${schema}"."${table}" WHERE key = 'lastProcessedHeight'`)
	return rows.length ? Number(rows[0].value) : undefined
}

const columns = async (db: pg.Client, schema: string, table: string) => {
	const { rows } = await db.query(
		`SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2`,
		[schema, table],
	)
	return new Set<string>(rows.map((row) => row.column_name))
}

async function main() {
	const chains = Object.entries(getConfigs()).flatMap(([chain, config]) =>
		config.type === "substrate" && config.governanceCutoverBlock !== undefined
			? [{ chain, chainId: config.chainId, cutover: config.governanceCutoverBlock }]
			: [],
	)
	if (chains.length !== 1) throw new Error(`Expected one chain with a governanceCutoverBlock, found ${chains.length}`)
	const [{ chain, chainId, cutover }] = chains
	if (!process.env.GOVERNANCE_DB_DATABASE) throw new Error("GOVERNANCE_DB_DATABASE is not set")

	const backfill = await connect(process.env.GOVERNANCE_DB_DATABASE)
	const main = await connect(process.env.DB_DATABASE)

	try {
		const backfilled = await lastProcessedHeight(backfill, BACKFILL_SCHEMA, "_metadata")
		if (backfilled === undefined || backfilled < cutover - 1) {
			throw new Error(`The ${chain} backfill is at ${backfilled}, it has to reach ${cutover - 1} first`)
		}
		const indexed = await lastProcessedHeight(main, MAIN_SCHEMA, metadataTableName(chainId))
		if (indexed === undefined || indexed >= cutover) {
			throw new Error(`The main ${chain} deployment is at ${indexed}, past the cutover at ${cutover}`)
		}

		const { rows: tables } = await backfill.query(
			`SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_name LIKE 'governance\\_%'`,
			[BACKFILL_SCHEMA],
		)

		await main.query("BEGIN")
		await main.query("SET CONSTRAINTS ALL DEFERRED")
		for (const { table_name: table } of tables) {
			const target = await columns(main, MAIN_SCHEMA, table)
			if (!target.size) throw new Error(`${MAIN_SCHEMA}.${table} does not exist`)
			const { rows } = await main.query(`SELECT EXISTS (SELECT 1 FROM "${MAIN_SCHEMA}"."${table}") AS filled`)
			if (rows[0].filled) throw new Error(`${MAIN_SCHEMA}.${table} already has rows`)

			const shared = [...(await columns(backfill, BACKFILL_SCHEMA, table))]
				.filter((column) => target.has(column) && column !== "_id" && column !== "_block_range")
				.map((column) => `"${column}"`)
			const into = [...shared]
			const values = [...shared]
			if (target.has("_id")) {
				into.push(`"_id"`)
				values.push("gen_random_uuid()")
			}
			// The backfill keeps no history, so each row is the current version, open from the start. The
			// range holds heights or, in a multichain schema, timestamps, and 0 is below either. The main
			// schema only rewinds above the cutover, so these rows are never rewound.
			if (target.has("_block_range")) {
				into.push(`"_block_range"`)
				values.push("int8range(0, NULL)")
			}

			// Rows travel between the databases as JSON text and are read back with the target table's
			// types. They stay text here, since parsing them would round balances to JavaScript numbers.
			const { rows: copied } = await backfill.query(
				`SELECT row_to_json(t)::text AS row FROM "${BACKFILL_SCHEMA}"."${table}" t ORDER BY id`,
			)
			for (let start = 0; start < copied.length; start += ROWS_PER_INSERT) {
				const batch = copied.slice(start, start + ROWS_PER_INSERT).map(({ row }) => row)
				await main.query(
					`INSERT INTO "${MAIN_SCHEMA}"."${table}" (${into.join(", ")})
					SELECT ${values.join(", ")} FROM json_populate_recordset(NULL::"${MAIN_SCHEMA}"."${table}", $1::json)`,
					[`[${batch.join(",")}]`],
				)
			}
			console.log(`Copied ${copied.length} rows into ${MAIN_SCHEMA}.${table}`)
		}
		await main.query("COMMIT")
		console.log(`The main ${chain} deployment can index governance from block ${cutover}`)
	} catch (error) {
		await main.query("ROLLBACK")
		throw error
	} finally {
		await backfill.end()
		await main.end()
	}
}

main().catch((error) => {
	console.error(error)
	process.exit(1)
})
