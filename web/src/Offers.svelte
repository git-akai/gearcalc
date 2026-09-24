<!-- **What can be done to the selection** — the one add menu, or a strip of
     verbs. Every entry is an offer of the core's (`offersAt`): an edit the
     graph admits at a piece, tried there already, with the refusal's key
     where it would be refused. A refused entry stays in its place, not
     pressable, and says why; every entry's dry run shows beside it on hover
     or focus (`previewEdit`) — what it would make and take, and what the
     headline path would come to — so nothing is pressed to find out.

     Which edits there are, and whether each is made, is the core's; this
     lists them and names them (`offers.ts`). -->
<script lang="ts">
  import {
    editTrain,
    note,
    offersAt,
    previewEdit,
    t,
    type Edit,
    type MaterialLibrary,
    type Offer,
    type Target,
    type Train,
  } from "./core";
  import { adds, destination, offerLabel, sections, type Names } from "./offers";

  interface Props {
    train: Train;
    /** The pieces offered at, each with the heading its entries go under. */
    at: { target: Target; heading: string }[];
    /** The one add menu, or the strip of verbs — moves, joins, holds and
     *  removals — for the pieces selected. */
    kind: "adds" | "verbs";
    names: Names;
    materials?: MaterialLibrary;
    /** Told of every edit made, with how many meshes the train had before
     *  it — where an add's first new mesh is. */
    made?: (edit: Edit, meshes: number) => void;
  }
  let { train, at, kind, names, materials, made }: Props = $props();

  /** Each piece's offers of this kind; a piece with none is left out. */
  const groups = $derived(
    at
      .map((g) => ({
        ...g,
        offers: offersAt(train, g.target).filter((o) => adds(o.edit) === (kind === "adds")),
      }))
      .filter((g) => g.offers.length > 0),
  );

  /** **Listed under a verb** rather than pressed as one: a join, which the
   *  body's *Join to…* lists by the body it goes to, and a gear's move,
   *  which its *Move … to…* lists by the body it goes on. */
  const listed = (target: Target, o: Offer): boolean =>
    "join" in o.edit || ("move" in o.edit && typeof target === "object" && "member" in target);
  const verbFor = (target: Target, first: Offer): string =>
    "join" in first.edit
      ? t("ui.train_join_to")
      : t("ui.train_move_to_of", { name: typeof target === "object" && "member" in target ? names.gear(target.member) : "" });

  /** The add menu open (`"menu"`), a verb's list open (its group's index),
   *  or nothing. */
  let open = $state<string | null>(null);
  const toggle = (key: string) => (open = open === key ? null : key);

  /** **The entry the reader is over**, and where its dry run is drawn — beside
   *  it where the window has room, under it where not. */
  let over = $state<{ edit: Edit; label: string; left: number; top: number; up: boolean } | null>(null);
  const preview = $derived(over === null ? null : previewEdit(train, { graph: over.edit }, materials));
  /** The dry run's width, as its style says. */
  const POP = 320;
  function show(e: Event, o: Offer, label: string) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const beside = r.right + 8 + POP <= window.innerWidth;
    const up = r.top > window.innerHeight * 0.6;
    over = {
      edit: o.edit,
      label,
      left: beside ? r.right + 8 : Math.max(8, Math.min(r.left, window.innerWidth - POP - 8)),
      top: up ? window.innerHeight - (beside ? r.bottom : r.top - 4) : beside ? r.top : r.bottom + 4,
      up,
    };
  }

  /** The refusal of an edit pressed, which the core said as it refused it —
   *  an entry marked refused is never sent, so this is one the offer did
   *  not foresee. */
  let refusal = $state<string | null>(null);
  function make(o: Offer) {
    if (o.refused !== null) return;
    const meshes = train.shape.meshes.length;
    refusal = editTrain(train, { graph: o.edit });
    if (refusal !== null) return;
    over = null;
    open = null;
    made?.(o.edit, meshes);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") {
      open = null;
      over = null;
    }
  }}
/>

