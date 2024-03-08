let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}


const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b)
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}
function __wbg_adapter_32(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h779f85bbaadeee15(arg0, arg1);
}

function __wbg_adapter_35(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h65ddd0d8d0aebe40(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_38(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h76f6c031be756281(arg0, arg1);
}

function __wbg_adapter_45(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__hf63ee2387a1d1b72(arg0, arg1);
}

function __wbg_adapter_48(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h117477c3333ccafe(arg0, arg1, addHeapObject(arg2));
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8Memory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
    return instance.ptr;
}
/**
* Functions takes in a post request and returns one of the following json strings variants
* Status variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
* `DestinationDelivered`, `Timeout`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @returns {Promise<any>}
*/
export function query_request_status(request, config_js) {
    _assertClass(request, JsPost);
    var ptr0 = request.__destroy_into_raw();
    _assertClass(config_js, JsClientConfig);
    var ptr1 = config_js.__destroy_into_raw();
    const ret = wasm.query_request_status(ptr0, ptr1);
    return takeObject(ret);
}

/**
* Function takes in a post response and returns one of the following json strings variants
* Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`, `HyperbridgeFinalized`,
* `DestinationDelivered`, `Timeout`
* @param {JsResponse} response
* @param {JsClientConfig} config_js
* @returns {Promise<any>}
*/
export function query_response_status(response, config_js) {
    _assertClass(response, JsResponse);
    var ptr0 = response.__destroy_into_raw();
    _assertClass(config_js, JsClientConfig);
    var ptr1 = config_js.__destroy_into_raw();
    const ret = wasm.query_response_status(ptr0, ptr1);
    return takeObject(ret);
}

/**
* Accepts a post request that has timed out returns a stream that yields the following json
* strings variants Status Variants: `Pending`, `DestinationFinalized`, `HyperbridgeTimedout`,
* `HyperbridgeFinalized`, `{ "TimeoutMessage": [...] }`. This function will not check if the
* request has timed out, only call it when sure that the request has timed out after calling
* `query_request_status`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @returns {Promise<ReadableStream>}
*/
export function timeout_post_request(request, config_js) {
    _assertClass(request, JsPost);
    var ptr0 = request.__destroy_into_raw();
    _assertClass(config_js, JsClientConfig);
    var ptr1 = config_js.__destroy_into_raw();
    const ret = wasm.timeout_post_request(ptr0, ptr1);
    return takeObject(ret);
}

/**
* Races between a timeout stream and request processing stream, and yields the following json
* strings variants Status Variants: `Pending`, `SourceFinalized`, `HyperbridgeDelivered`,
* `HyperbridgeFinalized`, `DestinationDelivered`, `Timeout`
* @param {JsPost} request
* @param {JsClientConfig} config_js
* @param {bigint} post_request_height
* @returns {Promise<ReadableStream>}
*/
export function subscribe_to_request_status(request, config_js, post_request_height) {
    _assertClass(request, JsPost);
    var ptr0 = request.__destroy_into_raw();
    _assertClass(config_js, JsClientConfig);
    var ptr1 = config_js.__destroy_into_raw();
    const ret = wasm.subscribe_to_request_status(ptr0, ptr1, post_request_height);
    return takeObject(ret);
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_229(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h2565e20f67d19d70(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr);
    }
    /**
    * @returns {string}
    */
    get type() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.intounderlyingbytesource_type(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @returns {number}
    */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @param {ReadableByteStreamController} controller
    */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
    * @param {ReadableByteStreamController} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr);
    }
    /**
    * @param {any} chunk
    * @returns {Promise<any>}
    */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, addHeapObject(chunk));
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} reason
    * @returns {Promise<any>}
    */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, addHeapObject(reason));
        return takeObject(ret);
    }
}

const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0));
/**
*/
export class IntoUnderlyingSource {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(IntoUnderlyingSource.prototype);
        obj.__wbg_ptr = ptr;
        IntoUnderlyingSourceFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr);
    }
    /**
    * @param {ReadableStreamDefaultController} controller
    * @returns {Promise<any>}
    */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
    */
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
}

const JsChainConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jschainconfig_free(ptr >>> 0));
/**
*/
export class JsChainConfig {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsChainConfig.prototype);
        obj.__wbg_ptr = ptr;
        JsChainConfigFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsChainConfigFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jschainconfig_free(ptr);
    }
    /**
    * @returns {string}
    */
    get rpc_url() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_rpc_url(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @param {string} arg0
    */
    set rpc_url(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_rpc_url(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get state_machine() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_state_machine(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @param {string} arg0
    */
    set state_machine(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_state_machine(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {Uint8Array}
    */
    get host_address() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_host_address(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} arg0
    */
    set host_address(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_host_address(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {Uint8Array}
    */
    get handler_address() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_handler_address(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} arg0
    */
    set handler_address(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_handler_address(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {Uint8Array}
    */
    get consensus_state_id() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_consensus_state_id(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} arg0
    */
    set consensus_state_id(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_consensus_state_id(this.__wbg_ptr, ptr0, len0);
    }
}

const JsClientConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsclientconfig_free(ptr >>> 0));
/**
*/
export class JsClientConfig {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsClientConfigFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsclientconfig_free(ptr);
    }
    /**
    * @returns {JsChainConfig}
    */
    get source() {
        const ret = wasm.__wbg_get_jsclientconfig_source(this.__wbg_ptr);
        return JsChainConfig.__wrap(ret);
    }
    /**
    * @param {JsChainConfig} arg0
    */
    set source(arg0) {
        _assertClass(arg0, JsChainConfig);
        var ptr0 = arg0.__destroy_into_raw();
        wasm.__wbg_set_jsclientconfig_source(this.__wbg_ptr, ptr0);
    }
    /**
    * @returns {JsChainConfig}
    */
    get dest() {
        const ret = wasm.__wbg_get_jsclientconfig_dest(this.__wbg_ptr);
        return JsChainConfig.__wrap(ret);
    }
    /**
    * @param {JsChainConfig} arg0
    */
    set dest(arg0) {
        _assertClass(arg0, JsChainConfig);
        var ptr0 = arg0.__destroy_into_raw();
        wasm.__wbg_set_jsclientconfig_dest(this.__wbg_ptr, ptr0);
    }
    /**
    * @returns {JsHyperbridgeConfig}
    */
    get hyperbridge() {
        const ret = wasm.__wbg_get_jsclientconfig_hyperbridge(this.__wbg_ptr);
        return JsHyperbridgeConfig.__wrap(ret);
    }
    /**
    * @param {JsHyperbridgeConfig} arg0
    */
    set hyperbridge(arg0) {
        _assertClass(arg0, JsHyperbridgeConfig);
        var ptr0 = arg0.__destroy_into_raw();
        wasm.__wbg_set_jsclientconfig_hyperbridge(this.__wbg_ptr, ptr0);
    }
}

const JsHyperbridgeConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jshyperbridgeconfig_free(ptr >>> 0));
/**
*/
export class JsHyperbridgeConfig {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsHyperbridgeConfig.prototype);
        obj.__wbg_ptr = ptr;
        JsHyperbridgeConfigFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsHyperbridgeConfigFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jshyperbridgeconfig_free(ptr);
    }
    /**
    * @returns {string}
    */
    get rpc_url() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jschainconfig_rpc_url(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * @param {string} arg0
    */
    set rpc_url(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jschainconfig_rpc_url(this.__wbg_ptr, ptr0, len0);
    }
}

const JsPostFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jspost_free(ptr >>> 0));
/**
*/
export class JsPost {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsPost.prototype);
        obj.__wbg_ptr = ptr;
        JsPostFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsPostFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jspost_free(ptr);
    }
    /**
    * The source state machine of this request.
    * @returns {string}
    */
    get source() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jspost_source(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * The source state machine of this request.
    * @param {string} arg0
    */
    set source(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jspost_source(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * The destination state machine of this request.
    * @returns {string}
    */
    get dest() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jspost_dest(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
    * The destination state machine of this request.
    * @param {string} arg0
    */
    set dest(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jspost_dest(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * The nonce of this request on the source chain
    * @returns {bigint}
    */
    get nonce() {
        const ret = wasm.__wbg_get_jspost_nonce(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The nonce of this request on the source chain
    * @param {bigint} arg0
    */
    set nonce(arg0) {
        wasm.__wbg_set_jspost_nonce(this.__wbg_ptr, arg0);
    }
    /**
    * Module Id of the sending module
    * @returns {Uint8Array}
    */
    get from() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jspost_from(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Module Id of the sending module
    * @param {Uint8Array} arg0
    */
    set from(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jspost_from(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * Module ID of the receiving module
    * @returns {Uint8Array}
    */
    get to() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jspost_to(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Module ID of the receiving module
    * @param {Uint8Array} arg0
    */
    set to(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jspost_to(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * Timestamp which this request expires in seconds.
    * @returns {bigint}
    */
    get timeout_timestamp() {
        const ret = wasm.__wbg_get_jspost_timeout_timestamp(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Timestamp which this request expires in seconds.
    * @param {bigint} arg0
    */
    set timeout_timestamp(arg0) {
        wasm.__wbg_set_jspost_timeout_timestamp(this.__wbg_ptr, arg0);
    }
    /**
    * Encoded Request.
    * @returns {Uint8Array}
    */
    get data() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jspost_data(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Encoded Request.
    * @param {Uint8Array} arg0
    */
    set data(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jspost_data(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * Gas limit for executing the request on destination
    * This value should be zero if destination module is not a contract
    * @returns {bigint}
    */
    get gas_limit() {
        const ret = wasm.__wbg_get_jspost_gas_limit(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Gas limit for executing the request on destination
    * This value should be zero if destination module is not a contract
    * @param {bigint} arg0
    */
    set gas_limit(arg0) {
        wasm.__wbg_set_jspost_gas_limit(this.__wbg_ptr, arg0);
    }
}

const JsResponseFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsresponse_free(ptr >>> 0));
/**
*/
export class JsResponse {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsResponseFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsresponse_free(ptr);
    }
    /**
    * The request that triggered this response.
    * @returns {JsPost}
    */
    get post() {
        const ret = wasm.__wbg_get_jsresponse_post(this.__wbg_ptr);
        return JsPost.__wrap(ret);
    }
    /**
    * The request that triggered this response.
    * @param {JsPost} arg0
    */
    set post(arg0) {
        _assertClass(arg0, JsPost);
        var ptr0 = arg0.__destroy_into_raw();
        wasm.__wbg_set_jsresponse_post(this.__wbg_ptr, ptr0);
    }
    /**
    * The response message.
    * @returns {Uint8Array}
    */
    get response() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_jsresponse_response(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v1 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1, 1);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * The response message.
    * @param {Uint8Array} arg0
    */
    set response(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jsresponse_response(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * Timestamp at which this response expires in seconds.
    * @returns {bigint}
    */
    get timeout_timestamp() {
        const ret = wasm.__wbg_get_jspost_nonce(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Timestamp at which this response expires in seconds.
    * @param {bigint} arg0
    */
    set timeout_timestamp(arg0) {
        wasm.__wbg_set_jspost_nonce(this.__wbg_ptr, arg0);
    }
    /**
    * Gas limit for executing the response on destination, only used for solidity modules.
    * @returns {bigint}
    */
    get gas_limit() {
        const ret = wasm.__wbg_get_jspost_timeout_timestamp(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Gas limit for executing the response on destination, only used for solidity modules.
    * @param {bigint} arg0
    */
    set gas_limit(arg0) {
        wasm.__wbg_set_jspost_timeout_timestamp(this.__wbg_ptr, arg0);
    }
}

export function __wbindgen_object_drop_ref(arg0) {
    takeObject(arg0);
};

export function __wbindgen_cb_drop(arg0) {
    const obj = takeObject(arg0).original;
    if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
    }
    const ret = false;
    return ret;
};

export function __wbindgen_as_number(arg0) {
    const ret = +getObject(arg0);
    return ret;
};

export function __wbindgen_object_clone_ref(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
};

export function __wbindgen_string_new(arg0, arg1) {
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

export function __wbindgen_error_new(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbindgen_is_object(arg0) {
    const val = getObject(arg0);
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
};

export function __wbindgen_string_get(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbindgen_number_new(arg0) {
    const ret = arg0;
    return addHeapObject(ret);
};

export function __wbg_set_f975102236d3c502(arg0, arg1, arg2) {
    getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
};

export function __wbg_newwithintounderlyingsource_a03a82aa1bbbb292(arg0, arg1) {
    const ret = new ReadableStream(IntoUnderlyingSource.__wrap(arg0), takeObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_sethighWaterMark_ea50ed3ec2143088(arg0, arg1) {
    getObject(arg0).highWaterMark = arg1;
};

export function __wbindgen_is_string(arg0) {
    const ret = typeof(getObject(arg0)) === 'string';
    return ret;
};

export function __wbg_setTimeout_75cb9b6991a4031d() { return handleError(function (arg0, arg1) {
    const ret = setTimeout(getObject(arg0), arg1);
    return addHeapObject(ret);
}, arguments) };

export function __wbg_clearTimeout_76877dbc010e786d(arg0) {
    const ret = clearTimeout(takeObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_fetch_6a2624d7f767e331(arg0) {
    const ret = fetch(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_queueMicrotask_f61ee94ee663068b(arg0) {
    queueMicrotask(getObject(arg0));
};

export function __wbg_queueMicrotask_f82fc5d1e8f816ae(arg0) {
    const ret = getObject(arg0).queueMicrotask;
    return addHeapObject(ret);
};

export function __wbindgen_is_function(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
};

export function __wbg_instanceof_Window_cee7a886d55e7df5(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Window;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_performance_4ca1873776fdb3d2(arg0) {
    const ret = getObject(arg0).performance;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_setTimeout_6ed7182ebad5d297() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
    return ret;
}, arguments) };

export function __wbg_fetch_10edd7d7da150227(arg0, arg1) {
    const ret = getObject(arg0).fetch(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_wasClean_06aba8a282b21973(arg0) {
    const ret = getObject(arg0).wasClean;
    return ret;
};

export function __wbg_code_c25ac89aa8108189(arg0) {
    const ret = getObject(arg0).code;
    return ret;
};

export function __wbg_reason_ab96417c470b0f79(arg0, arg1) {
    const ret = getObject(arg1).reason;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbg_newwitheventinitdict_ff303f34f1b980fa() { return handleError(function (arg0, arg1, arg2) {
    const ret = new CloseEvent(getStringFromWasm0(arg0, arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_close_23aa806471e38253() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments) };

export function __wbg_enqueue_fe9e340e2bc8714b() { return handleError(function (arg0, arg1) {
    getObject(arg0).enqueue(getObject(arg1));
}, arguments) };

export function __wbg_instanceof_Response_b5451a06784a2404(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Response;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_url_e319aee56d26ddf1(arg0, arg1) {
    const ret = getObject(arg1).url;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbg_status_bea567d1049f0b6a(arg0) {
    const ret = getObject(arg0).status;
    return ret;
};

export function __wbg_headers_96d9457941f08a33(arg0) {
    const ret = getObject(arg0).headers;
    return addHeapObject(ret);
};

export function __wbg_arrayBuffer_eb2005809be09726() { return handleError(function (arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_data_bbdd2d77ab2f7e78(arg0) {
    const ret = getObject(arg0).data;
    return addHeapObject(ret);
};

export function __wbg_view_38a0bacb59ad00ee(arg0) {
    const ret = getObject(arg0).view;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_respond_fee44bba73c2fc8a() { return handleError(function (arg0, arg1) {
    getObject(arg0).respond(arg1 >>> 0);
}, arguments) };

export function __wbg_readyState_2599ffe07703eeea(arg0) {
    const ret = getObject(arg0).readyState;
    return ret;
};

export function __wbg_setbinaryType_bfaa2b91f5e49737(arg0, arg1) {
    getObject(arg0).binaryType = takeObject(arg1);
};

export function __wbg_new_d3ba66fcfe3ebcc6() { return handleError(function (arg0, arg1) {
    const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_close_85838c8d50b026da() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments) };

export function __wbg_send_115b7e92eb793bd9() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).send(getStringFromWasm0(arg1, arg2));
}, arguments) };

export function __wbg_send_8e8f1c88be375fc1() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).send(getArrayU8FromWasm0(arg1, arg2));
}, arguments) };

export function __wbg_addEventListener_f984e99465a6a7f4() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments) };

export function __wbg_addEventListener_bc4a7ad4cc72c6bf() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
}, arguments) };

export function __wbg_dispatchEvent_1dc222127c2ec453() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).dispatchEvent(getObject(arg1));
    return ret;
}, arguments) };

export function __wbg_removeEventListener_acfc154b998d806b() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments) };

export function __wbg_now_ef71656beb948bc8(arg0) {
    const ret = getObject(arg0).now();
    return ret;
};

export function __wbg_signal_8fbb4942ce477464(arg0) {
    const ret = getObject(arg0).signal;
    return addHeapObject(ret);
};

export function __wbg_new_92cc7d259297256c() { return handleError(function () {
    const ret = new AbortController();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_abort_510372063dd66b29(arg0) {
    getObject(arg0).abort();
};

export function __wbg_newwithstrandinit_11fbc38beb4c26b0() { return handleError(function (arg0, arg1, arg2) {
    const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_4db22fd5d40c5665() { return handleError(function () {
    const ret = new Headers();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_append_b2e8ed692fc5eb6e() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments) };

export function __wbg_byobRequest_643426f0037311f0(arg0) {
    const ret = getObject(arg0).byobRequest;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_close_0b618a762cdb578b() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments) };

export function __wbg_new_75208e29bddfd88c() {
    const ret = new Array();
    return addHeapObject(ret);
};

export function __wbg_newnoargs_cfecb3965268594c(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_next_586204376d2ed373(arg0) {
    const ret = getObject(arg0).next;
    return addHeapObject(ret);
};

export function __wbg_next_b2d3366343a208b3() { return handleError(function (arg0) {
    const ret = getObject(arg0).next();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_done_90b14d6f6eacc42f(arg0) {
    const ret = getObject(arg0).done;
    return ret;
};

export function __wbg_value_3158be908c80a75e(arg0) {
    const ret = getObject(arg0).value;
    return addHeapObject(ret);
};

export function __wbg_iterator_40027cdd598da26b() {
    const ret = Symbol.iterator;
    return addHeapObject(ret);
};

export function __wbg_get_3fddfed2c83f434c() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_call_3f093dd26d5569f8() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_632630b5cec17f21() {
    const ret = new Object();
    return addHeapObject(ret);
};

export function __wbg_self_05040bd9523805b9() { return handleError(function () {
    const ret = self.self;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_window_adc720039f2cb14f() { return handleError(function () {
    const ret = window.window;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_globalThis_622105db80c1457d() { return handleError(function () {
    const ret = globalThis.globalThis;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_global_f56b013ed9bcf359() { return handleError(function () {
    const ret = global.global;
    return addHeapObject(ret);
}, arguments) };

export function __wbindgen_is_undefined(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
};

export function __wbg_set_79c308ecd9a1d091(arg0, arg1, arg2) {
    getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
};

export function __wbg_instanceof_ArrayBuffer_9221fa854ffb71b5(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ArrayBuffer;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_instanceof_Error_5869c4f17aac9eb2(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Error;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_new_73a5987615ec8862(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_message_2a19bb5b62cf8e22(arg0) {
    const ret = getObject(arg0).message;
    return addHeapObject(ret);
};

export function __wbg_name_405bb0aa047a1bf5(arg0) {
    const ret = getObject(arg0).name;
    return addHeapObject(ret);
};

export function __wbg_toString_07f01913ec9af122(arg0) {
    const ret = getObject(arg0).toString();
    return addHeapObject(ret);
};

export function __wbg_call_67f2111acd2dfdb6() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_70828a4353259d4b(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return __wbg_adapter_229(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = state0.b = 0;
    }
};

export function __wbg_resolve_5da6faf2c96fd1d5(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_then_f9e58f5a50f43eae(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_then_20a5920e447d1cb1(arg0, arg1, arg2) {
    const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};

export function __wbg_buffer_b914fb8b50ebbc3e(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
};

export function __wbg_newwithbyteoffsetandlength_0de9ee56e9f6ee6e(arg0, arg1, arg2) {
    const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
    return addHeapObject(ret);
};

export function __wbg_new_b1f2d6842d615181(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_set_7d988c98e6ced92d(arg0, arg1, arg2) {
    getObject(arg0).set(getObject(arg1), arg2 >>> 0);
};

export function __wbg_length_21c4b0ae73cba59d(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

export function __wbg_buffer_67e624f5a0ab2319(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
};

export function __wbg_byteLength_4f4b58172d990c0a(arg0) {
    const ret = getObject(arg0).byteLength;
    return ret;
};

export function __wbg_byteOffset_adbd2a554609eb4e(arg0) {
    const ret = getObject(arg0).byteOffset;
    return ret;
};

export function __wbg_stringify_865daa6fb8c83d5a() { return handleError(function (arg0) {
    const ret = JSON.stringify(getObject(arg0));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_has_ad45eb020184f624() { return handleError(function (arg0, arg1) {
    const ret = Reflect.has(getObject(arg0), getObject(arg1));
    return ret;
}, arguments) };

export function __wbg_set_961700853a212a39() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
    return ret;
}, arguments) };

export function __wbindgen_debug_string(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};

export function __wbindgen_throw(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

export function __wbindgen_memory() {
    const ret = wasm.memory;
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper1890(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 737, __wbg_adapter_32);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2415(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_35);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2417(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_38);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2419(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_35);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2421(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_35);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper3411(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 1395, __wbg_adapter_45);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper3505(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 1450, __wbg_adapter_48);
    return addHeapObject(ret);
};

