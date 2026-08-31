/** Page size for store reads that must not truncate. */
export const STORE_PAGE_SIZE = 100

/**
 * Reads a store query to exhaustion. The SubQuery store caps every read at its `limit`, so any
 * single call over an unbounded row set silently truncates; this pages until a short page proves
 * the set is drained. `fetchPage` must apply a stable order (e.g. `orderBy: "id"`) so paging
 * never skips or repeats rows.
 */
export async function readAllPages<T>(fetchPage: (limit: number, offset: number) => Promise<T[]>): Promise<T[]> {
	const rows: T[] = []
	for (let offset = 0; ; offset += STORE_PAGE_SIZE) {
		const page = await fetchPage(STORE_PAGE_SIZE, offset)
		rows.push(...page)
		if (page.length < STORE_PAGE_SIZE) return rows
	}
}
