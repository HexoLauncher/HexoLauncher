/* tslint:disable */
/* eslint-disable */

/**
 * One seed's world generation. Biome sources are built on first use per dimension and
 * reused, so keep one instance per seed rather than one per tile.
 */
export class SeedMap {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Biome id (e.g. `minecraft:cherry_grove`) of the 4×4×4 cell containing a block.
     */
    biomeAt(dimension: string, x: number, y: number, z: number): string;
    /**
     * Same grid as [`biome_tile`](Self::biome_tile), but one byte per pixel: an index into
     * [`biomeNames`] / [`biomeColors`]. Use it for hover info or your own palette.
     */
    biomeIds(dimension: string, x0: number, z0: number, width: number, height: number, scale: number): Uint8Array;
    /**
     * RGBA pixels (`width * height * 4` bytes) of a surface biome map, one pixel per
     * `scale` blocks, top-left pixel at block (`x0`, `z0`). Feed it to `ImageData`.
     */
    biomeTile(dimension: string, x0: number, z0: number, width: number, height: number, scale: number): Uint8Array;
    /**
     * Whether one chunk is a slime chunk.
     */
    isSlimeChunk(chunk_x: number, chunk_z: number): boolean;
    /**
     * `seed` is the text typed on the world creation screen (numbers are used as-is,
     * anything else is hashed like the game does). `version` defaults to the newest
     * supported, `preset` to `normal` (`large_biomes`, `amplified`).
     */
    constructor(seed: string, version?: string | null, preset?: string | null);
    /**
     * Slime chunks in a `width` × `height` chunk area starting at chunk (`chunk_x`,
     * `chunk_z`), row-major, 1 for a slime chunk. Mushroom fields never spawn slimes,
     * whatever this says.
     */
    slimeChunks(chunk_x: number, chunk_z: number, width: number, height: number): Uint8Array;
    /**
     * The first `limit` (max 128) stronghold positions, ring by ring:
     * `[{ chunkX, chunkZ, x, z }]` with `x`/`z` at the chunk centre.
     */
    strongholds(limit: number): any;
    /**
     * Structure sets of this version: `[{ name, dimension, structures }]`.
     */
    structureSets(): any;
    /**
     * Structures of one set whose start chunk lies in the block rectangle
     * (`x0`, `z0`)–(`x1`, `z1`): `[{ set, structure, chunkX, chunkZ, x, z }]`, where
     * `x`/`z` is what `/locate` reports. With `check_biomes`, placements in a biome the
     * structure cannot start in are dropped and `structure` names the variant (the check
     * is approximate); without it `structure` is `null`. Strongholds: use
     * [`strongholds`](Self::strongholds).
     */
    structures(set: string, x0: number, z0: number, x1: number, z1: number, check_biomes: boolean): any;
    /**
     * Numeric seed as a decimal string.
     */
    readonly seed: string;
    /**
     * Rule set in use: `1.18`, `1.19`, `1.20`, `1.21`, `1.21.4` or `26`.
     */
    readonly version: string;
}

/**
 * Map colours (`0xRRGGBB`) in index order for [`SeedMap::biome_ids`].
 */
export function biomeColors(): Uint32Array;

/**
 * Biome ids (`minecraft:…`) in index order for [`SeedMap::biome_ids`].
 */
export function biomeNames(): string[];

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_seedmap_free: (a: number, b: number) => void;
    readonly biomeColors: () => [number, number];
    readonly biomeNames: () => [number, number];
    readonly seedmap_biomeAt: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly seedmap_biomeIds: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly seedmap_biomeTile: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly seedmap_isSlimeChunk: (a: number, b: number, c: number) => number;
    readonly seedmap_new: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly seedmap_seed: (a: number) => [number, number];
    readonly seedmap_slimeChunks: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly seedmap_strongholds: (a: number, b: number) => [number, number, number];
    readonly seedmap_structureSets: (a: number) => [number, number, number];
    readonly seedmap_structures: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly seedmap_version: (a: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
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
