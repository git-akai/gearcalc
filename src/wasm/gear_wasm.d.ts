/* tslint:disable */
/* eslint-disable */

/**
 * **One member of a geartrain, as a gear tab would hold it.**
 *
 * `{ train, materials, stage, member }` JSON in — a train request with the
 * stage's index and the member's, in the order the stage's cards show them
 * — and `{ params, internal, cutter }` out: the tooth the stage cut that
 * member with, every automatic value resolved and every convention applied
 * (`GearResult::params`), whether it is a ring, and the pinion cutter that
 * cut it where it is. The gear tab **adopts** the member — a word chosen so
 * it cannot be mistaken for the TOML `import_train`, which reads a document
 * this tool wrote — and shows the tooth the stage rated rather than a
 * rebuild from the inputs.
 *
 * The train is solved here, in microseconds, because the tooth as built is
 * an output: a shift the stage chose, an addendum a tip width held down, a
 * helix shared out of a shaft angle. Nothing on the other side of the
 * boundary could know those, and nothing should try.
 *
 * # Errors
 *
 * A malformed request; a stage or member index the train does not have; a
 * train that has no answer, with its reason; and a **worm**, which is not a
 * gear the tab can hold — a thread's proportions are its own — and which the
 * panel lists greyed rather than omitted so a reader can see why it is not
 * offered.
 */
export function adopt_member(input: string): string;

export function default_materials(): string;

/**
 * The values a fresh tab starts at, as JSON.
 *
 * # Errors
 *
 * Only if the defaults cannot be encoded, which would be a build-time defect.
 */
export function defaults(): string;

/**
 * The gear as a DXF drawing, ready to be handed to the browser as a download.
 */
export function export_dxf(input: string): string;

/**
 * Export a material library: JSON in, TOML text out, ready for a download.
 */
export function export_materials(library_json: string): string;

/**
 * A ring's bore as DXF.
 *
 * # Errors
 *
 * A malformed request.
 */
export function export_ring_dxf(input: string): string;

/**
 * Export a geartrain: `{ name, train }` JSON in, TOML text out, ready for a
 * download.
 *
 * # Errors
 *
 * A malformed document, which would be a defect on this side of the boundary.
 */
export function export_train(document_json: string): string;

/**
 * The closed cross-section as a flat `[x0, y0, x1, y1, ...]` array, ready for
 * a canvas path. Flat rather than nested to keep the crossing cheap.
 */
export function gear_profile(input: string, points_per_tooth: number): Float64Array;

/**
 * Import a material library: TOML text in, JSON out.
 *
 * The TOML never reaches TypeScript — the browser reads a file as text and
 * hands it straight here, so exactly one parser exists and it is the tested
 * one. A malformed library returns the parser's own complaint, which names the
 * line, rather than a generic failure.
 */
export function import_materials(toml_text: string): string;

/**
 * Import a geartrain: TOML text in, `{ document: { name, train }, adjusted }`
 * JSON out.
 *
 * `adjusted` says whether any stage was relieved on the way in — a toggle
 * the file had given that no stage can honour, such as a crossed pair's
 * axial contact ratio, turned back automatic with its number kept. The panel
 * says so in one sentence; the values are the file's own throughout
 * (`gear_io::train`, *What is adjusted on import*).
 *
 * The same arrangement as the material library, and for the same reason: the
 * TOML never reaches TypeScript, so exactly one parser exists and it is the
 * tested one. A malformed file comes back as the parser's own complaint, which
 * names the line.
 *
 * The train's **inputs** are what the file holds; everything derived is
 * recomputed by `solve_train` once the tab exists. A stage may name a material
 * this library does not have — that is not an import failure, and `solve_train`
 * reports it by name.
 *
 * # Errors
 *
 * A document that is not a geartrain, or one with no stages.
 */
export function import_train(toml_text: string): string;

/**
 * The languages this build ships. See [`languages_impl`].
 *
 * # Errors
 *
 * Only if the list cannot be encoded, which would be a build-time defect.
 */
export function languages(): string;

/**
 * **A load case with its over-determined figures relieved.**
 *
 * `{ train, library, case, just }` JSON in — the train as it stands, its
 * materials, the case by index and the figure the designer has this moment
 * pinned (`null` where what changed was not a toggle) — and the case out,
 * with exactly the train's mobility of its speeds given and the torques one
 * statics equation short of the shafts that carry one, every figure relief
 * turned derived seeded from what the case comes to
 * ([`Train::relieve_case`]). The same relation [`relieve_stage`] keeps on a
 * stage's geometry, kept on a case's loads: a pair with a speed at each end
 * has asked for a contradiction, and the one not this moment pinned gives
 * way. A case with fewer given than that is left short — relief never
 * invents a given — and [`solve_train`] says so on the case.
 *
 * # Errors
 *
 * A malformed request, or a train whose shafts cannot be counted, which
 * [`solve_train`] would refuse the same way.
 */
