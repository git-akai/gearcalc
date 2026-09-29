<!-- **The one number box.** Every number a reader types goes through here, on
     both tabs, so there is one answer to "what does the box show" and one to
     "what reaches the core".

     The box shows what the state holds. The only text it keeps of its own is a
     draft: what the reader is typing while it does not parse, or while it
     parses to something the state does not hold and the box still has focus.
     A change of the value from outside — a change of kind, an edit, relief —
     drops the draft, so the box cannot go on showing a number the gear no
     longer has.

     It commits only a finite number that passes the whole-number check and
     the bound. Anything else keeps the last committed number and says why
     under the box. The complaint is derived from what the box shows against
     the bound as it stands now, so it follows a bound that moves either way.

     The note under the box is drawn here, because the complaint belongs in it.
     `note` is what the row says otherwise; omitted, and with nothing to
     complain of, the row draws no note at all. -->
<script lang="ts">
  import { untrack } from "svelte";
  import { outside, t, type Bound } from "./core";
  import FieldNote from "./FieldNote.svelte";
  import { notes } from "./notes";

  let {
    value,
    set = () => {},
    step,
    integer = false,
    bound = null,
    auto = false,
    inherited = false,
    note,
    warn,
    onchange,
  }: {
    /** What the state holds. `null` shows an empty box: nothing is known. */
    value: number | null;
    /** Where a committed number goes. */
    set?: (v: number) => void;
    step?: number | string;
    /** A count: a whole number in the range the wire's `u32` carries. */
    integer?: boolean;
    /** The core's bound for this input, where it has one. */
    bound?: Bound | null;
    /** The value is the solve's, not the reader's: shown greyed and locked. */
    auto?: boolean;
    /** The value is a default the reader may override: shown greyed. */
    inherited?: boolean;
    note?: string | null;
    /** A finding about the committed value, shown when nothing is wrong
     *  with what is typed. */
    warn?: string | null;
    /** After a commit, for a row whose edit asks something more of the core. */
    onchange?: () => void;
  } = $props();

  /** The largest count the wire carries: counts cross as Rust's `u32`. */
  const U32_MAX = 2 ** 32 - 1;

  type Draft = { text: string; bad: boolean };
  let draft = $state<Draft | null>(null);
  let input: HTMLInputElement;

  /** The number a draft reads as, or why it reads as none. */
  function parse(d: Draft): number | string {
    // The browser empties `value` for text it cannot read as a number and
    // says so only in `badInput`; an empty box and an unreadable one differ.
    if (d.bad) return t("ui.validation_not_a_number");
    if (d.text.trim() === "") return t("ui.validation_required");
    const v = Number(d.text);
    return Number.isFinite(v) ? v : t("ui.validation_not_a_number");
  }

  /** Why a number is not acceptable here, or null. */
  function judge(v: number): string | null {
    if (integer && !(Number.isInteger(v) && v >= 0 && v <= U32_MAX))
      return t("ui.validation_not_a_whole_number");
    return bound ? outside(v, bound) : null;
  }

  const reading = $derived(draft === null ? value : parse(draft));
  const error = $derived(
    auto || reading === null ? null : typeof reading === "string" ? reading : judge(reading),
  );
  /** The text for the value. A locked figure is rounded for reading; a
   *  value the reader holds is shown whole, since it is what they hold. */
  const shown = $derived(
    value === null ? "" : auto ? String(Number(value.toFixed(4))) : String(value),
  );

  // A value changed from outside drops a draft that does not read as it.
  // Remembered outside the reactive graph: only a change is the question.
  let seen: number | null = untrack(() => value);
  $effect.pre(() => {
    const v = value;
    if (v === seen) return;
    seen = v;
    if (draft !== null && parse(draft) !== v) draft = null;
  });

  // The box shows the value whenever there is no draft. Written to the
  // element rather than bound, so a draft the browser holds as unreadable is
  // never overwritten by the empty string it reports for it.
  $effect(() => {
    if (draft === null && input.value !== shown) input.value = shown;
  });

  function oninput(e: Event & { currentTarget: HTMLInputElement }) {
    const el = e.currentTarget;
    draft = { text: el.value, bad: el.validity.badInput };
    const r = parse(draft);
    if (typeof r === "number" && judge(r) === null && r !== value) {
      set(r);
      onchange?.();
    }
  }

  /** Leaving the box keeps a draft only while it does not read as a number:
   *  a number the box refused goes back to the one the state holds. */
  function onblur() {
    if (draft !== null && typeof parse(draft) === "number") draft = null;
  }
</script>

<input
  bind:this={input}
  type="number"
  inputmode={integer ? "numeric" : "decimal"}
  {step}
  disabled={auto}
  class:computed={auto || inherited}
  class:invalid={error !== null}
  aria-invalid={error !== null}
  {oninput}
  {onblur}
/>
{#if note !== undefined || warn !== undefined || error !== null}
  <FieldNote notes={notes(note ?? null, error ?? warn ?? null)} />
{/if}
