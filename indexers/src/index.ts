import { URL } from "url";
// @ts-ignore
global.URL = URL;

// Required for ethers to work in node
import "@ethersproject/shims";

//Exports all handler functions
export * from "./mappings/mappingHandlers";
