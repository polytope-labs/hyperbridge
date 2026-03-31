import { readFileSync } from "node:fs"
import { join } from "node:path"
import { TextDecoder, TextEncoder } from "node:util"

let wasm

// new URL('.', import.meta.url).pathname doesn't work fine with Nextjs
// so I switched to this method of resolving the directory
const full_path = import.meta.url.split("/").slice(1)
const __dirname = `${full_path.slice(0, full_path.length - 1).join("/")}/`

let cachedTextDecoder = new TextDecoder("utf-8", { ignoreBOM: true, fatal: true })

cachedTextDecoder.decode()

let cachedUint8ArrayMemory0 = null

function getUint8ArrayMemory0() {
	if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
		cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer)
	}
	return cachedUint8ArrayMemory0
}

function getStringFromWasm0(ptr, len) {
	ptr = ptr >>> 0
	return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len))
}

let WASM_VECTOR_LEN = 0

let cachedTextEncoder = new TextEncoder("utf-8")

const encodeString =
	typeof cachedTextEncoder.encodeInto === "function"
		? function (arg, view) {
				return cachedTextEncoder.encodeInto(arg, view)
			}
		: function (arg, view) {
				const buf = cachedTextEncoder.encode(arg)
				view.set(buf)
				return {
					read: arg.length,
					written: buf.length,
				}
			}

function passStringToWasm0(arg, malloc, realloc) {
	if (realloc === undefined) {
		const buf = cachedTextEncoder.encode(arg)
		const ptr = malloc(buf.length, 1) >>> 0
		getUint8ArrayMemory0()
			.subarray(ptr, ptr + buf.length)
			.set(buf)
		WASM_VECTOR_LEN = buf.length
		return ptr
	}

	let len = arg.length
	let ptr = malloc(len, 1) >>> 0

	const mem = getUint8ArrayMemory0()

	let offset = 0

	for (; offset < len; offset++) {
		const code = arg.charCodeAt(offset)
		if (code > 0x7f) break
		mem[ptr + offset] = code
	}

	if (offset !== len) {
		if (offset !== 0) {
			arg = arg.slice(offset)
		}
		ptr = realloc(ptr, len, (len = offset + arg.length * 3), 1) >>> 0
		const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len)
		const ret = encodeString(arg, view)

		offset += ret.written
		ptr = realloc(ptr, len, offset, 1) >>> 0
	}

	WASM_VECTOR_LEN = offset
	return ptr
}

function isLikeNone(x) {
	return x === undefined || x === null
}

let cachedDataViewMemory0 = null

function getDataViewMemory0() {
	if (
		cachedDataViewMemory0 === null ||
		cachedDataViewMemory0.buffer.detached === true ||
		(cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)
	) {
		cachedDataViewMemory0 = new DataView(wasm.memory.buffer)
	}
	return cachedDataViewMemory0
}

function passArray8ToWasm0(arg, malloc) {
	const ptr = malloc(arg.length * 1, 1) >>> 0
	getUint8ArrayMemory0().set(arg, ptr / 1)
	WASM_VECTOR_LEN = arg.length
	return ptr
}

function takeFromExternrefTable0(idx) {
	const value = wasm.__wbindgen_export_0.get(idx)
	wasm.__externref_table_dealloc(idx)
	return value
}

/**
 * @param {Uint8Array} calldata_bytes
 * @param {bigint} tree_size
 * @returns {string}
 */
export function generate_root_with_proof(calldata_bytes, tree_size) {
	let deferred3_0
	let deferred3_1
	try {
		const ptr0 = passArray8ToWasm0(calldata_bytes, wasm.__wbindgen_malloc)
		const len0 = WASM_VECTOR_LEN
		const ret = wasm.generate_root_with_proof(ptr0, len0, tree_size)
		var ptr2 = ret[0]
		var len2 = ret[1]
		if (ret[3]) {
			ptr2 = 0
			len2 = 0
			throw takeFromExternrefTable0(ret[2])
		}
		deferred3_0 = ptr2
		deferred3_1 = len2
		return getStringFromWasm0(ptr2, len2)
	} finally {
		wasm.__wbindgen_free(deferred3_0, deferred3_1, 1)
	}
}

