/* tslint:disable */
/* eslint-disable */
/**
 * Initialize panic hook for better error messages in browser console
 */
export function init_panic_hook(): void;
/**
 * Boundary information for the entire Gerber layer
 */
export class Boundary {
  free(): void;
  [Symbol.dispose](): void;
  constructor(min_x: number, max_x: number, min_y: number, max_y: number);
  readonly max_x: number;
  readonly max_y: number;
  readonly min_x: number;
  readonly min_y: number;
}
/**
 * Main Gerber processor with stateful WebGL renderer
 */
export class GerberProcessor {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get the boundary of the parsed Gerber data for fitToView
   *
   * # Returns
   * * `Boundary` containing min/max x/y coordinates
   *
   * # Errors
   * * Returns error if parse() has not been called yet
   */
  get_boundary(): Boundary;
  /**
   * Remove a layer from the renderer
   *
   * # Arguments
   * * `layer_id` - Layer ID returned from add_layer()
   *
   * # Returns
   * * `"remove_done"` signal on success
   */
  remove_layer(layer_id: number): string;
  /**
   * Create a new GerberProcessor instance
   */
  constructor();
  /**
   * Initialize with WebGL 2.0 context
   *
   * # Arguments
   * * `gl` - WebGL 2.0 rendering context from canvas
   *
   * # Returns
   * * `"init_done"` signal on success
   */
  init(gl: WebGL2RenderingContext): string;
  /**
   * Clear all layers
   *
   * # Returns
   * * `"clear_done"` signal on success
   */
  clear(): string;
  /**
   * DEPRECATED: Use add_layer() instead
   * Parse Gerber file data and create renderer
   *
   * # Arguments
   * * `content` - Gerber file content as string
   *
   * # Returns
   * * `"parse_done"` signal on success
   */
  parse(content: string): string;
  /**
   * Render geometry to FBOs and composite to canvas
   *
   * # Arguments
   * * `active_layer_ids` - Array of layer IDs to render (in order)
   * * `color_data` - Flat array of [r, g, b] for each active layer (NO alpha)
   * * `zoom_x` - Horizontal zoom factor
   * * `zoom_y` - Vertical zoom factor
   * * `offset_x` - Horizontal pan offset
   * * `offset_y` - Vertical pan offset
   * * `alpha` - Global alpha for all layers
   *
   * # Returns
   * * `"render_done"` signal on success
   */
  render(active_layer_ids: Uint32Array, color_data: Float32Array, zoom_x: number, zoom_y: number, offset_x: number, offset_y: number, alpha: number): string;
  /**
   * Resize framebuffers when canvas dimensions change (e.g., fullscreen)
   *
   * # Returns
   * * `"resize_done"` signal on success
   *
   * # Errors
   * * Returns error if renderer is not initialized
   */
  resize(): string;
  /**
   * Add a new layer to the renderer
   *
   * # Arguments
   * * `content` - Gerber file content as string
   *
   * # Returns
   * * Layer ID (u32) for tracking this layer
   */
  add_layer(content: string): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_gerberprocessor_free: (a: number, b: number) => void;
  readonly gerberprocessor_add_layer: (a: number, b: number, c: number) => [number, number, number];
  readonly gerberprocessor_clear: (a: number) => [number, number, number, number];
  readonly gerberprocessor_get_boundary: (a: number) => [number, number, number];
  readonly gerberprocessor_init: (a: number, b: any) => [number, number, number, number];
  readonly gerberprocessor_new: () => number;
  readonly gerberprocessor_parse: (a: number, b: number, c: number) => [number, number, number, number];
  readonly gerberprocessor_remove_layer: (a: number, b: number) => [number, number, number, number];
  readonly gerberprocessor_render: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number, number];
  readonly gerberprocessor_resize: (a: number) => [number, number, number, number];
  readonly init_panic_hook: () => void;
  readonly __wbg_boundary_free: (a: number, b: number) => void;
  readonly boundary_max_x: (a: number) => number;
  readonly boundary_max_y: (a: number) => number;
  readonly boundary_min_x: (a: number) => number;
  readonly boundary_min_y: (a: number) => number;
  readonly boundary_new: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
