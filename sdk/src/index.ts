import { URL } from "url";
// @ts-ignore
global.URL = URL;

// Required for ethers to work in node
import "@ethersproject/shims";

// @ts-ignore
import { logger } from '@subql/types-core';

//Exports all handler functions
export * from "./mappings/mappingHandlers";

logger.info("Hello, world!");