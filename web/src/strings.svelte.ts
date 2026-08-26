// The words, and nothing else.
//
// Per DESIGN.md §12 the catalogue comes from Rust — `gear_wasm::strings` — for
// the same reason the defaults do. This module only holds it and fills in the
// blanks.
//
// **It is `.svelte.ts` because the catalogue arrives late.** The core loads
// asynchronously and the sidebar draws before it finishes, so a plain module
// variable would hand those components the fallback and never tell them the
// real text had arrived. Holding it in `$state` makes every `t()` call a
// reactive read, and the labels fill themselves in when the core is ready.

/** The catalogue, keyed `section.key`. Empty until the core has loaded. */
let catalogue = $state<Record<string, string>>({});

/** Called once by `loadCore`. */
export function setCatalogue(messages: Record<string, string>) {
  catalogue = messages;
}

/** Look up a message, with `{name}` filled in from `values`.
 *
 *  A missing message renders as its key and a missing value leaves its
 *  placeholder standing, both deliberately: a half-translated catalogue should
 *  show a reader something they can report rather than swallow the sentence
 *  that was trying to warn them. `gear_io::strings` does the same. */
export function t(key: string, values: Record<string, string> = {}): string {
  const template = catalogue[key];
  if (template === undefined) return key;
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? values[name] : whole,
  );
}

/** A note the core wants read: what happened, and the values it happened with.
 *
 *  The core sends this rather than a sentence — see `gear_core::note` — because
 *  the words are a display decision and the numbers are not. Everything in
 *  `values` is **already formatted**: how many decimals a quantity deserves is
 *  a judgement about the quantity, made where the model is. */
export interface Note {
  key: string;
  values: Record<string, string>;
}

/** Render a note from the core. */
export function note(n: Note): string {
  return t(n.key, n.values);
}
