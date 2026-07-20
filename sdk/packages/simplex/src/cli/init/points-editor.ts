import { confirm, log, text } from "@clack/prompts"
import { guard } from "./prompt-utils"

/** Parses "amount,value" pair lines used by the curve/point editors. */
export function parsePointInput(input: string): { first: string; second: string } | null {
	const parts = input.split(",").map((part) => part.trim())
	if (parts.length !== 2 || !parts[0] || !parts[1]) return null
	if (!Number.isFinite(Number(parts[0])) || !Number.isFinite(Number(parts[1]))) return null
	return { first: parts[0], second: parts[1] }
}

export interface EditPointsOptions<P> {
	prompt: string
	minPoints: number
	initial?: P[]
	toPoint: (pair: { first: string; second: string }) => P
}

/** Loop collecting comma-separated point pairs until an empty line, enforcing a minimum count. */
export async function editPoints<P>(options: EditPointsOptions<P>): Promise<P[]> {
	const points: P[] = []
	if (options.initial?.length) {
		const keep = guard(
			await confirm({
				message: `Keep the ${options.initial.length} existing points and add more?`,
				initialValue: true,
			}),
		)
		if (keep) points.push(...options.initial)
	}
	for (;;) {
		const input = guard(
			await text({
				message: options.prompt,
				defaultValue: "",
				validate: (value) => {
					const trimmed = (value ?? "").trim()
					if (!trimmed) return undefined
					return parsePointInput(trimmed) ? undefined : "Expected two comma-separated numbers, e.g. `1000,50`"
				},
			}),
		)
		const trimmed = (input ?? "").trim()
		if (!trimmed) {
			if (points.length >= options.minPoints) return points
			log.error(`At least ${options.minPoints} point${options.minPoints > 1 ? "s" : ""} required.`)
			continue
		}
		points.push(options.toPoint(parsePointInput(trimmed)!))
	}
}
