import stringify from "safe-stable-stringify"

class EventDecodeError extends Error {
	constructor(message: string) {
		super(message)
		this.name = "EventDecodeError"
	}
}

/**
 * Creates a safe proxy wrapper around an event object that handles errors gracefully
 * @param event - The event object to wrap
 * @returns event | error
 */
export function getSafeEvent<T>(event: T): T & { args: object } {
	return new Proxy(event as T & { args: object }, {
		get(target: T & { args: object }, prop: string | symbol, receiver: T) {
			try {
				const value = target[prop as keyof (T & { args: object })]
				stringify(value)

				return value
			} catch (error) {
				logger.error(`Error accessing property '${String(prop)}' on event: ${stringify(error)}`)

				// @ts-expect-error
				throw new EventDecodeError(error.message)
			}
		},
	})
}

export function wrap<const T>(handler: (event: T) => Promise<void>) {
	return async (event: T) => {
		try {
			await handler(getSafeEvent(event))
		} catch (error) {
			if (error instanceof EventDecodeError) {
				logger.error(`Error decoding event: ${error.message}`)
				return
			}

			throw error
		}
	}
}
