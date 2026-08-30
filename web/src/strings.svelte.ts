// The words, and nothing else.
//
// Per docs/corrections.md the catalogue comes from Rust — `gear_wasm::strings` — for
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

/** Called by `loadCore`, and again whenever the language changes. */
export function setCatalogue(messages: Record<string, string>) {
  catalogue = messages;
}

/** A language this build can be read in. The list comes from Rust — see
 *  `gear_wasm::languages` for why it is not written down here as well. */
export interface LanguageOption {
  code: string;
  /** The language's name in itself — what a reader looks for. */
  name: string;
  /** ...and in English, so a reader stranded in a script they cannot read has
   *  something they can recognise. */
  english: string;
}

// **Which words is the same kind of fact as the words**, and arrives at the same
// moment — from the core, after the first render. So the language lives here
// beside the catalogue and in `$state` for the same reason it does: held in a
// plain module variable the picker drew itself once, empty, and never again
// (`docs/corrections.md`).
let available = $state<LanguageOption[]>([]);
let current = $state("en");

/** The languages available. Empty until the core has loaded. */
export function languages(): LanguageOption[] {
  return available;
}

/** The language in force. */
export function language(): string {
  return current;
}

/** Called by `loadCore` with what the core ships. */
export function setLanguages(list: LanguageOption[]) {
  available = list;
}

/** Called by `setLanguage` with the **resolved** code, never a raw tag. */
export function setCurrentLanguage(code: string) {
  current = code;
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