<!-- An entry, named as its place in the menu leaves it to be — `label` —
     and said whole over its dry run, where nothing around it says the rest. -->
{#snippet entry(o: Offer, label: string, danger: boolean)}
  {@const whole = offerLabel(o, names)}
  <button
    type="button"
    class="action entry"
    class:danger
    class:refused={o.refused !== null}
    aria-disabled={o.refused !== null}
    aria-label={whole}
    onmouseenter={(e) => show(e, o, whole)}
    onfocus={(e) => show(e, o, whole)}
    onmouseleave={() => (over = null)}
    onblur={() => (over = null)}
    onclick={() => make(o)}>{label}</button
  >
{/snippet}

{#if kind === "adds"}
  <div class="adds">
    <button type="button" class="action" aria-expanded={open === "menu"} onclick={() => toggle("menu")}
      >{t("ui.train_add_menu")}</button
    >
    {#if open === "menu"}
      <div class="menu">
        <!-- **Headings group, entries tell apart**: the piece an add is
             offered at heads its group, what it adds heads a run within it,
             and an entry is named by what is left — a preset by its name
             under its family, a gear by where it goes or what it meshes. -->
        {#each groups as g, gi (gi)}
          <h5>{g.heading}</h5>
          {#each sections(g.offers, g.target, names) as sec, si (si)}
            <div class="run" class:headed={sec.heading !== null}>
              {#if sec.heading !== null}
                <h6>{sec.heading}</h6>
              {/if}
              {#each sec.entries as x, xi (xi)}
                {@render entry(x.offer, x.label, false)}
              {/each}
            </div>
          {/each}
        {:else}
          <p class="hint">{t("ui.train_offer_nothing")}</p>
        {/each}
      </div>
    {/if}
  </div>
{:else if groups.length > 0}
  <div class="verbs">
    {#each groups as g, gi (gi)}
      {@const under = g.offers.filter((o) => listed(g.target, o))}
      {#if under.length > 0}
        <button type="button" class="action" aria-expanded={open === String(gi)} onclick={() => toggle(String(gi))}
          >{verbFor(g.target, under[0])}</button
        >
      {/if}
      {#each g.offers.filter((o) => !listed(g.target, o)) as o, oi (oi)}
        {@render entry(o, offerLabel(o, names), "remove" in o.edit)}
      {/each}
    {/each}
  </div>
  {#each groups as g, gi (gi)}
    {#if open === String(gi)}
      <div class="destinations">
        {#each g.offers.filter((o) => listed(g.target, o)) as o, oi (oi)}
          {@render entry(o, destination(o.edit, names) ?? offerLabel(o, names), false)}
        {/each}
      </div>
    {/if}
  {/each}
{/if}
{#if refusal !== null}
  <p class="refusal">{t(refusal)}</p>
{/if}

<!-- **The dry run**: the core made the edit on a copy and solved both. -->
{#if over !== null && preview !== null}
  <div
    class="pop"
    role="status"
    style:left="{over.left}px"
    style:top={over.up ? undefined : `${over.top}px`}
    style:bottom={over.up ? `${over.top}px` : undefined}
  >
    <h6>{over.label}</h6>
    {#if preview.refused !== null}
      <p class="refused">{t("ui.train_offer_refused", { reason: note(preview.refused) })}</p>
    {:else}
      {#each preview.changes as n, i (i)}
        <p>{note(n)}</p>
      {/each}
      {#each preview.paths as n, i (i)}
        <p class="path">{note(n)}</p>
      {/each}
      {#if preview.unsolved !== null}
        <p class="refused">{t("ui.train_offer_unsolved", { reason: note(preview.unsolved) })}</p>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .adds {
    margin-top: 0.6rem;
  }
  .menu {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.15rem;
    margin-top: 0.4rem;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--rule);
    border-radius: 4px;
  }
  .menu h5 {
    margin: 0.4rem 0 0.1rem;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .menu h5:first-child {
    margin-top: 0;
  }
  .run {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.15rem;
  }
  .run h6 {
    margin: 0.3rem 0 0 0.4rem;
    font-size: 0.74rem;
    font-weight: 600;
    color: var(--muted);
  }
  .run.headed .entry {
    padding-left: 1.2rem;
  }
  .menu .entry {
    text-align: left;
    border-color: transparent;
    background: transparent;
  }
  .menu .entry:hover:not(.refused) {
    background: var(--hover);
  }
  .verbs,
  .destinations {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-bottom: 0.5rem;
  }
  .destinations {
    padding-left: 0.8rem;
  }
  .entry.refused {
    opacity: 0.5;
    cursor: default;
  }
  .refusal {
    color: var(--warn);
    font-size: 0.78rem;
  }
  .hint {
    color: var(--muted);
    font-size: 0.78rem;
    margin: 0;
  }
  .pop {
    position: fixed;
    z-index: 20;
    width: min(20rem, calc(100vw - 16px));
    max-height: 60vh;
    overflow: auto;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--bg);
    box-shadow: 0 3px 10px rgba(27, 31, 36, 0.12);
    padding: 0.55rem 0.65rem;
    font-size: 0.76rem;
    line-height: 1.45;
    pointer-events: none;
  }
  .pop h6 {
    margin: 0 0 0.25rem;
    font-size: 0.76rem;
  }
  .pop p {
    margin: 0;
  }
  .pop .path {
    margin-top: 0.25rem;
    padding-top: 0.25rem;
    border-top: 1px solid var(--rule);
  }
  .pop .refused {
    color: var(--warn);
  }
</style>