function addToExternrefTable0(obj) {
	const idx = wasm.__externref_table_alloc()
	wasm.__wbindgen_export_0.set(idx, obj)
	return idx
}

function passArrayJsValueToWasm0(array, malloc) {
	const ptr = malloc(array.length * 4, 4) >>> 0
	for (let i = 0; i < array.length; i++) {
		const add = addToExternrefTable0(array[i])
		getDataViewMemory0().setUint32(ptr + 4 * i, add, true)
	}
	WASM_VECTOR_LEN = array.length
	return ptr
}

/**
 * @param {string} root_hex
 * @param {string[]} proof_hex
 * @param {bigint} mmr_size
 * @param {bigint} leaf_position
 * @param {Uint8Array} calldata_bytes
 * @returns {boolean}
 */
export function verify_proof(root_hex, proof_hex, mmr_size, leaf_position, calldata_bytes) {
	const ptr0 = passStringToWasm0(root_hex, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc)
	const len0 = WASM_VECTOR_LEN
	const ptr1 = passArrayJsValueToWasm0(proof_hex, wasm.__wbindgen_malloc)
	const len1 = WASM_VECTOR_LEN
	const ptr2 = passArray8ToWasm0(calldata_bytes, wasm.__wbindgen_malloc)
	const len2 = WASM_VECTOR_LEN
	const ret = wasm.verify_proof(ptr0, len0, ptr1, len1, mmr_size, leaf_position, ptr2, len2)
	if (ret[2]) {
		throw takeFromExternrefTable0(ret[1])
	}
	return ret[0] !== 0
}

const KeccakMergeFinalization =
	typeof FinalizationRegistry === "undefined"
		? { register: () => {}, unregister: () => {} }
		: new FinalizationRegistry((ptr) => wasm.__wbg_keccakmerge_free(ptr >>> 0, 1))

export class KeccakMerge {
	__destroy_into_raw() {
		const ptr = this.__wbg_ptr
		this.__wbg_ptr = 0
		KeccakMergeFinalization.unregister(this)
		return ptr
	}

	free() {
		const ptr = this.__destroy_into_raw()
		wasm.__wbg_keccakmerge_free(ptr, 0)
	}
}

export function __wbindgen_error_new(arg0, arg1) {
	const ret = new Error(getStringFromWasm0(arg0, arg1))
	return ret
}

export function __wbindgen_init_externref_table() {
	const table = wasm.__wbindgen_export_0
	const offset = table.grow(4)
	table.set(0, undefined)
	table.set(offset + 0, undefined)
	table.set(offset + 1, null)
	table.set(offset + 2, true)
	table.set(offset + 3, false)
}

export function __wbindgen_string_get(arg0, arg1) {
	const obj = arg1
	const ret = typeof obj === "string" ? obj : undefined
	var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc)
	var len1 = WASM_VECTOR_LEN
	getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true)
	getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true)
}

export function __wbindgen_throw(arg0, arg1) {
	throw new Error(getStringFromWasm0(arg0, arg1))
}

// Load and initialize the WebAssembly module
const wasmPath = join(__dirname, "./node_bg.wasm")
const wasmBytes = readFileSync(wasmPath)

const bindings = {
	generate_root_with_proof,
	verify_proof,
	KeccakMerge,
	__wbindgen_error_new,
	__wbindgen_init_externref_table,
	__wbindgen_string_get,
	__wbindgen_throw,
}

const wasmModule = new WebAssembly.Module(wasmBytes)
const wasmInstance = new WebAssembly.Instance(wasmModule, {
	__wbindgen_placeholder__: bindings,
})
wasm = wasmInstance.exports
export const __wasm = wasm

wasm.__wbindgen_start()

export default function init() {
	console.log("CKB MMR WASM initialized")
}
