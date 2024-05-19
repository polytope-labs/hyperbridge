// Required for ethers to work in node
import "@ethersproject/shims";

//Exports all handler functions
export * from "./mappings/mappingHandlers";

import { URLSearchParams, URL } from "url";

// @ts-ignore
global.URLSearchParams = URLSearchParams;
// @ts-ignore
global.URL = URL;
