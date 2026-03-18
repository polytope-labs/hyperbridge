import { Store } from "@subql/types-core"
import { Provider, Signer, providers } from "ethers"
import { Logger } from "@subql/types"
import { ApiPromise } from "@polkadot/api"

import "@types/node-fetch"

declare global {
	const store: Store
	const api: Provider | Signer | ApiPromise
	const unsafeApi: providers.JsonRpcProvider | undefined
	const logger: Logger
	const chainId: string
}

export {}
