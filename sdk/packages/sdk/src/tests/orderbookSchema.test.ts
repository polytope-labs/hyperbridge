import { readFileSync } from "node:fs"
import { buildSchema, parse, validate } from "graphql"
import { ORDERBOOK_QUERIES } from "@/protocols/intents/orderbook/client"

/**
 * The documents the SDK sends, and the examples the docs publish, against the schema the
 * orderbook serves.
 *
 * The schema is simplex's pinned copy of the orderbook's own; `schema.test.ts` there carries the
 * command that refreshes it. Every other orderbook test answers from a stub that never reads a
 * query, so a renamed field or a wrong argument type would pass them all and fail on the first
 * real request.
 */
const schema = buildSchema(
	readFileSync(new URL("../../../simplex/src/tests/fixtures/orderbook-schema.graphql", import.meta.url), "utf8"),
)

const docsPage = readFileSync(
	new URL("../../../../../docs/content/developers/evm/intent-gateway/orderbook.mdx", import.meta.url),
	"utf8",
)
const docsExamples = [...docsPage.matchAll(/```graphql\n([\s\S]*?)```/g)].map((match) => match[1])

describe("orderbook documents", () => {
	it.each(Object.entries(ORDERBOOK_QUERIES))("the SDK's %s query is one the orderbook accepts", (_name, document) => {
		expect(validate(schema, parse(document)).map((error) => error.message)).toEqual([])
	})

	it("finds the docs page's GraphQL examples", () => {
		expect(docsExamples.length).toBeGreaterThan(0)
	})

	it.each(docsExamples.map((example, index) => [index, example]))(
		"docs example %i is one the orderbook accepts",
		(_index, example) => {
			expect(validate(schema, parse(example)).map((error) => error.message)).toEqual([])
		},
	)
})
