import type { Plugin } from "vite"

/**
 * A Vite plugin that copies the WebAssembly file from hyperbridge-sdk to the Vite cache directory.
 * This ensures the WASM file is available for browser imports when using Vite.
 * 
 * @returns {Plugin} A Vite plugin
 */
export function copyWasm(): Plugin

export default copyWasm
