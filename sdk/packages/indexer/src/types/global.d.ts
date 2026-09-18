import { Cache, Store } from "@subql/types-core"
import { Provider, Signer, providers } from "ethers"
import { Logger } from "@subql/types"
import { ApiPromise } from "@polkadot/api"

import "@types/node-fetch"

declare global {
	const store: Store
	/** Shared across every worker thread: the object itself lives on the main thread. */
	const cache: Cache
	const api: Provider | Signer | ApiPromise
	const unsafeApi: providers.JsonRpcProvider | undefined
	const logger: Logger
	const chainId: string
}

export {}