export function relieve_case(input: string): string;

/**
 * **A stage with its over-determined inputs relieved.**
 *
 * `{ stage, just, figures }` JSON in — the stage as it now stands, the
 * [`Freedom`] the designer has this moment pinned (`null` where what changed
 * was not a toggle), and what the stage's inputs last came to
 * ([`TrainOutcome::figures`] for it) — and the corrected stage out, with
 * every box relief turned given seeded from its figure.
 *
 * A designer who pins a pair's distance *and* both its shifts has asked for a
 * contradiction: the three are bound by one relation, so one would have to be
 * ignored. Rather than accept an input and quietly disregard it, the first one
 * in relief order that they are not this moment pinning goes back to
 * automatic.
 *
 * Which inputs argue, how many may stand and which gives way first are facts
 * about the geometry, and they used to live in the panel as three functions,
 * one per stage type, restating a relation the core already enforces. **It is
 * the same relation the solve reads from the other end**, so the two have to
 * agree or a designer is offered an input the solve will disregard.
 *
 * Nothing here decides a value: relief says which inputs are still being
 * read, and a box it turns given holds what it was showing — a number the
 * core computed, copied where the designer would have copied it.
 *
 * # Errors
 *
 * A malformed stage or freedom, which would be a defect on this side of the
 * boundary.
 */
export function relieve_stage(input: string): string;

/**
 * Which shipped language a BCP 47 tag should be read in — `zh-TW` answers
 * `zh-Hant`, `de-CH` answers `de`, anything unknown answers `en`.
 *
 * Exposed so the picker can show the option actually in force, and so the
 * mapping from a browser's `navigator.language` lives beside the language list
 * rather than being written down a second time in TypeScript.
 */
export function resolve_language(tag: string): string;

/**
 * A ring's closed outline as flat `[x, y, x, y, ...]`, for the viewport.
 *
 * # Errors
 *
 * A malformed request.
 */
export function ring_profile(input: string, points_per_tooth: number): Float64Array;

/**
 * Derived geometry and metrology for one gear.
 */
export function solve_gear(input: string): string;

/**
 * The material library the tool ships with, as JSON.
 *
 * Includes each value's `basis` and `note`, because the UI is expected to show
 * which numbers are measured and which are estimates — see `docs/rationale.md#material-data-ships-estimates-deliberately`
 * . Dropping that on the floor would present a class estimate with the
 * same authority as a datasheet reading.
 * Derived geometry for one internal gear. JSON in, JSON out.
 *
 * # Errors
 *
 * A malformed request.
 */
export function solve_ring(input: string): string;

/**
 * Derived results for a whole geartrain.
 *
 * The third of the three entry points docs/rationale.md#the-stack planned. Like the others it
 * is JSON in, JSON out, with no state held across the boundary: the UI owns the
 * inputs and this recomputes everything from them on each change.
 */
export function solve_train(input: string): string;

/**
 * The string catalogue for a language tag, as JSON. See [`strings_impl`].
 *
 * An unknown tag answers with English rather than an error: a language
 * preference is not an engineering input, and a stale one stored in a browser
 * should leave a working application rather than a blank one.
 *
 * # Errors
 *
 * Only if the catalogue cannot be encoded, which would be a build-time defect.
 */
export function strings(language: string): string;

/**
 * **A train with one stage told what drives it and what it holds.**
 *
 * `{ train, stage, driven, held }` JSON in — the train as it stands, the
 * stage by index, and two of its ports by local shaft — and the train out
 * with that stage's constraints stated in full and, where the stage is in
 * the middle of a chain, the chain moved to enter at `driven`.
 *
 * Version of the core, so the UI can show what it is actually running.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly adopt_member: (a: number, b: number, c: number) => void;
    readonly default_materials: (a: number) => void;
    readonly defaults: (a: number) => void;
    readonly export_dxf: (a: number, b: number, c: number) => void;
    readonly export_materials: (a: number, b: number, c: number) => void;
    readonly export_ring_dxf: (a: number, b: number, c: number) => void;
    readonly export_train: (a: number, b: number, c: number) => void;
    readonly gear_profile: (a: number, b: number, c: number, d: number) => void;
    readonly import_materials: (a: number, b: number, c: number) => void;
    readonly import_train: (a: number, b: number, c: number) => void;
    readonly languages: (a: number) => void;
    readonly relieve_case: (a: number, b: number, c: number) => void;
    readonly relieve_stage: (a: number, b: number, c: number) => void;
    readonly resolve_language: (a: number, b: number, c: number) => void;
    readonly ring_profile: (a: number, b: number, c: number, d: number) => void;
    readonly solve_gear: (a: number, b: number, c: number) => void;
    readonly solve_ring: (a: number, b: number, c: number) => void;
    readonly solve_train: (a: number, b: number, c: number) => void;
    readonly strings: (a: number, b: number, c: number) => void;
    readonly version: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
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
