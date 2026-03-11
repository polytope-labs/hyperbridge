import { URL } from "url"
// @ts-ignore
global.URL = URL

// Required for ethers to work in node
import "@ethersproject/shims"

// required for scale-ts
import "fast-text-encoding"

// @ts-ignore
import { logger } from "@subql/types-core"

//Exports all handler functions
export * from "@/mappings/mappingHandlers"
