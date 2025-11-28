let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}


function isLikeNone(x) {
    return x === undefined || x === null;
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_1.set(idx, obj);
    return idx;
}

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let WASM_VECTOR_LEN = 0;

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

const cachedTextEncoder = new lTextEncoder('utf-8');

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
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

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
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;

function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

let cachedUint32ArrayMemory0 = null;

function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_export_1.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}
/**
 * Initialize panic hook for better error messages in browser console
 */
export function init_panic_hook() {
    wasm.init_panic_hook();
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
 * Triangulate a polygon with optional holes
 *
 * # Arguments
 * * `flat_vertices` - Flattened vertex coordinates [x1, y1, x2, y2, ...]
 * * `hole_indices` - Indices where holes start in the vertex array
 *
 * # Returns
 * * `TriangulationResult` containing triangulated vertices and indices
 * @param {Float32Array} flat_vertices
 * @param {Uint32Array} hole_indices
 * @returns {TriangulationResult}
 */
export function triangulate_polygon(flat_vertices, hole_indices) {
    const ptr0 = passArrayF32ToWasm0(flat_vertices, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray32ToWasm0(hole_indices, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.triangulate_polygon(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return TriangulationResult.__wrap(ret[0]);
}

const ArcsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_arcs_free(ptr >>> 0, 1));
/**
 * Arc primitive data structure
 */
export class Arcs {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Arcs.prototype);
        obj.__wbg_ptr = ptr;
        ArcsFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ArcsFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_arcs_free(ptr, 0);
    }
    /**
     * @param {Float32Array} data
     */
    constructor(data) {
        const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.arcs_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        ArcsFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Float32Array}
     */
    get data() {
        const ret = wasm.arcs_data(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}

const CirclesFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_circles_free(ptr >>> 0, 1));
/**
 * Circle primitive data structure
 */
export class Circles {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Circles.prototype);
        obj.__wbg_ptr = ptr;
        CirclesFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CirclesFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_circles_free(ptr, 0);
    }
    /**
     * @param {Float32Array} data
     */
    constructor(data) {
        const ptr0 = passArrayF32ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.arcs_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        CirclesFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Float32Array}
     */
    get data() {
        const ret = wasm.circles_data(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}

const GerberDataFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_gerberdata_free(ptr >>> 0, 1));
/**
 * Container for all parsed Gerber data
 */
export class GerberData {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(GerberData.prototype);
        obj.__wbg_ptr = ptr;
        GerberDataFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GerberDataFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gerberdata_free(ptr, 0);
    }
    /**
     * @param {Triangles} triangles
     * @param {Circles} circles
     * @param {Arcs} arcs
     */
    constructor(triangles, circles, arcs) {
        _assertClass(triangles, Triangles);
        var ptr0 = triangles.__destroy_into_raw();
        _assertClass(circles, Circles);
        var ptr1 = circles.__destroy_into_raw();
        _assertClass(arcs, Arcs);
        var ptr2 = arcs.__destroy_into_raw();
        const ret = wasm.gerberdata_new(ptr0, ptr1, ptr2);
        this.__wbg_ptr = ret >>> 0;
        GerberDataFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Arcs}
     */
    get arcs() {
        const ret = wasm.gerberdata_arcs(this.__wbg_ptr);
        return Arcs.__wrap(ret);
    }
    /**
     * @returns {Circles}
     */
    get circles() {
        const ret = wasm.gerberdata_circles(this.__wbg_ptr);
        return Circles.__wrap(ret);
    }
    /**
     * @returns {Triangles}
     */
    get triangles() {
        const ret = wasm.gerberdata_triangles(this.__wbg_ptr);
        return Triangles.__wrap(ret);
    }
}

const GerberProcessorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_gerberprocessor_free(ptr >>> 0, 1));
/**
 * Main Gerber processor with stateful WebGL renderer
 */
