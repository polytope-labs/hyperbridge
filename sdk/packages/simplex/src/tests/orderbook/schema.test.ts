import { readFileSync } from "node:fs"
import { buildSchema, parse, validate } from "graphql"
import { describe, expect, it } from "vitest"
import { ORDERBOOK_DOCUMENTS } from "@/orderbook/client"
import { FAILURE_CODES, MESSAGE_REJECTION_CODES, REJECTION_CODES } from "@/orderbook/types"

/**
 * The documents we send, against the schema the orderbook publishes.
 *
 * `orderbook-schema.graphql` is copied from polytope-labs/hyperfx-orderbook.
 * Refresh it with
 * `gh api repos/polytope-labs/hyperfx-orderbook/contents/schema.graphql -q .content | base64 -d`.
 *
 * A copy that has fallen behind still passes everything below, which is the one
 * failure this file exists to prevent, so `.github/workflows/check-orderbook-schema.yml`
 * compares it with the server on a schedule and on any change to this package's
 * orderbook code.
 *
 * Every other test in this package answers the client from a stub that never
 * reads a query, so a field renamed on the server, an argument of the wrong
 * type, or a selection set on a union without its `... on` would pass all of
 * them and fail on the first real request.
 */

const schema = buildSchema(readFileSync(new URL("../fixtures/orderbook-schema.graphql", import.meta.url), "utf8"))

describe("the documents the client sends", () => {
	it.each(Object.entries(ORDERBOOK_DOCUMENTS))("%s is one the orderbook would accept", (_name, document) => {
		expect(validate(schema, parse(document)).map((error) => error.message)).toEqual([])
	})
})

describe("the codes we handle", () => {
	const enumValues = (name: string): string[] => {
		const type = schema.getType(name)
		if (!type || !("getValues" in type)) throw new Error(`${name} is not an enum in the schema`)
		return type.getValues().map((value) => value.name)
	}

	// Drift here is silent: an unknown code reaches the operator as a string on
	// the row either way, so nothing fails until someone reads the union and
	// believes it.
	it("covers every RejectionCode the schema declares", () => {
		expect([...REJECTION_CODES].sort()).toEqual(enumValues("RejectionCode").sort())
	})

	it("covers every MessageRejectionCode the schema declares", () => {
		expect([...MESSAGE_REJECTION_CODES].sort()).toEqual(enumValues("MessageRejectionCode").sort())
	})

	it("covers every FailureCode the schema declares", () => {
		expect([...FAILURE_CODES].sort()).toEqual(enumValues("FailureCode").sort())
	})
})
