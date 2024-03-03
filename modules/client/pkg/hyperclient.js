import * as wasm from "./hyperclient_bg.wasm";
import { __wbg_set_wasm } from "./hyperclient_bg.js";
__wbg_set_wasm(wasm);
export * from "./hyperclient_bg.js";