export class GerberProcessor {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        GerberProcessorFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gerberprocessor_free(ptr, 0);
    }
    /**
     * Get the parsed Gerber data for JavaScript rendering
     *
     * # Returns
     * * `GerberData` containing triangles, circles, and arcs
     *
     * # Errors
     * * Returns error if parse() has not been called yet
     * @returns {GerberData}
     */
    get_gerber_data() {
        const ret = wasm.gerberprocessor_get_gerber_data(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return GerberData.__wrap(ret[0]);
    }
    /**
     * Create a new GerberProcessor instance
     */
    constructor() {
        const ret = wasm.gerberprocessor_new();
        this.__wbg_ptr = ret >>> 0;
        GerberProcessorFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Initialize with WebGL 2.0 context
     *
     * # Arguments
     * * `gl` - WebGL 2.0 rendering context from canvas
     *
     * # Returns
     * * `"init_done"` signal on success
     * @param {WebGL2RenderingContext} gl
     * @returns {string}
     */
    init(gl) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.gerberprocessor_init(this.__wbg_ptr, gl);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Parse Gerber file data and create renderer
     *
     * # Arguments
     * * `content` - Gerber file content as string
     *
     * # Returns
     * * `"parse_done"` signal on success
     * @param {string} content
     * @returns {string}
     */
    parse(content) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(content, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.gerberprocessor_parse(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Render the stored Gerber data with camera transformations and color
     *
     * # Arguments
     * * `zoom_x` - Horizontal zoom factor
     * * `zoom_y` - Vertical zoom factor
     * * `offset_x` - Horizontal pan offset
     * * `offset_y` - Vertical pan offset
     * * `color_r` - Red component (0.0-1.0)
     * * `color_g` - Green component (0.0-1.0)
     * * `color_b` - Blue component (0.0-1.0)
     * * `alpha` - Alpha/transparency (0.0-1.0)
     *
     * # Returns
     * * `"render_done"` signal on success
     * @param {number} zoom_x
     * @param {number} zoom_y
     * @param {number} offset_x
     * @param {number} offset_y
     * @param {number} color_r
     * @param {number} color_g
     * @param {number} color_b
     * @param {number} alpha
     * @returns {string}
     */
    render(zoom_x, zoom_y, offset_x, offset_y, color_r, color_g, color_b, alpha) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.gerberprocessor_render(this.__wbg_ptr, zoom_x, zoom_y, offset_x, offset_y, color_r, color_g, color_b, alpha);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}

const TrianglesFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_triangles_free(ptr >>> 0, 1));
/**
 * Triangle mesh data structure
 */
export class Triangles {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Triangles.prototype);
        obj.__wbg_ptr = ptr;
        TrianglesFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TrianglesFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_triangles_free(ptr, 0);
    }
    /**
     * @param {Float32Array} vertices
     * @param {Uint32Array} indices
     */
    constructor(vertices, indices) {
        const ptr0 = passArrayF32ToWasm0(vertices, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray32ToWasm0(indices, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.triangles_new(ptr0, len0, ptr1, len1);
        this.__wbg_ptr = ret >>> 0;
        TrianglesFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Uint32Array}
     */
    get indices() {
        const ret = wasm.triangles_indices(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Float32Array}
     */
    get vertices() {
        const ret = wasm.triangles_vertices(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}

const TriangulationResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_triangulationresult_free(ptr >>> 0, 1));
/**
 * Triangulation result containing both vertices and triangle indices
 */
export class TriangulationResult {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(TriangulationResult.prototype);
        obj.__wbg_ptr = ptr;
        TriangulationResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        TriangulationResultFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_triangulationresult_free(ptr, 0);
    }
    /**
     * @returns {Float32Array}
     */
    get points() {
        const ret = wasm.triangulationresult_points(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {Uint32Array}
     */
    get indices() {
        const ret = wasm.triangulationresult_indices(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
}

export function __wbg_attachShader_7344865bad31ff22(arg0, arg1, arg2) {
    arg0.attachShader(arg1, arg2);
};

export function __wbg_bindBuffer_160bae3abde73dc9(arg0, arg1, arg2) {
    arg0.bindBuffer(arg1 >>> 0, arg2);
};

export function __wbg_blendEquation_2d9663ebd2e159ac(arg0, arg1) {
    arg0.blendEquation(arg1 >>> 0);
};

export function __wbg_blendFuncSeparate_7a59273bc74c9d28(arg0, arg1, arg2, arg3, arg4) {
    arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
};

export function __wbg_bufferData_55f80a5f5138c7d5(arg0, arg1, arg2, arg3) {
    arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
};

export function __wbg_canvas_f310c9cf2744a692(arg0) {
    const ret = arg0.canvas;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_clearColor_110dabf1000c7ee5(arg0, arg1, arg2, arg3, arg4) {
    arg0.clearColor(arg1, arg2, arg3, arg4);
};

export function __wbg_clear_26640723efdaebca(arg0, arg1) {
    arg0.clear(arg1 >>> 0);
};

export function __wbg_compileShader_c1b3393c7cdf1824(arg0, arg1) {
    arg0.compileShader(arg1);
};

export function __wbg_createBuffer_87d961c44d9d1d59(arg0) {
    const ret = arg0.createBuffer();
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_createProgram_5747e39a3fb4c26a(arg0) {
    const ret = arg0.createProgram();
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_createShader_d4dc332f2bbd2f36(arg0, arg1) {
    const ret = arg0.createShader(arg1 >>> 0);
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_drawArraysInstanced_49bdf242b7c02191(arg0, arg1, arg2, arg3, arg4) {
    arg0.drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
};

export function __wbg_drawElements_b02dae2f88405a4f(arg0, arg1, arg2, arg3, arg4) {
    arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
};

export function __wbg_enableVertexAttribArray_7dfcf4bc3485c8b0(arg0, arg1) {
    arg0.enableVertexAttribArray(arg1 >>> 0);
};

export function __wbg_enable_035cbf48e9eb2bc1(arg0, arg1) {
    arg0.enable(arg1 >>> 0);
};

export function __wbg_error_7534b8e9a36f1ab4(arg0, arg1) {
    let deferred0_0;
    let deferred0_1;
    try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
    }
};

export function __wbg_getAttribLocation_dd8019990a1caae9(arg0, arg1, arg2, arg3) {
    const ret = arg0.getAttribLocation(arg1, getStringFromWasm0(arg2, arg3));
    return ret;
};

export function __wbg_getProgramInfoLog_779f5c8ec0030b22(arg0, arg1, arg2) {
    const ret = arg1.getProgramInfoLog(arg2);
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbg_getProgramParameter_87fe6f7389891b2c(arg0, arg1, arg2) {
    const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
    return ret;
};

export function __wbg_getShaderInfoLog_7f3ba3f1801efedf(arg0, arg1, arg2) {
    const ret = arg1.getShaderInfoLog(arg2);
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbg_getShaderParameter_1c34688a40197ca2(arg0, arg1, arg2) {
    const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
    return ret;
};

export function __wbg_getUniformLocation_f12782d656fba732(arg0, arg1, arg2, arg3) {
    const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_height_e9e050efa83de7e3(arg0) {
    const ret = arg0.height;
    return ret;
};

export function __wbg_instanceof_HtmlCanvasElement_44db5e45fcfeae73(arg0) {
    let result;
    try {
        result = arg0 instanceof HTMLCanvasElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_linkProgram_9cbb85e4f519d443(arg0, arg1) {
    arg0.linkProgram(arg1);
};

export function __wbg_log_8316aef0a82fc256(arg0) {
    console.log(arg0);
};

export function __wbg_new_8a6f238a6ece86ea() {
    const ret = new Error();
    return ret;
};

export function __wbg_shaderSource_87c1621317e8b392(arg0, arg1, arg2, arg3) {
    arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
};

export function __wbg_stack_0ed75d68575b0f3c(arg0, arg1) {
    const ret = arg1.stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbg_uniform3fv_594f869bd296238a(arg0, arg1, arg2, arg3) {
    arg0.uniform3fv(arg1, getArrayF32FromWasm0(arg2, arg3));
};

export function __wbg_uniformMatrix3fv_ba6e099f1fed5ef6(arg0, arg1, arg2, arg3, arg4) {
    arg0.uniformMatrix3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
};

export function __wbg_useProgram_38b7a0c064bfecaf(arg0, arg1) {
    arg0.useProgram(arg1);
};

export function __wbg_vertexAttribDivisor_851cc4c4a4756d7d(arg0, arg1, arg2) {
    arg0.vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
};

export function __wbg_vertexAttribPointer_056bf27fa66523f6(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
};

export function __wbg_viewport_a850b219e6c203f5(arg0, arg1, arg2, arg3, arg4) {
    arg0.viewport(arg1, arg2, arg3, arg4);
};

export function __wbg_wbindgenbooleanget_527bfac1bf7c06df(arg0) {
    const v = arg0;
    const ret = typeof(v) === 'boolean' ? v : undefined;
    return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
};

export function __wbg_wbindgenthrow_681185b504fabc8e(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

export function __wbg_width_0e180d4c82487234(arg0) {
    const ret = arg0.width;
    return ret;
};

export function __wbindgen_cast_2241b6af4c4b2941(arg0, arg1) {
    // Cast intrinsic for `Ref(String) -> Externref`.
    const ret = getStringFromWasm0(arg0, arg1);
    return ret;
};

export function __wbindgen_cast_7c316abdc43840a3(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
    const ret = getArrayU32FromWasm0(arg0, arg1);
    return ret;
};

export function __wbindgen_cast_cd07b1914aa3d62c(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
    const ret = getArrayF32FromWasm0(arg0, arg1);
    return ret;
};

export function __wbindgen_init_externref_table() {
    const table = wasm.__wbindgen_export_1;
    const offset = table.grow(4);
    table.set(0, undefined);
    table.set(offset + 0, undefined);
    table.set(offset + 1, null);
    table.set(offset + 2, true);
    table.set(offset + 3, false);
    ;
};

